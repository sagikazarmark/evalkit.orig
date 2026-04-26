use crate::{
    OutputSource, OutputSourceError, Dataset, MapError, Mapper, RunMetadata, RunResult, Sample,
    SampleResult, Score, ScoreDefinition, ScoreOutcome, Scorer, ScorerContext, ScorerError,
    ScorerResources, ScorerSet, TrialResult,
};
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
type OutputSourceFuture<'a, O> = Pin<Box<dyn Future<Output = Result<O, OutputSourceError>> + 'a>>;

struct ExecutedTrial {
    result: TrialResult,
    resources: ScorerResources,
}

struct FlattenedTrial {
    scores: HashMap<String, Result<Score, ScorerError>>,
    resources: ScorerResources,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum RunBuildError {
    NoDataset,
    NoSource,
    NoScorer,
    EmptyDataset,
    DuplicateSampleIds(Vec<String>),
    DuplicateScorerNames(String),
    MissingSampleIds,
}

impl Display for RunBuildError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDataset => f.write_str("run is missing a dataset"),
            Self::NoSource => f.write_str("run is missing an output source"),
            Self::NoScorer => f.write_str("run is missing a scorer or scorer set"),
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
    code_commit: Option<String>,
    code_fingerprint: Option<String>,
    judge_model_pins: Vec<String>,
    source_mode: &'static str,
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
        let mut resources = ScorerResources::default();

        for trial_index in 0..self.trial_count {
            let trial = self.execute_trial(run_id, sample, trial_index).await;
            resources.merge(&trial.resources);
            trials.push(trial.result);
        }

        let scored_count = trials
            .iter()
            .filter(|trial| trial.scores.values().any(Result::is_ok))
            .count();

        SampleResult {
            sample_id: sample.id.clone(),
            trial_count: self.trial_count,
            error_count: self.trial_count - scored_count,
            scored_count,
            trials,
            token_usage: resources.token_usage,
            cost_usd: resources.cost_usd,
        }
    }

    async fn execute_trial(
        &self,
        run_id: &str,
        sample: &Sample<I, R>,
        trial_index: usize,
    ) -> ExecutedTrial {
        let started = Instant::now();

        let flattened = match AssertUnwindSafe(self.produce_output(sample))
            .catch_unwind()
            .await
        {
            Ok(Ok(output)) => {
                let ctx = ScorerContext {
                    run_id,
                    sample_id: &sample.id,
                    trial_index,
                    metadata: &sample.metadata,
                    input: &sample.input,
                    output: &output,
                    reference: sample.reference.as_ref(),
                };

                match AssertUnwindSafe(self.executor.execute(&ctx))
                    .catch_unwind()
                    .await
                {
                    Ok(scores) => flatten_scores(scores),
                    Err(_) => FlattenedTrial {
                        scores: scorer_panic_scores(&self.definitions),
                        resources: ScorerResources::default(),
                    },
                }
            }
            Ok(Err(err)) => FlattenedTrial {
                scores: source_failure_scores(&self.definitions, err),
                resources: ScorerResources::default(),
            },
            Err(_) => FlattenedTrial {
                scores: source_failure_scores(&self.definitions, OutputSourceError::Panicked),
                resources: ScorerResources::default(),
            },
        };

        ExecutedTrial {
            result: TrialResult {
                scores: flattened.scores,
                duration: started.elapsed(),
                trial_index,
            },
            resources: flattened.resources,
        }
    }

    async fn produce_output(&self, sample: &Sample<I, R>) -> Result<O, OutputSourceError> {
        crate::source::with_current_sample_id(
            &sample.id,
            self.produce_output_inner(&sample.input),
        )
        .await
    }

    async fn produce_output_inner(&self, input: &I) -> Result<O, OutputSourceError> {
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
        let source_mode = source.metadata().mode;

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

impl<I: 'static, O: 'static, R: 'static> RunBuilderWithTargets<I, O, R, O, R, Unmapped, Unmapped> {
    pub fn build(self) -> Result<Run<I, O, R>, RunBuildError> {
        let this = self.resolved_code_identity().normalized_judge_model_pins();
        let definitions = this.validate()?;

        Ok(Run {
            dataset: this.dataset,
            source: this.source,
            definitions,
            executor: Box::new(RawRunExecutor {
                targets: this.targets,
            }),
            trial_count: this.trial_count,
            concurrency: this.concurrency,
            sample_timeout: this.sample_timeout,
            seed: this.seed,
            code_commit: this.code_commit,
            code_fingerprint: this.code_fingerprint,
            judge_model_pins: this.judge_model_pins,
            source_mode: this.source_mode,
        })
    }
}

impl<I: 'static, O: 'static, R: 'static, O2: 'static>
    RunBuilderWithTargets<I, O, R, O2, R, Mapped, Unmapped>
{
    pub fn build(self) -> Result<Run<I, O, R>, RunBuildError> {
        let this = self.resolved_code_identity().normalized_judge_model_pins();
        let definitions = this.validate()?;
        let output_mapper = this
            .output_mapper
            .expect("global output mapper must exist for mapped runs");

        Ok(Run {
            dataset: this.dataset,
            source: this.source,
            definitions,
            executor: Box::new(OutputMappedRunExecutor {
                output_mapper,
                targets: this.targets,
            }),
            trial_count: this.trial_count,
            concurrency: this.concurrency,
            sample_timeout: this.sample_timeout,
            seed: this.seed,
            code_commit: this.code_commit,
            code_fingerprint: this.code_fingerprint,
            judge_model_pins: this.judge_model_pins,
            source_mode: this.source_mode,
        })
    }
}

impl<I: 'static, O: 'static, R: 'static, R2: 'static>
    RunBuilderWithTargets<I, O, R, O, R2, Unmapped, Mapped>
{
    pub fn build(self) -> Result<Run<I, O, R>, RunBuildError> {
        let this = self.resolved_code_identity().normalized_judge_model_pins();
        let definitions = this.validate()?;
        let reference_mapper = this
            .reference_mapper
            .expect("global reference mapper must exist for mapped runs");

        Ok(Run {
            dataset: this.dataset,
            source: this.source,
            definitions,
            executor: Box::new(ReferenceMappedRunExecutor {
                reference_mapper,
                targets: this.targets,
            }),
            trial_count: this.trial_count,
            concurrency: this.concurrency,
            sample_timeout: this.sample_timeout,
            seed: this.seed,
            code_commit: this.code_commit,
            code_fingerprint: this.code_fingerprint,
            judge_model_pins: this.judge_model_pins,
            source_mode: this.source_mode,
        })
    }
}

impl<I: 'static, O: 'static, R: 'static, O2: 'static, R2: 'static>
    RunBuilderWithTargets<I, O, R, O2, R2, Mapped, Mapped>
{
    pub fn build(self) -> Result<Run<I, O, R>, RunBuildError> {
        let this = self.resolved_code_identity().normalized_judge_model_pins();
        let definitions = this.validate()?;
        let output_mapper = this
            .output_mapper
            .expect("global output mapper must exist for mapped runs");
        let reference_mapper = this
            .reference_mapper
            .expect("global reference mapper must exist for mapped runs");

        Ok(Run {
            dataset: this.dataset,
            source: this.source,
            definitions,
            executor: Box::new(FullyMappedRunExecutor {
                output_mapper,
                reference_mapper,
                targets: this.targets,
            }),
            trial_count: this.trial_count,
            concurrency: this.concurrency,
            sample_timeout: this.sample_timeout,
            seed: this.seed,
            code_commit: this.code_commit,
            code_fingerprint: this.code_fingerprint,
            judge_model_pins: this.judge_model_pins,
            source_mode: this.source_mode,
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
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

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

struct RawRunExecutor<I, O, R> {
    targets: Vec<ScoringTarget<I, O, R>>,
}

impl<I, O, R> RunExecutor<I, O, R> for RawRunExecutor<I, O, R> {
    fn execute<'a>(&'a self, ctx: &'a ScorerContext<'a, I, O, R>) -> TrialFuture<'a> {
        Box::pin(execute_targets(&self.targets, ctx))
    }
}

struct OutputMappedRunExecutor<I, O, R, O2> {
    output_mapper: Box<dyn Mapper<O, O2>>,
    targets: Vec<ScoringTarget<I, O2, R>>,
}

impl<I, O, R, O2> RunExecutor<I, O, R> for OutputMappedRunExecutor<I, O, R, O2> {
    fn execute<'a>(&'a self, ctx: &'a ScorerContext<'a, I, O, R>) -> TrialFuture<'a> {
        Box::pin(async move {
            let mapped_output = match self.output_mapper.map(ctx.output) {
                Ok(mapped_output) => mapped_output,
                Err(err) => return map_failure_results(&self.targets, err),
            };
            let mapped_ctx = ScorerContext {
                run_id: ctx.run_id,
                sample_id: ctx.sample_id,
                trial_index: ctx.trial_index,
                metadata: ctx.metadata,
                input: ctx.input,
                output: &mapped_output,
                reference: ctx.reference,
            };

            execute_targets(&self.targets, &mapped_ctx).await
        })
    }
}

struct ReferenceMappedRunExecutor<I, O, R, R2> {
    reference_mapper: Box<dyn Mapper<R, R2>>,
    targets: Vec<ScoringTarget<I, O, R2>>,
}

impl<I, O, R, R2> RunExecutor<I, O, R> for ReferenceMappedRunExecutor<I, O, R, R2> {
    fn execute<'a>(&'a self, ctx: &'a ScorerContext<'a, I, O, R>) -> TrialFuture<'a> {
        Box::pin(async move {
            let mapped_reference = match ctx.reference {
                Some(reference) => match self.reference_mapper.map(reference) {
                    Ok(mapped_reference) => Some(mapped_reference),
                    Err(err) => return map_failure_results(&self.targets, err),
                },
                None => None,
            };
            let mapped_ctx = ScorerContext {
                run_id: ctx.run_id,
                sample_id: ctx.sample_id,
                trial_index: ctx.trial_index,
                metadata: ctx.metadata,
                input: ctx.input,
                output: ctx.output,
                reference: mapped_reference.as_ref(),
            };

            execute_targets(&self.targets, &mapped_ctx).await
        })
    }
}

struct FullyMappedRunExecutor<I, O, R, O2, R2> {
    output_mapper: Box<dyn Mapper<O, O2>>,
    reference_mapper: Box<dyn Mapper<R, R2>>,
    targets: Vec<ScoringTarget<I, O2, R2>>,
}

impl<I, O, R, O2, R2> RunExecutor<I, O, R> for FullyMappedRunExecutor<I, O, R, O2, R2> {
    fn execute<'a>(&'a self, ctx: &'a ScorerContext<'a, I, O, R>) -> TrialFuture<'a> {
        Box::pin(async move {
            let mapped_output = match self.output_mapper.map(ctx.output) {
                Ok(mapped_output) => mapped_output,
                Err(err) => return map_failure_results(&self.targets, err),
            };
            let mapped_reference = match ctx.reference {
                Some(reference) => match self.reference_mapper.map(reference) {
                    Ok(mapped_reference) => Some(mapped_reference),
                    Err(err) => return map_failure_results(&self.targets, err),
                },
                None => None,
            };
            let mapped_ctx = ScorerContext {
                run_id: ctx.run_id,
                sample_id: ctx.sample_id,
                trial_index: ctx.trial_index,
                metadata: ctx.metadata,
                input: ctx.input,
                output: &mapped_output,
                reference: mapped_reference.as_ref(),
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
    let mut resources = ScorerResources::default();

    for (definition, result) in results {
        let validated = match result {
            Ok(outcome) => {
                resources.merge(&outcome.resources);
                validate_score(outcome.score)
            }
            Err(err) => Err(err),
        };

        scores.insert(definition.name, validated);
    }

    FlattenedTrial { scores, resources }
}

fn scorer_panic_scores(
    definitions: &[ScoreDefinition],
) -> HashMap<String, Result<Score, ScorerError>> {
    definitions
        .iter()
        .map(|definition| {
            (
                definition.name.clone(),
                Err(ScorerError::internal(ScorerPanicError)),
            )
        })
        .collect()
}

fn source_failure_scores(
    definitions: &[ScoreDefinition],
    err: OutputSourceError,
) -> HashMap<String, Result<Score, ScorerError>> {
    let shared_err = Arc::new(err);

    definitions
        .iter()
        .map(|definition| {
            (
                definition.name.clone(),
                Err(ScorerError::provider(SharedOutputSourceError(Arc::clone(
                    &shared_err,
                )))),
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
