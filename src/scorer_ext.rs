use futures::join;

use crate::{Score, ScoreDefinition, Scorer, ScorerContext, ScorerError};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::marker::PhantomData;

/// Extension methods for composing scorers into new scorers.
///
/// Implemented automatically for every type that implements [`Scorer`].
pub trait ScorerExt<I, O, R = ()>: Scorer<I, O, R> + Sized {
    /// Combines two scorers with boolean AND: both must return `Binary(true)`.
    ///
    /// Both scorers run concurrently. The result is `Binary(true)` only when
    /// both return `Binary(true)`. If either scorer returns a non-Binary score
    /// the composed scorer returns an error.
    fn and<S>(self, other: S) -> AndScorer<Self, S>
    where
        S: Scorer<I, O, R>,
    {
        let left_name = self.definition().name;
        let right_name = other.definition().name;
        AndScorer {
            left: self,
            right: other,
            definition: ScoreDefinition::maximize(format!("{left_name} AND {right_name}")),
        }
    }

    /// Combines two numeric scorers into a weighted average.
    ///
    /// Both scorers run concurrently. The result is
    /// `Numeric((left * left_weight + right * right_weight) / (left_weight + right_weight))`.
    /// Both scorers must return `Numeric` or `Metric`; any other combination
    /// returns an error. Panics if both weights are zero.
    fn weighted<S>(self, other: S, left_weight: f64, right_weight: f64) -> WeightedScorer<Self, S>
    where
        S: Scorer<I, O, R>,
    {
        assert!(
            left_weight != 0.0 || right_weight != 0.0,
            "at least one weight must be non-zero"
        );
        let left_name = self.definition().name;
        let right_name = other.definition().name;
        WeightedScorer {
            left: self,
            right: other,
            left_weight,
            right_weight,
            definition: ScoreDefinition::maximize(format!("{left_name}+{right_name}")),
        }
    }

    /// Gates the secondary scorer behind a binary check.
    ///
    /// The gate scorer runs first. If it returns `Binary(true)`, the secondary
    /// scorer runs and its result is returned. If the gate returns `Binary(false)`,
    /// execution short-circuits and `Binary(false)` is returned without running
    /// the secondary scorer. If the gate returns a non-Binary score or errors,
    /// the error propagates without running the secondary scorer.
    fn then<S>(self, secondary: S) -> ThenScorer<Self, S>
    where
        S: Scorer<I, O, R>,
    {
        let gate_name = self.definition().name;
        let secondary_def = secondary.definition();
        ThenScorer {
            gate: self,
            secondary,
            definition: ScoreDefinition {
                name: format!("{gate_name} THEN {}", secondary_def.name),
                direction: secondary_def.direction,
            },
        }
    }
}

impl<T, I, O, R> ScorerExt<I, O, R> for T where T: Scorer<I, O, R> {}

// --- AndScorer -----------------------------------------------------------

pub struct AndScorer<A, B> {
    left: A,
    right: B,
    definition: ScoreDefinition,
}

impl<I, O, R, A, B> Scorer<I, O, R> for AndScorer<A, B>
where
    A: Scorer<I, O, R>,
    B: Scorer<I, O, R>,
{
    async fn score(&self, ctx: &ScorerContext<'_, I, O, R>) -> Result<Score, ScorerError> {
        let (left_result, right_result) = join!(self.left.score(ctx), self.right.score(ctx));
        let left_score = left_result
            .map_err(|err| sub_scorer_error("left", &self.left.definition().name, err))?;
        let right_score = right_result
            .map_err(|err| sub_scorer_error("right", &self.right.definition().name, err))?;
        match (left_score, right_score) {
            (Score::Binary(a), Score::Binary(b)) => Ok(Score::Binary(a && b)),
            _ => Err(ScorerError(Box::new(AndTypeMismatchError))),
        }
    }

    fn definition(&self) -> ScoreDefinition {
        self.definition.clone()
    }
}

// --- WeightedScorer ------------------------------------------------------

pub struct WeightedScorer<A, B> {
    left: A,
    right: B,
    left_weight: f64,
    right_weight: f64,
    definition: ScoreDefinition,
}

impl<I, O, R, A, B> Scorer<I, O, R> for WeightedScorer<A, B>
where
    A: Scorer<I, O, R>,
    B: Scorer<I, O, R>,
{
    async fn score(&self, ctx: &ScorerContext<'_, I, O, R>) -> Result<Score, ScorerError> {
        let (left_result, right_result) = join!(self.left.score(ctx), self.right.score(ctx));
        let left_score = left_result
            .map_err(|err| sub_scorer_error("left", &self.left.definition().name, err))?;
        let right_score = right_result
            .map_err(|err| sub_scorer_error("right", &self.right.definition().name, err))?;

        let left_value = numeric_value(left_score)
            .ok_or_else(|| ScorerError(Box::new(WeightedTypeMismatchError)))?;
        let right_value = numeric_value(right_score)
            .ok_or_else(|| ScorerError(Box::new(WeightedTypeMismatchError)))?;

        let total_weight = self.left_weight + self.right_weight;
        Ok(Score::Numeric(
            (left_value * self.left_weight + right_value * self.right_weight) / total_weight,
        ))
    }

    fn definition(&self) -> ScoreDefinition {
        self.definition.clone()
    }
}

// --- ThenScorer ----------------------------------------------------------

pub struct ThenScorer<G, S> {
    gate: G,
    secondary: S,
    definition: ScoreDefinition,
}

impl<I, O, R, G, S> Scorer<I, O, R> for ThenScorer<G, S>
where
    G: Scorer<I, O, R>,
    S: Scorer<I, O, R>,
{
    async fn score(&self, ctx: &ScorerContext<'_, I, O, R>) -> Result<Score, ScorerError> {
        let gate_score = self
            .gate
            .score(ctx)
            .await
            .map_err(|err| sub_scorer_error("gate", &self.gate.definition().name, err))?;

        match gate_score {
            Score::Binary(true) => self
                .secondary
                .score(ctx)
                .await
                .map_err(|err| sub_scorer_error("secondary", &self.secondary.definition().name, err)),
            Score::Binary(false) => Ok(Score::Binary(false)),
            _ => Err(ScorerError(Box::new(ThenGateNotBinaryError {
                scorer_name: self.gate.definition().name,
            }))),
        }
    }

    fn definition(&self) -> ScoreDefinition {
        self.definition.clone()
    }
}

// --- IgnoreReferenceScorer -----------------------------------------------

/// Wraps a `Scorer<I, O>` so it can be used as a `Scorer<I, O, R>` for any R.
///
/// Useful when mixing reference-free scorers (like [`crate::scorers::regex`] or
/// [`crate::scorers::json_schema`]) alongside reference-aware scorers
/// (like [`crate::scorers::exact_match`]) in the same run.
pub struct IgnoreReferenceScorer<I, O, R, S> {
    inner: S,
    definition: ScoreDefinition,
    _marker: PhantomData<fn(I, O, R)>,
}

/// Lifts a `Scorer<I, O>` into `Scorer<I, O, R>` for any `R` by discarding the reference.
pub fn ignore_reference<I, O, R, S>(scorer: S) -> IgnoreReferenceScorer<I, O, R, S>
where
    S: Scorer<I, O>,
{
    let definition = scorer.definition();
    IgnoreReferenceScorer {
        inner: scorer,
        definition,
        _marker: PhantomData,
    }
}

impl<I, O, R, S> Scorer<I, O, R> for IgnoreReferenceScorer<I, O, R, S>
where
    S: Scorer<I, O>,
{
    async fn score(&self, ctx: &ScorerContext<'_, I, O, R>) -> Result<Score, ScorerError> {
        let inner_ctx = ScorerContext {
            input: ctx.input,
            output: ctx.output,
            reference: None,
        };
        self.inner.score(&inner_ctx).await
    }

    fn definition(&self) -> ScoreDefinition {
        self.definition.clone()
    }
}

// --- helpers and error types ---------------------------------------------

fn numeric_value(score: Score) -> Option<f64> {
    match score {
        Score::Numeric(v) => Some(v),
        Score::Metric { value, .. } => Some(value),
        _ => None,
    }
}

fn sub_scorer_error(role: &'static str, scorer_name: &str, source: ScorerError) -> ScorerError {
    ScorerError(Box::new(SubScorerError {
        role,
        scorer_name: scorer_name.to_owned(),
        source,
    }))
}

#[derive(Debug)]
struct SubScorerError {
    role: &'static str,
    scorer_name: String,
    source: ScorerError,
}

impl Display for SubScorerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} scorer '{}' failed: {}",
            self.role, self.scorer_name, self.source
        )
    }
}

impl Error for SubScorerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

#[derive(Debug)]
struct AndTypeMismatchError;

impl Display for AndTypeMismatchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("and() requires both scorers to return Binary scores")
    }
}

impl Error for AndTypeMismatchError {}

#[derive(Debug)]
struct WeightedTypeMismatchError;

impl Display for WeightedTypeMismatchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("weighted() requires both scorers to return Numeric or Metric scores")
    }
}

impl Error for WeightedTypeMismatchError {}

#[derive(Debug)]
struct ThenGateNotBinaryError {
    scorer_name: String,
}

impl Display for ThenGateNotBinaryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "then() gate scorer '{}' must return a Binary score",
            self.scorer_name
        )
    }
}

impl Error for ThenGateNotBinaryError {}

#[cfg(test)]
mod tests {
    use super::ScorerExt;
    use crate::{Direction, Score, ScoreDefinition, Scorer, ScorerContext, ScorerError};

    struct ConstScorer {
        score: Score,
        name: &'static str,
    }

    impl Scorer<(), ()> for ConstScorer {
        async fn score(&self, _ctx: &ScorerContext<'_, (), ()>) -> Result<Score, ScorerError> {
            Ok(self.score.clone())
        }

        fn definition(&self) -> ScoreDefinition {
            ScoreDefinition::maximize(self.name)
        }
    }

    struct FailingScorer(&'static str);

    impl Scorer<(), ()> for FailingScorer {
        async fn score(&self, _ctx: &ScorerContext<'_, (), ()>) -> Result<Score, ScorerError> {
            Err(ScorerError(Box::new(std::io::Error::other(self.0))))
        }

        fn definition(&self) -> ScoreDefinition {
            ScoreDefinition::new(self.0)
        }
    }

    fn ctx() -> ((), (), ScorerContext<'static, (), ()>) {
        // Return owned values alongside the context so lifetimes work in tests.
        // The context holds references, so we need the data to outlive the context.
        // This helper returns a tuple (input, output, ctx) where the context
        // points into the stack — callers use the ctx from the returned tuple.
        ((), (), ScorerContext { input: &(), output: &(), reference: None })
    }

    #[tokio::test(flavor = "current_thread")]
    async fn and_returns_true_when_both_pass() {
        let scorer = ConstScorer { score: Score::Binary(true), name: "a" }
            .and(ConstScorer { score: Score::Binary(true), name: "b" });
        let (i, o, ctx) = ctx();
        let _ = (&i, &o); // suppress unused warnings
        let score = scorer.score(&ctx).await.unwrap();
        assert_eq!(score, Score::Binary(true));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn and_returns_false_when_either_fails() {
        let scorer = ConstScorer { score: Score::Binary(true), name: "a" }
            .and(ConstScorer { score: Score::Binary(false), name: "b" });
        let (i, o, ctx) = ctx();
        let _ = (&i, &o);
        let score = scorer.score(&ctx).await.unwrap();
        assert_eq!(score, Score::Binary(false));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn and_errors_on_non_binary_scores() {
        let scorer = ConstScorer { score: Score::Numeric(1.0), name: "a" }
            .and(ConstScorer { score: Score::Numeric(0.5), name: "b" });
        let (i, o, ctx) = ctx();
        let _ = (&i, &o);
        assert!(scorer.score(&ctx).await.is_err());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn and_wraps_sub_scorer_errors_with_role() {
        let scorer = FailingScorer("left_scorer").and(FailingScorer("right_scorer"));
        let (i, o, ctx) = ctx();
        let _ = (&i, &o);
        let err = scorer.score(&ctx).await.unwrap_err();
        assert!(err.to_string().contains("left scorer"));
        assert!(err.to_string().contains("left_scorer"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn weighted_returns_weighted_average() {
        let scorer = ConstScorer { score: Score::Numeric(0.8), name: "a" }
            .weighted(ConstScorer { score: Score::Numeric(0.4), name: "b" }, 0.7, 0.3);
        let (i, o, ctx) = ctx();
        let _ = (&i, &o);
        let score = scorer.score(&ctx).await.unwrap();
        // 0.7*0.8 + 0.3*0.4 = 0.56 + 0.12 = 0.68
        assert!(matches!(score, Score::Numeric(v) if (v - 0.68).abs() < 1e-10));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn weighted_accepts_metric_scores() {
        let scorer = ConstScorer {
            score: Score::Metric { name: "latency".into(), value: 100.0, unit: None },
            name: "a",
        }
        .weighted(
            ConstScorer {
                score: Score::Metric { name: "cost".into(), value: 0.5, unit: None },
                name: "b",
            },
            0.5,
            0.5,
        );
        let (i, o, ctx) = ctx();
        let _ = (&i, &o);
        let score = scorer.score(&ctx).await.unwrap();
        assert!(matches!(score, Score::Numeric(v) if (v - 50.25).abs() < 1e-10));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn weighted_errors_on_binary_scores() {
        let scorer = ConstScorer { score: Score::Binary(true), name: "a" }
            .weighted(ConstScorer { score: Score::Binary(true), name: "b" }, 0.5, 0.5);
        let (i, o, ctx) = ctx();
        let _ = (&i, &o);
        assert!(scorer.score(&ctx).await.is_err());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn then_runs_secondary_when_gate_passes() {
        let scorer = ConstScorer { score: Score::Binary(true), name: "gate" }
            .then(ConstScorer { score: Score::Numeric(0.9), name: "quality" });
        let (i, o, ctx) = ctx();
        let _ = (&i, &o);
        let score = scorer.score(&ctx).await.unwrap();
        assert_eq!(score, Score::Numeric(0.9));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn then_short_circuits_when_gate_fails() {
        let scorer = ConstScorer { score: Score::Binary(false), name: "gate" }
            .then(FailingScorer("should_not_run"));
        let (i, o, ctx) = ctx();
        let _ = (&i, &o);
        let score = scorer.score(&ctx).await.unwrap();
        assert_eq!(score, Score::Binary(false));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn then_errors_when_gate_returns_non_binary() {
        let scorer = ConstScorer { score: Score::Numeric(1.0), name: "gate" }
            .then(ConstScorer { score: Score::Binary(true), name: "secondary" });
        let (i, o, ctx) = ctx();
        let _ = (&i, &o);
        let err = scorer.score(&ctx).await.unwrap_err();
        assert!(err.to_string().contains("gate scorer"));
        assert!(err.to_string().contains("Binary"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn composition_is_recursive() {
        // (a AND b) THEN c — composed scorer is itself a Scorer
        let scorer = ConstScorer { score: Score::Binary(true), name: "a" }
            .and(ConstScorer { score: Score::Binary(true), name: "b" })
            .then(ConstScorer { score: Score::Numeric(0.95), name: "c" });
        let (i, o, ctx) = ctx();
        let _ = (&i, &o);
        let score = scorer.score(&ctx).await.unwrap();
        assert_eq!(score, Score::Numeric(0.95));
    }

    #[test]
    fn and_definition_combines_names_and_uses_maximize() {
        let scorer = ConstScorer { score: Score::Binary(true), name: "format" }
            .and(ConstScorer { score: Score::Binary(true), name: "accuracy" });
        let def = scorer.definition();
        assert_eq!(def.name, "format AND accuracy");
        assert_eq!(def.direction, Some(Direction::Maximize));
    }

    #[test]
    fn weighted_definition_combines_names() {
        let scorer = ConstScorer { score: Score::Numeric(0.8), name: "fluency" }
            .weighted(ConstScorer { score: Score::Numeric(0.6), name: "factuality" }, 0.7, 0.3);
        let def = scorer.definition();
        assert_eq!(def.name, "fluency+factuality");
    }

    #[test]
    fn then_definition_uses_secondary_direction() {
        let scorer = ConstScorer { score: Score::Binary(true), name: "format_check" }
            .then(ConstScorer { score: Score::Numeric(0.9), name: "quality" });
        let def = scorer.definition();
        assert_eq!(def.name, "format_check THEN quality");
        assert_eq!(def.direction, Some(Direction::Maximize));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn ignore_reference_passes_score_through() {
        let base = ConstScorer { score: Score::Binary(true), name: "check" };
        let wrapped = super::ignore_reference::<(), (), String, _>(base);
        // reference is Some(&"ref"), but the inner scorer (R=()) should not see it
        let reference = String::from("ignored");
        let ctx = ScorerContext { input: &(), output: &(), reference: Some(&reference) };
        let score = wrapped.score(&ctx).await.unwrap();
        assert_eq!(score, Score::Binary(true));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn ignore_reference_works_without_reference() {
        let base = ConstScorer { score: Score::Numeric(0.75), name: "quality" };
        let wrapped = super::ignore_reference::<(), (), String, _>(base);
        let ctx = ScorerContext { input: &(), output: &(), reference: None };
        let score = wrapped.score(&ctx).await.unwrap();
        assert_eq!(score, Score::Numeric(0.75));
    }

    #[test]
    fn ignore_reference_inherits_inner_definition() {
        let base = ConstScorer { score: Score::Binary(true), name: "regex" };
        let wrapped = super::ignore_reference::<(), (), String, _>(base);
        assert_eq!(wrapped.definition().name, "regex");
    }
}
