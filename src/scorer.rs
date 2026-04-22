use crate::{Score, ScoreDefinition, ScorerContext, ScorerError};

#[allow(async_fn_in_trait)]
pub trait Scorer<I, O, R = ()>: Send + Sync {
    async fn score(&self, ctx: &ScorerContext<'_, I, O, R>) -> Result<Score, ScorerError>;

    fn definition(&self) -> ScoreDefinition;
}

#[cfg(test)]
mod tests {
    use super::Scorer;
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
}
