//! Quickstart example for the happy-path `Eval` facade.
//!
//! Run with: cargo run --example quickstart

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

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let samples = vec![
        Sample::new("What is 2 + 2?".to_string(), "4".to_string()),
        Sample::new(
            "What is the capital of France?".to_string(),
            "Paris".to_string(),
        ),
    ];

    let source = |input: &String| {
        let answer = match input.as_str() {
            "What is 2 + 2?" => "4",
            "What is the capital of France?" => "Paris",
            _ => "",
        }
        .to_string();
        async move { Ok::<_, OutputSourceError>(answer) }
    };

    let result = Eval::new(samples)
        .source(source)
        .scorer(ExactMatchScorer)
        .trials(3)
        .run()
        .await?;

    println!("{}", result.stats().summary());

    Ok(())
}
