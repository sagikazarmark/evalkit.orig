//! End-to-end example: evaluate a mock question-answering "model".
//!
//! Run with: cargo run --example basic

use std::time::Duration;

use evalkit::prelude::*;

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

struct ContainsReferenceScorer;

impl Scorer<String, String, String> for ContainsReferenceScorer {
    async fn score(
        &self,
        ctx: &ScorerContext<'_, String, String, String>,
    ) -> Result<Score, ScorerError> {
        Ok(Score::Binary(
            ctx.reference
                .is_some_and(|reference| ctx.output.contains(reference)),
        ))
    }

    fn definition(&self) -> ScoreDefinition {
        ScoreDefinition::maximize("contains")
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- Dataset -------------------------------------------------------
    // Each Sample pairs an input with the expected (reference) answer.
    let samples = vec![
        Sample::new("What is 2 + 2?".to_string(), "4".to_string()),
        Sample::new(
            "What is the capital of France?".to_string(),
            "Paris".to_string(),
        ),
        // Explicit ID: useful when you want stable cross-run comparisons.
        Sample::builder("What color is the sky?".to_string())
            .id("sky-color")
            .reference("blue".to_string())
            .build()?,
    ];
    let dataset = Dataset::new(samples);

    // --- Source --------------------------------------------------------
    // In production this would call an LLM. Here we return canned answers
    // so the example runs without any API key.
    let source = |input: &String| {
        let answer = match input.as_str() {
            "What is 2 + 2?" => "4",
            "What is the capital of France?" => "Paris",
            // Intentionally wrong to show a non-perfect pass rate.
            "What color is the sky?" => "The sky is blue",
            _ => "",
        }
        .to_string();
        async move { Ok::<_, OutputSourceError>(answer) }
    };

    // --- Run -----------------------------------------------------------
    let run = Run::builder()
        .dataset(dataset)
        .source(source)
        // exact_match: output must equal reference exactly.
        .scorer(ExactMatchScorer)
        // contains: output must contain the reference as a substring.
        .scorer(ContainsReferenceScorer)
        .trials(3)
        .sample_timeout(Duration::from_secs(5))
        .build()?;

    let result = run.execute().await?;

    // --- Results -------------------------------------------------------
    let stats = result.stats();
    println!("{}", stats.summary());
    println!("\ntotal_trials_executed: {}", stats.total_trials_executed);

    Ok(())
}
