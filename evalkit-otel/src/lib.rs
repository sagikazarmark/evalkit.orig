use bytes::Bytes;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
pub use evalkit::{Observe, Span, SpanEvent, TraceBackend, TraceBackendError};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::time::sleep;

#[derive(Clone, Debug)]
pub struct JaegerBackend {
    base_url: String,
    client: reqwest::Client,
    headers: HeaderMap,
    retry_count: usize,
}

impl JaegerBackend {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
            headers: HeaderMap::new(),
            retry_count: 0,
        }
    }

    pub fn with_retry_count(mut self, retry_count: usize) -> Self {
        self.retry_count = retry_count;
        self
    }

    pub fn with_header(
        mut self,
        name: impl AsRef<str>,
        value: impl AsRef<str>,
    ) -> Result<Self, TraceBackendError> {
        let name = HeaderName::from_bytes(name.as_ref().as_bytes())
            .map_err(|err| TraceBackendError(Box::new(err)))?;
        let value = HeaderValue::from_str(value.as_ref())
            .map_err(|err| TraceBackendError(Box::new(err)))?;
        self.headers.insert(name, value);
        Ok(self)
    }

    async fn fetch_spans_once(
        &self,
        correlation_id: &str,
        sample_attribute: &str,
        timeout: Duration,
    ) -> Result<HashMap<String, Vec<Span>>, TraceBackendError> {
        let endpoint = format!("{}/api/traces", self.base_url);
        let tags = serde_json::json!({ "eval.run_id": correlation_id }).to_string();

        let response = self
            .client
            .get(endpoint)
            .headers(self.headers.clone())
            .query(&[("tags", tags)])
            .timeout(timeout)
            .send()
            .await
            .map_err(|err| TraceBackendError(Box::new(err)))?
            .error_for_status()
            .map_err(|err| TraceBackendError(Box::new(err)))?;

        let payload: JaegerTraceResponse = response
            .json()
            .await
            .map_err(|err| TraceBackendError(Box::new(err)))?;

        group_spans_by_sample(payload, sample_attribute)
    }
}

impl TraceBackend for JaegerBackend {
    async fn fetch_spans(
        &self,
        correlation_id: &str,
        sample_attribute: &str,
        timeout: Duration,
    ) -> Result<HashMap<String, Vec<Span>>, TraceBackendError> {
        let mut attempts_remaining = self.retry_count;
        let mut delay = Duration::from_millis(200);

        loop {
            match self
                .fetch_spans_once(correlation_id, sample_attribute, timeout)
                .await
            {
                Ok(grouped) => return Ok(grouped),
                Err(err) if attempts_remaining > 0 => {
                    let _ = &err;
                    attempts_remaining -= 1;
                    sleep(delay).await;
                    delay = (delay * 2).min(Duration::from_secs(5));
                }
                Err(err) => return Err(err),
            }
        }
    }
}

#[derive(Debug)]
struct ParseTraceError(String);

impl Display for ParseTraceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for ParseTraceError {}

#[derive(Debug, Deserialize)]
struct JaegerTraceResponse {
    #[serde(default)]
    data: Vec<JaegerTrace>,
}

#[derive(Debug, Deserialize)]
struct JaegerTrace {
    #[serde(default)]
    spans: Vec<JaegerSpan>,
}

#[derive(Debug, Deserialize)]
struct JaegerSpan {
    #[serde(rename = "traceID")]
    trace_id: String,
    #[serde(rename = "spanID")]
    span_id: String,
    #[serde(default)]
    references: Vec<JaegerReference>,
    #[serde(rename = "operationName")]
    operation_name: String,
    #[serde(rename = "startTime")]
    start_time: Value,
    duration: Value,
    #[serde(default)]
    tags: Vec<JaegerKeyValue>,
    #[serde(default, rename = "logs")]
    logs: Vec<JaegerLog>,
}

#[derive(Debug, Deserialize)]
struct JaegerReference {
    #[serde(rename = "refType")]
    ref_type: String,
    #[serde(rename = "spanID")]
    span_id: String,
}

#[derive(Debug, Deserialize)]
struct JaegerKeyValue {
    key: String,
    value: Value,
}

#[derive(Debug, Deserialize)]
struct JaegerLog {
    timestamp: Value,
    #[serde(default)]
    fields: Vec<JaegerKeyValue>,
}

fn group_spans_by_sample(
    payload: JaegerTraceResponse,
    sample_attribute: &str,
) -> Result<HashMap<String, Vec<Span>>, TraceBackendError> {
    let mut grouped = HashMap::new();

    for trace in payload.data {
        for span in trace.spans {
            let span = convert_span(span)?;
            let Some(sample_id) = attribute_group_key(&span.attributes, sample_attribute) else {
                continue;
            };

            grouped.entry(sample_id).or_insert_with(Vec::new).push(span);
        }
    }

    Ok(grouped)
}

fn convert_span(span: JaegerSpan) -> Result<Span, TraceBackendError> {
    let start_time = parse_datetime(&span.start_time, "startTime")?;
    let duration = parse_duration(&span.duration, "duration")?;
    let end_time = start_time
        + ChronoDuration::from_std(duration).map_err(|err| {
            TraceBackendError(Box::new(ParseTraceError(format!(
                "invalid Jaeger span duration: {err}"
            ))))
        })?;

    let attributes = key_values_to_map(span.tags);
    let events = span
        .logs
        .into_iter()
        .map(convert_event)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Span {
        trace_id: span.trace_id,
        span_id: span.span_id,
        parent_span_id: span
            .references
            .iter()
            .find(|reference| reference.ref_type == "CHILD_OF")
            .or_else(|| span.references.first())
            .map(|reference| reference.span_id.clone()),
        operation_name: span.operation_name,
        start_time,
        end_time,
        attributes,
        events,
    })
}

fn convert_event(log: JaegerLog) -> Result<SpanEvent, TraceBackendError> {
    let timestamp = parse_datetime(&log.timestamp, "log.timestamp")?;
    let attributes = key_values_to_map(log.fields);
    let name = attributes
        .get("event")
        .and_then(attribute_value_to_string)
        .or_else(|| {
            attributes
                .get("message")
                .and_then(attribute_value_to_string)
        })
        .unwrap_or_else(|| String::from("event"));

    Ok(SpanEvent {
        name,
        timestamp,
        attributes,
    })
}

fn key_values_to_map(values: Vec<JaegerKeyValue>) -> HashMap<String, Value> {
    values
        .into_iter()
        .map(|field| (field.key, field.value))
        .collect()
}

fn attribute_group_key(attributes: &HashMap<String, Value>, name: &str) -> Option<String> {
    attributes.get(name).and_then(attribute_value_to_string)
}

fn attribute_value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn parse_datetime(value: &Value, field_name: &str) -> Result<DateTime<Utc>, TraceBackendError> {
    if let Some(micros) = value.as_i64() {
        return DateTime::<Utc>::from_timestamp_micros(micros).ok_or_else(|| {
            TraceBackendError(Box::new(ParseTraceError(format!(
                "Jaeger field `{field_name}` contains an out-of-range microsecond timestamp"
            ))))
        });
    }

    if let Some(timestamp) = value.as_str() {
        return DateTime::parse_from_rfc3339(timestamp)
            .map(|parsed| parsed.with_timezone(&Utc))
            .map_err(|err| {
                TraceBackendError(Box::new(ParseTraceError(format!(
                    "Jaeger field `{field_name}` is not a valid RFC3339 timestamp: {err}"
                ))))
            });
    }

    Err(TraceBackendError(Box::new(ParseTraceError(format!(
        "Jaeger field `{field_name}` must be a microsecond timestamp or RFC3339 string"
    )))))
}

fn parse_duration(value: &Value, field_name: &str) -> Result<Duration, TraceBackendError> {
    if let Some(micros) = value.as_u64() {
        return Ok(Duration::from_micros(micros));
    }

    if let Some(micros) = value.as_str().and_then(|raw| raw.parse::<u64>().ok()) {
        return Ok(Duration::from_micros(micros));
    }

    Err(TraceBackendError(Box::new(ParseTraceError(format!(
        "Jaeger field `{field_name}` must be a non-negative microsecond duration"
    )))))
}

type SpanStoreInner = HashMap<String, Vec<Span>>;

#[derive(Clone)]
struct SpanStore(Arc<Mutex<SpanStoreInner>>);

impl SpanStore {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }

    fn insert_spans(&self, correlation_id: String, spans: Vec<Span>) {
        self.0
            .lock()
            .expect("span store poisoned")
            .entry(correlation_id)
            .or_default()
            .extend(spans);
    }

    fn get_spans(&self, correlation_id: &str) -> Vec<Span> {
        self.0
            .lock()
            .expect("span store poisoned")
            .get(correlation_id)
            .cloned()
            .unwrap_or_default()
    }
}

pub struct OtlpReceiver {
    addr: SocketAddr,
    store: SpanStore,
}

impl OtlpReceiver {
    pub async fn start() -> Result<Self, TraceBackendError> {
        Self::start_on_port(4318).await
    }

    pub async fn start_on_port(port: u16) -> Result<Self, TraceBackendError> {
        let listener = TcpListener::bind(("0.0.0.0", port))
            .await
            .map_err(|err| TraceBackendError(Box::new(err)))?;
        let addr = listener
            .local_addr()
            .map_err(|err| TraceBackendError(Box::new(err)))?;
        let store = SpanStore::new();

        let store_clone = store.clone();
        tokio::spawn(async move {
            run_otlp_server(listener, store_clone).await;
        });

        Ok(Self { addr, store })
    }

    pub fn port(&self) -> u16 {
        self.addr.port()
    }
}

impl TraceBackend for OtlpReceiver {
    async fn fetch_spans(
        &self,
        correlation_id: &str,
        sample_attribute: &str,
        fetch_timeout: Duration,
    ) -> Result<HashMap<String, Vec<Span>>, TraceBackendError> {
        let deadline = Instant::now() + fetch_timeout;
        let poll = Duration::from_millis(200);

        loop {
            let spans = self.store.get_spans(correlation_id);
            if !spans.is_empty() {
                return Ok(group_by_attribute(spans, sample_attribute));
            }

            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(HashMap::new());
            }
            sleep(poll.min(remaining)).await;
        }
    }
}

async fn run_otlp_server(listener: TcpListener, store: SpanStore) {
    loop {
        let Ok((stream, _peer)) = listener.accept().await else {
            break;
        };
        let store = store.clone();
        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            let _ = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(move |req| {
                        let store = store.clone();
                        handle_otlp_request(req, store)
                    }),
                )
                .await;
        });
    }
}

async fn handle_otlp_request(
    req: Request<Incoming>,
    store: SpanStore,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    if req.method() == Method::POST && req.uri().path() == "/v1/traces" {
        let body = req.into_body().collect().await?.to_bytes();
        if let Ok(payload) = serde_json::from_slice::<OtlpTracesPayload>(&body) {
            ingest_payload(payload, &store);
        }
        Ok(Response::new(Full::new(Bytes::new())))
    } else {
        let mut resp = Response::new(Full::new(Bytes::new()));
        *resp.status_mut() = StatusCode::NOT_FOUND;
        Ok(resp)
    }
}

fn ingest_payload(payload: OtlpTracesPayload, store: &SpanStore) {
    for resource_span in payload.resource_spans {
        for scope_span in resource_span.scope_spans {
            for raw in scope_span.spans {
                let attributes = otlp_attributes_to_map(raw.attributes);

                let Some(correlation_id) = attribute_group_key(&attributes, "eval.run_id") else {
                    continue;
                };

                let Ok(start_time) = parse_unix_nanos(&raw.start_time_unix_nano) else {
                    continue;
                };
                let Ok(end_time) = parse_unix_nanos(&raw.end_time_unix_nano) else {
                    continue;
                };

                let parent_span_id = raw.parent_span_id.filter(|s| !s.is_empty());
                let events = raw
                    .events
                    .into_iter()
                    .map(|e| {
                        let ev_attrs = otlp_attributes_to_map(e.attributes);
                        let name = ev_attrs
                            .get("name")
                            .and_then(attribute_value_to_string)
                            .or_else(|| ev_attrs.get("event").and_then(attribute_value_to_string))
                            .unwrap_or_else(|| e.name.clone())
                            .to_owned();
                        SpanEvent {
                            name,
                            timestamp: start_time,
                            attributes: ev_attrs,
                        }
                    })
                    .collect();

                let span = Span {
                    trace_id: raw.trace_id,
                    span_id: raw.span_id,
                    parent_span_id,
                    operation_name: raw.name,
                    start_time,
                    end_time,
                    attributes,
                    events,
                };

                store.insert_spans(correlation_id, vec![span]);
            }
        }
    }
}

fn group_by_attribute(spans: Vec<Span>, attribute: &str) -> HashMap<String, Vec<Span>> {
    let mut grouped: HashMap<String, Vec<Span>> = HashMap::new();
    for span in spans {
        let Some(key) = attribute_group_key(&span.attributes, attribute) else {
            continue;
        };
        grouped.entry(key).or_default().push(span);
    }
    grouped
}

fn otlp_attributes_to_map(attrs: Vec<OtlpAttribute>) -> HashMap<String, Value> {
    attrs
        .into_iter()
        .map(|a| (a.key, otlp_value_to_json(a.value)))
        .collect()
}

fn otlp_value_to_json(v: OtlpAnyValue) -> Value {
    match v {
        OtlpAnyValue::String { string_value } => Value::String(string_value),
        OtlpAnyValue::Bool { bool_value } => Value::Bool(bool_value),
        OtlpAnyValue::Int { int_value } => int_value
            .parse::<i64>()
            .ok()
            .map(|n| Value::Number(n.into()))
            .unwrap_or(Value::Null),
        OtlpAnyValue::Double { double_value } => serde_json::Number::from_f64(double_value)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        OtlpAnyValue::Unknown => Value::Null,
    }
}

fn parse_unix_nanos(v: &Value) -> Result<DateTime<Utc>, ()> {
    let nanos: i64 = if let Some(n) = v.as_i64() {
        n
    } else if let Some(s) = v.as_str() {
        s.parse().map_err(|_| ())?
    } else {
        return Err(());
    };
    Ok(DateTime::from_timestamp_nanos(nanos))
}

#[derive(Deserialize)]
struct OtlpTracesPayload {
    #[serde(rename = "resourceSpans", default)]
    resource_spans: Vec<OtlpResourceSpan>,
}

#[derive(Deserialize)]
struct OtlpResourceSpan {
    #[serde(rename = "scopeSpans", default)]
    scope_spans: Vec<OtlpScopeSpan>,
}

#[derive(Deserialize)]
struct OtlpScopeSpan {
    #[serde(default)]
    spans: Vec<OtlpSpan>,
}

#[derive(Deserialize)]
struct OtlpSpan {
    #[serde(rename = "traceId")]
    trace_id: String,
    #[serde(rename = "spanId")]
    span_id: String,
    #[serde(rename = "parentSpanId", default)]
    parent_span_id: Option<String>,
    name: String,
    #[serde(rename = "startTimeUnixNano")]
    start_time_unix_nano: Value,
    #[serde(rename = "endTimeUnixNano")]
    end_time_unix_nano: Value,
    #[serde(default)]
    attributes: Vec<OtlpAttribute>,
    #[serde(default)]
    events: Vec<OtlpSpanEvent>,
}

#[derive(Deserialize)]
struct OtlpSpanEvent {
    #[serde(default)]
    name: String,
    #[serde(default)]
    attributes: Vec<OtlpAttribute>,
}

#[derive(Deserialize)]
struct OtlpAttribute {
    key: String,
    value: OtlpAnyValue,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OtlpAnyValue {
    String {
        #[serde(rename = "stringValue")]
        string_value: String,
    },
    Bool {
        #[serde(rename = "boolValue")]
        bool_value: bool,
    },
    Int {
        #[serde(rename = "intValue")]
        int_value: String,
    },
    Double {
        #[serde(rename = "doubleValue")]
        double_value: f64,
    },
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;

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
                    attributes: HashMap::from([(
                        String::from(sample_attribute),
                        json!("sample-1"),
                    )]),
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
            "data": [{
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
                        "logs": [{
                            "timestamp": 1712131200000500i64,
                            "fields": [{"key": "event", "type": "string", "value": "parsed"}]
                        }]
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
            }]
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
        let (base_url, requests, server) =
            spawn_http_server(vec![retry_response, success_response]);

        let backend = JaegerBackend::new(base_url).with_retry_count(1);
        let spans = backend
            .fetch_spans("run-456", "eval.sample_id", Duration::from_secs(1))
            .await
            .unwrap();

        server.join().unwrap();

        assert!(spans.is_empty());
        assert_eq!(requests.lock().unwrap().len(), 2);
    }

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
}
