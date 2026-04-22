use crate::{Score, ScoreDefinition, ScorerContext, ScorerError, TokenUsage};

#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct ScorerResources {
    pub token_usage: TokenUsage,
    pub cost_usd: Option<f64>,
}

impl ScorerResources {
    pub fn token_usage(mut self, token_usage: TokenUsage) -> Self {
        self.token_usage = token_usage;
        self
    }

    pub fn cost_usd(mut self, cost_usd: f64) -> Self {
        self.cost_usd = Some(cost_usd);
        self
    }

    pub fn merge(&mut self, other: &Self) {
        self.token_usage.input += other.token_usage.input;
        self.token_usage.output += other.token_usage.output;
        self.token_usage.cache_read += other.token_usage.cache_read;
        self.token_usage.cache_write += other.token_usage.cache_write;

        self.cost_usd = match (self.cost_usd, other.cost_usd) {
            (Some(left), Some(right)) => Some(left + right),
            (Some(left), None) => Some(left),
            (None, Some(right)) => Some(right),
            (None, None) => None,
        };
    }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ScoreOutcome {
    pub score: Score,
    pub resources: ScorerResources,
}

impl ScoreOutcome {
    pub fn new(score: Score) -> Self {
        Self {
            score,
            resources: ScorerResources::default(),
        }
    }

    pub fn with_resources(mut self, resources: ScorerResources) -> Self {
        self.resources = resources;
        self
    }
}

impl From<Score> for ScoreOutcome {
    fn from(score: Score) -> Self {
        Self::new(score)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ScorerMetadata {
    pub judge_model_pins: Vec<String>,
}

impl ScorerMetadata {
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
}

#[allow(async_fn_in_trait)]
pub trait Scorer<I, O, R = ()>: Send + Sync {
    async fn score(&self, ctx: &ScorerContext<'_, I, O, R>) -> Result<Score, ScorerError>;

    async fn score_with_resources(
        &self,
        ctx: &ScorerContext<'_, I, O, R>,
    ) -> Result<ScoreOutcome, ScorerError> {
        self.score(ctx).await.map(ScoreOutcome::from)
    }

    fn definition(&self) -> ScoreDefinition;

    fn metadata(&self) -> ScorerMetadata {
        ScorerMetadata::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{ScoreOutcome, Scorer, ScorerMetadata};
    use crate::{Direction, Score, ScoreDefinition, ScorerContext, ScorerError};
    use std::error::Error;
    use std::fmt::{self, Display, Formatter};

    #[derive(Debug)]
    struct TestError(&'static str);

    impl Display for TestError {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            f.write_str(self.0)
        }
    }

    impl Error for TestError {}

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

    struct ContainsScorer;

    impl Scorer<String, String> for ContainsScorer {
        async fn score(
            &self,
            ctx: &ScorerContext<'_, String, String>,
        ) -> Result<Score, ScorerError> {
            Ok(Score::Binary(ctx.output.contains(ctx.input)))
        }

        fn definition(&self) -> ScoreDefinition {
            ScoreDefinition::maximize("contains")
        }
    }

    struct FailingScorer;

    impl Scorer<String, String, String> for FailingScorer {
        async fn score(
            &self,
            _ctx: &ScorerContext<'_, String, String, String>,
        ) -> Result<Score, ScorerError> {
            Err(ScorerError::internal(TestError(
                "invalid scorer configuration",
            )))
        }

        fn definition(&self) -> ScoreDefinition {
            ScoreDefinition::new("failing")
        }
    }

    fn assert_send_sync<T: Send + Sync>() {}

    #[tokio::test(flavor = "current_thread")]
    async fn scorer_score_returns_score_result() {
        let input = String::from("What is 2 + 2?");
        let output = String::from("4");
        let reference = String::from("4");
        let scorer = ExactMatchScorer;
        let ctx = ScorerContext::new(&input, &output, Some(&reference));

        let score = scorer.score(&ctx).await.unwrap();

        assert_eq!(score, Score::Binary(true));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn scorer_errors_are_distinct_from_scores() {
        let input = String::from("prompt");
        let output = String::from("answer");
        let reference = String::from("reference");
        let scorer = FailingScorer;
        let ctx = ScorerContext::new(&input, &output, Some(&reference));

        let err = scorer.score(&ctx).await.unwrap_err();

        assert_eq!(err.to_string(), "invalid scorer configuration");
    }

    #[test]
    fn scorer_definition_returns_name_and_direction() {
        let definition = ExactMatchScorer.definition();

        assert_eq!(definition.name, "exact_match");
        assert_eq!(definition.direction, Some(Direction::Maximize));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn scorer_trait_supports_default_reference_type_and_send_sync() {
        assert_send_sync::<ContainsScorer>();

        let input = String::from("needle");
        let output = String::from("haystack with needle inside");
        let scorer = ContainsScorer;
        let ctx: ScorerContext<'_, String, String> = ScorerContext::new(&input, &output, None);

        let score = scorer.score(&ctx).await.unwrap();

        assert_eq!(score, Score::Binary(true));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn scorer_score_with_resources_defaults_to_empty_resources() {
        let input = String::from("needle");
        let output = String::from("haystack with needle inside");
        let scorer = ContainsScorer;
        let ctx: ScorerContext<'_, String, String> = ScorerContext::new(&input, &output, None);

        let outcome = scorer.score_with_resources(&ctx).await.unwrap();

        assert_eq!(outcome, ScoreOutcome::new(Score::Binary(true)));
    }

    #[test]
    fn scorer_metadata_defaults_to_no_judge_model_pins() {
        let scorer = ContainsScorer;

        assert_eq!(scorer.metadata(), ScorerMetadata::default());
    }
}
