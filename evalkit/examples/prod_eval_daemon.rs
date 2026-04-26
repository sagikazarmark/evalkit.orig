//! Minimal Phase 2 style daemon example built from library primitives.
//!
//! Run with: cargo run --example prod_eval_daemon

use evalkit::prelude::*;
use evalkit_otel::OtelResultSink;
use evalkit_runtime::{AlwaysSampler, DatasetSource, Executor, PullExecutor};

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
    let dataset = Dataset::new(vec![
        Sample::new("hello".to_string(), "echo::hello".to_string()),
        Sample::new("world".to_string(), "echo::world".to_string()),
    ]);

    let source = |input: &String| {
        let output = format!("echo::{input}");
        async move { Ok::<_, OutputSourceError>(output) }
    };
    let scorer_set = ScorerSet::<String, String, String>::builder()
        .scorer(ExactMatchScorer)
        .build();

    let sink = OtelResultSink::new();
    let spans = sink.spans();

    let mut executor = PullExecutor::new(
        DatasetSource::new(dataset),
        source,
        scorer_set,
        AlwaysSampler,
        sink,
    )
    .trials(2);

    let result = executor.execute().await?;

    println!("{}", result.stats().summary());
    println!(
        "emitted_spans: {}",
        spans.lock().expect("OTel span sink mutex poisoned").len()
    );

    Ok(())
}
