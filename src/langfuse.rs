use reqwest::Client;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use crate::{RunResult, Score, SampleResult};

/// Error returned when a Langfuse export fails.
#[derive(Debug)]
pub struct LangfuseExportError(pub Box<dyn Error + Send + Sync>);

impl fmt::Display for LangfuseExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Langfuse export failed: {}", self.0)
    }
}

impl Error for LangfuseExportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.0.as_ref())
    }
}

/// Connection settings for a Langfuse instance.
///
/// ```no_run
/// use evalkit::LangfuseConfig;
///
/// let config = LangfuseConfig {
///     host: "https://cloud.langfuse.com".into(),
///     public_key: std::env::var("LANGFUSE_PUBLIC_KEY").unwrap(),
///     secret_key: std::env::var("LANGFUSE_SECRET_KEY").unwrap(),
/// };
/// ```
pub struct LangfuseConfig {
    /// Base URL of the Langfuse instance (e.g. `"https://cloud.langfuse.com"`).
    pub host: String,
    /// Public key (`pk-lf-...`).
    pub public_key: String,
    /// Secret key (`sk-lf-...`).
    pub secret_key: String,
}

/// Push a completed eval run to Langfuse.
///
/// Creates one trace per sample and one score per scorer per sample.
/// Scores are aggregated across trials:
/// - `Binary` → pass rate (0.0–1.0)
/// - `Numeric` / `Metric` → mean
/// - `Label` → skipped (not representable as a number)
///
/// Events are sent in batches of up to 500 to stay within Langfuse's
/// recommended request size.
pub async fn export_run(
    result: &RunResult,
    config: &LangfuseConfig,
) -> Result<(), LangfuseExportError> {
    let batch = build_batch(result);
    if batch.is_empty() {
        return Ok(());
    }

    let client = Client::new();
    let host = config.host.trim_end_matches('/');
    let url = format!("{host}/api/public/ingestion");

    for chunk in batch.chunks(500) {
        client
            .post(&url)
            .basic_auth(&config.public_key, Some(&config.secret_key))
            .json(&json!({ "batch": chunk }))
            .send()
            .await
            .map_err(|e| LangfuseExportError(Box::new(e)))?
            .error_for_status()
            .map_err(|e| LangfuseExportError(Box::new(e)))?;
    }

    Ok(())
}

fn build_batch(result: &RunResult) -> Vec<Value> {
    let run_id = &result.metadata.run_id;
    let timestamp = result.metadata.completed_at.to_rfc3339();
    let mut batch = Vec::new();

    for sample in &result.samples {
        // Trace ID is stable within a run so scores can be associated to it.
        let trace_id = format!("{run_id}/{}", sample.sample_id);

        batch.push(json!({
            "id": new_event_id(),
            "timestamp": timestamp,
            "type": "trace-create",
            "body": {
                "id": trace_id,
                "name": format!("eval:{run_id}"),
                "metadata": {
                    "run_id": run_id,
                    "sample_id": sample.sample_id,
                    "trial_count": sample.trial_count,
                    "scored_count": sample.scored_count,
                    "error_count": sample.error_count,
                },
                "tags": ["evalkit"]
            }
        }));

        for (scorer_name, (value, comment)) in aggregate_scores(sample) {
            batch.push(json!({
                "id": new_event_id(),
                "timestamp": timestamp,
                "type": "score-create",
                "body": {
                    "id": new_event_id(),
                    "traceId": trace_id,
                    "name": scorer_name,
                    "value": value,
                    "dataType": "NUMERIC",
                    "comment": comment
                }
            }));
        }
    }

    batch
}

/// Aggregate trial scores for a single sample into `scorer_name → (value, comment)`.
fn aggregate_scores(sample: &SampleResult) -> HashMap<String, (f64, String)> {
    let mut buckets: HashMap<String, Vec<f64>> = HashMap::new();

    for trial in &sample.trials {
        for (name, result) in &trial.scores {
            let Some(v) = score_to_f64(result.as_ref().ok()) else {
                continue;
            };
            buckets.entry(name.clone()).or_default().push(v);
        }
    }

    buckets
        .into_iter()
        .map(|(name, values)| {
            let n = values.len();
            let mean = values.iter().sum::<f64>() / n as f64;
            let comment = format!("mean across {n} trial(s)");
            (name, (mean, comment))
        })
        .collect()
}

/// Convert a score to a single f64 value suitable for Langfuse.
///
/// Returns `None` for `Label` scores (no natural numeric representation).
fn score_to_f64(score: Option<&Score>) -> Option<f64> {
    match score? {
        Score::Binary(b) => Some(if *b { 1.0 } else { 0.0 }),
        Score::Numeric(v) => Some(*v),
        Score::Structured { score, .. } => Some(*score),
        Score::Metric { value, .. } => Some(*value),
        Score::Label(_) => None,
    }
}

fn new_event_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RunMetadata, RunResult, SampleResult, Score, ScoreDefinition, TrialResult};
    use chrono::Utc;
    use std::collections::HashMap;
    use std::time::Duration;

    fn make_result(samples: Vec<(&str, Vec<HashMap<&str, Score>>)>) -> RunResult {
        let now = Utc::now();
        RunResult {
            metadata: RunMetadata {
                run_id: "run-abc".into(),
                seed: None,
                dataset_fingerprint: "dataset-langfuse-tests".into(),
                scorer_fingerprint: "scorers-langfuse-tests".into(),
                started_at: now,
                completed_at: now,
                duration: Duration::from_secs(1),
                trial_count: 1,
                score_definitions: vec![],
                acquisition_mode: "inline".into(),
            },
            samples: samples
                .into_iter()
                .map(|(id, trials)| SampleResult {
                    sample_id: id.into(),
                    trial_count: trials.len(),
                    scored_count: trials.len(),
                    error_count: 0,
                    trials: trials
                        .into_iter()
                        .enumerate()
                        .map(|(i, scores)| TrialResult {
                            scores: scores
                                .into_iter()
                                .map(|(k, v)| (k.into(), Ok(v)))
                                .collect(),
                            duration: Duration::from_millis(10),
                            trial_index: i,
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    #[test]
    fn build_batch_creates_trace_and_score_events() {
        let result = make_result(vec![(
            "sample-1",
            vec![HashMap::from([("exact_match", Score::Binary(true))])],
        )]);

        let batch = build_batch(&result);

        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0]["type"], "trace-create");
        assert_eq!(batch[1]["type"], "score-create");
        assert_eq!(batch[1]["body"]["name"], "exact_match");
        assert_eq!(batch[1]["body"]["value"], 1.0_f64);
    }

    #[test]
    fn build_batch_aggregates_binary_scores_as_pass_rate() {
        let result = make_result(vec![(
            "s1",
            vec![
                HashMap::from([("m", Score::Binary(true))]),
                HashMap::from([("m", Score::Binary(false))]),
                HashMap::from([("m", Score::Binary(true))]),
            ],
        )]);

        let batch = build_batch(&result);
        let score_value = batch[1]["body"]["value"].as_f64().unwrap();
        let expected = 2.0 / 3.0;
        assert!((score_value - expected).abs() < 1e-10);
    }

    #[test]
    fn build_batch_aggregates_numeric_scores_as_mean() {
        let result = make_result(vec![(
            "s1",
            vec![
                HashMap::from([("bleu", Score::Numeric(0.6))]),
                HashMap::from([("bleu", Score::Numeric(0.8))]),
            ],
        )]);

        let batch = build_batch(&result);
        let score_value = batch[1]["body"]["value"].as_f64().unwrap();
        assert!((score_value - 0.7).abs() < 1e-10);
    }

    #[test]
    fn build_batch_skips_label_scores() {
        let result = make_result(vec![(
            "s1",
            vec![HashMap::from([("category", Score::Label("A".into()))])],
        )]);

        let batch = build_batch(&result);
        // trace-create only, no score-create for Label
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0]["type"], "trace-create");
    }

    #[test]
    fn build_batch_sets_trace_id_from_run_and_sample_id() {
        let result = make_result(vec![("my-sample", vec![HashMap::new()])]);

        let batch = build_batch(&result);
        assert_eq!(batch[0]["body"]["id"], "run-abc/my-sample");
    }

    #[test]
    fn build_batch_chunks_not_needed_for_small_runs() {
        let result = make_result(
            (0..10)
                .map(|i| {
                    (
                        Box::leak(format!("s{i}").into_boxed_str()) as &str,
                        vec![HashMap::from([("m", Score::Binary(true))])],
                    )
                })
                .collect(),
        );

        let batch = build_batch(&result);
        // 10 traces + 10 scores = 20 events, well under 500
        assert_eq!(batch.len(), 20);
    }
}
