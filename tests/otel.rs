#![cfg(feature = "otel")]

use chrono::{DateTime, Utc};
use evalkit::{JaegerBackend, Span, SpanEvent, TraceBackend, TraceBackendError};
use serde_json::json;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct StaticBackend;

impl TraceBackend for StaticBackend {
    async fn fetch_spans(
        &self,
        correlation_id: &str,
        sample_attribute: &str,
        _timeout: Duration,
    ) -> Result<HashMap<String, Vec<Span>>, TraceBackendError> {
        let mut grouped = HashMap::new();
        grouped.insert(
            String::from("sample-1"),
            vec![Span {
                trace_id: correlation_id.to_string(),
                span_id: String::from("span-1"),
                parent_span_id: None,
                operation_name: sample_attribute.to_string(),
                start_time: parse_time("2026-04-03T10:00:00Z"),
                end_time: parse_time("2026-04-03T10:00:01Z"),
                attributes: HashMap::from([(String::from(sample_attribute), json!("sample-1"))]),
                events: vec![SpanEvent {
                    name: String::from("generated"),
                    timestamp: parse_time("2026-04-03T10:00:00Z"),
                    attributes: HashMap::new(),
                }],
            }],
        );
        Ok(grouped)
    }
}

#[tokio::test(flavor = "current_thread")]
async fn trace_backend_trait_supports_custom_implementations() {
    let backend = StaticBackend;

    let spans = backend
        .fetch_spans("run-123", "eval.sample_id", Duration::from_secs(1))
        .await
        .unwrap();

    assert_eq!(spans.len(), 1);
    assert_eq!(spans["sample-1"][0].trace_id, "run-123");
    assert_eq!(spans["sample-1"][0].operation_name, "eval.sample_id");
}

#[tokio::test(flavor = "current_thread")]
async fn jaeger_backend_groups_matching_spans_by_sample_attribute() {
    let response = json!({
        "data": [
            {
                "spans": [
                    {
                        "traceID": "trace-a",
                        "spanID": "span-1",
                        "references": [],
                        "operationName": "llm.call",
                        "startTime": 1712131200000000i64,
                        "duration": 1500,
                        "tags": [
                            {"key": "eval.run_id", "type": "string", "value": "run-123"},
                            {"key": "eval.sample_id", "type": "string", "value": "sample-a"},
                            {"key": "response", "type": "string", "value": "hello"}
                        ],
                        "logs": [
                            {
                                "timestamp": 1712131200000500i64,
                                "fields": [
                                    {"key": "event", "type": "string", "value": "parsed"}
                                ]
                            }
                        ]
                    },
                    {
                        "traceID": "trace-a",
                        "spanID": "span-2",
                        "references": [{"refType": "CHILD_OF", "spanID": "span-1"}],
                        "operationName": "tool.call",
                        "startTime": 1712131200002000i64,
                        "duration": 500,
                        "tags": [
                            {"key": "eval.run_id", "type": "string", "value": "run-123"},
                            {"key": "eval.sample_id", "type": "string", "value": "sample-a"}
                        ],
                        "logs": []
                    },
                    {
                        "traceID": "trace-b",
                        "spanID": "span-3",
                        "references": [],
                        "operationName": "llm.call",
                        "startTime": 1712131200004000i64,
                        "duration": 1000,
                        "tags": [
                            {"key": "eval.run_id", "type": "string", "value": "run-123"},
                            {"key": "eval.sample_id", "type": "string", "value": "sample-b"}
                        ],
                        "logs": []
                    }
                ]
            }
        ]
    });
    let (base_url, requests, server) =
        spawn_http_server(vec![http_response(200, response.to_string())]);

    let backend = JaegerBackend::new(base_url)
        .with_header("x-api-key", "secret-token")
        .unwrap();
    let spans = backend
        .fetch_spans("run-123", "eval.sample_id", Duration::from_secs(1))
        .await
        .unwrap();

    server.join().unwrap();

    assert_eq!(spans.len(), 2);
    assert_eq!(spans["sample-a"].len(), 2);
    assert_eq!(spans["sample-b"].len(), 1);
    assert_eq!(spans["sample-a"][0].events[0].name, "parsed");
    assert_eq!(
        spans["sample-a"][1].parent_span_id.as_deref(),
        Some("span-1")
    );

    let request = requests.lock().unwrap()[0].to_ascii_lowercase();
    assert!(request.contains("get /api/traces?tags="));
    assert!(request.contains("run-123"));
    assert!(request.contains("x-api-key: secret-token"));
}

#[tokio::test(flavor = "current_thread")]
async fn jaeger_backend_retries_failed_requests_before_returning() {
    let retry_response = http_response(500, String::from("{"));
    let success_response = http_response(200, json!({ "data": [] }).to_string());
    let (base_url, requests, server) = spawn_http_server(vec![retry_response, success_response]);

    let backend = JaegerBackend::new(base_url).with_retry_count(1);
    let spans = backend
        .fetch_spans("run-456", "eval.sample_id", Duration::from_secs(1))
        .await
        .unwrap();

    server.join().unwrap();

    assert!(spans.is_empty());
    assert_eq!(requests.lock().unwrap().len(), 2);
}

fn parse_time(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .unwrap()
        .with_timezone(&Utc)
}

fn http_response(status: u16, body: String) -> String {
    let status_text = match status {
        200 => "OK",
        500 => "Internal Server Error",
        _ => "Status",
    };

    format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
}

fn spawn_http_server(
    responses: Vec<String>,
) -> (String, Arc<Mutex<Vec<String>>>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let recorded_requests = Arc::clone(&requests);

    let handle = thread::spawn(move || {
        for response in responses {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 8192];
            let read = stream.read(&mut buffer).unwrap();
            recorded_requests
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(&buffer[..read]).into_owned());
            stream.write_all(response.as_bytes()).unwrap();
            stream.flush().unwrap();
        }
    });

    (address, requests, handle)
}
