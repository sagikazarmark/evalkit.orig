#![allow(dead_code)]

use crate::{MapError, Mapper, Score, ScoreDefinition, Scorer, ScorerContext, ScorerError};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

type ScoreFuture<'a> = Pin<Box<dyn Future<Output = ScoreResults> + 'a>>;
type ScoreResults = Vec<(ScoreDefinition, Result<Score, ScorerError>)>;

pub struct ScorerSet<I, O, R = ()> {
    definitions: Vec<ScoreDefinition>,
    executor: Box<dyn ScorerSetExecutor<I, O, R>>,
}

impl<I, O, R> ScorerSet<I, O, R> {
    pub fn builder() -> ScorerSetBuilder<I, O, R> {
        ScorerSetBuilder::<I, O, R> {
            output_mapper: None,
            reference_mapper: None,
            _mapped: PhantomData,
        }
    }

    pub(crate) fn definitions(&self) -> &[ScoreDefinition] {
        &self.definitions
    }

    pub(crate) async fn score(
        &self,
        ctx: &ScorerContext<'_, I, O, R>,
    ) -> Vec<(ScoreDefinition, Result<Score, ScorerError>)> {
        self.executor.execute(ctx).await
    }
}

pub struct ScorerSetBuilder<
    I,
    O,
    R,
    O2 = O,
    R2 = R,
    OutputState = Unmapped,
    ReferenceState = Unmapped,
> {
    output_mapper: Option<Box<dyn Mapper<O, O2>>>,
    reference_mapper: Option<Box<dyn Mapper<R, R2>>>,
    _mapped: PhantomData<fn(I) -> (OutputState, ReferenceState)>,
}

pub struct ScorerSetBuilderWithScorers<
    I,
    O,
    R,
    O2 = O,
    R2 = R,
    OutputState = Unmapped,
    ReferenceState = Unmapped,
> {
    output_mapper: Option<Box<dyn Mapper<O, O2>>>,
    reference_mapper: Option<Box<dyn Mapper<R, R2>>>,
    scorers: Vec<ScorerEntry<I, O2, R2>>,
    _mapped: PhantomData<fn(I) -> (OutputState, ReferenceState)>,
}

pub struct Unmapped;
pub struct Mapped;

impl<I, O, R, R2, ReferenceState> ScorerSetBuilder<I, O, R, O, R2, Unmapped, ReferenceState> {
    pub fn map_output<O3, M>(
        self,
        mapper: M,
    ) -> ScorerSetBuilder<I, O, R, O3, R2, Mapped, ReferenceState>
    where
        M: Mapper<O, O3> + 'static,
    {
        ScorerSetBuilder::<I, O, R, O3, R2, Mapped, ReferenceState> {
            output_mapper: Some(Box::new(mapper)),
            reference_mapper: self.reference_mapper,
            _mapped: PhantomData,
        }
    }
}

impl<I, O, R, O2, OutputState> ScorerSetBuilder<I, O, R, O2, R, OutputState, Unmapped> {
    pub fn map_reference<R3, M>(
        self,
        mapper: M,
    ) -> ScorerSetBuilder<I, O, R, O2, R3, OutputState, Mapped>
    where
        M: Mapper<R, R3> + 'static,
    {
        ScorerSetBuilder::<I, O, R, O2, R3, OutputState, Mapped> {
            output_mapper: self.output_mapper,
            reference_mapper: Some(Box::new(mapper)),
            _mapped: PhantomData,
        }
    }
}

impl<I, O, R, O2, R2, OutputState, ReferenceState>
    ScorerSetBuilder<I, O, R, O2, R2, OutputState, ReferenceState>
{
    pub fn scorer<S>(
        self,
        scorer: S,
    ) -> ScorerSetBuilderWithScorers<I, O, R, O2, R2, OutputState, ReferenceState>
    where
        S: Scorer<I, O2, R2> + 'static,
    {
        ScorerSetBuilderWithScorers {
            output_mapper: self.output_mapper,
            reference_mapper: self.reference_mapper,
            scorers: vec![ScorerEntry::new(scorer)],
            _mapped: PhantomData,
        }
    }
}

impl<I, O, R, O2, R2, OutputState, ReferenceState>
    ScorerSetBuilderWithScorers<I, O, R, O2, R2, OutputState, ReferenceState>
{
    pub fn scorer<S>(mut self, scorer: S) -> Self
    where
        S: Scorer<I, O2, R2> + 'static,
    {
        self.scorers.push(ScorerEntry::new(scorer));
        self
    }
}

impl<I: 'static, O: 'static, R: 'static>
    ScorerSetBuilderWithScorers<I, O, R, O, R, Unmapped, Unmapped>
{
    pub fn build(self) -> ScorerSet<I, O, R> {
        let definitions = collect_definitions(&self.scorers);

        ScorerSet {
            definitions,
            executor: Box::new(RawExecutor {
                scorers: self.scorers,
            }),
        }
    }
}

impl<I: 'static, O: 'static, R: 'static, O2: 'static>
    ScorerSetBuilderWithScorers<I, O, R, O2, R, Mapped, Unmapped>
{
    pub fn build(self) -> ScorerSet<I, O, R> {
        let definitions = collect_definitions(&self.scorers);
        let output_mapper = self
            .output_mapper
            .expect("output mapper must exist for mapped scorer sets");
        ScorerSet {
            definitions,
            executor: Box::new(OutputMappedExecutor {
                output_mapper,
                scorers: self.scorers,
            }),
        }
    }
}

impl<I: 'static, O: 'static, R: 'static, R2: 'static>
    ScorerSetBuilderWithScorers<I, O, R, O, R2, Unmapped, Mapped>
{
    pub fn build(self) -> ScorerSet<I, O, R> {
        let definitions = collect_definitions(&self.scorers);
        let reference_mapper = self
            .reference_mapper
            .expect("reference mapper must exist for mapped scorer sets");
        ScorerSet {
            definitions,
            executor: Box::new(ReferenceMappedExecutor {
                reference_mapper,
                scorers: self.scorers,
            }),
        }
    }
}

impl<I: 'static, O: 'static, R: 'static, O2: 'static, R2: 'static>
    ScorerSetBuilderWithScorers<I, O, R, O2, R2, Mapped, Mapped>
{
    pub fn build(self) -> ScorerSet<I, O, R> {
        let definitions = collect_definitions(&self.scorers);
        let output_mapper = self
            .output_mapper
            .expect("output mapper must exist for mapped scorer sets");
        let reference_mapper = self
            .reference_mapper
            .expect("reference mapper must exist for mapped scorer sets");
        ScorerSet {
            definitions,
            executor: Box::new(FullyMappedExecutor {
                output_mapper,
                reference_mapper,
                scorers: self.scorers,
            }),
        }
    }
}

trait ScorerSetExecutor<I, O, R>: Send + Sync {
    fn execute<'a>(&'a self, ctx: &'a ScorerContext<'a, I, O, R>) -> ScoreFuture<'a>;
}

struct RawExecutor<I, O, R> {
    scorers: Vec<ScorerEntry<I, O, R>>,
}

impl<I, O, R> ScorerSetExecutor<I, O, R> for RawExecutor<I, O, R> {
    fn execute<'a>(&'a self, ctx: &'a ScorerContext<'a, I, O, R>) -> ScoreFuture<'a> {
        Box::pin(score_entries(&self.scorers, ctx))
    }
}

struct OutputMappedExecutor<I, O, R, O2> {
    output_mapper: Box<dyn Mapper<O, O2>>,
    scorers: Vec<ScorerEntry<I, O2, R>>,
}

impl<I, O, R, O2> ScorerSetExecutor<I, O, R> for OutputMappedExecutor<I, O, R, O2> {
    fn execute<'a>(&'a self, ctx: &'a ScorerContext<'a, I, O, R>) -> ScoreFuture<'a> {
        Box::pin(async move {
            let mapped_output = match self.output_mapper.map(ctx.output) {
                Ok(mapped_output) => mapped_output,
                Err(err) => return mapper_failure_results(&self.scorers, err),
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

            score_entries(&self.scorers, &mapped_ctx).await
        })
    }
}

struct ReferenceMappedExecutor<I, O, R, R2> {
    reference_mapper: Box<dyn Mapper<R, R2>>,
    scorers: Vec<ScorerEntry<I, O, R2>>,
}

impl<I, O, R, R2> ScorerSetExecutor<I, O, R> for ReferenceMappedExecutor<I, O, R, R2> {
    fn execute<'a>(&'a self, ctx: &'a ScorerContext<'a, I, O, R>) -> ScoreFuture<'a> {
        Box::pin(async move {
            let mapped_reference = match ctx.reference {
                Some(reference) => match self.reference_mapper.map(reference) {
                    Ok(mapped_reference) => Some(mapped_reference),
                    Err(err) => return mapper_failure_results(&self.scorers, err),
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

            score_entries(&self.scorers, &mapped_ctx).await
        })
    }
}

struct FullyMappedExecutor<I, O, R, O2, R2> {
    output_mapper: Box<dyn Mapper<O, O2>>,
    reference_mapper: Box<dyn Mapper<R, R2>>,
    scorers: Vec<ScorerEntry<I, O2, R2>>,
}

impl<I, O, R, O2, R2> ScorerSetExecutor<I, O, R> for FullyMappedExecutor<I, O, R, O2, R2> {
    fn execute<'a>(&'a self, ctx: &'a ScorerContext<'a, I, O, R>) -> ScoreFuture<'a> {
        Box::pin(async move {
            let mapped_output = match self.output_mapper.map(ctx.output) {
                Ok(mapped_output) => mapped_output,
                Err(err) => return mapper_failure_results(&self.scorers, err),
            };
            let mapped_reference = match ctx.reference {
                Some(reference) => match self.reference_mapper.map(reference) {
                    Ok(mapped_reference) => Some(mapped_reference),
                    Err(err) => return mapper_failure_results(&self.scorers, err),
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

            score_entries(&self.scorers, &mapped_ctx).await
        })
    }
}

fn collect_definitions<I, O, R>(scorers: &[ScorerEntry<I, O, R>]) -> Vec<ScoreDefinition> {
    scorers
        .iter()
        .map(|scorer| scorer.definition.clone())
        .collect()
}

async fn score_entries<I, O, R>(
    scorers: &[ScorerEntry<I, O, R>],
    ctx: &ScorerContext<'_, I, O, R>,
) -> ScoreResults {
    let mut results = Vec::with_capacity(scorers.len());

    for scorer in scorers {
        results.push((
            scorer.definition.clone(),
            scorer.scorer.score_boxed(ctx).await,
        ));
    }

    results
}

fn mapper_failure_results<I, O, R>(
    scorers: &[ScorerEntry<I, O, R>],
    err: MapError,
) -> ScoreResults {
    let shared_err: Arc<dyn Error + Send + Sync> = Arc::from(err.0);

    scorers
        .iter()
        .map(|scorer| {
            (
                scorer.definition.clone(),
                Err(ScorerError::invalid_input(SharedMapError(
                    shared_err.clone(),
                ))),
            )
        })
        .collect()
}

struct ScorerEntry<I, O, R> {
    definition: ScoreDefinition,
    scorer: Box<dyn ErasedScorer<I, O, R>>,
}

impl<I, O, R> ScorerEntry<I, O, R> {
    fn new<S>(scorer: S) -> Self
    where
        S: Scorer<I, O, R> + 'static,
    {
        Self {
            definition: scorer.definition(),
            scorer: Box::new(scorer),
        }
    }
}

trait ErasedScorer<I, O, R>: Send + Sync {
    fn score_boxed<'a>(
        &'a self,
        ctx: &'a ScorerContext<'a, I, O, R>,
    ) -> Pin<Box<dyn Future<Output = Result<Score, ScorerError>> + 'a>>;
}

impl<I, O, R, S> ErasedScorer<I, O, R> for S
where
    S: Scorer<I, O, R> + Send + Sync,
{
    fn score_boxed<'a>(
        &'a self,
        ctx: &'a ScorerContext<'a, I, O, R>,
    ) -> Pin<Box<dyn Future<Output = Result<Score, ScorerError>> + 'a>> {
        Box::pin(self.score(ctx))
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

#[cfg(test)]
mod tests {
    use super::ScorerSet;
    use crate::{Direction, MapError, Score, ScoreDefinition, Scorer, ScorerContext, ScorerError};
    use std::error::Error;
    use std::fmt::{self, Display, Formatter};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug)]
    struct TestMapError(&'static str);

    impl Display for TestMapError {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            f.write_str(self.0)
        }
    }

    impl Error for TestMapError {}

    struct LengthScorer {
        name: &'static str,
    }

    impl Scorer<String, usize, String> for LengthScorer {
        async fn score(
            &self,
            ctx: &ScorerContext<'_, String, usize, String>,
        ) -> Result<Score, ScorerError> {
            Ok(Score::Binary(
                *ctx.output == ctx.reference.map_or(0, String::len),
            ))
        }

        fn definition(&self) -> ScoreDefinition {
            ScoreDefinition::maximize(self.name)
        }
    }

    struct ReferenceLengthScorer;

    impl Scorer<String, String, usize> for ReferenceLengthScorer {
        async fn score(
            &self,
            ctx: &ScorerContext<'_, String, String, usize>,
        ) -> Result<Score, ScorerError> {
            Ok(Score::Binary(
                ctx.reference
                    .is_some_and(|reference| ctx.output.len() == *reference),
            ))
        }

        fn definition(&self) -> ScoreDefinition {
            ScoreDefinition::maximize("reference_length")
        }
    }

    struct LengthValueScorer {
        name: &'static str,
    }

    impl Scorer<String, usize> for LengthValueScorer {
        async fn score(
            &self,
            ctx: &ScorerContext<'_, String, usize>,
        ) -> Result<Score, ScorerError> {
            Ok(Score::Numeric(*ctx.output as f64))
        }

        fn definition(&self) -> ScoreDefinition {
            ScoreDefinition {
                name: self.name.to_string(),
                direction: Some(Direction::Maximize),
            }
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn scorer_set_builds_with_map_output_and_scored_results() {
        let scorer_set = ScorerSet::<String, String, String>::builder()
            .map_output(|output: &String| Ok(output.len()))
            .scorer(LengthScorer { name: "len_a" })
            .scorer(LengthScorer { name: "len_b" })
            .build();

        let input = String::from("prompt");
        let output = String::from("four");
        let reference = String::from("size");
        let ctx = ScorerContext::new(&input, &output, Some(&reference));

        let results = scorer_set.score(&ctx).await;

        assert_eq!(scorer_set.definitions().len(), 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0.name, "len_a");
        assert_eq!(results[1].0.name, "len_b");
        assert_eq!(results[0].1.as_ref().unwrap(), &Score::Binary(true));
        assert_eq!(results[1].1.as_ref().unwrap(), &Score::Binary(true));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn scorer_set_builds_with_map_reference() {
        let scorer_set = ScorerSet::<String, String, String>::builder()
            .map_reference(|reference: &String| Ok(reference.len()))
            .scorer(ReferenceLengthScorer)
            .build();

        let input = String::from("prompt");
        let output = String::from("four");
        let reference = String::from("size");
        let ctx = ScorerContext::new(&input, &output, Some(&reference));

        let results = scorer_set.score(&ctx).await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.name, "reference_length");
        assert_eq!(results[0].1.as_ref().unwrap(), &Score::Binary(true));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn output_mapper_runs_once_per_trial_and_is_shared() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mapper_calls = Arc::clone(&calls);
        let scorer_set = ScorerSet::<String, String>::builder()
            .map_output(move |output: &String| {
                mapper_calls.fetch_add(1, Ordering::SeqCst);
                Ok(output.len())
            })
            .scorer(LengthValueScorer { name: "len_a" })
            .scorer(LengthValueScorer { name: "len_b" })
            .build();

        let input = String::from("prompt");
        let output = String::from("four");
        let ctx: ScorerContext<'_, String, String> = ScorerContext::new(&input, &output, None);

        let results = scorer_set.score(&ctx).await;

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].1.as_ref().unwrap(), &Score::Numeric(4.0));
        assert_eq!(results[1].1.as_ref().unwrap(), &Score::Numeric(4.0));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mapper_errors_become_scorer_errors_for_all_scorers() {
        let scorer_set = ScorerSet::<String, String>::builder()
            .map_output(|_: &String| Err(MapError(Box::new(TestMapError("mapping failed")))))
            .scorer(LengthValueScorer { name: "len_a" })
            .scorer(LengthValueScorer { name: "len_b" })
            .build();

        let input = String::from("prompt");
        let output = String::from("four");
        let ctx: ScorerContext<'_, String, String> = ScorerContext::new(&input, &output, None);

        let results = scorer_set.score(&ctx).await;

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0.name, "len_a");
        assert_eq!(results[1].0.name, "len_b");
        assert_eq!(
            results[0].1.as_ref().unwrap_err().to_string(),
            "mapping failed"
        );
        assert_eq!(
            results[1].1.as_ref().unwrap_err().to_string(),
            "mapping failed"
        );
    }
}
