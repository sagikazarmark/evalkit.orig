//! Happy-path facade for `Run`.
//!
//! `Eval` is an additive, semver-safe quickstart API built on top of the
//! `Run::builder()` state machine. It is intended as the recommended first
//! entry point for small evals and one-off scripts. For runs that need
//! trial-level control, custom mappers, or explicit metadata wiring, keep
//! using `Run::builder()` directly — nothing about it is deprecated.
//!
//! Every `Eval` chain compiles down to the same kernel `Run` path, so scores,
//! result shape, and `RunMetadata` semantics are identical.

use std::time::Duration;

use crate::run::{RunBuilderConfigured, RunBuilderWithTargets, Unmapped};
use crate::{Acquisition, Dataset, Run, RunError, RunResult, Sample, Scorer};

/// Entry point of the happy-path facade.
///
/// ```ignore
/// let result = Eval::new(samples)
///     .acquire(acquisition)
///     .scorer(MyScorer)
///     .trials(3)
///     .run()
///     .await?;
/// ```
pub struct Eval<I, R = ()> {
    dataset: Dataset<I, R>,
}

impl<I, R> Eval<I, R> {
    /// Build an `Eval` from an iterator of `Sample<I, R>`.
    pub fn new<S>(samples: S) -> Self
    where
        S: IntoIterator<Item = Sample<I, R>>,
    {
        Self {
            dataset: Dataset::new(samples.into_iter().collect()),
        }
    }

    /// Build an `Eval` from an existing `Dataset`.
    pub fn from_dataset(dataset: Dataset<I, R>) -> Self {
        Self { dataset }
    }

    /// Attach an acquisition step and move to the next phase.
    pub fn acquire<O, A>(self, acquisition: A) -> EvalTask<I, O, R>
    where
        A: Acquisition<I, O> + 'static,
        O: 'static,
    {
        EvalTask {
            inner: Run::builder()
                .dataset(self.dataset)
                .acquisition(acquisition),
        }
    }
}

/// Post-acquire, pre-scorer state.
///
/// Exists so the type system enforces that every `EvalRun` has at least one
/// scorer attached. The only legal next move is `scorer(...)`.
pub struct EvalTask<I, O, R> {
    inner: RunBuilderConfigured<I, O, R>,
}

impl<I, O, R> EvalTask<I, O, R> {
    pub fn scorer<S>(self, scorer: S) -> EvalRun<I, O, R>
    where
        S: Scorer<I, O, R> + 'static,
    {
        EvalRun {
            inner: self.inner.scorer(scorer),
        }
    }
}

/// Post-scorer, runnable state.
///
/// Holds a `RunBuilderWithTargets` under the hood and forwards the subset of
/// configuration that matters for the quickstart path. Everything here is a
/// thin pass-through — no duplicate execution engine.
pub struct EvalRun<I, O, R> {
    inner: RunBuilderWithTargets<I, O, R, O, R, Unmapped, Unmapped>,
}

impl<I: 'static, O: 'static, R: 'static> EvalRun<I, O, R> {
    pub fn scorer<S>(mut self, scorer: S) -> Self
    where
        S: Scorer<I, O, R> + 'static,
    {
        self.inner = self.inner.scorer(scorer);
        self
    }

    pub fn trials(mut self, trial_count: usize) -> Self {
        self.inner = self.inner.trials(trial_count);
        self
    }

    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.inner = self.inner.concurrency(concurrency);
        self
    }

    pub fn sample_timeout(mut self, sample_timeout: Duration) -> Self {
        self.inner = self.inner.sample_timeout(sample_timeout);
        self
    }

    pub fn seed(mut self, seed: u64) -> Self {
        self.inner = self.inner.seed(seed);
        self
    }

    pub fn code_commit(mut self, code_commit: impl Into<String>) -> Self {
        self.inner = self.inner.code_commit(code_commit);
        self
    }

    pub fn judge_model_pin(mut self, judge_model_pin: impl Into<String>) -> Self {
        self.inner = self.inner.judge_model_pin(judge_model_pin);
        self
    }

    /// Build the underlying `Run` and execute it.
    pub async fn run(self) -> Result<RunResult, RunError> {
        self.inner.build().map_err(RunError::Build)?.execute().await
    }

    /// Escape hatch: return the underlying `Run` instead of executing. Useful
    /// when you want to inspect build errors or keep the run for later.
    pub fn into_run(self) -> Result<Run<I, O, R>, RunError> {
        self.inner.build().map_err(RunError::Build)
    }
}

#[cfg(test)]
mod tests {
    use super::Eval;
    use crate::{
        AcquisitionError, Run, Sample, Score, ScoreDefinition, Scorer, ScorerContext, ScorerError,
    };

    struct ExactMatch;

    impl Scorer<String, String, String> for ExactMatch {
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

    struct Contains;

    impl Scorer<String, String, String> for Contains {
        async fn score(
            &self,
            ctx: &ScorerContext<'_, String, String, String>,
        ) -> Result<Score, ScorerError> {
            Ok(Score::Binary(
                ctx.reference
                    .is_some_and(|r| ctx.output.contains(r)),
            ))
        }

        fn definition(&self) -> ScoreDefinition {
            ScoreDefinition::maximize("contains")
        }
    }

    fn fixture_samples() -> Vec<Sample<String, String>> {
        vec![
            Sample::builder("q1".to_string())
                .id("s1")
                .reference("q1".to_string())
                .build()
                .unwrap(),
            Sample::builder("q2".to_string())
                .id("s2")
                .reference("q2-not-matched".to_string())
                .build()
                .unwrap(),
        ]
    }

    #[tokio::test]
    async fn facade_produces_same_shape_as_kernel_path() {
        let trials = 2;

        let facade_acquire = |input: &String| {
            let input = input.clone();
            async move { Ok::<_, AcquisitionError>(input) }
        };
        let kernel_acquire = |input: &String| {
            let input = input.clone();
            async move { Ok::<_, AcquisitionError>(input) }
        };

        let facade_result = Eval::new(fixture_samples())
            .acquire(facade_acquire)
            .scorer(ExactMatch)
            .scorer(Contains)
            .trials(trials)
            .seed(42)
            .run()
            .await
            .expect("facade run");

        let kernel_result = Run::builder()
            .dataset(crate::Dataset::new(fixture_samples()))
            .acquisition(kernel_acquire)
            .scorer(ExactMatch)
            .scorer(Contains)
            .trials(trials)
            .seed(42)
            .build()
            .expect("build kernel run")
            .execute()
            .await
            .expect("kernel run");

        let facade_defs: Vec<_> = facade_result
            .metadata
            .score_definitions
            .iter()
            .map(|d| d.name.clone())
            .collect();
        let kernel_defs: Vec<_> = kernel_result
            .metadata
            .score_definitions
            .iter()
            .map(|d| d.name.clone())
            .collect();
        assert_eq!(facade_defs, kernel_defs);

        assert_eq!(facade_result.samples.len(), kernel_result.samples.len());
        for (f, k) in facade_result
            .samples
            .iter()
            .zip(kernel_result.samples.iter())
        {
            assert_eq!(f.sample_id, k.sample_id);
            assert_eq!(f.trials.len(), k.trials.len());
            assert_eq!(f.trials.len(), trials);
            for (ft, kt) in f.trials.iter().zip(k.trials.iter()) {
                let mut fs: Vec<_> = ft
                    .scores
                    .iter()
                    .map(|(name, outcome)| (name.clone(), outcome.as_ref().ok().cloned()))
                    .collect();
                fs.sort_by(|(a, _), (b, _)| a.cmp(b));
                let mut ks: Vec<_> = kt
                    .scores
                    .iter()
                    .map(|(name, outcome)| (name.clone(), outcome.as_ref().ok().cloned()))
                    .collect();
                ks.sort_by(|(a, _), (b, _)| a.cmp(b));
                assert_eq!(fs, ks);
            }
        }

        assert_eq!(
            facade_result.metadata.trial_count,
            kernel_result.metadata.trial_count
        );
        assert_eq!(facade_result.metadata.trial_count, trials);
        assert_eq!(facade_result.metadata.seed, kernel_result.metadata.seed);
        assert_eq!(
            facade_result.metadata.acquisition_mode,
            kernel_result.metadata.acquisition_mode
        );
        assert_eq!(
            facade_result.metadata.score_definitions,
            kernel_result.metadata.score_definitions
        );
    }
}
