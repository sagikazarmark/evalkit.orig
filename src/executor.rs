use crate::{
    Acquisition, AcquisitionError, Dataset, RunMetadata, RunResult, Sample, SampleResult, Score,
    ScoreDefinition, ScoreOutcome, ScorerContext, ScorerError, ScorerResources, ScorerSet,
    TrialResult,
};
use chrono::Utc;
use futures::FutureExt;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs::File;
use std::future::Future;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};
use uuid::Uuid;

type TrialScores = Vec<(ScoreDefinition, Result<ScoreOutcome, ScorerError>)>;
type AcquisitionFuture<'a, O> = Pin<Box<dyn Future<Output = Result<O, AcquisitionError>> + 'a>>;
type JudgeModelTierPredicate<I, R> =
    dyn Fn(&Sample<I, R>, &HashMap<String, Result<Score, ScorerError>>) -> bool + Send + Sync;
type ShutdownPredicate = dyn Fn(&SampleResult) -> bool + Send + Sync;
type PartialScoringFuture<'a> = Pin<Box<dyn Future<Output = TrialScores> + 'a>>;
pub type ExecutorBoxError = Box<dyn Error + Send + Sync>;

#[allow(async_fn_in_trait)]
pub trait SampleSource<I, R = ()>: Send {
    async fn next_sample(&mut self) -> Result<Option<Sample<I, R>>, ExecutorBoxError>;

    fn metadata(&self) -> HashMap<String, Value> {
        HashMap::new()
    }
}

pub struct DatasetSource<I, R = ()> {
    samples: std::vec::IntoIter<Sample<I, R>>,
    metadata: HashMap<String, Value>,
}

impl<I, R> DatasetSource<I, R> {
    pub fn new(dataset: Dataset<I, R>) -> Self {
        Self {
            samples: dataset.samples.into_iter(),
            metadata: dataset.metadata,
        }
    }
}

impl<I, R> From<Dataset<I, R>> for DatasetSource<I, R> {
    fn from(dataset: Dataset<I, R>) -> Self {
        Self::new(dataset)
    }
}

pub struct JsonlFileTailSource<I, R = ()> {
    path: PathBuf,
    poll_interval: Duration,
    idle_timeout: Duration,
    offset: u64,
    partial_line: String,
    pending: VecDeque<Sample<I, R>>,
}

impl<I, R> JsonlFileTailSource<I, R> {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            poll_interval: Duration::from_millis(200),
            idle_timeout: Duration::from_secs(5),
            offset: 0,
            partial_line: String::new(),
            pending: VecDeque::new(),
        }
    }

    pub fn poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }

    pub fn idle_timeout(mut self, idle_timeout: Duration) -> Self {
        self.idle_timeout = idle_timeout;
        self
    }
}

impl<I, R> JsonlFileTailSource<I, R>
where
    I: DeserializeOwned,
    R: DeserializeOwned,
{
    fn read_available_samples(&mut self) -> Result<(), ExecutorBoxError> {
        let length = match std::fs::metadata(&self.path) {
            Ok(metadata) => metadata.len(),
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
            Err(err) => return Err(Box::new(err)),
        };

        if length < self.offset {
            self.offset = 0;
            self.partial_line.clear();
        }

        let mut file = File::open(&self.path).map_err(|err| Box::new(err) as ExecutorBoxError)?;
        file.seek(SeekFrom::Start(self.offset))
            .map_err(|err| Box::new(err) as ExecutorBoxError)?;

        let mut chunk = String::new();
        file.read_to_string(&mut chunk)
            .map_err(|err| Box::new(err) as ExecutorBoxError)?;
        self.offset += chunk.len() as u64;
        self.partial_line.push_str(&chunk);

        while let Some(newline_index) = self.partial_line.find('\n') {
            let line = self
                .partial_line
                .drain(..=newline_index)
                .collect::<String>();
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            let sample = serde_json::from_str(trimmed).map_err(|err| {
                Box::new(std::io::Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "invalid sample JSON in tailed file {}: {err}",
                        self.path.display()
                    ),
                )) as ExecutorBoxError
            })?;
            self.pending.push_back(sample);
        }

        Ok(())
    }
}

#[allow(async_fn_in_trait)]
impl<I, R> SampleSource<I, R> for JsonlFileTailSource<I, R>
where
    I: DeserializeOwned + Send,
    R: DeserializeOwned + Send,
{
    async fn next_sample(&mut self) -> Result<Option<Sample<I, R>>, ExecutorBoxError> {
        if let Some(sample) = self.pending.pop_front() {
            return Ok(Some(sample));
        }

        let deadline = Instant::now() + self.idle_timeout;

        loop {
            self.read_available_samples()?;

            if let Some(sample) = self.pending.pop_front() {
                return Ok(Some(sample));
            }

            if Instant::now() >= deadline {
                return Ok(None);
            }

            sleep(self.poll_interval).await;
        }
    }

    fn metadata(&self) -> HashMap<String, Value> {
        HashMap::from([
            (
                String::from("source.kind"),
                Value::String(String::from("jsonl_file_tail")),
            ),
            (
                String::from("source.path"),
                Value::String(self.path.display().to_string()),
            ),
        ])
    }
}

#[allow(async_fn_in_trait)]
impl<I, R> SampleSource<I, R> for DatasetSource<I, R>
where
    I: Send,
    R: Send,
{
    async fn next_sample(&mut self) -> Result<Option<Sample<I, R>>, ExecutorBoxError> {
        Ok(self.samples.next())
    }

    fn metadata(&self) -> HashMap<String, Value> {
        self.metadata.clone()
    }
}

pub trait Sampler<I, R = ()>: Send + Sync {
    fn should_sample(&self, sample: &Sample<I, R>) -> bool;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AlwaysSampler;

impl<I, R> Sampler<I, R> for AlwaysSampler {
    fn should_sample(&self, _sample: &Sample<I, R>) -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PercentSampler {
    percent: f64,
}

impl PercentSampler {
    pub fn new(percent: f64) -> Result<Self, SamplerBuildError> {
        if !(0.0..=100.0).contains(&percent) || !percent.is_finite() {
            return Err(SamplerBuildError::InvalidPercent(percent));
        }

        Ok(Self { percent })
    }
}

impl<I, R> Sampler<I, R> for PercentSampler {
    fn should_sample(&self, sample: &Sample<I, R>) -> bool {
        if self.percent <= 0.0 {
            return false;
        }

        if self.percent >= 100.0 {
            return true;
        }

        stable_fraction(sample.id.as_bytes()) < self.percent / 100.0
    }
}

pub struct TargetedSampler<P> {
    predicate: P,
}

impl<P> TargetedSampler<P> {
    pub fn new(predicate: P) -> Self {
        Self { predicate }
    }
}

impl<I, R, P> Sampler<I, R> for TargetedSampler<P>
where
    P: Fn(&Sample<I, R>) -> bool + Send + Sync,
{
    fn should_sample(&self, sample: &Sample<I, R>) -> bool {
        (self.predicate)(sample)
    }
}

#[allow(async_fn_in_trait)]
pub trait ExecutionSink: Send {
    async fn push_sample(&mut self, _sample: &SampleResult) -> Result<(), ExecutorBoxError> {
        Ok(())
    }

    async fn finish(&mut self, _result: &RunResult) -> Result<(), ExecutorBoxError> {
        Ok(())
    }
}

#[derive(Default)]
pub struct NoopSink;

impl ExecutionSink for NoopSink {}

#[allow(async_fn_in_trait)]
pub trait Executor {
    async fn execute(&mut self) -> Result<RunResult, ExecutorError>;
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ExecutorError {
    Source(ExecutorBoxError),
    Sink(ExecutorBoxError),
    Configuration(String),
}

impl Display for ExecutorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(err) => write!(f, "executor source failed: {err}"),
            Self::Sink(err) => write!(f, "executor sink failed: {err}"),
            Self::Configuration(message) => write!(f, "executor configuration failed: {message}"),
        }
    }
}

impl Error for ExecutorError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Source(err) | Self::Sink(err) => Some(err.as_ref()),
            Self::Configuration(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SamplerBuildError {
    InvalidPercent(f64),
}

impl Display for SamplerBuildError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPercent(percent) => {
                write!(
                    f,
                    "percent sampler requires a finite value between 0.0 and 100.0, got {percent}"
                )
            }
        }
    }
}

impl Error for SamplerBuildError {}

pub struct PullExecutor<I, O, R, Src, Samp, Sink> {
    source: Src,
    acquisition: Box<dyn ErasedAcquisition<I, O>>,
    scorer_set: ScorerSet<I, O, R>,
    judge_model_tier: Option<JudgeModelTier<I, O, R>>,
    partial_scoring: Option<Box<dyn PartialScoringPlan<I, O, R>>>,
    sampler: Samp,
    sink: Sink,
    trial_count: usize,
    queue_capacity: usize,
    max_samples: Option<usize>,
    shutdown: Option<ShutdownControl>,
    shutdown_mode: ShutdownMode,
    sample_timeout: Option<Duration>,
    seed: Option<u64>,
    code_commit: Option<String>,
    code_fingerprint: Option<String>,
    acquisition_mode: &'static str,
}

struct JudgeModelTier<I, O, R> {
    scorer_set: ScorerSet<I, O, R>,
    predicate: Box<JudgeModelTierPredicate<I, R>>,
}

trait PartialScoringPlan<I, O, R>: Send + Sync {
    fn definitions(&self) -> Vec<ScoreDefinition>;
    fn judge_model_pins(&self) -> Vec<String>;
    fn score<'a>(&'a self, ctx: &'a ScorerContext<'a, I, O, R>) -> PartialScoringFuture<'a>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StringPrefixCheckpoint {
    name: String,
    char_count: usize,
}

impl StringPrefixCheckpoint {
    pub fn new(name: impl Into<String>, char_count: usize) -> Self {
        Self {
            name: name.into(),
            char_count,
        }
    }
}

struct StringPrefixPartialScoring<I, R> {
    scorer_set: ScorerSet<I, String, R>,
    checkpoints: Vec<StringPrefixCheckpoint>,
}

impl<I, R> StringPrefixPartialScoring<I, R> {
    fn new(scorer_set: ScorerSet<I, String, R>, checkpoints: Vec<StringPrefixCheckpoint>) -> Self {
        Self {
            scorer_set,
            checkpoints,
        }
    }
}

impl<I, R> PartialScoringPlan<I, String, R> for StringPrefixPartialScoring<I, R> {
    fn definitions(&self) -> Vec<ScoreDefinition> {
        self.checkpoints
            .iter()
            .flat_map(|checkpoint| {
                self.scorer_set.definitions().iter().map(move |definition| {
                    rename_score_definition(definition, &format!("partial:{}", checkpoint.name))
                })
            })
            .collect()
    }

    fn judge_model_pins(&self) -> Vec<String> {
        self.scorer_set.judge_model_pins().to_vec()
    }

    fn score<'a>(&'a self, ctx: &'a ScorerContext<'a, I, String, R>) -> PartialScoringFuture<'a> {
        Box::pin(async move {
            let mut results = Vec::new();

            for checkpoint in &self.checkpoints {
                let Some(output) = incomplete_string_prefix(ctx.output, checkpoint.char_count)
                else {
                    continue;
                };

                let partial_ctx = ScorerContext {
                    run_id: ctx.run_id,
                    sample_id: ctx.sample_id,
                    trial_index: ctx.trial_index,
                    metadata: ctx.metadata,
                    input: ctx.input,
                    output: &output,
                    reference: ctx.reference,
                };

                results.extend(self.scorer_set.score(&partial_ctx).await.into_iter().map(
                    |(definition, result)| {
                        (
                            rename_score_definition(
                                &definition,
                                &format!("partial:{}", checkpoint.name),
                            ),
                            result,
                        )
                    },
                ));
            }

            results
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShutdownMode {
    #[default]
    DrainQueue,
    DiscardQueue,
}

struct ShutdownControl {
    predicate: Box<ShutdownPredicate>,
}

impl<I, O, R, Src, Samp, Sink> PullExecutor<I, O, R, Src, Samp, Sink>
where
    Src: SampleSource<I, R>,
    Samp: Sampler<I, R>,
    Sink: ExecutionSink,
{
    pub fn new<A>(
        source: Src,
        acquisition: A,
        scorer_set: ScorerSet<I, O, R>,
        sampler: Samp,
        sink: Sink,
    ) -> Self
    where
        A: Acquisition<I, O> + 'static,
        O: 'static,
    {
        let acquisition_mode = acquisition.metadata().mode;
        let detected = detect_code_identity_from_current_dir();

        Self {
            source,
            acquisition: Box::new(acquisition),
            scorer_set,
            judge_model_tier: None,
            partial_scoring: None,
            sampler,
            sink,
            trial_count: 1,
            queue_capacity: 1,
            max_samples: None,
            shutdown: None,
            shutdown_mode: ShutdownMode::DrainQueue,
            sample_timeout: None,
            seed: None,
            code_commit: detected.code_commit,
            code_fingerprint: detected.code_fingerprint,
            acquisition_mode,
        }
    }

    pub fn trials(mut self, trial_count: usize) -> Self {
        self.trial_count = trial_count.max(1);
        self
    }

    pub fn queue_capacity(mut self, queue_capacity: usize) -> Self {
        self.queue_capacity = queue_capacity.max(1);
        self
    }

    pub fn max_samples(mut self, max_samples: usize) -> Self {
        self.max_samples = Some(max_samples.max(1));
        self
    }

    pub fn shutdown_when<P>(mut self, predicate: P) -> Self
    where
        P: Fn(&SampleResult) -> bool + Send + Sync + 'static,
    {
        self.shutdown = Some(ShutdownControl {
            predicate: Box::new(predicate),
        });
        self
    }

    pub fn shutdown_mode(mut self, shutdown_mode: ShutdownMode) -> Self {
        self.shutdown_mode = shutdown_mode;
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

    pub fn judge_model_tier<P>(mut self, scorer_set: ScorerSet<I, O, R>, predicate: P) -> Self
    where
        P: Fn(&Sample<I, R>, &HashMap<String, Result<Score, ScorerError>>) -> bool
            + Send
            + Sync
            + 'static,
    {
        self.judge_model_tier = Some(JudgeModelTier {
            scorer_set,
            predicate: Box::new(predicate),
        });
        self
    }
}

impl<I: 'static, R: 'static, Src, Samp, Sink> PullExecutor<I, String, R, Src, Samp, Sink>
where
    Src: SampleSource<I, R>,
    Samp: Sampler<I, R>,
    Sink: ExecutionSink,
{
    pub fn partial_string_scoring(
        mut self,
        scorer_set: ScorerSet<I, String, R>,
        checkpoints: Vec<StringPrefixCheckpoint>,
    ) -> Self {
        self.partial_scoring = Some(Box::new(StringPrefixPartialScoring::new(
            scorer_set,
            checkpoints,
        )));
        self
    }
}

impl<I, O, R, Src, Samp, Sink> Executor for PullExecutor<I, O, R, Src, Samp, Sink>
where
    Src: SampleSource<I, R>,
    Samp: Sampler<I, R>,
    Sink: ExecutionSink,
{
    async fn execute(&mut self) -> Result<RunResult, ExecutorError> {
        let started_at = Utc::now();
        let started = Instant::now();
        let run_id = Uuid::new_v4().to_string();
        let definitions = merged_definitions(
            &self.scorer_set,
            self.judge_model_tier.as_ref(),
            self.partial_scoring.as_deref(),
        )
        .map_err(ExecutorError::Configuration)?;
        let judge_model_pins = merged_judge_model_pins(
            &self.scorer_set,
            self.judge_model_tier.as_ref(),
            self.partial_scoring.as_deref(),
        );
        let source_metadata = self.source.metadata();

        let mut sampled = Vec::new();
        let mut sample_results = Vec::new();

        let mut queue = VecDeque::with_capacity(self.queue_capacity);
        let mut source_exhausted = false;
        let mut shutting_down = false;
        let mut processed_samples = 0usize;

        loop {
            if !source_exhausted && !shutting_down {
                self.fill_queue(&mut queue, &mut source_exhausted).await?;
            }

            let Some(sample) = queue.pop_front() else {
                break;
            };

            sampled.push(sample);
            let sample_result = self
                .execute_sample(
                    &run_id,
                    sampled.last().expect("sample exists"),
                    &definitions,
                )
                .await;
            self.sink
                .push_sample(&sample_result)
                .await
                .map_err(ExecutorError::Sink)?;
            processed_samples += 1;
            if self.should_shutdown(processed_samples, &sample_result) {
                shutting_down = true;
                if self.shutdown_mode == ShutdownMode::DiscardQueue {
                    queue.clear();
                }
            }
            sample_results.push(sample_result);
        }

        let completed_at = Utc::now();
        let result = RunResult {
            metadata: RunMetadata {
                run_id,
                seed: self.seed,
                dataset_fingerprint: fingerprint_samples(&source_metadata, &sampled),
                scorer_fingerprint: fingerprint_definitions(&definitions),
                code_commit: self.code_commit.clone(),
                code_fingerprint: self.code_fingerprint.clone(),
                judge_model_pins,
                started_at,
                completed_at,
                duration: started.elapsed(),
                trial_count: self.trial_count,
                score_definitions: definitions,
                acquisition_mode: self.acquisition_mode.to_string(),
            },
            samples: sample_results,
        };

        self.sink
            .finish(&result)
            .await
            .map_err(ExecutorError::Sink)?;

        Ok(result)
    }
}

impl<I, O, R, Src, Samp, Sink> PullExecutor<I, O, R, Src, Samp, Sink>
where
    Src: SampleSource<I, R>,
    Samp: Sampler<I, R>,
    Sink: ExecutionSink,
{
    async fn execute_sample(
        &self,
        run_id: &str,
        sample: &Sample<I, R>,
        definitions: &[ScoreDefinition],
    ) -> SampleResult {
        let mut trials = Vec::with_capacity(self.trial_count);
        let mut resources = ScorerResources::default();

        for trial_index in 0..self.trial_count {
            let trial = self
                .execute_trial(run_id, sample, trial_index, definitions)
                .await;
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
            scored_count,
            error_count: self.trial_count - scored_count,
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
        definitions: &[ScoreDefinition],
    ) -> ExecutedTrial {
        let started = Instant::now();

        let flattened = match AssertUnwindSafe(self.acquire_output(sample))
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

                match AssertUnwindSafe(self.scorer_set.score(&ctx))
                    .catch_unwind()
                    .await
                {
                    Ok(scores) => {
                        let tiered = self
                            .maybe_execute_judge_model_tier(sample, &ctx, flatten_scores(scores))
                            .await;
                        self.maybe_execute_partial_scoring(&ctx, tiered).await
                    }
                    Err(_) => FlattenedTrial {
                        scores: scorer_panic_scores(definitions),
                        resources: ScorerResources::default(),
                    },
                }
            }
            Ok(Err(err)) => FlattenedTrial {
                scores: acquisition_failure_scores(definitions, err),
                resources: ScorerResources::default(),
            },
            Err(_) => FlattenedTrial {
                scores: acquisition_failure_scores(definitions, AcquisitionError::Panicked),
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

    async fn acquire_output(&self, sample: &Sample<I, R>) -> Result<O, AcquisitionError> {
        crate::acquisition::with_current_sample_id(
            &sample.id,
            self.acquire_output_inner(&sample.input),
        )
        .await
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

    async fn maybe_execute_judge_model_tier(
        &self,
        sample: &Sample<I, R>,
        ctx: &ScorerContext<'_, I, O, R>,
        primary: FlattenedTrial,
    ) -> FlattenedTrial {
        let Some(tier) = self.judge_model_tier.as_ref() else {
            return primary;
        };

        let should_run = match std::panic::catch_unwind(AssertUnwindSafe(|| {
            (tier.predicate)(sample, &primary.scores)
        })) {
            Ok(should_run) => should_run,
            Err(_) => {
                return merge_flattened_trials(
                    primary,
                    FlattenedTrial {
                        scores: tier_predicate_panic_scores(tier.scorer_set.definitions()),
                        resources: ScorerResources::default(),
                    },
                );
            }
        };

        if !should_run {
            return primary;
        }

        match AssertUnwindSafe(tier.scorer_set.score(ctx))
            .catch_unwind()
            .await
        {
            Ok(scores) => merge_flattened_trials(primary, flatten_scores(scores)),
            Err(_) => merge_flattened_trials(
                primary,
                FlattenedTrial {
                    scores: scorer_panic_scores(tier.scorer_set.definitions()),
                    resources: ScorerResources::default(),
                },
            ),
        }
    }

    async fn maybe_execute_partial_scoring(
        &self,
        ctx: &ScorerContext<'_, I, O, R>,
        primary: FlattenedTrial,
    ) -> FlattenedTrial {
        let Some(plan) = self.partial_scoring.as_deref() else {
            return primary;
        };

        let definitions = plan.definitions();

        match AssertUnwindSafe(plan.score(ctx)).catch_unwind().await {
            Ok(scores) => merge_flattened_trials(primary, flatten_scores(scores)),
            Err(_) => merge_flattened_trials(
                primary,
                FlattenedTrial {
                    scores: scorer_panic_scores(&definitions),
                    resources: ScorerResources::default(),
                },
            ),
        }
    }

    async fn fill_queue(
        &mut self,
        queue: &mut VecDeque<Sample<I, R>>,
        source_exhausted: &mut bool,
    ) -> Result<(), ExecutorError> {
        while queue.len() < self.queue_capacity && !*source_exhausted {
            match self
                .source
                .next_sample()
                .await
                .map_err(ExecutorError::Source)?
            {
                Some(sample) if self.sampler.should_sample(&sample) => queue.push_back(sample),
                Some(_) => continue,
                None => *source_exhausted = true,
            }
        }

        Ok(())
    }

    fn should_shutdown(&self, processed_samples: usize, sample_result: &SampleResult) -> bool {
        self.max_samples
            .is_some_and(|max_samples| processed_samples >= max_samples)
            || self
                .shutdown
                .as_ref()
                .is_some_and(|shutdown| (shutdown.predicate)(sample_result))
    }
}

struct ExecutedTrial {
    result: TrialResult,
    resources: ScorerResources,
}

struct FlattenedTrial {
    scores: HashMap<String, Result<Score, ScorerError>>,
    resources: ScorerResources,
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

fn merge_flattened_trials(
    mut primary: FlattenedTrial,
    secondary: FlattenedTrial,
) -> FlattenedTrial {
    primary.resources.merge(&secondary.resources);
    primary.scores.extend(secondary.scores);
    primary
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

fn tier_predicate_panic_scores(
    definitions: &[ScoreDefinition],
) -> HashMap<String, Result<Score, ScorerError>> {
    definitions
        .iter()
        .map(|definition| {
            (
                definition.name.clone(),
                Err(ScorerError::internal(JudgeModelTierPredicatePanicError)),
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
struct JudgeModelTierPredicatePanicError;

impl Display for JudgeModelTierPredicatePanicError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("judge-model tier predicate panicked")
    }
}

impl Error for JudgeModelTierPredicatePanicError {}

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

fn stable_fraction(bytes: &[u8]) -> f64 {
    let mut fingerprint = StableFingerprint::default();
    fingerprint.write_bytes(bytes);
    fingerprint.state as f64 / u64::MAX as f64
}

fn fingerprint_samples<I, R>(
    metadata: &HashMap<String, Value>,
    samples: &[Sample<I, R>],
) -> String {
    let mut fingerprint = StableFingerprint::default();
    fingerprint.write_bytes(canonical_metadata_json(metadata).as_bytes());

    for sample in samples {
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

fn rename_score_definition(definition: &ScoreDefinition, stage: &str) -> ScoreDefinition {
    ScoreDefinition {
        name: format!("{}@{stage}", definition.name),
        direction: definition.direction,
    }
}

fn incomplete_string_prefix(output: &str, char_count: usize) -> Option<String> {
    let total_chars = output.chars().count();

    if char_count >= total_chars {
        return None;
    }

    Some(output.chars().take(char_count).collect())
}

fn merged_definitions<I, O, R>(
    primary: &ScorerSet<I, O, R>,
    tier: Option<&JudgeModelTier<I, O, R>>,
    partial: Option<&dyn PartialScoringPlan<I, O, R>>,
) -> Result<Vec<ScoreDefinition>, String> {
    let mut definitions = primary.definitions().to_vec();
    let mut seen = definitions
        .iter()
        .map(|definition| definition.name.clone())
        .collect::<HashSet<_>>();

    if let Some(tier) = tier {
        for definition in tier.scorer_set.definitions() {
            if !seen.insert(definition.name.clone()) {
                return Err(format!(
                    "duplicate score definition `{}` across primary scorer set and judge-model tier",
                    definition.name
                ));
            }

            definitions.push(definition.clone());
        }
    }

    if let Some(partial) = partial {
        for definition in partial.definitions() {
            if !seen.insert(definition.name.clone()) {
                return Err(format!(
                    "duplicate score definition `{}` across executor scoring stages",
                    definition.name
                ));
            }

            definitions.push(definition);
        }
    }

    Ok(definitions)
}

fn merged_judge_model_pins<I, O, R>(
    primary: &ScorerSet<I, O, R>,
    tier: Option<&JudgeModelTier<I, O, R>>,
    partial: Option<&dyn PartialScoringPlan<I, O, R>>,
) -> Vec<String> {
    let mut judge_model_pins = primary.judge_model_pins().to_vec();

    if let Some(tier) = tier {
        judge_model_pins.extend(tier.scorer_set.judge_model_pins().iter().cloned());
    }

    if let Some(partial) = partial {
        judge_model_pins.extend(partial.judge_model_pins());
    }

    judge_model_pins.sort();
    judge_model_pins.dedup();
    judge_model_pins
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

#[derive(Debug, Default)]
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

#[cfg(test)]
mod tests {
    use super::{
        AlwaysSampler, DatasetSource, ExecutionSink, Executor, JsonlFileTailSource,
        PercentSampler, PullExecutor, Sampler, SamplerBuildError, ShutdownMode,
        StringPrefixCheckpoint, TargetedSampler,
    };
    use crate::{
        AcquisitionError, Dataset, RunResult, Sample, SampleResult, SampleSource, Score,
        ScoreDefinition, Scorer, ScorerContext, ScorerError, ScorerMetadata, ScorerSet,
    };
    use serde_json::Value;
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tempfile::tempdir;

    struct ExactMatchScorer;

    impl Scorer<String, String, String> for ExactMatchScorer {
        async fn score(
            &self,
            ctx: &ScorerContext<'_, String, String, String>,
        ) -> Result<Score, ScorerError> {
            Ok(Score::Binary(ctx.reference == Some(ctx.output)))
        }

        fn definition(&self) -> ScoreDefinition {
            ScoreDefinition::maximize("exact_match")
        }
    }

    struct ReviewGateScorer;

    impl Scorer<String, String, String> for ReviewGateScorer {
        async fn score(
            &self,
            ctx: &ScorerContext<'_, String, String, String>,
        ) -> Result<Score, ScorerError> {
            Ok(Score::Binary(ctx.reference == Some(ctx.output)))
        }

        fn definition(&self) -> ScoreDefinition {
            ScoreDefinition::maximize("cheap_match")
        }

        fn metadata(&self) -> ScorerMetadata {
            ScorerMetadata::default().judge_model_pin("cheap-judge@v1")
        }
    }

    struct EscalationScorer;

    impl Scorer<String, String, String> for EscalationScorer {
        async fn score(
            &self,
            _ctx: &ScorerContext<'_, String, String, String>,
        ) -> Result<Score, ScorerError> {
            Ok(Score::Label(String::from("needs_review")))
        }

        fn definition(&self) -> ScoreDefinition {
            ScoreDefinition::new("tier_review")
        }

        fn metadata(&self) -> ScorerMetadata {
            ScorerMetadata::default().judge_model_pin("expensive-judge@v2")
        }
    }

    struct PrefixLengthScorer;

    impl Scorer<String, String, String> for PrefixLengthScorer {
        async fn score(
            &self,
            ctx: &ScorerContext<'_, String, String, String>,
        ) -> Result<Score, ScorerError> {
            Ok(Score::Numeric(ctx.output.chars().count() as f64))
        }

        fn definition(&self) -> ScoreDefinition {
            ScoreDefinition::new("prefix_length")
        }
    }

    #[derive(Default)]
    struct RecordingSink {
        pushed_sample_ids: Arc<Mutex<Vec<String>>>,
        finished_run_id: Arc<Mutex<Option<String>>>,
    }

    impl ExecutionSink for RecordingSink {
        async fn push_sample(
            &mut self,
            sample: &SampleResult,
        ) -> Result<(), super::ExecutorBoxError> {
            self.pushed_sample_ids
                .lock()
                .expect("recording sink mutex poisoned")
                .push(sample.sample_id.clone());
            Ok(())
        }

        async fn finish(&mut self, result: &RunResult) -> Result<(), super::ExecutorBoxError> {
            *self
                .finished_run_id
                .lock()
                .expect("recording sink mutex poisoned") = Some(result.metadata.run_id.clone());
            Ok(())
        }
    }

    #[test]
    fn percent_sampler_rejects_invalid_ranges() {
        assert_eq!(
            PercentSampler::new(-1.0).unwrap_err(),
            SamplerBuildError::InvalidPercent(-1.0)
        );
        assert_eq!(
            PercentSampler::new(101.0).unwrap_err(),
            SamplerBuildError::InvalidPercent(101.0)
        );
    }

    #[test]
    fn targeted_sampler_filters_by_predicate() {
        let sampler = TargetedSampler::new(|sample: &Sample<String, String>| sample.id == "keep");
        let keep = Sample::builder("input".to_string())
            .id("keep")
            .reference("ref".to_string())
            .build()
            .unwrap();
        let drop = Sample::builder("input".to_string())
            .id("drop")
            .reference("ref".to_string())
            .build()
            .unwrap();

        assert!(sampler.should_sample(&keep));
        assert!(!sampler.should_sample(&drop));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn pull_executor_processes_sampled_dataset_and_notifies_sink() {
        let dataset = Dataset::new(vec![
            Sample::builder("hello".to_string())
                .id("keep-1")
                .reference("echo::hello".to_string())
                .build()
                .unwrap(),
            Sample::builder("skip".to_string())
                .id("skip-1")
                .reference("echo::skip".to_string())
                .build()
                .unwrap(),
        ]);
        let sink = RecordingSink::default();
        let pushed = Arc::clone(&sink.pushed_sample_ids);
        let finished = Arc::clone(&sink.finished_run_id);

        let acquisition = |input: &String| {
            let output = format!("echo::{input}");
            async move { Ok::<_, AcquisitionError>(output) }
        };
        let scorer_set = ScorerSet::<String, String, String>::builder()
            .scorer(ExactMatchScorer)
            .build();

        let mut executor = PullExecutor::new(
            DatasetSource::new(dataset),
            acquisition,
            scorer_set,
            TargetedSampler::new(|sample: &Sample<String, String>| sample.id.starts_with("keep")),
            sink,
        )
        .trials(2);

        let result = executor.execute().await.unwrap();

        assert_eq!(result.samples.len(), 1);
        assert_eq!(result.samples[0].sample_id, "keep-1");
        assert_eq!(result.metadata.trial_count, 2);
        assert_eq!(result.metadata.acquisition_mode, "inline");
        assert_eq!(
            pushed
                .lock()
                .expect("recording sink mutex poisoned")
                .as_slice(),
            ["keep-1"]
        );
        assert!(
            finished
                .lock()
                .expect("recording sink mutex poisoned")
                .as_deref()
                .is_some()
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn pull_executor_with_always_sampler_keeps_all_samples() {
        let dataset = Dataset::new(vec![
            Sample::new("one".to_string(), "echo::one".to_string()),
            Sample::new("two".to_string(), "echo::two".to_string()),
        ]);
        let acquisition = |input: &String| {
            let output = format!("echo::{input}");
            async move { Ok::<_, AcquisitionError>(output) }
        };
        let scorer_set = ScorerSet::<String, String, String>::builder()
            .scorer(ExactMatchScorer)
            .build();

        let mut executor = PullExecutor::new(
            DatasetSource::new(dataset),
            acquisition,
            scorer_set,
            AlwaysSampler,
            super::NoopSink,
        );

        let result = executor.execute().await.unwrap();

        assert_eq!(result.samples.len(), 2);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn pull_executor_runs_judge_model_tier_for_flagged_samples_only() {
        let dataset = Dataset::new(vec![
            Sample::new("pass".to_string(), "echo::pass".to_string()),
            Sample::builder("fail".to_string())
                .id("needs-review")
                .reference("expected::fail".to_string())
                .build()
                .unwrap(),
        ]);
        let acquisition = |input: &String| {
            let output = format!("echo::{input}");
            async move { Ok::<_, AcquisitionError>(output) }
        };
        let scorer_set = ScorerSet::<String, String, String>::builder()
            .scorer(ReviewGateScorer)
            .build();
        let tier = ScorerSet::<String, String, String>::builder()
            .scorer(EscalationScorer)
            .build();

        let mut executor = PullExecutor::new(
            DatasetSource::new(dataset),
            acquisition,
            scorer_set,
            AlwaysSampler,
            super::NoopSink,
        )
        .judge_model_tier(tier, |_, scores| {
            matches!(scores.get("cheap_match"), Some(Ok(Score::Binary(false))))
        });

        let result = executor.execute().await.unwrap();

        assert_eq!(result.metadata.score_definitions.len(), 2);
        assert_eq!(
            result.metadata.judge_model_pins,
            vec![
                String::from("cheap-judge@v1"),
                String::from("expensive-judge@v2"),
            ]
        );
        assert!(
            !result.samples[0].trials[0]
                .scores
                .contains_key("tier_review")
        );
        assert!(matches!(
            result.samples[1].trials[0].scores.get("tier_review"),
            Some(Ok(Score::Label(label))) if label == "needs_review"
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn pull_executor_rejects_duplicate_score_names_across_tiers() {
        let dataset = Dataset::new(vec![Sample::new(
            "one".to_string(),
            "echo::one".to_string(),
        )]);
        let acquisition = |input: &String| {
            let output = format!("echo::{input}");
            async move { Ok::<_, AcquisitionError>(output) }
        };
        let primary = ScorerSet::<String, String, String>::builder()
            .scorer(ExactMatchScorer)
            .build();
        let tier = ScorerSet::<String, String, String>::builder()
            .scorer(ExactMatchScorer)
            .build();

        let mut executor = PullExecutor::new(
            DatasetSource::new(dataset),
            acquisition,
            primary,
            AlwaysSampler,
            super::NoopSink,
        )
        .judge_model_tier(tier, |_, _| true);

        let err = executor.execute().await.unwrap_err();

        assert_eq!(
            err.to_string(),
            "executor configuration failed: duplicate score definition `exact_match` across primary scorer set and judge-model tier"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn jsonl_file_tail_source_reads_existing_and_appended_samples() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("samples.jsonl");
        let first = Sample::builder("hello".to_string())
            .id("sample-1")
            .reference("echo::hello".to_string())
            .build()
            .unwrap();
        let second = Sample::builder("world".to_string())
            .id("sample-2")
            .reference("echo::world".to_string())
            .build()
            .unwrap();

        std::fs::write(
            &path,
            format!("{}\n", serde_json::to_string(&first).unwrap()),
        )
        .unwrap();

        let mut source: JsonlFileTailSource<String, String> = JsonlFileTailSource::new(&path)
            .poll_interval(Duration::from_millis(5))
            .idle_timeout(Duration::from_millis(20));

        let first_read = source.next_sample().await.unwrap().unwrap();
        assert_eq!(first_read.id, "sample-1");

        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(file, "{}", serde_json::to_string(&second).unwrap()).unwrap();
        file.flush().unwrap();

        let second_read = source.next_sample().await.unwrap().unwrap();
        let done = source.next_sample().await.unwrap();

        assert_eq!(second_read.id, "sample-2");
        assert!(done.is_none());
        assert_eq!(
            source.metadata().get("source.kind"),
            Some(&Value::String(String::from("jsonl_file_tail")))
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn pull_executor_partial_string_scoring_records_checkpoint_scores() {
        let dataset = Dataset::new(vec![Sample::new(
            "hello".to_string(),
            "echo::hello".to_string(),
        )]);
        let acquisition = |input: &String| {
            let output = format!("echo::{input}");
            async move { Ok::<_, AcquisitionError>(output) }
        };
        let primary = ScorerSet::<String, String, String>::builder()
            .scorer(ExactMatchScorer)
            .build();
        let partial = ScorerSet::<String, String, String>::builder()
            .scorer(PrefixLengthScorer)
            .build();

        let mut executor = PullExecutor::new(
            DatasetSource::new(dataset),
            acquisition,
            primary,
            AlwaysSampler,
            super::NoopSink,
        )
        .partial_string_scoring(partial, vec![StringPrefixCheckpoint::new("char-2", 2)]);

        let result = executor.execute().await.unwrap();
        let partial_score = result.samples[0].trials[0]
            .scores
            .get("prefix_length@partial:char-2")
            .unwrap()
            .as_ref()
            .unwrap();

        assert_eq!(result.metadata.score_definitions.len(), 2);
        assert_eq!(*partial_score, Score::Numeric(2.0));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn pull_executor_max_samples_discards_prefetched_queue() {
        let dataset = Dataset::new(vec![
            Sample::builder("one".to_string())
                .id("one")
                .reference("echo::one".to_string())
                .build()
                .unwrap(),
            Sample::builder("two".to_string())
                .id("two")
                .reference("echo::two".to_string())
                .build()
                .unwrap(),
            Sample::builder("three".to_string())
                .id("three")
                .reference("echo::three".to_string())
                .build()
                .unwrap(),
        ]);
        let acquisition = |input: &String| {
            let output = format!("echo::{input}");
            async move { Ok::<_, AcquisitionError>(output) }
        };
        let scorer_set = ScorerSet::<String, String, String>::builder()
            .scorer(ExactMatchScorer)
            .build();

        let mut executor = PullExecutor::new(
            DatasetSource::new(dataset),
            acquisition,
            scorer_set,
            AlwaysSampler,
            super::NoopSink,
        )
        .queue_capacity(3)
        .max_samples(1)
        .shutdown_mode(ShutdownMode::DiscardQueue);

        let result = executor.execute().await.unwrap();

        assert_eq!(result.samples.len(), 1);
        assert_eq!(result.samples[0].sample_id, "one");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn pull_executor_shutdown_mode_drains_prefetched_queue() {
        let dataset = Dataset::new(vec![
            Sample::builder("one".to_string())
                .id("one")
                .reference("echo::one".to_string())
                .build()
                .unwrap(),
            Sample::builder("two".to_string())
                .id("two")
                .reference("echo::two".to_string())
                .build()
                .unwrap(),
            Sample::builder("three".to_string())
                .id("three")
                .reference("echo::three".to_string())
                .build()
                .unwrap(),
        ]);
        let acquisition = |input: &String| {
            let output = format!("echo::{input}");
            async move { Ok::<_, AcquisitionError>(output) }
        };
        let scorer_set = ScorerSet::<String, String, String>::builder()
            .scorer(ExactMatchScorer)
            .build();

        let mut executor = PullExecutor::new(
            DatasetSource::new(dataset),
            acquisition,
            scorer_set,
            AlwaysSampler,
            super::NoopSink,
        )
        .queue_capacity(3)
        .shutdown_when(|sample_result| sample_result.sample_id == "one")
        .shutdown_mode(ShutdownMode::DrainQueue);

        let result = executor.execute().await.unwrap();

        assert_eq!(result.samples.len(), 3);
        assert_eq!(
            result
                .samples
                .iter()
                .map(|sample| sample.sample_id.as_str())
                .collect::<Vec<_>>(),
            vec!["one", "two", "three"]
        );
    }
}
