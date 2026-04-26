use evalkit::{
    OutputSourceError, Direction, MapError, Run, RunBuildError, Sample, Score, ScoreDefinition,
    ScoreOutcome, Scorer, ScorerContext, ScorerError, ScorerMetadata, ScorerResources, ScorerSet,
    TokenUsage,
};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

#[derive(Debug)]
struct TestError(&'static str);

impl Display for TestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl Error for TestError {}

struct LengthScorer {
    name: &'static str,
}

impl Scorer<String, usize> for LengthScorer {
    async fn score(&self, ctx: &ScorerContext<'_, String, usize>) -> Result<Score, ScorerError> {
        Ok(Score::Numeric(*ctx.output as f64))
    }

    fn definition(&self) -> ScoreDefinition {
        ScoreDefinition {
            name: self.name.to_string(),
            direction: Some(Direction::Maximize),
        }
    }
}

impl Scorer<String, usize, String> for LengthScorer {
    async fn score(
        &self,
        ctx: &ScorerContext<'_, String, usize, String>,
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

impl Scorer<String, usize, usize> for LengthScorer {
    async fn score(
        &self,
        ctx: &ScorerContext<'_, String, usize, usize>,
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

struct ReferenceLengthScorer;

impl Scorer<String, usize, usize> for ReferenceLengthScorer {
    async fn score(
        &self,
        ctx: &ScorerContext<'_, String, usize, usize>,
    ) -> Result<Score, ScorerError> {
        Ok(Score::Binary(
            ctx.reference
                .is_some_and(|reference| ctx.output == reference),
        ))
    }

    fn definition(&self) -> ScoreDefinition {
        ScoreDefinition::maximize("reference_length")
    }
}

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

struct NaNScorer;

impl Scorer<String, String, String> for NaNScorer {
    async fn score(
        &self,
        _ctx: &ScorerContext<'_, String, String, String>,
    ) -> Result<Score, ScorerError> {
        Ok(Score::Numeric(f64::NAN))
    }

    fn definition(&self) -> ScoreDefinition {
        ScoreDefinition::maximize("nan_score")
    }
}

struct JudgePinnedScorer {
    name: &'static str,
    judge_model_pin: &'static str,
}

struct ResourceReportingScorer {
    name: &'static str,
    score: Score,
    token_usage: TokenUsage,
    cost_usd: Option<f64>,
}

impl Scorer<String, String, String> for ResourceReportingScorer {
    async fn score(
        &self,
        _ctx: &ScorerContext<'_, String, String, String>,
    ) -> Result<Score, ScorerError> {
        Ok(self.score.clone())
    }

    async fn score_with_resources(
        &self,
        _ctx: &ScorerContext<'_, String, String, String>,
    ) -> Result<ScoreOutcome, ScorerError> {
        let mut resources = ScorerResources::default().token_usage(self.token_usage.clone());
        if let Some(cost_usd) = self.cost_usd {
            resources = resources.cost_usd(cost_usd);
        }

        Ok(ScoreOutcome::new(self.score.clone()).with_resources(resources))
    }

    fn definition(&self) -> ScoreDefinition {
        ScoreDefinition::maximize(self.name)
    }
}

impl Scorer<String, String, String> for JudgePinnedScorer {
    async fn score(
        &self,
        _ctx: &ScorerContext<'_, String, String, String>,
    ) -> Result<Score, ScorerError> {
        Ok(Score::Numeric(1.0))
    }

    fn definition(&self) -> ScoreDefinition {
        ScoreDefinition::maximize(self.name)
    }

    fn metadata(&self) -> ScorerMetadata {
        ScorerMetadata::default().judge_model_pin(self.judge_model_pin)
    }
}

#[tokio::test(flavor = "current_thread")]
async fn run_builder_executes_dataset_and_returns_sample_results() {
    let samples = vec![
        Sample::new(String::from("What is 2+2?"), String::from("4")),
        Sample::new(String::from("Capital of France?"), String::from("Paris")),
    ];
    let sample_ids: Vec<_> = samples.iter().map(|sample| sample.id.clone()).collect();

    let run = Run::builder()
        .dataset(samples)
        .source(|input: &String| {
            let output = match input.as_str() {
                "What is 2+2?" => String::from("4"),
                _ => String::from("Paris"),
            };
            async move { Ok::<_, OutputSourceError>(output) }
        })
        .scorer(ExactMatchScorer)
        .build()
        .unwrap();

    let result = run.execute().await.unwrap();

    assert_eq!(result.samples.len(), 2);
    assert_eq!(result.metadata.trial_count, 1);
    assert_eq!(result.metadata.seed, None);
    assert_eq!(result.metadata.source_mode, "inline");
    assert_eq!(result.metadata.dataset_fingerprint.len(), 16);
    assert_eq!(result.metadata.scorer_fingerprint.len(), 16);
    assert_eq!(result.metadata.score_definitions.len(), 1);
    assert_eq!(result.metadata.score_definitions[0].name, "exact_match");
    assert_eq!(result.samples[0].sample_id, sample_ids[0]);
    assert_eq!(result.samples[1].sample_id, sample_ids[1]);
    assert_eq!(result.samples[0].trial_count, 1);
    assert_eq!(result.samples[0].scored_count, 1);
    assert_eq!(result.samples[0].error_count, 0);
    assert_eq!(result.samples[0].trials[0].trial_index, 0);
    assert_eq!(result.samples[0].trials[0].scores.len(), 1);
    assert_eq!(
        result.samples[0].trials[0]
            .scores
            .get("exact_match")
            .unwrap()
            .as_ref()
            .unwrap(),
        &Score::Binary(true)
    );
    assert_eq!(
        result.samples[1].trials[0]
            .scores
            .get("exact_match")
            .unwrap()
            .as_ref()
            .unwrap(),
        &Score::Binary(true)
    );
}

#[tokio::test(flavor = "current_thread")]
async fn run_accepts_multiple_scorers_and_scorer_sets() {
    let sample = Sample::new(String::from("prompt"), String::from("four"));
    let scorer_set = ScorerSet::<String, String, String>::builder()
        .map_output(|output: &String| Ok(output.len()))
        .map_reference(|reference: &String| Ok(reference.len()))
        .scorer(ReferenceLengthScorer)
        .build();

    let run = Run::builder()
        .dataset(vec![sample])
        .source(|_: &String| async { Ok::<_, OutputSourceError>(String::from("four")) })
        .scorer(ExactMatchScorer)
        .scorer_set(scorer_set)
        .trials(2)
        .concurrency(4)
        .build()
        .unwrap();

    let result = run.execute().await.unwrap();
    let first_sample = &result.samples[0];

    assert_eq!(first_sample.trials.len(), 2);
    assert_eq!(first_sample.trial_count, 2);
    assert_eq!(first_sample.scored_count, 2);
    assert_eq!(first_sample.error_count, 0);
    assert_eq!(first_sample.trials[0].scores.len(), 2);
    assert_eq!(
        first_sample.trials[0]
            .scores
            .get("exact_match")
            .unwrap()
            .as_ref()
            .unwrap(),
        &Score::Binary(true)
    );
    assert_eq!(
        first_sample.trials[0]
            .scores
            .get("reference_length")
            .unwrap()
            .as_ref()
            .unwrap(),
        &Score::Binary(true)
    );
}

#[tokio::test(flavor = "current_thread")]
async fn run_aggregates_scorer_resources_into_sample_results() {
    let sample = Sample::new(String::from("prompt"), String::from("reference"));
    let scorer_set = ScorerSet::<String, String, String>::builder()
        .scorer(ResourceReportingScorer {
            name: "judge_b",
            score: Score::Numeric(0.8),
            token_usage: TokenUsage {
                input: 7,
                output: 5,
                cache_read: 2,
                cache_write: 1,
            },
            cost_usd: Some(0.02),
        })
        .build();

    let run = Run::builder()
        .dataset(vec![sample])
        .source(|_: &String| async { Ok::<_, OutputSourceError>(String::from("output")) })
        .scorer(ResourceReportingScorer {
            name: "judge_a",
            score: Score::Numeric(0.5),
            token_usage: TokenUsage {
                input: 11,
                output: 3,
                cache_read: 1,
                cache_write: 0,
            },
            cost_usd: Some(0.01),
        })
        .scorer_set(scorer_set)
        .trials(2)
        .build()
        .unwrap();

    let result = run.execute().await.unwrap();
    let sample = &result.samples[0];

    assert_eq!(sample.token_usage.input, 36);
    assert_eq!(sample.token_usage.output, 16);
    assert_eq!(sample.token_usage.cache_read, 6);
    assert_eq!(sample.token_usage.cache_write, 2);
    assert_eq!(sample.cost_usd, Some(0.06));
}

#[tokio::test(flavor = "current_thread")]
async fn run_metadata_captures_an_explicit_seed() {
    let sample = Sample::new(String::from("prompt"), String::from("four"));

    let run = Run::builder()
        .dataset(vec![sample])
        .source(|_: &String| async { Ok::<_, OutputSourceError>(String::from("four")) })
        .scorer(ExactMatchScorer)
        .seed(42)
        .build()
        .unwrap();

    let result = run.execute().await.unwrap();

    assert_eq!(result.metadata.seed, Some(42));
}

#[tokio::test(flavor = "current_thread")]
async fn run_metadata_auto_populates_code_identity_when_git_is_available() {
    let sample = Sample::new(String::from("prompt"), String::from("four"));

    let run = Run::builder()
        .dataset(vec![sample])
        .source(|_: &String| async { Ok::<_, OutputSourceError>(String::from("four")) })
        .scorer(ExactMatchScorer)
        .build()
        .unwrap();

    let result = run.execute().await.unwrap();

    assert!(result.metadata.code_commit.is_some());
    assert!(result.metadata.code_fingerprint.is_some());
}

#[tokio::test(flavor = "current_thread")]
async fn run_metadata_collects_judge_model_pins_from_standalone_scorers() {
    let sample = Sample::new(String::from("prompt"), String::from("four"));

    let run = Run::builder()
        .dataset(vec![sample])
        .source(|_: &String| async { Ok::<_, OutputSourceError>(String::from("four")) })
        .scorer(JudgePinnedScorer {
            name: "judge_a",
            judge_model_pin: "gpt-4o@2026-04-01",
        })
        .scorer(JudgePinnedScorer {
            name: "judge_b",
            judge_model_pin: "claude-3.7@2026-03-01",
        })
        .build()
        .unwrap();

    let result = run.execute().await.unwrap();

    assert_eq!(
        result.metadata.judge_model_pins,
        vec![
            String::from("claude-3.7@2026-03-01"),
            String::from("gpt-4o@2026-04-01")
        ]
    );
}

#[tokio::test(flavor = "current_thread")]
async fn run_metadata_collects_judge_model_pins_from_scorer_sets() {
    let sample = Sample::new(String::from("prompt"), String::from("four"));
    let scorer_set = ScorerSet::<String, String, String>::builder()
        .scorer(JudgePinnedScorer {
            name: "judge_a",
            judge_model_pin: "gpt-4o@2026-04-01",
        })
        .scorer(JudgePinnedScorer {
            name: "judge_b",
            judge_model_pin: "claude-3.7@2026-03-01",
        })
        .build();

    let run = Run::builder()
        .dataset(vec![sample])
        .source(|_: &String| async { Ok::<_, OutputSourceError>(String::from("four")) })
        .scorer_set(scorer_set)
        .judge_model_pin("gpt-4o@2026-04-01")
        .build()
        .unwrap();

    let result = run.execute().await.unwrap();

    assert_eq!(
        result.metadata.judge_model_pins,
        vec![
            String::from("claude-3.7@2026-03-01"),
            String::from("gpt-4o@2026-04-01")
        ]
    );
}

#[tokio::test(flavor = "current_thread")]
async fn run_metadata_captures_explicit_reproducibility_fields() {
    let sample = Sample::new(String::from("prompt"), String::from("four"));

    let run = Run::builder()
        .dataset(vec![sample])
        .source(|_: &String| async { Ok::<_, OutputSourceError>(String::from("four")) })
        .scorer(ExactMatchScorer)
        .code_commit("abc123")
        .code_fingerprint("tree:deadbeef")
        .judge_model_pin("gpt-4o@2026-04-01")
        .judge_model_pin("claude-3.7@2026-03-01")
        .judge_model_pin("gpt-4o@2026-04-01")
        .build()
        .unwrap();

    let result = run.execute().await.unwrap();

    assert_eq!(result.metadata.code_commit.as_deref(), Some("abc123"));
    assert_eq!(
        result.metadata.code_fingerprint.as_deref(),
        Some("tree:deadbeef")
    );
    assert_eq!(
        result.metadata.judge_model_pins,
        vec![
            String::from("claude-3.7@2026-03-01"),
            String::from("gpt-4o@2026-04-01")
        ]
    );
}

#[tokio::test(flavor = "current_thread")]
async fn global_mappers_apply_before_standalone_scorers_and_scorer_sets() {
    let sample = Sample::new(String::from("prompt"), String::from("four"));
    let scorer_set = ScorerSet::<String, usize, usize>::builder()
        .scorer(ReferenceLengthScorer)
        .build();

    let run = Run::builder()
        .dataset(vec![sample])
        .source(|_: &String| async { Ok::<_, OutputSourceError>(String::from("four")) })
        .map_output(|output: &String| Ok(output.len()))
        .map_reference(|reference: &String| Ok(reference.len()))
        .scorer(LengthScorer { name: "global_len" })
        .scorer_set(scorer_set)
        .build()
        .unwrap();

    let result = run.execute().await.unwrap();
    let scores = &result.samples[0].trials[0].scores;

    assert_eq!(scores.len(), 2);
    assert_eq!(
        scores.get("global_len").unwrap().as_ref().unwrap(),
        &Score::Numeric(4.0)
    );
    assert_eq!(
        scores.get("reference_length").unwrap().as_ref().unwrap(),
        &Score::Binary(true)
    );
}

#[tokio::test(flavor = "current_thread")]
async fn build_validates_duplicates_and_execute_validates_scores_and_timeouts() {
    let duplicate_a = Sample::builder(String::from("prompt-a"))
        .id("duplicate")
        .reference(String::from("a"))
        .build()
        .unwrap();
    let duplicate_b = Sample::builder(String::from("prompt-b"))
        .id("duplicate")
        .reference(String::from("b"))
        .build()
        .unwrap();

    let duplicate_sample_error = match Run::builder()
        .dataset(vec![duplicate_a, duplicate_b])
        .source(|_: &String| async { Ok::<_, OutputSourceError>(String::from("output")) })
        .scorer(ExactMatchScorer)
        .build()
    {
        Err(err) => err,
        Ok(_) => panic!("expected duplicate sample ids to fail"),
    };

    assert!(matches!(
        duplicate_sample_error,
        RunBuildError::DuplicateSampleIds(ids) if ids == vec![String::from("duplicate")]
    ));

    let duplicate_name_error = match Run::builder()
        .dataset(vec![Sample::new(
            String::from("prompt"),
            String::from("ref"),
        )])
        .source(|_: &String| async { Ok::<_, OutputSourceError>(String::from("output")) })
        .scorer(ExactMatchScorer)
        .scorer(ExactMatchScorer)
        .build()
    {
        Err(err) => err,
        Ok(_) => panic!("expected duplicate scorer names to fail"),
    };

    assert!(matches!(
        duplicate_name_error,
        RunBuildError::DuplicateScorerNames(name) if name == "exact_match"
    ));

    let invalid_score_run = Run::builder()
        .dataset(vec![Sample::new(
            String::from("prompt"),
            String::from("ref"),
        )])
        .source(|_: &String| async { Ok::<_, OutputSourceError>(String::from("output")) })
        .scorer(NaNScorer)
        .build()
        .unwrap();

    let invalid_score_result = invalid_score_run.execute().await.unwrap();
    assert_eq!(
        invalid_score_result.samples[0].trials[0]
            .scores
            .get("nan_score")
            .unwrap()
            .as_ref()
            .unwrap_err()
            .to_string(),
        "numeric scores must be finite (not NaN or infinity)"
    );

    let timeout_run = Run::builder()
        .dataset(vec![Sample::new(
            String::from("slow"),
            String::from("reference"),
        )])
        .source(|_: &String| async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            Ok::<_, OutputSourceError>(String::from("late"))
        })
        .scorer(ExactMatchScorer)
        .sample_timeout(Duration::from_millis(1))
        .build()
        .unwrap();

    let timeout_result = timeout_run.execute().await.unwrap();
    assert_eq!(timeout_result.samples[0].scored_count, 0);
    assert_eq!(timeout_result.samples[0].error_count, 1);
    assert_eq!(
        timeout_result.samples[0].trials[0]
            .scores
            .get("exact_match")
            .unwrap()
            .as_ref()
            .unwrap_err()
            .to_string(),
        OutputSourceError::Timeout(Duration::from_millis(1)).to_string()
    );
}

#[tokio::test(flavor = "current_thread")]
async fn global_mapper_failures_propagate_to_every_affected_scorer() {
    let run = Run::builder()
        .dataset(vec![Sample::new(
            String::from("prompt"),
            String::from("ref"),
        )])
        .source(|_: &String| async { Ok::<_, OutputSourceError>(String::from("output")) })
        .map_output(|_: &String| Err(MapError(Box::new(TestError("map failed")))))
        .scorer(LengthScorer { name: "len_a" })
        .scorer(LengthScorer { name: "len_b" })
        .build()
        .unwrap();

    let result = run.execute().await.unwrap();
    let scores = &result.samples[0].trials[0].scores;

    assert_eq!(
        scores
            .get("len_a")
            .unwrap()
            .as_ref()
            .unwrap_err()
            .to_string(),
        "map failed"
    );
    assert_eq!(
        scores
            .get("len_b")
            .unwrap()
            .as_ref()
            .unwrap_err()
            .to_string(),
        "map failed"
    );
}
