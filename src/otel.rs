use crate::{Acquisition, AcquisitionError};
use bytes::Bytes;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
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
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::{task_local, time::sleep};

type FetchSpansFuture<'a> =
    Pin<Box<dyn Future<Output = Result<HashMap<String, Vec<Span>>, TraceBackendError>> + 'a>>;

task_local! {
    static OBSERVE_SAMPLE_ID: String;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub attributes: HashMap<String, Value>,
    pub events: Vec<SpanEvent>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpanEvent {
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub attributes: HashMap<String, Value>,
}

#[derive(Debug)]
pub struct TraceBackendError(pub Box<dyn Error + Send + Sync>);

impl TraceBackendError {
    fn new(err: impl Error + Send + Sync + 'static) -> Self {
        Self(Box::new(err))
    }
}

impl Display for TraceBackendError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Error for TraceBackendError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.0.as_ref())
    }
}

#[allow(async_fn_in_trait)]
pub trait TraceBackend: Send + Sync {
    async fn fetch_spans(
        &self,
        correlation_id: &str,
        sample_attribute: &str,
        timeout: Duration,
    ) -> Result<HashMap<String, Vec<Span>>, TraceBackendError>;
}

trait ErasedTraceBackend: Send + Sync {
    fn fetch_spans_boxed<'a>(
        &'a self,
        correlation_id: &'a str,
        sample_attribute: &'a str,
        timeout: Duration,
    ) -> FetchSpansFuture<'a>;
}

impl<B> ErasedTraceBackend for B
where
    B: TraceBackend,
{
    fn fetch_spans_boxed<'a>(
        &'a self,
        correlation_id: &'a str,
        sample_attribute: &'a str,
        timeout: Duration,
    ) -> FetchSpansFuture<'a> {
        Box::pin(async move {
            self.fetch_spans(correlation_id, sample_attribute, timeout)
                .await
        })
    }
}

pub struct Observe {
    backend: Box<dyn ErasedTraceBackend>,
    correlation_id: String,
    sample_attribute: String,
    timeout: Duration,
    cached_spans: Mutex<Option<HashMap<String, Vec<Span>>>>,
}

impl Observe {
    pub fn builder() -> ObserveBuilder {
        ObserveBuilder
    }

    async fn grouped_spans(&self) -> Result<HashMap<String, Vec<Span>>, AcquisitionError> {
        if let Some(cached) = self
            .cached_spans
            .lock()
            .expect("observe cache poisoned")
            .clone()
        {
            return Ok(cached);
        }

        let grouped = match tokio::time::timeout(
            self.timeout,
            self.backend.fetch_spans_boxed(
                &self.correlation_id,
                &self.sample_attribute,
                self.timeout,
            ),
        )
        .await
        {
            Ok(Ok(grouped)) => grouped,
            Ok(Err(err)) => return Err(AcquisitionError::BackendUnavailable(Box::new(err))),
            Err(_) => return Err(AcquisitionError::Timeout(self.timeout)),
        };

        *self.cached_spans.lock().expect("observe cache poisoned") = Some(grouped.clone());

        Ok(grouped)
    }
}

impl<I> Acquisition<I, Vec<Span>> for Observe {
    async fn acquire(&self, _input: &I) -> Result<Vec<Span>, AcquisitionError> {
        let sample_id = current_observe_sample_id()?;
        let grouped = self.grouped_spans().await?;

        grouped
            .get(&sample_id)
            .cloned()
            .ok_or_else(|| AcquisitionError::TraceNotFound {
                correlation_id: self.correlation_id.clone(),
                sample_id,
            })
    }
}

pub struct ObserveBuilder;

impl ObserveBuilder {
    pub fn backend<B>(self, backend: B) -> ObserveBuilderWithBackend
    where
        B: TraceBackend + 'static,
    {
        ObserveBuilderWithBackend {
            backend: Box::new(backend),
        }
    }
}

pub struct ObserveBuilderWithBackend {
    backend: Box<dyn ErasedTraceBackend>,
}

impl ObserveBuilderWithBackend {
    pub fn correlation_id(
        self,
        correlation_id: impl Into<String>,
    ) -> ObserveBuilderWithCorrelationId {
        ObserveBuilderWithCorrelationId {
            backend: self.backend,
            correlation_id: correlation_id.into(),
        }
    }
}

pub struct ObserveBuilderWithCorrelationId {
    backend: Box<dyn ErasedTraceBackend>,
    correlation_id: String,
}

impl ObserveBuilderWithCorrelationId {
    pub fn sample_attribute(
        self,
        sample_attribute: impl Into<String>,
    ) -> ObserveBuilderWithSampleAttribute {
        ObserveBuilderWithSampleAttribute {
            backend: self.backend,
            correlation_id: self.correlation_id,
            sample_attribute: sample_attribute.into(),
        }
    }
}

pub struct ObserveBuilderWithSampleAttribute {
    backend: Box<dyn ErasedTraceBackend>,
    correlation_id: String,
    sample_attribute: String,
}

impl ObserveBuilderWithSampleAttribute {
    pub fn timeout(self, timeout: Duration) -> ObserveBuilderReady {
        ObserveBuilderReady {
            backend: self.backend,
            correlation_id: self.correlation_id,
            sample_attribute: self.sample_attribute,
            timeout,
        }
    }
}

pub struct ObserveBuilderReady {
    backend: Box<dyn ErasedTraceBackend>,
    correlation_id: String,
    sample_attribute: String,
    timeout: Duration,
}

impl ObserveBuilderReady {
    pub fn build(self) -> Observe {
        Observe {
            backend: self.backend,
            correlation_id: self.correlation_id,
            sample_attribute: self.sample_attribute,
            timeout: self.timeout,
            cached_spans: Mutex::new(None),
        }
    }
}

pub(crate) async fn with_observe_sample_id<Fut>(sample_id: &str, future: Fut) -> Fut::Output
where
    Fut: Future,
{
    OBSERVE_SAMPLE_ID.scope(sample_id.to_string(), future).await
}

fn current_observe_sample_id() -> Result<String, AcquisitionError> {
    OBSERVE_SAMPLE_ID.try_with(Clone::clone).map_err(|_| {
        AcquisitionError::ExecutionFailed(Box::new(ParseTraceError(String::from(
            "observe acquisition requires Run to provide the current sample id",
        ))))
    })
}

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
        let name =
            HeaderName::from_bytes(name.as_ref().as_bytes()).map_err(TraceBackendError::new)?;
        let value = HeaderValue::from_str(value.as_ref()).map_err(TraceBackendError::new)?;
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
            .map_err(TraceBackendError::new)?
            .error_for_status()
            .map_err(TraceBackendError::new)?;

        let payload: JaegerTraceResponse = response.json().await.map_err(TraceBackendError::new)?;
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
            TraceBackendError::new(ParseTraceError(format!(
                "invalid Jaeger span duration: {err}"
            )))
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
            TraceBackendError::new(ParseTraceError(format!(
                "Jaeger field `{field_name}` contains an out-of-range microsecond timestamp"
            )))
        });
    }

    if let Some(timestamp) = value.as_str() {
        return DateTime::parse_from_rfc3339(timestamp)
            .map(|parsed| parsed.with_timezone(&Utc))
            .map_err(|err| {
                TraceBackendError::new(ParseTraceError(format!(
                    "Jaeger field `{field_name}` is not a valid RFC3339 timestamp: {err}"
                )))
            });
    }

    Err(TraceBackendError::new(ParseTraceError(format!(
        "Jaeger field `{field_name}` must be a microsecond timestamp or RFC3339 string"
    ))))
}

fn parse_duration(value: &Value, field_name: &str) -> Result<Duration, TraceBackendError> {
    if let Some(micros) = value.as_u64() {
        return Ok(Duration::from_micros(micros));
    }

    if let Some(micros) = value.as_str().and_then(|raw| raw.parse::<u64>().ok()) {
        return Ok(Duration::from_micros(micros));
    }

    Err(TraceBackendError::new(ParseTraceError(format!(
        "Jaeger field `{field_name}` must be a non-negative microsecond duration"
    ))))
}

// ---------------------------------------------------------------------------
// OtlpReceiver — embedded OTLP/HTTP receiver (no external backend required)
//
// Start one before running the system under test, then pass it as the
// TraceBackend for Observe.  Point the system under test's OTLP exporter
// at http://localhost:<port>.
//
// Protocol: POST /v1/traces with an OTLP/HTTP JSON body.
// Spans are stored by their `eval.run_id` attribute and returned when
// fetch_spans is called with the matching correlation_id.
// ---------------------------------------------------------------------------

/// Shared span store: `correlation_id` → flat list of spans.
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

/// An embedded OTLP/HTTP receiver that stores incoming spans in memory.
///
/// Use instead of `JaegerBackend` when you want evalkit to receive spans
/// directly from the system under test rather than querying an external
/// tracing backend.
pub struct OtlpReceiver {
    addr: SocketAddr,
    store: SpanStore,
}

impl OtlpReceiver {
    /// Start an OTLP/HTTP receiver on the default port (4318).
    pub async fn start() -> Result<Self, TraceBackendError> {
        Self::start_on_port(4318).await
    }

    /// Start an OTLP/HTTP receiver on the given port. Use port `0` to let the
    /// OS pick a free port (useful in tests).
    pub async fn start_on_port(port: u16) -> Result<Self, TraceBackendError> {
        let listener = TcpListener::bind(("0.0.0.0", port))
            .await
            .map_err(TraceBackendError::new)?;
        let addr = listener.local_addr().map_err(TraceBackendError::new)?;
        let store = SpanStore::new();

        let store_clone = store.clone();
        tokio::spawn(async move {
            run_otlp_server(listener, store_clone).await;
        });

        Ok(Self { addr, store })
    }

    /// Returns the port the receiver is listening on.
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

                let Some(correlation_id) =
                    attribute_group_key(&attributes, "eval.run_id")
                else {
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
                            .or_else(|| {
                                ev_attrs
                                    .get("event")
                                    .and_then(attribute_value_to_string)
                            })
                            .unwrap_or_else(|| e.name.clone())
                            .to_owned();
                        SpanEvent { name, timestamp: start_time, attributes: ev_attrs }
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

// ---------------------------------------------------------------------------
// OTLP/HTTP JSON deserialization types
// ---------------------------------------------------------------------------

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

/// OTLP AnyValue — only the scalar kinds evalkit cares about.
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
        // int64 is encoded as a decimal string in OTLP JSON to avoid precision loss
        #[serde(rename = "intValue")]
        int_value: String,
    },
    Double {
        #[serde(rename = "doubleValue")]
        double_value: f64,
    },
    Unknown,
}
