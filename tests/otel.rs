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

// ---------------------------------------------------------------------------
// OtlpReceiver tests
// ---------------------------------------------------------------------------

use evalkit::OtlpReceiver;
use std::net::TcpStream;

/// Send a single OTLP/HTTP POST on tokio's blocking thread pool.
///
/// Using `spawn_blocking` keeps the current-thread tokio runtime free to run
/// the server accept/handler tasks concurrently with the blocking TCP exchange.
/// The thread MUST drain the server response before closing the socket — dropping
/// without reading causes the OS to send TCP RST, aborting the in-flight request.
async fn post_otlp(port: u16, body: &str) {
    let body = body.to_owned();
    tokio::task::spawn_blocking(move || {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let request = format!(
            "POST /v1/traces HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(request.as_bytes()).unwrap();
        // Drain the response before closing so the OS sends FIN, not RST.
        let mut buf = [0u8; 256];
        loop {
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn otlp_receiver_stores_spans_by_correlation_id() {
    let receiver = OtlpReceiver::start_on_port(0).await.unwrap();
    let port = receiver.port();

    let now_ns = Utc::now().timestamp_nanos_opt().unwrap();
    let body = json!({
        "resourceSpans": [{
            "scopeSpans": [{
                "spans": [{
                    "traceId": "abc123",
                    "spanId": "def456",
                    "name": "llm.call",
                    "startTimeUnixNano": now_ns.to_string(),
                    "endTimeUnixNano": (now_ns + 1_000_000).to_string(),
                    "attributes": [
                        {"key": "eval.run_id", "value": {"stringValue": "run-xyz"}},
                        {"key": "eval.sample_id", "value": {"stringValue": "sample-1"}}
                    ]
                }]
            }]
        }]
    })
    .to_string();

    post_otlp(port, &body).await;
    // No explicit sleep: fetch_spans polls every 200 ms until spans arrive.

    let spans = receiver
        .fetch_spans("run-xyz", "eval.sample_id", Duration::from_secs(1))
        .await
        .unwrap();

    assert_eq!(spans.len(), 1);
    assert_eq!(spans["sample-1"].len(), 1);
    assert_eq!(spans["sample-1"][0].span_id, "def456");
    assert_eq!(spans["sample-1"][0].operation_name, "llm.call");
}

#[tokio::test(flavor = "current_thread")]
async fn otlp_receiver_groups_spans_across_multiple_samples() {
    let receiver = OtlpReceiver::start_on_port(0).await.unwrap();
    let port = receiver.port();

    let now_ns = Utc::now().timestamp_nanos_opt().unwrap();
    let body = json!({
        "resourceSpans": [{
            "scopeSpans": [{
                "spans": [
                    {
                        "traceId": "t1",
                        "spanId": "s1",
                        "name": "op",
                        "startTimeUnixNano": now_ns.to_string(),
                        "endTimeUnixNano": (now_ns + 1_000).to_string(),
                        "attributes": [
                            {"key": "eval.run_id", "value": {"stringValue": "run-multi"}},
                            {"key": "eval.sample_id", "value": {"stringValue": "sample-a"}}
                        ]
                    },
                    {
                        "traceId": "t2",
                        "spanId": "s2",
                        "name": "op",
                        "startTimeUnixNano": now_ns.to_string(),
                        "endTimeUnixNano": (now_ns + 1_000).to_string(),
                        "attributes": [
                            {"key": "eval.run_id", "value": {"stringValue": "run-multi"}},
                            {"key": "eval.sample_id", "value": {"stringValue": "sample-b"}}
                        ]
                    },
                    {
                        "traceId": "t3",
                        "spanId": "s3",
                        "name": "op",
                        "startTimeUnixNano": now_ns.to_string(),
                        "endTimeUnixNano": (now_ns + 1_000).to_string(),
                        "attributes": [
                            {"key": "eval.run_id", "value": {"stringValue": "run-multi"}},
                            {"key": "eval.sample_id", "value": {"stringValue": "sample-a"}}
                        ]
                    }
                ]
            }]
        }]
    })
    .to_string();

    post_otlp(port, &body).await;
    // No explicit sleep: fetch_spans polls every 200 ms until spans arrive.

    let spans = receiver
        .fetch_spans("run-multi", "eval.sample_id", Duration::from_secs(1))
        .await
        .unwrap();

    assert_eq!(spans.len(), 2);
    assert_eq!(spans["sample-a"].len(), 2);
    assert_eq!(spans["sample-b"].len(), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn otlp_receiver_drops_spans_without_run_id() {
    let receiver = OtlpReceiver::start_on_port(0).await.unwrap();
    let port = receiver.port();

    let now_ns = Utc::now().timestamp_nanos_opt().unwrap();
    let body = json!({
        "resourceSpans": [{
            "scopeSpans": [{
                "spans": [{
                    "traceId": "t1",
                    "spanId": "s1",
                    "name": "untagged",
                    "startTimeUnixNano": now_ns.to_string(),
                    "endTimeUnixNano": (now_ns + 1_000).to_string(),
                    "attributes": []
                }]
            }]
        }]
    })
    .to_string();

    post_otlp(port, &body).await;
    // No explicit sleep: fetch_spans polls every 200 ms until spans arrive.

    let spans = receiver
        .fetch_spans("run-nobody", "eval.sample_id", Duration::from_millis(100))
        .await
        .unwrap();

    assert!(spans.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn otlp_receiver_parses_scalar_attribute_types() {
    let receiver = OtlpReceiver::start_on_port(0).await.unwrap();
    let port = receiver.port();

    let now_ns = Utc::now().timestamp_nanos_opt().unwrap();
    let body = json!({
        "resourceSpans": [{
            "scopeSpans": [{
                "spans": [{
                    "traceId": "t1",
                    "spanId": "s1",
                    "name": "op",
                    "startTimeUnixNano": now_ns.to_string(),
                    "endTimeUnixNano": (now_ns + 1_000).to_string(),
                    "attributes": [
                        {"key": "eval.run_id",    "value": {"stringValue": "run-types"}},
                        {"key": "eval.sample_id", "value": {"stringValue": "s1"}},
                        {"key": "str_attr",       "value": {"stringValue": "hello"}},
                        {"key": "bool_attr",      "value": {"boolValue": true}},
                        {"key": "int_attr",       "value": {"intValue": "42"}},
                        {"key": "double_attr",    "value": {"doubleValue": 3.14}}
                    ]
                }]
            }]
        }]
    })
    .to_string();

    post_otlp(port, &body).await;
    // No explicit sleep: fetch_spans polls every 200 ms until spans arrive.

    let spans = receiver
        .fetch_spans("run-types", "eval.sample_id", Duration::from_secs(1))
        .await
        .unwrap();

    let attrs = &spans["s1"][0].attributes;
    assert_eq!(attrs["str_attr"], json!("hello"));
    assert_eq!(attrs["bool_attr"], json!(true));
    assert_eq!(attrs["int_attr"], json!(42));
    assert_eq!(attrs["double_attr"].as_f64().unwrap(), 3.14);
}
