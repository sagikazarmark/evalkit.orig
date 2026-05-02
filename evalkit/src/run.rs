use crate::{
    Budget, OutputSource, OutputSourceError, Dataset, MapError, Mapper, ProductionOutput, ResourceUsage,
    RunMetadata, RunResult, Sample, SampleResult, Score, ScoreDefinition, ScoreOutcome, ScoredEntry,
    Scorer, ScorerContext, ScorerError, ScorerSet, TrialResult,
};
use tokio_util::sync::CancellationToken;
use chrono::Utc;
use futures::{FutureExt, StreamExt, stream};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::marker::PhantomData;
use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::pin::Pin;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use uuid::Uuid;

type TrialScores = Vec<(ScoreDefinition, Result<ScoreOutcome, ScorerError>)>;
type TrialFuture<'a> = Pin<Box<dyn Future<Output = TrialScores> + 'a>>;
type OutputSourceFuture<'a, O> = Pin<Box<dyn Future<Output = Result<ProductionOutput<O>, OutputSourceError>> + 'a>>;

struct ExecutedTrial {
    result: TrialResult,
    scorer_resources: ResourceUsage,
    source_resources: ResourceUsage,
}

struct FlattenedTrial {
    scores: HashMap<String, ScoredEntry>,
    resources: ResourceUsage,
    source_metadata: HashMap<String, Value>,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum RunBuildError {
    EmptyDataset,
    DuplicateSampleIds(Vec<String>),
    DuplicateScorerNames(String),
    MissingSampleIds,
}

impl Display for RunBuildError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDataset => f.write_str("run dataset must contain at least one sample"),
            Self::DuplicateSampleIds(ids) => {
                write!(
                    f,
                    "run dataset contains duplicate sample ids: {}",
                    ids.join(", ")
                )
            }
            Self::DuplicateScorerNames(name) => {
                write!(f, "run contains duplicate scorer definition name `{name}`")
            }
            Self::MissingSampleIds => f.write_str("observe-mode runs require explicit sample ids"),
        }
    }
}

impl Error for RunBuildError {}

#[derive(Debug)]
#[non_exhaustive]
pub enum RunError {
    Build(RunBuildError),
    Internal(Box<dyn Error + Send + Sync>),
}

impl Display for RunError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Build(err) => write!(f, "run build failed: {err}"),
            Self::Internal(err) => write!(f, "internal run error: {err}"),
        }
    }
}

impl Error for RunError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Build(err) => Some(err),
            Self::Internal(err) => Some(err.as_ref()),
        }
    }
}

pub struct Run<I, O, R = ()> {
    dataset: Dataset<I, R>,
    source: Box<dyn ErasedOutputSource<I, O>>,
    definitions: Vec<ScoreDefinition>,
    executor: Box<dyn RunExecutor<I, O, R>>,
    trial_count: usize,
    concurrency: usize,
    sample_timeout: Option<Duration>,
    seed: Option<u64>,
    budget: Option<Budget>,
    code_commit: Option<String>,
    code_fingerprint: Option<String>,
    judge_model_pins: Vec<String>,
    source_mode: &'static str,
    cancel: CancellationToken,
}

impl Run<(), (), ()> {
    pub fn builder() -> RunBuilder {
        RunBuilder
    }
}

impl<I, O, R> Run<I, O, R> {
    pub async fn execute(&self) -> Result<RunResult, RunError> {
        let started_at = Utc::now();
        let started = Instant::now();
        let run_id = Uuid::new_v4().to_string();

        let samples = stream::iter(self.dataset.samples.iter())
            .map(|sample| self.execute_sample(&run_id, sample))
            .buffered(self.concurrency)
            .collect::<Vec<_>>()
            .await;

        let completed_at = Utc::now();
        let duration = started.elapsed();

        Ok(RunResult {
            metadata: RunMetadata {
                run_id,
                seed: self.seed,
                dataset_fingerprint: fingerprint_dataset(&self.dataset),
                scorer_fingerprint: fingerprint_definitions(&self.definitions),
                code_commit: self.code_commit.clone(),
                code_fingerprint: self.code_fingerprint.clone(),
                judge_model_pins: self.judge_model_pins.clone(),
                started_at,
                completed_at,
                duration,
                trial_count: self.trial_count,
                score_definitions: self.definitions.clone(),
                source_mode: self.source_mode.to_string(),
            },
            samples,
        })
    }

    async fn execute_sample(&self, run_id: &str, sample: &Sample<I, R>) -> SampleResult {
        let mut trials = Vec::with_capacity(self.trial_count);
        let mut scorer_resources = ResourceUsage::default();
        let mut source_resources = ResourceUsage::default();

        for trial_index in 0..self.trial_count {
            let trial = self.execute_trial(run_id, sample, trial_index).await;
            scorer_resources.merge(&trial.scorer_resources);
            source_resources.merge(&trial.source_resources);
            trials.push(trial.result);
        }

        let scored_count = trials
            .iter()
            .filter(|trial| trial.scores.values().any(|e| e.result.is_ok()))
            .count();

        let mut combined = source_resources.clone();
        combined.merge(&scorer_resources);

        SampleResult {
            sample_id: sample.id.clone(),
            trial_count: self.trial_count,
            error_count: self.trial_count - scored_count,
            scored_count,
            trials,
            token_usage: combined.token_usage,
            cost_usd: combined.cost_usd,
            source_resources,
            scorer_resources,
        }
    }

    async fn execute_trial(
        &self,
        run_id: &str,
        sample: &Sample<I, R>,
        trial_index: usize,
    ) -> ExecutedTrial {
        let started = Instant::now();

        match AssertUnwindSafe(self.produce_output(sample))
            .catch_unwind()
            .await
        {
            Ok(Ok(production)) => {
                let ProductionOutput {
                    output,
                    usage,
                    cost_usd,
                    latency,
                    metadata: source_metadata,
                } = production;

                let mut source_resources = ResourceUsage::default();
                if let Some(usage) = usage {
                    source_resources.token_usage = usage;
                }
                if let Some(cost) = cost_usd {
                    source_resources.cost_usd = Some(cost);
                }
                if let Some(latency) = latency {
                    source_resources.latency = Some(latency);
                }

                let previous_scores: HashMap<String, Score> = HashMap::new(); // populated by ScorerSet in Task 14
                let ctx = ScorerContext {
                    run_id,
                    sample_id: &sample.id,
                    trial_index,
                    seed: self.seed,
                    cancel: &self.cancel,
                    budget: self.budget.as_ref(),
                    previous_scores: &previous_scores,
                    metadata: &sample.metadata,
                    input: &sample.input,
                    output: &output,
                    reference: sample.reference.as_ref(),
                };

                match AssertUnwindSafe(self.executor.execute(&ctx))
                    .catch_unwind()
                    .await
                {
                    Ok(scores) => {
                        let mut flattened = flatten_scores(scores);
                        flattened.source_metadata = source_metadata;
                        ExecutedTrial {
                            result: TrialResult {
                                scores: flattened.scores,
                                duration: started.elapsed(),
                                trial_index,
                                source_metadata: flattened.source_metadata,
                            },
                            scorer_resources: flattened.resources,
                            source_resources,
                        }
                    }
                    Err(_) => ExecutedTrial {
                        result: TrialResult {
                            scores: scorer_panic_scores(&self.definitions),
                            duration: started.elapsed(),
                            trial_index,
                            source_metadata,
                        },
                        scorer_resources: ResourceUsage::default(),
                        source_resources,
                    },
                }
            }
            Ok(Err(err)) => ExecutedTrial {
                result: TrialResult {
                    scores: source_failure_scores(&self.definitions, err),
                    duration: started.elapsed(),
                    trial_index,
                    source_metadata: HashMap::new(),
                },
                scorer_resources: ResourceUsage::default(),
                source_resources: ResourceUsage::default(),
            },
            Err(payload) => ExecutedTrial {
                result: TrialResult {
                    scores: source_failure_scores(
                        &self.definitions,
                        OutputSourceError::Panicked(panic_message(payload)),
                    ),
                    duration: started.elapsed(),
                    trial_index,
                    source_metadata: HashMap::new(),
                },
                scorer_resources: ResourceUsage::default(),
                source_resources: ResourceUsage::default(),
            },
        }
    }

    async fn produce_output(
        &self,
        sample: &Sample<I, R>,
    ) -> Result<ProductionOutput<O>, OutputSourceError> {
        crate::source::with_current_sample_id(
            &sample.id,
            self.produce_output_inner(&sample.input),
        )
        .await
    }

    async fn produce_output_inner(
        &self,
        input: &I,
    ) -> Result<ProductionOutput<O>, OutputSourceError> {
        match self.sample_timeout {
            Some(duration) => {
                match timeout(duration, self.source.produce_boxed(input)).await {
                    Ok(result) => result,
                    Err(_) => Err(OutputSourceError::Timeout(duration)),
                }
            }
            None => self.source.produce_boxed(input).await,
        }
    }
}

pub struct RunBuilder;

impl RunBuilder {
    pub fn dataset<I, R, D>(self, dataset: D) -> RunBuilderWithDataset<I, R>
    where
        D: Into<Dataset<I, R>>,
    {
        RunBuilderWithDataset {
            dataset: dataset.into(),
        }
    }
}

pub struct RunBuilderWithDataset<I, R> {
    dataset: Dataset<I, R>,
}

impl<I, R> RunBuilderWithDataset<I, R> {
    pub fn source<O, S>(self, source: S) -> RunBuilderConfigured<I, O, R>
    where
        S: OutputSource<I, O> + 'static,
        O: 'static,
    {
        let source_mode = source.metadata_mode();

        RunBuilderConfigured::<I, O, R> {
            dataset: self.dataset,
            source: Box::new(source),
            output_mapper: None,
            reference_mapper: None,
            trial_count: 1,
            concurrency: 1,
            sample_timeout: None,
            seed: None,
            code_commit: None,
            code_fingerprint: None,
            judge_model_pins: Vec::new(),
            source_mode,
            _mapped: PhantomData,
        }
    }
}

pub struct RunBuilderConfigured<
    I,
    O,
    R,
    O2 = O,
    R2 = R,
    OutputState = Unmapped,
    ReferenceState = Unmapped,
> {
    dataset: Dataset<I, R>,
    source: Box<dyn ErasedOutputSource<I, O>>,
    output_mapper: Option<Box<dyn Mapper<O, O2>>>,
    reference_mapper: Option<Box<dyn Mapper<R, R2>>>,
    trial_count: usize,
    concurrency: usize,
    sample_timeout: Option<Duration>,
    seed: Option<u64>,
    code_commit: Option<String>,
    code_fingerprint: Option<String>,
    judge_model_pins: Vec<String>,
    source_mode: &'static str,
    _mapped: PhantomData<fn() -> (O2, R2, OutputState, ReferenceState)>,
}

pub struct RunBuilderWithTargets<
    I,
    O,
    R,
    O2 = O,
    R2 = R,
    OutputState = Unmapped,
    ReferenceState = Unmapped,
> {
    dataset: Dataset<I, R>,
    source: Box<dyn ErasedOutputSource<I, O>>,
    output_mapper: Option<Box<dyn Mapper<O, O2>>>,
    reference_mapper: Option<Box<dyn Mapper<R, R2>>>,
    targets: Vec<ScoringTarget<I, O2, R2>>,
    trial_count: usize,
    concurrency: usize,
    sample_timeout: Option<Duration>,
    seed: Option<u64>,
    budget: Option<Budget>,
    code_commit: Option<String>,
    code_fingerprint: Option<String>,
    judge_model_pins: Vec<String>,
    source_mode: &'static str,
    _mapped: PhantomData<fn() -> (O2, R2, OutputState, ReferenceState)>,
}

pub struct Unmapped;
pub struct Mapped;

impl<I, O, R, R2, ReferenceState> RunBuilderConfigured<I, O, R, O, R2, Unmapped, ReferenceState> {
    pub fn map_output<O3, M>(
        self,
        mapper: M,
    ) -> RunBuilderConfigured<I, O, R, O3, R2, Mapped, ReferenceState>
    where
        M: Mapper<O, O3> + 'static,
    {
        RunBuilderConfigured::<I, O, R, O3, R2, Mapped, ReferenceState> {
            dataset: self.dataset,
            source: self.source,
            output_mapper: Some(Box::new(mapper)),
            reference_mapper: self.reference_mapper,
            trial_count: self.trial_count,
            concurrency: self.concurrency,
            sample_timeout: self.sample_timeout,
            seed: self.seed,
            code_commit: self.code_commit,
            code_fingerprint: self.code_fingerprint,
            judge_model_pins: self.judge_model_pins,
            source_mode: self.source_mode,
            _mapped: PhantomData,
        }
    }
}

impl<I, O, R, O2, OutputState> RunBuilderConfigured<I, O, R, O2, R, OutputState, Unmapped> {
    pub fn map_reference<R3, M>(
        self,
        mapper: M,
    ) -> RunBuilderConfigured<I, O, R, O2, R3, OutputState, Mapped>
    where
        M: Mapper<R, R3> + 'static,
    {
        RunBuilderConfigured::<I, O, R, O2, R3, OutputState, Mapped> {
            dataset: self.dataset,
            source: self.source,
            output_mapper: self.output_mapper,
            reference_mapper: Some(Box::new(mapper)),
            trial_count: self.trial_count,
            concurrency: self.concurrency,
            sample_timeout: self.sample_timeout,
            seed: self.seed,
            code_commit: self.code_commit,
            code_fingerprint: self.code_fingerprint,
            judge_model_pins: self.judge_model_pins,
            source_mode: self.source_mode,
            _mapped: PhantomData,
        }
    }
}

impl<I, O, R, O2, R2, OutputState, ReferenceState>
    RunBuilderConfigured<I, O, R, O2, R2, OutputState, ReferenceState>
{
    pub fn scorer<S>(
        self,
        scorer: S,
    ) -> RunBuilderWithTargets<I, O, R, O2, R2, OutputState, ReferenceState>
    where
        S: Scorer<I, O2, R2> + 'static,
    {
        let target = ScoringTarget::from_scorer(scorer);
        let mut judge_model_pins = self.judge_model_pins;
        judge_model_pins.extend(target.judge_model_pins().iter().cloned());

        RunBuilderWithTargets {
            dataset: self.dataset,
            source: self.source,
            output_mapper: self.output_mapper,
            reference_mapper: self.reference_mapper,
            targets: vec![target],
            trial_count: self.trial_count,
            concurrency: self.concurrency,
            sample_timeout: self.sample_timeout,
            seed: self.seed,
            budget: None,
            code_commit: self.code_commit,
            code_fingerprint: self.code_fingerprint,
            judge_model_pins,
            source_mode: self.source_mode,
            _mapped: PhantomData,
        }
    }

    pub fn scorer_set(
        self,
        scorer_set: ScorerSet<I, O2, R2>,
    ) -> RunBuilderWithTargets<I, O, R, O2, R2, OutputState, ReferenceState>
    where
        I: 'static,
        O2: 'static,
        R2: 'static,
    {
        let target = ScoringTarget::from_scorer_set(scorer_set);
        let mut judge_model_pins = self.judge_model_pins;
        judge_model_pins.extend(target.judge_model_pins().iter().cloned());

        RunBuilderWithTargets {
            dataset: self.dataset,
            source: self.source,
            output_mapper: self.output_mapper,
            reference_mapper: self.reference_mapper,
            targets: vec![target],
            trial_count: self.trial_count,
            concurrency: self.concurrency,
            sample_timeout: self.sample_timeout,
            seed: self.seed,
            budget: None,
            code_commit: self.code_commit,
            code_fingerprint: self.code_fingerprint,
            judge_model_pins,
            source_mode: self.source_mode,
            _mapped: PhantomData,
        }
    }
}

impl<I, O, R, O2, R2, OutputState, ReferenceState>
    RunBuilderWithTargets<I, O, R, O2, R2, OutputState, ReferenceState>
{
    pub fn scorer<S>(mut self, scorer: S) -> Self
    where
        S: Scorer<I, O2, R2> + 'static,
    {
        let target = ScoringTarget::from_scorer(scorer);
        self.judge_model_pins
            .extend(target.judge_model_pins().iter().cloned());
        self.targets.push(target);
        self
    }

    pub fn scorer_set(mut self, scorer_set: ScorerSet<I, O2, R2>) -> Self
    where
        I: 'static,
        O2: 'static,
        R2: 'static,
    {
        let target = ScoringTarget::from_scorer_set(scorer_set);
        self.judge_model_pins
            .extend(target.judge_model_pins().iter().cloned());
        self.targets.push(target);
        self
    }

    pub fn trials(mut self, trial_count: usize) -> Self {
        self.trial_count = trial_count.max(1);
        self
    }

    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency.max(1);
        self
    }

    pub fn sample_timeout(mut self, sample_timeout: Duration) -> Self {
        self.sample_timeout = Some(sample_timeout);
        self
    }

    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    pub fn budget(mut self, budget: Budget) -> Self {
        self.budget = Some(budget);
        self
    }

    pub fn code_commit(mut self, code_commit: impl Into<String>) -> Self {
        self.code_commit = Some(code_commit.into());
        self
    }

    pub fn code_fingerprint(mut self, code_fingerprint: impl Into<String>) -> Self {
        self.code_fingerprint = Some(code_fingerprint.into());
        self
    }

    pub fn judge_model_pin(mut self, judge_model_pin: impl Into<String>) -> Self {
        self.judge_model_pins.push(judge_model_pin.into());
        self
    }

    pub fn judge_model_pins<P>(mut self, judge_model_pins: P) -> Self
    where
        P: IntoIterator,
        P::Item: Into<String>,
    {
        self.judge_model_pins
            .extend(judge_model_pins.into_iter().map(Into::into));
        self
    }

    fn definitions(&self) -> Vec<ScoreDefinition> {
        self.targets
            .iter()
            .flat_map(|target| target.definitions.iter().cloned())
            .collect()
    }

    fn validate(&self) -> Result<Vec<ScoreDefinition>, RunBuildError> {
        if self.dataset.samples.is_empty() {
            return Err(RunBuildError::EmptyDataset);
        }

        let mut seen_sample_ids = HashSet::new();
        let mut duplicate_sample_ids = Vec::new();

        for sample in &self.dataset.samples {
            if !seen_sample_ids.insert(sample.id.as_str())
                && !duplicate_sample_ids
                    .iter()
                    .any(|existing| existing == &sample.id)
            {
                duplicate_sample_ids.push(sample.id.clone());
            }
        }

        if !duplicate_sample_ids.is_empty() {
            duplicate_sample_ids.sort();
            return Err(RunBuildError::DuplicateSampleIds(duplicate_sample_ids));
        }

        if self.source_mode == "observe"
            && self
                .dataset
                .samples
                .iter()
                .any(|sample| looks_like_generated_sample_id(&sample.id))
        {
            return Err(RunBuildError::MissingSampleIds);
        }

        let definitions = self.definitions();
        let mut scorer_names = HashSet::new();

        for definition in &definitions {
            if !scorer_names.insert(definition.name.as_str()) {
                return Err(RunBuildError::DuplicateScorerNames(definition.name.clone()));
            }
        }

        Ok(definitions)
    }

    fn normalized_judge_model_pins(mut self) -> Self {
        self.judge_model_pins.sort();
        self.judge_model_pins.dedup();
        self
    }

    fn resolved_code_identity(mut self) -> Self {
        let detected = detect_code_identity_from_current_dir();

        if self.code_commit.is_none() {
            self.code_commit = detected.code_commit;
        }

        if self.code_fingerprint.is_none() {
            self.code_fingerprint = detected.code_fingerprint;
        }

        self
    }
}

fn looks_like_generated_sample_id(id: &str) -> bool {
    id.len() == 16
        && id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn fingerprint_dataset<I, R>(dataset: &Dataset<I, R>) -> String {
    let mut fingerprint = StableFingerprint::default();

    fingerprint.write_bytes(canonical_metadata_json(&dataset.metadata).as_bytes());

    for sample in &dataset.samples {
        fingerprint.write_bytes(sample.id.as_bytes());
        fingerprint.write_bytes(canonical_metadata_json(&sample.metadata).as_bytes());
    }

    fingerprint.finish_hex()
}

fn fingerprint_definitions(definitions: &[ScoreDefinition]) -> String {
    let mut entries = definitions
        .iter()
        .map(|definition| {
            format!(
                "{}:{}",
                definition.name,
                match definition.direction {
                    Some(crate::Direction::Maximize) => "maximize",
                    Some(crate::Direction::Minimize) => "minimize",
                    None => "none",
                }
            )
        })
        .collect::<Vec<_>>();
    entries.sort();

    let mut fingerprint = StableFingerprint::default();
    for entry in entries {
        fingerprint.write_bytes(entry.as_bytes());
    }

    fingerprint.finish_hex()
}

fn canonical_metadata_json(metadata: &HashMap<String, Value>) -> String {
    let mut entries = metadata.iter().collect::<Vec<_>>();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));

    let mut object = Map::new();
    for (key, value) in entries {
        object.insert(key.clone(), canonicalize_value(value));
    }

    serde_json::to_string(&Value::Object(object)).expect("metadata maps are always serializable")
}

fn canonicalize_value(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut entries = object.iter().collect::<Vec<_>>();
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));

            let mut canonical = Map::new();
            for (key, value) in entries {
                canonical.insert(key.clone(), canonicalize_value(value));
            }

            Value::Object(canonical)
        }
        Value::Array(values) => Value::Array(values.iter().map(canonicalize_value).collect()),
        _ => value.clone(),
    }
}

#[derive(Default)]
struct StableFingerprint {
    state: u64,
}

impl StableFingerprint {
    fn write_bytes(&mut self, bytes: &[u8]) {
        const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x0000_0100_0000_01b3;

        if self.state == 0 {
            self.state = OFFSET_BASIS;
        }

        for byte in bytes {
            self.state ^= u64::from(*byte);
            self.state = self.state.wrapping_mul(PRIME);
        }
    }

    fn finish_hex(&self) -> String {
        format!("{:016x}", self.state)
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct DetectedCodeIdentity {
    code_commit: Option<String>,
    code_fingerprint: Option<String>,
}

fn detect_code_identity_from_current_dir() -> DetectedCodeIdentity {
    std::env::current_dir()
        .ok()
        .map(|cwd| detect_code_identity(&cwd))
        .unwrap_or_default()
}

fn detect_code_identity(cwd: &Path) -> DetectedCodeIdentity {
    let code_commit = git_stdout(cwd, &["rev-parse", "--verify", "HEAD"]);
    let tree = git_stdout(cwd, &["rev-parse", "--verify", "HEAD^{tree}"]);
    let diff = git_stdout_bytes(cwd, &["diff", "--binary", "--no-ext-diff", "HEAD", "--"])
        .unwrap_or_default();
    let untracked = git_stdout_bytes(cwd, &["ls-files", "--others", "--exclude-standard", "-z"])
        .unwrap_or_default();
    let code_fingerprint = fingerprint_git_state(cwd, tree.as_deref(), &diff, &untracked);

    DetectedCodeIdentity {
        code_commit,
        code_fingerprint,
    }
}

fn fingerprint_git_state(
    cwd: &Path,
    tree: Option<&str>,
    diff: &[u8],
    untracked: &[u8],
) -> Option<String> {
    let mut has_untracked = false;
    let mut dirty = StableFingerprint::default();

    if !diff.is_empty() {
        dirty.write_bytes(diff);
    }

    for path in untracked
        .split(|byte| *byte == b'\0')
        .filter(|entry| !entry.is_empty())
    {
        has_untracked = true;
        dirty.write_bytes(path);

        let relative_path = String::from_utf8_lossy(path);
        let absolute_path = cwd.join(relative_path.as_ref());
        let contents = std::fs::read(&absolute_path).ok()?;
        dirty.write_bytes(&contents);
    }

    if diff.is_empty() && !has_untracked {
        return tree.map(|tree| format!("tree:{tree}"));
    }

    let dirty_hash = dirty.finish_hex();
    Some(match tree {
        Some(tree) => format!("tree:{tree}+dirty:{dirty_hash}"),
        None => format!("dirty:{dirty_hash}"),
    })
}

fn git_stdout(cwd: &Path, args: &[&str]) -> Option<String> {
    let output = git_stdout_bytes(cwd, args)?;
    let text = String::from_utf8(output).ok()?;
    let trimmed = text.trim();

    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn git_stdout_bytes(cwd: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .ok()?;

    if output.status.success() {
        Some(output.stdout)
    } else {
        None
    }
}

impl<I: 'static, O: 'static, R: 'static, O2: 'static, R2: 'static, OS, RS>
    RunBuilderWithTargets<I, O, R, O2, R2, OS, RS>
{
    pub fn build(self) -> Result<Run<I, O, R>, RunBuildError> {
        let this = self.resolved_code_identity().normalized_judge_model_pins();
        let definitions = this.validate()?;

        Ok(Run {
            dataset: this.dataset,
            source: this.source,
            definitions,
            executor: Box::new(MappedRunExecutor {
                output_mapper: this.output_mapper,
                reference_mapper: this.reference_mapper,
                targets: this.targets,
            }),
            trial_count: this.trial_count,
            concurrency: this.concurrency,
            sample_timeout: this.sample_timeout,
            seed: this.seed,
            budget: this.budget,
            code_commit: this.code_commit,
            code_fingerprint: this.code_fingerprint,
            judge_model_pins: this.judge_model_pins,
            source_mode: this.source_mode,
            cancel: CancellationToken::new(),
        })
    }
}

trait ErasedOutputSource<I, O>: Send + Sync {
    fn produce_boxed<'a>(&'a self, input: &'a I) -> OutputSourceFuture<'a, O>;
}

impl<I, O, S> ErasedOutputSource<I, O> for S
where
    S: OutputSource<I, O> + Send + Sync,
    O: 'static,
{
    fn produce_boxed<'a>(&'a self, input: &'a I) -> OutputSourceFuture<'a, O> {
        Box::pin(async move { self.produce(input).await })
    }
}

#[cfg(test)]
mod tests {
    use super::{detect_code_identity, git_stdout};
    use crate::{
        Dataset, OutputSourceError, Run, Sample, Score, ScoreDefinition, Scorer, ScorerContext,
        ScorerError,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct ContainsScorer;

    impl Scorer<String, String> for ContainsScorer {
        async fn score(
            &self,
            ctx: &ScorerContext<'_, String, String>,
        ) -> Result<Score, ScorerError> {
            Ok(Score::Binary(ctx.output.contains(ctx.input.as_str())))
        }

        fn definition(&self) -> ScoreDefinition {
            ScoreDefinition::maximize("contains")
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mapper_executor_handles_no_mappers() {
        let dataset = Dataset::new(vec![
            Sample::builder("x".to_string()).id("s1").build().unwrap(),
        ]);
        let run = Run::builder()
            .dataset(dataset)
            .source(|input: &String| {
                let input = input.clone();
                async move { Ok::<_, OutputSourceError>(input) }
            })
            .scorer(ContainsScorer)
            .build()
            .unwrap();
        let result = run.execute().await.unwrap();
        assert_eq!(result.samples.len(), 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mapper_executor_applies_output_mapper() {
        use crate::Mapper;
        struct ToLen;
        impl Mapper<String, usize> for ToLen {
            fn map(&self, input: &String) -> Result<usize, crate::MapError> {
                Ok(input.len())
            }
        }
        struct LenScorer;
        impl Scorer<String, usize, String> for LenScorer {
            async fn score(
                &self,
                ctx: &ScorerContext<'_, String, usize, String>,
            ) -> Result<Score, ScorerError> {
                Ok(Score::Numeric(*ctx.output as f64))
            }
            fn definition(&self) -> ScoreDefinition {
                ScoreDefinition::new("len")
            }
        }

        let dataset = Dataset::new(vec![
            Sample::builder("hello".to_string())
                .id("s1")
                .reference(String::new())
                .build()
                .unwrap(),
        ]);
        let run = Run::builder()
            .dataset(dataset)
            .source(|input: &String| {
                let input = input.clone();
                async move { Ok::<_, OutputSourceError>(input) }
            })
            .map_output(ToLen)
            .scorer(LenScorer)
            .build()
            .unwrap();
        let result = run.execute().await.unwrap();
        let entry = &result.samples[0].trials[0].scores["len"];
        assert!(matches!(&entry.result, Ok(Score::Numeric(v)) if (v - 5.0).abs() < f64::EPSILON));
    }

    #[test]
    fn detect_code_identity_uses_head_and_tree_for_clean_repos() {
        let repo = temp_git_repo("clean");
        fs::write(repo.join("tracked.txt"), "clean state\n").expect("write tracked file");
        git(&repo, &["add", "tracked.txt"]);
        git(
            &repo,
            &[
                "-c",
                "user.name=Evalkit Tests",
                "-c",
                "user.email=tests@example.com",
                "commit",
                "--quiet",
                "-m",
                "initial",
            ],
        );

        let detected = detect_code_identity(&repo);
        let commit = git_stdout(&repo, &["rev-parse", "--verify", "HEAD"]).expect("head sha");
        let tree = git_stdout(&repo, &["rev-parse", "--verify", "HEAD^{tree}"]).expect("tree sha");

        assert_eq!(detected.code_commit.as_deref(), Some(commit.as_str()));
        assert_eq!(
            detected.code_fingerprint.as_deref(),
            Some(format!("tree:{tree}").as_str())
        );

        fs::remove_dir_all(repo).expect("remove temp repo");
    }

    #[test]
    fn detect_code_identity_hashes_dirty_changes_against_the_head_tree() {
        let repo = temp_git_repo("dirty");
        fs::write(repo.join("tracked.txt"), "clean state\n").expect("write tracked file");
        git(&repo, &["add", "tracked.txt"]);
        git(
            &repo,
            &[
                "-c",
                "user.name=Evalkit Tests",
                "-c",
                "user.email=tests@example.com",
                "commit",
                "--quiet",
                "-m",
                "initial",
            ],
        );

        let clean = detect_code_identity(&repo);
        fs::write(repo.join("tracked.txt"), "dirty state\n").expect("mutate tracked file");
        fs::write(repo.join("notes.txt"), "untracked\n").expect("write untracked file");

        let dirty = detect_code_identity(&repo);
        let tree = git_stdout(&repo, &["rev-parse", "--verify", "HEAD^{tree}"]).expect("tree sha");
        let dirty_fingerprint = dirty
            .code_fingerprint
            .as_deref()
            .expect("dirty fingerprint should exist");

        assert_eq!(dirty.code_commit, clean.code_commit);
        assert_ne!(dirty.code_fingerprint, clean.code_fingerprint);
        assert!(dirty_fingerprint.starts_with(&format!("tree:{tree}+dirty:")));

        fs::remove_dir_all(repo).expect("remove temp repo");
    }

    fn temp_git_repo(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after unix epoch")
            .as_nanos();
        let repo = std::env::temp_dir().join(format!(
            "evalkit-run-tests-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&repo).expect("create temp repo");
        git(&repo, &["init", "--quiet"]);
        repo
    }

    fn git(repo: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .status()
            .expect("git command should run");

        assert!(status.success(), "git command failed: {:?}", args);
    }
}

trait RunExecutor<I, O, R>: Send + Sync {
    fn execute<'a>(&'a self, ctx: &'a ScorerContext<'a, I, O, R>) -> TrialFuture<'a>;
}

struct MappedRunExecutor<I, O, R, O2, R2> {
    output_mapper: Option<Box<dyn Mapper<O, O2>>>,
    reference_mapper: Option<Box<dyn Mapper<R, R2>>>,
    targets: Vec<ScoringTarget<I, O2, R2>>,
}

impl<I, O, R, O2, R2> RunExecutor<I, O, R> for MappedRunExecutor<I, O, R, O2, R2>
where
    O: 'static,
    R: 'static,
    O2: 'static,
    R2: 'static,
{
    fn execute<'a>(&'a self, ctx: &'a ScorerContext<'a, I, O, R>) -> TrialFuture<'a> {
        Box::pin(async move {
            // Output mapping: apply the mapper if present, otherwise reinterpret in place.
            let mapped_output_storage;
            let mapped_output: &O2 = match &self.output_mapper {
                Some(mapper) => match mapper.map(ctx.output) {
                    Ok(value) => {
                        mapped_output_storage = value;
                        &mapped_output_storage
                    }
                    Err(err) => return map_failure_results(&self.targets, err),
                },
                None => {
                    // SAFETY: When no output mapper is present, `O2 == O` is enforced by the
                    // `RunBuilder` type-state machine: the `Unmapped` marker on `OutputState`
                    // causes the builder to fix `O2 = O` (see `RunBuilderWithTargets` default
                    // type parameter `O2 = O`). The cast is therefore a no-op reinterpretation
                    // of the same memory layout. The `MappedRunExecutor` is only ever
                    // constructed from a `build()` call, which preserves this invariant.
                    unsafe { &*(ctx.output as *const O as *const O2) }
                }
            };

            // Reference mapping: same pattern.
            let mapped_reference_storage;
            let mapped_reference: Option<&R2> = match (&self.reference_mapper, ctx.reference) {
                (Some(mapper), Some(reference)) => match mapper.map(reference) {
                    Ok(value) => {
                        mapped_reference_storage = value;
                        Some(&mapped_reference_storage)
                    }
                    Err(err) => return map_failure_results(&self.targets, err),
                },
                (None, Some(reference)) => {
                    // SAFETY: When no reference mapper is present, `R2 == R` is enforced by the
                    // `RunBuilder` type-state machine: the `Unmapped` marker on `ReferenceState`
                    // causes the builder to fix `R2 = R` (see `RunBuilderWithTargets` default
                    // type parameter `R2 = R`). Same invariant as the output cast above.
                    Some(unsafe { &*(reference as *const R as *const R2) })
                }
                (_, None) => None,
            };

            let mapped_ctx = ScorerContext {
                run_id: ctx.run_id,
                sample_id: ctx.sample_id,
                trial_index: ctx.trial_index,
                seed: ctx.seed,
                cancel: ctx.cancel,
                budget: ctx.budget,
                previous_scores: ctx.previous_scores,
                metadata: ctx.metadata,
                input: ctx.input,
                output: mapped_output,
                reference: mapped_reference,
            };

            execute_targets(&self.targets, &mapped_ctx).await
        })
    }
}

struct ScoringTarget<I, O, R> {
    definitions: Vec<ScoreDefinition>,
    judge_model_pins: Vec<String>,
    executor: Box<dyn TargetExecutor<I, O, R>>,
}

impl<I, O, R> ScoringTarget<I, O, R> {
    fn from_scorer<S>(scorer: S) -> Self
    where
        S: Scorer<I, O, R> + 'static,
    {
        let definition = scorer.definition();
        let mut judge_model_pins = scorer.metadata().judge_model_pins;
        judge_model_pins.sort();
        judge_model_pins.dedup();
        Self {
            definitions: vec![definition.clone()],
            judge_model_pins,
            executor: Box::new(SingleScorerTarget { scorer, definition }),
        }
    }

    fn from_scorer_set(scorer_set: ScorerSet<I, O, R>) -> Self
    where
        I: 'static,
        O: 'static,
        R: 'static,
    {
        Self {
            definitions: scorer_set.definitions().to_vec(),
            judge_model_pins: scorer_set.judge_model_pins().to_vec(),
            executor: Box::new(ScorerSetTarget { scorer_set }),
        }
    }

    fn judge_model_pins(&self) -> &[String] {
        &self.judge_model_pins
    }
}

trait TargetExecutor<I, O, R>: Send + Sync {
    fn execute<'a>(&'a self, ctx: &'a ScorerContext<'a, I, O, R>) -> TrialFuture<'a>;
}

struct SingleScorerTarget<S> {
    scorer: S,
    definition: ScoreDefinition,
}

impl<I, O, R, S> TargetExecutor<I, O, R> for SingleScorerTarget<S>
where
    S: Scorer<I, O, R> + Send + Sync,
{
    fn execute<'a>(&'a self, ctx: &'a ScorerContext<'a, I, O, R>) -> TrialFuture<'a> {
        Box::pin(async move {
            vec![(
                self.definition.clone(),
                self.scorer.score_with_resources(ctx).await,
            )]
        })
    }
}

struct ScorerSetTarget<I, O, R> {
    scorer_set: ScorerSet<I, O, R>,
}

impl<I, O, R> TargetExecutor<I, O, R> for ScorerSetTarget<I, O, R> {
    fn execute<'a>(&'a self, ctx: &'a ScorerContext<'a, I, O, R>) -> TrialFuture<'a> {
        Box::pin(self.scorer_set.score(ctx))
    }
}

async fn execute_targets<I, O, R>(
    targets: &[ScoringTarget<I, O, R>],
    ctx: &ScorerContext<'_, I, O, R>,
) -> TrialScores {
    let mut results = Vec::new();

    for target in targets {
        results.extend(target.executor.execute(ctx).await);
    }

    results
}

fn flatten_scores(results: TrialScores) -> FlattenedTrial {
    let mut scores = HashMap::with_capacity(results.len());
    let mut resources = ResourceUsage::default();

    for (definition, result) in results {
        let entry = match result {
            Ok(outcome) => {
                resources.merge(&outcome.resources);
                ScoredEntry {
                    result: validate_score(outcome.score),
                    reasoning: outcome.reasoning,
                    metadata: outcome.metadata,
                }
            }
            Err(err) => ScoredEntry {
                result: Err(err),
                reasoning: None,
                metadata: HashMap::new(),
            },
        };

        scores.insert(definition.name, entry);
    }

    FlattenedTrial {
        scores,
        resources,
        source_metadata: HashMap::new(),
    }
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        return (*s).to_string();
    }
    if let Some(s) = payload.downcast_ref::<String>() {
        return s.clone();
    }
    "<non-string panic payload>".to_string()
}

fn scorer_panic_scores(
    definitions: &[ScoreDefinition],
) -> HashMap<String, ScoredEntry> {
    definitions
        .iter()
        .map(|definition| {
            (
                definition.name.clone(),
                ScoredEntry {
                    result: Err(ScorerError::internal(ScorerPanicError)),
                    reasoning: None,
                    metadata: HashMap::new(),
                },
            )
        })
        .collect()
}

fn source_failure_scores(
    definitions: &[ScoreDefinition],
    err: OutputSourceError,
) -> HashMap<String, ScoredEntry> {
    let shared_err = Arc::new(err);

    definitions
        .iter()
        .map(|definition| {
            (
                definition.name.clone(),
                ScoredEntry {
                    result: Err(ScorerError::provider(SharedOutputSourceError(Arc::clone(
                        &shared_err,
                    )))),
                    reasoning: None,
                    metadata: HashMap::new(),
                },
            )
        })
        .collect()
}

fn map_failure_results<I, O, R>(targets: &[ScoringTarget<I, O, R>], err: MapError) -> TrialScores {
    let shared_err: Arc<dyn Error + Send + Sync> = Arc::from(err.0);

    targets
        .iter()
        .flat_map(|target| {
            target.definitions.iter().cloned().map({
                let shared_err = Arc::clone(&shared_err);
                move |definition| {
                    (
                        definition,
                        Err(ScorerError::invalid_input(SharedMapError(Arc::clone(
                            &shared_err,
                        )))),
                    )
                }
            })
        })
        .collect()
}

fn validate_score(score: Score) -> Result<Score, ScorerError> {
    match score {
        Score::Numeric(value) if !value.is_finite() => Err(invalid_score_error(
            "numeric scores must be finite (not NaN or infinity)",
        )),
        Score::Label(label) if label.is_empty() => {
            Err(invalid_score_error("label scores must not be empty"))
        }
        Score::Structured { score, .. } if !score.is_finite() => Err(invalid_score_error(
            "structured scores must have a finite score value (not NaN or infinity)",
        )),
        Score::Metric { name, .. } if name.is_empty() => Err(invalid_score_error(
            "metric scores must have a non-empty name",
        )),
        Score::Metric { value, .. } if !value.is_finite() => Err(invalid_score_error(
            "metric scores must have a finite value (not NaN or infinity)",
        )),
        _ => Ok(score),
    }
}

fn invalid_score_error(message: &'static str) -> ScorerError {
    ScorerError::invalid_input(InvalidScoreError(message))
}

#[derive(Debug)]
struct ScorerPanicError;

impl Display for ScorerPanicError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("scorer panicked")
    }
}

impl Error for ScorerPanicError {}

#[derive(Debug)]
struct InvalidScoreError(&'static str);

impl Display for InvalidScoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl Error for InvalidScoreError {}

#[derive(Debug)]
struct SharedOutputSourceError(Arc<OutputSourceError>);

impl Display for SharedOutputSourceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Error for SharedOutputSourceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}

#[derive(Debug)]
struct SharedMapError(Arc<dyn Error + Send + Sync>);

impl Display for SharedMapError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Error for SharedMapError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.0.as_ref())
    }
}
