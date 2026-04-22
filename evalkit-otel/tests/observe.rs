use chrono::{DateTime, Utc};
use evalkit::{
    AcquisitionError, Run, RunBuildError, Sample, Score, ScoreDefinition, Scorer, ScorerContext,
    ScorerError,
};
use evalkit_otel::{Observe, Span, SpanEvent, TraceBackend, TraceBackendError};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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

struct RecordingBackend {
    grouped_spans: HashMap<String, Vec<Span>>,
    calls: Arc<Mutex<Vec<(String, String, Duration)>>>,
}

impl TraceBackend for RecordingBackend {
    async fn fetch_spans(
        &self,
        correlation_id: &str,
        sample_attribute: &str,
        timeout: Duration,
    ) -> Result<HashMap<String, Vec<Span>>, TraceBackendError> {
        self.calls.lock().unwrap().push((
            correlation_id.to_string(),
            sample_attribute.to_string(),
            timeout,
        ));
        Ok(self.grouped_spans.clone())
    }
}

struct DelayedBackend {
    delay: Duration,
}

impl TraceBackend for DelayedBackend {
    async fn fetch_spans(
        &self,
        _correlation_id: &str,
        _sample_attribute: &str,
        _timeout: Duration,
    ) -> Result<HashMap<String, Vec<Span>>, TraceBackendError> {
        tokio::time::sleep(self.delay).await;
        Ok(HashMap::new())
    }
}

#[tokio::test(flavor = "current_thread")]
async fn observe_mode_runs_from_grouped_spans_and_scores_like_inline_mode() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let sample = Sample::builder(String::from("prompt"))
        .id("sample-a")
        .reference(String::from("hello"))
        .build()
        .unwrap();
    let observe = Observe::builder()
        .backend(RecordingBackend {
            grouped_spans: HashMap::from([(
                String::from("sample-a"),
                vec![response_span("sample-a", "hello")],
            )]),
            calls: Arc::clone(&calls),
        })
        .correlation_id("run-abc-123")
        .sample_attribute("eval.sample_id")
        .timeout(Duration::from_millis(25))
        .build();

    let run = Run::builder()
        .dataset(vec![sample])
        .acquisition(observe)
        .map_output(|spans: &Vec<Span>| Ok(output_attribute(spans)))
        .scorer(ExactMatchScorer)
        .trials(2)
        .build()
        .unwrap();

    let result = run.execute().await.unwrap();

    assert_eq!(result.metadata.acquisition_mode, "observe");
    assert_eq!(result.samples[0].trial_count, 2);
    assert_eq!(result.samples[0].scored_count, 2);
    assert_eq!(result.samples[0].error_count, 0);
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
        calls.lock().unwrap().as_slice(),
        &[(
            String::from("run-abc-123"),
            String::from("eval.sample_id"),
            Duration::from_millis(25),
        )]
    );
}

#[tokio::test(flavor = "current_thread")]
async fn observe_mode_maps_missing_sample_spans_to_trace_not_found() {
    let sample = Sample::builder(String::from("prompt"))
        .id("sample-missing")
        .reference(String::from("hello"))
        .build()
        .unwrap();
    let observe = Observe::builder()
        .backend(RecordingBackend {
            grouped_spans: HashMap::from([(
                String::from("sample-other"),
                vec![response_span("sample-other", "hello")],
            )]),
            calls: Arc::new(Mutex::new(Vec::new())),
        })
        .correlation_id("run-missing")
        .sample_attribute("eval.sample_id")
        .timeout(Duration::from_millis(25))
        .build();

    let run = Run::builder()
        .dataset(vec![sample])
        .acquisition(observe)
        .map_output(|spans: &Vec<Span>| Ok(output_attribute(spans)))
        .scorer(ExactMatchScorer)
        .build()
        .unwrap();

    let result = run.execute().await.unwrap();

    assert_eq!(result.samples[0].scored_count, 0);
    assert_eq!(result.samples[0].error_count, 1);
    assert_eq!(
        result.samples[0].trials[0]
            .scores
            .get("exact_match")
            .unwrap()
            .as_ref()
            .unwrap_err()
            .to_string(),
        AcquisitionError::TraceNotFound {
            correlation_id: String::from("run-missing"),
            sample_id: String::from("sample-missing"),
        }
        .to_string()
    );
}

#[tokio::test(flavor = "current_thread")]
async fn observe_mode_uses_collection_timeout_for_backend_fetches() {
    let sample = Sample::builder(String::from("prompt"))
        .id("sample-timeout")
        .reference(String::from("hello"))
        .build()
        .unwrap();
    let observe = Observe::builder()
        .backend(DelayedBackend {
            delay: Duration::from_millis(20),
        })
        .correlation_id("run-timeout")
        .sample_attribute("eval.sample_id")
        .timeout(Duration::from_millis(1))
        .build();

    let run = Run::builder()
        .dataset(vec![sample])
        .acquisition(observe)
        .map_output(|spans: &Vec<Span>| Ok(output_attribute(spans)))
        .scorer(ExactMatchScorer)
        .sample_timeout(Duration::from_millis(50))
        .build()
        .unwrap();

    let result = run.execute().await.unwrap();

    assert_eq!(
        result.samples[0].trials[0]
            .scores
            .get("exact_match")
            .unwrap()
            .as_ref()
            .unwrap_err()
            .to_string(),
        AcquisitionError::Timeout(Duration::from_millis(1)).to_string()
    );
}

#[test]
fn observe_mode_requires_non_generated_sample_ids() {
    let observe = Observe::builder()
        .backend(RecordingBackend {
            grouped_spans: HashMap::new(),
            calls: Arc::new(Mutex::new(Vec::new())),
        })
        .correlation_id("run-build")
        .sample_attribute("eval.sample_id")
        .timeout(Duration::from_secs(1))
        .build();

    let build_error = match Run::builder()
        .dataset(vec![Sample::new(
            String::from("prompt"),
            String::from("hello"),
        )])
        .acquisition(observe)
        .map_output(|spans: &Vec<Span>| Ok(output_attribute(spans)))
        .scorer(ExactMatchScorer)
        .build()
    {
        Err(err) => err,
        Ok(_) => panic!("expected observe-mode build to reject generated sample ids"),
    };

    assert!(matches!(build_error, RunBuildError::MissingSampleIds));
}

fn response_span(sample_id: &str, output: &str) -> Span {
    Span {
        trace_id: String::from("trace-1"),
        span_id: String::from("span-1"),
        parent_span_id: None,
        operation_name: String::from("llm.call"),
        start_time: parse_time("2026-04-03T10:00:00Z"),
        end_time: parse_time("2026-04-03T10:00:01Z"),
        attributes: HashMap::from([
            (String::from("eval.sample_id"), json!(sample_id)),
            (String::from("output"), json!(output)),
        ]),
        events: vec![SpanEvent {
            name: String::from("response.ready"),
            timestamp: parse_time("2026-04-03T10:00:00Z"),
            attributes: HashMap::new(),
        }],
    }
}

fn output_attribute(spans: &[Span]) -> String {
    spans[0]
        .attributes
        .get("output")
        .and_then(|value| value.as_str())
        .unwrap()
        .to_string()
}

fn parse_time(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .unwrap()
        .with_timezone(&Utc)
}
