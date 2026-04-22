use crate::{
    Acquisition, AcquisitionError, Dataset, MapError, Mapper, RunMetadata, RunResult, Sample,
    SampleResult, Score, ScoreDefinition, Scorer, ScorerContext, ScorerError, ScorerSet,
    TrialResult,
};
use chrono::Utc;
use futures::{FutureExt, StreamExt, stream};
use serde_json::{Map, Value};
#[cfg(feature = "otel")]
use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::marker::PhantomData;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use uuid::Uuid;

type TrialScores = Vec<(ScoreDefinition, Result<Score, ScorerError>)>;
type TrialFuture<'a> = Pin<Box<dyn Future<Output = TrialScores> + 'a>>;
type AcquisitionFuture<'a, O> = Pin<Box<dyn Future<Output = Result<O, AcquisitionError>> + 'a>>;

#[derive(Debug)]
#[non_exhaustive]
pub enum RunBuildError {
    NoDataset,
    NoAcquisition,
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
            Self::NoAcquisition => f.write_str("run is missing an acquisition"),
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
    acquisition: Box<dyn ErasedAcquisition<I, O>>,
    definitions: Vec<ScoreDefinition>,
    executor: Box<dyn RunExecutor<I, O, R>>,
    trial_count: usize,
    concurrency: usize,
    sample_timeout: Option<Duration>,
    seed: Option<u64>,
    acquisition_mode: &'static str,
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
                started_at,
                completed_at,
                duration,
                trial_count: self.trial_count,
                score_definitions: self.definitions.clone(),
                acquisition_mode: self.acquisition_mode.to_string(),
            },
            samples,
        })
    }

    async fn execute_sample(&self, run_id: &str, sample: &Sample<I, R>) -> SampleResult {
        let mut trials = Vec::with_capacity(self.trial_count);

        for trial_index in 0..self.trial_count {
            trials.push(self.execute_trial(run_id, sample, trial_index).await);
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
            token_usage: Default::default(),
            cost_usd: None,
        }
    }

    async fn execute_trial(
        &self,
        run_id: &str,
        sample: &Sample<I, R>,
        trial_index: usize,
    ) -> TrialResult {
        let started = Instant::now();

        let scores = match AssertUnwindSafe(self.acquire_output(sample))
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
                    Err(_) => scorer_panic_scores(&self.definitions),
                }
            }
            Ok(Err(err)) => acquisition_failure_scores(&self.definitions, err),
            Err(_) => acquisition_failure_scores(&self.definitions, AcquisitionError::Panicked),
        };

        TrialResult {
            scores,
            duration: started.elapsed(),
            trial_index,
        }
    }

    async fn acquire_output(&self, sample: &Sample<I, R>) -> Result<O, AcquisitionError> {
        #[cfg(feature = "otel")]
        if self.acquisition_mode == "observe" {
            return crate::otel::with_observe_sample_id(
                &sample.id,
                self.acquire_output_inner(&sample.input),
            )
            .await;
        }

        self.acquire_output_inner(&sample.input).await
    }

    async fn acquire_output_inner(&self, input: &I) -> Result<O, AcquisitionError> {
        match self.sample_timeout {
            Some(duration) => {
                match timeout(duration, self.acquisition.acquire_boxed(input)).await {
                    Ok(result) => result,
                    Err(_) => Err(AcquisitionError::Timeout(duration)),
                }
            }
            None => self.acquisition.acquire_boxed(input).await,
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
    pub fn acquisition<O, A>(self, acquisition: A) -> RunBuilderConfigured<I, O, R>
    where
        A: Acquisition<I, O> + 'static,
        O: 'static,
    {
        #[cfg(feature = "otel")]
        let acquisition_mode = if TypeId::of::<A>() == TypeId::of::<crate::otel::Observe>() {
            "observe"
        } else {
            "inline"
        };

        #[cfg(not(feature = "otel"))]
        let acquisition_mode = "inline";

        RunBuilderConfigured::<I, O, R> {
            dataset: self.dataset,
            acquisition: Box::new(acquisition),
            output_mapper: None,
            reference_mapper: None,
            trial_count: 1,
            concurrency: 1,
            sample_timeout: None,
            seed: None,
            acquisition_mode,
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
    acquisition: Box<dyn ErasedAcquisition<I, O>>,
    output_mapper: Option<Box<dyn Mapper<O, O2>>>,
    reference_mapper: Option<Box<dyn Mapper<R, R2>>>,
    trial_count: usize,
    concurrency: usize,
    sample_timeout: Option<Duration>,
    seed: Option<u64>,
    acquisition_mode: &'static str,
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
    acquisition: Box<dyn ErasedAcquisition<I, O>>,
    output_mapper: Option<Box<dyn Mapper<O, O2>>>,
    reference_mapper: Option<Box<dyn Mapper<R, R2>>>,
    targets: Vec<ScoringTarget<I, O2, R2>>,
    trial_count: usize,
    concurrency: usize,
    sample_timeout: Option<Duration>,
    seed: Option<u64>,
    acquisition_mode: &'static str,
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
            acquisition: self.acquisition,
            output_mapper: Some(Box::new(mapper)),
            reference_mapper: self.reference_mapper,
            trial_count: self.trial_count,
            concurrency: self.concurrency,
            sample_timeout: self.sample_timeout,
            seed: self.seed,
            acquisition_mode: self.acquisition_mode,
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
            acquisition: self.acquisition,
            output_mapper: self.output_mapper,
            reference_mapper: Some(Box::new(mapper)),
            trial_count: self.trial_count,
            concurrency: self.concurrency,
            sample_timeout: self.sample_timeout,
            seed: self.seed,
            acquisition_mode: self.acquisition_mode,
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
        RunBuilderWithTargets {
            dataset: self.dataset,
            acquisition: self.acquisition,
            output_mapper: self.output_mapper,
            reference_mapper: self.reference_mapper,
            targets: vec![ScoringTarget::from_scorer(scorer)],
            trial_count: self.trial_count,
            concurrency: self.concurrency,
            sample_timeout: self.sample_timeout,
            seed: self.seed,
            acquisition_mode: self.acquisition_mode,
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
        RunBuilderWithTargets {
            dataset: self.dataset,
            acquisition: self.acquisition,
            output_mapper: self.output_mapper,
            reference_mapper: self.reference_mapper,
            targets: vec![ScoringTarget::from_scorer_set(scorer_set)],
            trial_count: self.trial_count,
            concurrency: self.concurrency,
            sample_timeout: self.sample_timeout,
            seed: self.seed,
            acquisition_mode: self.acquisition_mode,
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
        self.targets.push(ScoringTarget::from_scorer(scorer));
        self
    }

    pub fn scorer_set(mut self, scorer_set: ScorerSet<I, O2, R2>) -> Self
    where
        I: 'static,
        O2: 'static,
        R2: 'static,
    {
        self.targets
            .push(ScoringTarget::from_scorer_set(scorer_set));
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

        if self.acquisition_mode == "observe"
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

impl<I: 'static, O: 'static, R: 'static> RunBuilderWithTargets<I, O, R, O, R, Unmapped, Unmapped> {
    pub fn build(self) -> Result<Run<I, O, R>, RunBuildError> {
        let definitions = self.validate()?;

        Ok(Run {
            dataset: self.dataset,
            acquisition: self.acquisition,
            definitions,
            executor: Box::new(RawRunExecutor {
                targets: self.targets,
            }),
            trial_count: self.trial_count,
            concurrency: self.concurrency,
            sample_timeout: self.sample_timeout,
            seed: self.seed,
            acquisition_mode: self.acquisition_mode,
        })
    }
}

impl<I: 'static, O: 'static, R: 'static, O2: 'static>
    RunBuilderWithTargets<I, O, R, O2, R, Mapped, Unmapped>
{
    pub fn build(self) -> Result<Run<I, O, R>, RunBuildError> {
        let definitions = self.validate()?;
        let output_mapper = self
            .output_mapper
            .expect("global output mapper must exist for mapped runs");

        Ok(Run {
            dataset: self.dataset,
            acquisition: self.acquisition,
            definitions,
            executor: Box::new(OutputMappedRunExecutor {
                output_mapper,
                targets: self.targets,
            }),
            trial_count: self.trial_count,
            concurrency: self.concurrency,
            sample_timeout: self.sample_timeout,
            seed: self.seed,
            acquisition_mode: self.acquisition_mode,
        })
    }
}

impl<I: 'static, O: 'static, R: 'static, R2: 'static>
    RunBuilderWithTargets<I, O, R, O, R2, Unmapped, Mapped>
{
    pub fn build(self) -> Result<Run<I, O, R>, RunBuildError> {
        let definitions = self.validate()?;
        let reference_mapper = self
            .reference_mapper
            .expect("global reference mapper must exist for mapped runs");

        Ok(Run {
            dataset: self.dataset,
            acquisition: self.acquisition,
            definitions,
            executor: Box::new(ReferenceMappedRunExecutor {
                reference_mapper,
                targets: self.targets,
            }),
            trial_count: self.trial_count,
            concurrency: self.concurrency,
            sample_timeout: self.sample_timeout,
            seed: self.seed,
            acquisition_mode: self.acquisition_mode,
        })
    }
}

impl<I: 'static, O: 'static, R: 'static, O2: 'static, R2: 'static>
    RunBuilderWithTargets<I, O, R, O2, R2, Mapped, Mapped>
{
    pub fn build(self) -> Result<Run<I, O, R>, RunBuildError> {
        let definitions = self.validate()?;
        let output_mapper = self
            .output_mapper
            .expect("global output mapper must exist for mapped runs");
        let reference_mapper = self
            .reference_mapper
            .expect("global reference mapper must exist for mapped runs");

        Ok(Run {
            dataset: self.dataset,
            acquisition: self.acquisition,
            definitions,
            executor: Box::new(FullyMappedRunExecutor {
                output_mapper,
                reference_mapper,
                targets: self.targets,
            }),
            trial_count: self.trial_count,
            concurrency: self.concurrency,
            sample_timeout: self.sample_timeout,
            seed: self.seed,
            acquisition_mode: self.acquisition_mode,
        })
    }
}

trait ErasedAcquisition<I, O>: Send + Sync {
    fn acquire_boxed<'a>(&'a self, input: &'a I) -> AcquisitionFuture<'a, O>;
}

impl<I, O, A> ErasedAcquisition<I, O> for A
where
    A: Acquisition<I, O> + Send + Sync,
    O: 'static,
{
    fn acquire_boxed<'a>(&'a self, input: &'a I) -> AcquisitionFuture<'a, O> {
        Box::pin(async move { self.acquire(input).await })
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
    executor: Box<dyn TargetExecutor<I, O, R>>,
}

impl<I, O, R> ScoringTarget<I, O, R> {
    fn from_scorer<S>(scorer: S) -> Self
    where
        S: Scorer<I, O, R> + 'static,
    {
        let definition = scorer.definition();
        Self {
            definitions: vec![definition.clone()],
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
            executor: Box::new(ScorerSetTarget { scorer_set }),
        }
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
        Box::pin(async move { vec![(self.definition.clone(), self.scorer.score(ctx).await)] })
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

fn flatten_scores(results: TrialScores) -> HashMap<String, Result<Score, ScorerError>> {
    results
        .into_iter()
        .map(|(definition, result)| {
            let validated = match result {
                Ok(score) => validate_score(score),
                Err(err) => Err(err),
            };

            (definition.name, validated)
        })
        .collect()
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

fn acquisition_failure_scores(
    definitions: &[ScoreDefinition],
    err: AcquisitionError,
) -> HashMap<String, Result<Score, ScorerError>> {
    let shared_err = Arc::new(err);

    definitions
        .iter()
        .map(|definition| {
            (
                definition.name.clone(),
                Err(ScorerError::provider(SharedAcquisitionError(Arc::clone(
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
struct SharedAcquisitionError(Arc<AcquisitionError>);

impl Display for SharedAcquisitionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Error for SharedAcquisitionError {
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
