//! Executor throughput benchmark for the runtime extraction.
//!
//! Fixture: 5_000 string samples run through `PullExecutor` with an inline
//! acquisition and a single exact-match scorer. Reported numbers:
//!
//! - throughput: samples per second (wall-clock)
//! - peak resident memory: VmHWM from `/proc/self/status`, in KiB
//!
//! Run in release mode:
//!
//!   cargo run --release -p evalkit-runtime --example throughput_bench
//!
//! Interpretation rules (from docs/issues/05):
//!
//! - throughput must not regress by more than 10% vs. the recorded baseline
//! - peak RSS must not regress by more than 15% vs. the recorded baseline
//!
//! Numbers are reported in `docs/benchmarks.md`.

use std::fs;
use std::time::Instant;

use evalkit::{
    AcquisitionError, Dataset, Sample, Score, ScoreDefinition, Scorer, ScorerContext, ScorerError,
    ScorerSet,
};
use evalkit_runtime::{AlwaysSampler, DatasetSource, Executor, NoopSink, PullExecutor};

const SAMPLE_COUNT: usize = 5_000;
const TRIALS: usize = 1;

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

fn build_dataset() -> Dataset<String, String> {
    let samples = (0..SAMPLE_COUNT)
        .map(|i| {
            let input = format!("input-{i}");
            let reference = input.clone();
            Sample::builder(input)
                .id(format!("s-{i}"))
                .reference(reference)
                .build()
                .expect("build sample")
        })
        .collect::<Vec<_>>();
    Dataset::new(samples)
}

fn peak_rss_kib() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            let kib = rest
                .split_whitespace()
                .next()?
                .parse::<u64>()
                .ok()?;
            return Some(kib);
        }
    }
    None
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dataset = build_dataset();
    let acquisition = |input: &String| {
        let output = input.clone();
        async move { Ok::<_, AcquisitionError>(output) }
    };
    let scorer_set = ScorerSet::<String, String, String>::builder()
        .scorer(ExactMatch)
        .build();

    let started = Instant::now();
    let mut executor = PullExecutor::new(
        DatasetSource::new(dataset),
        acquisition,
        scorer_set,
        AlwaysSampler,
        NoopSink,
    )
    .trials(TRIALS);

    let result = executor.execute().await?;
    let elapsed = started.elapsed();

    let throughput = SAMPLE_COUNT as f64 / elapsed.as_secs_f64();
    let rss = peak_rss_kib().unwrap_or(0);

    println!("samples: {SAMPLE_COUNT}");
    println!("trials: {TRIALS}");
    println!("elapsed_ms: {}", elapsed.as_millis());
    println!("throughput_samples_per_sec: {throughput:.1}");
    println!("peak_rss_kib: {rss}");
    println!(
        "sample_count_in_result: {}",
        result.samples.len()
    );

    Ok(())
}
