use bytes::Bytes;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use evalkit::{
    OutputSource, OutputSourceError, SourceMetadata, RunResult, Sample, Score,
};
use evalkit_runtime::{ExecutionSink, ExecutorBoxError, SampleSource, current_sample_id};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::time::sleep;

type FetchSpansFuture<'a> =
    Pin<Box<dyn Future<Output = Result<HashMap<String, Vec<Span>>, TraceBackendError>> + 'a>>;

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

pub struct OtelObserver {
    backend: Box<dyn ErasedTraceBackend>,
    correlation_id: String,
    sample_attribute: String,
    timeout: Duration,
    cached_spans: Mutex<Option<HashMap<String, Vec<Span>>>>,
}

impl OtelObserver {
    pub fn builder() -> ObserveBuilder {
        ObserveBuilder
    }

    async fn grouped_spans(&self) -> Result<HashMap<String, Vec<Span>>, OutputSourceError> {
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
            Ok(Err(err)) => return Err(OutputSourceError::BackendUnavailable(Box::new(err))),
            Err(_) => return Err(OutputSourceError::Timeout(self.timeout)),
        };

        *self.cached_spans.lock().expect("observe cache poisoned") = Some(grouped.clone());

        Ok(grouped)
    }
}

impl<I> OutputSource<I, Vec<Span>> for OtelObserver {
    async fn produce(&self, _input: &I) -> Result<Vec<Span>, OutputSourceError> {
        let sample_id = current_sample_id().ok_or_else(|| {
            OutputSourceError::ExecutionFailed(Box::new(ParseTraceError(String::from(
                "OtelObserver requires Run to provide the current sample id",
            ))))
        })?;
        let grouped = self.grouped_spans().await?;

        grouped
            .get(&sample_id)
            .cloned()
            .ok_or_else(|| OutputSourceError::TraceNotFound {
                correlation_id: self.correlation_id.clone(),
                sample_id,
            })
    }

    fn metadata(&self) -> SourceMetadata {
        SourceMetadata::default().mode("observe")
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
    pub fn build(self) -> OtelObserver {
        OtelObserver {
            backend: self.backend,
            correlation_id: self.correlation_id,
            sample_attribute: self.sample_attribute,
            timeout: self.timeout,
            cached_spans: Mutex::new(None),
        }
    }
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

pub struct OtlpReceiverSource {
    receiver: OtlpReceiver,
    correlation_id: String,
    sample_attribute: String,
    poll_interval: Duration,
    idle_timeout: Duration,
    yielded_sample_ids: HashSet<String>,
    pending: VecDeque<Sample<Vec<Span>>>,
}

pub struct OtelResultEmitter {
    namespace: String,
}

pub struct OtelResultSink {
    emitter: OtelResultEmitter,
    spans: Arc<Mutex<Vec<Span>>>,
}

impl OtelResultEmitter {
    pub fn new() -> Self {
        Self {
            namespace: String::from("evalkit"),
        }
    }

    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    pub fn emit(&self, result: &RunResult) -> Vec<Span> {
        let trace_id = result.metadata.run_id.clone();
        let run_span_id = String::from("run");
        let mut spans = Vec::with_capacity(result.samples.len() + 1);

        spans.push(Span {
            trace_id: trace_id.clone(),
            span_id: run_span_id.clone(),
            parent_span_id: None,
            operation_name: String::from("eval.run"),
            start_time: result.metadata.started_at,
            end_time: result.metadata.completed_at,
            attributes: self.run_attributes(result),
            events: Vec::new(),
        });

        for (sample_index, sample) in result.samples.iter().enumerate() {
            let mut events = Vec::new();

            for trial in &sample.trials {
                for (scorer_name, score_result) in &trial.scores {
                    let mut attributes = HashMap::from([
                        (self.key("scorer_name"), Value::String(scorer_name.clone())),
                        (
                            self.key("trial_index"),
                            Value::from(trial.trial_index as u64),
                        ),
                        (
                            self.key("trial_duration_ms"),
                            Value::from(trial.duration.as_millis() as u64),
                        ),
                    ]);

                    match score_result {
                        Ok(score) => {
                            attributes.insert(self.key("score"), score_json(score));
                        }
                        Err(error) => {
                            attributes.insert(self.key("error"), Value::String(error.to_string()));
                        }
                    }

                    events.push(SpanEvent {
                        name: String::from("eval.score"),
                        timestamp: result.metadata.completed_at,
                        attributes,
                    });
                }
            }

            spans.push(Span {
                trace_id: trace_id.clone(),
                span_id: format!("sample-{}", sample_index + 1),
                parent_span_id: Some(run_span_id.clone()),
                operation_name: String::from("eval.sample"),
                start_time: result.metadata.started_at,
                end_time: result.metadata.completed_at,
                attributes: HashMap::from([
                    (
                        self.key("sample_id"),
                        Value::String(sample.sample_id.clone()),
                    ),
                    (
                        self.key("trial_count"),
                        Value::from(sample.trial_count as u64),
                    ),
                    (
                        self.key("scored_count"),
                        Value::from(sample.scored_count as u64),
                    ),
                    (
                        self.key("error_count"),
                        Value::from(sample.error_count as u64),
                    ),
                    (
                        self.key("token_usage"),
                        serde_json::to_value(&sample.token_usage).unwrap_or(Value::Null),
                    ),
                    (
                        self.key("cost_usd"),
                        serde_json::to_value(sample.cost_usd).unwrap_or(Value::Null),
                    ),
                ]),
                events,
            });
        }

        spans
    }

    fn run_attributes(&self, result: &RunResult) -> HashMap<String, Value> {
        HashMap::from([
            (
                self.key("result_schema_version"),
                Value::String(String::from("evalkit.result.v1")),
            ),
            (
                self.key("run_id"),
                Value::String(result.metadata.run_id.clone()),
            ),
            (
                self.key("seed"),
                serde_json::to_value(result.metadata.seed).unwrap_or(Value::Null),
            ),
            (
                self.key("dataset_fingerprint"),
                Value::String(result.metadata.dataset_fingerprint.clone()),
            ),
            (
                self.key("scorer_fingerprint"),
                Value::String(result.metadata.scorer_fingerprint.clone()),
            ),
            (
                self.key("code_commit"),
                serde_json::to_value(&result.metadata.code_commit).unwrap_or(Value::Null),
            ),
            (
                self.key("code_fingerprint"),
                serde_json::to_value(&result.metadata.code_fingerprint).unwrap_or(Value::Null),
            ),
            (
                self.key("judge_model_pins"),
                serde_json::to_value(&result.metadata.judge_model_pins).unwrap_or(Value::Null),
            ),
            (
                self.key("trial_count"),
                Value::from(result.metadata.trial_count as u64),
            ),
            (
                self.key("source_mode"),
                Value::String(result.metadata.source_mode.clone()),
            ),
        ])
    }

    fn key(&self, suffix: &str) -> String {
        format!("{}.{}", self.namespace, suffix)
    }
}

impl OtelResultSink {
    pub fn new() -> Self {
        Self {
            emitter: OtelResultEmitter::new(),
            spans: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.emitter = self.emitter.with_namespace(namespace);
        self
    }

    pub fn spans(&self) -> Arc<Mutex<Vec<Span>>> {
        Arc::clone(&self.spans)
    }
}

impl Default for OtelResultSink {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionSink for OtelResultSink {
    async fn finish(&mut self, result: &RunResult) -> Result<(), ExecutorBoxError> {
        let emitted = self.emitter.emit(result);
        let mut stored = self.spans.lock().map_err(|_| {
            Box::new(ResultSinkError("OTel span sink mutex poisoned")) as ExecutorBoxError
        })?;
        stored.extend(emitted);
        Ok(())
    }
}

impl Default for OtelResultEmitter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct ResultSinkError(&'static str);

impl Display for ResultSinkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl Error for ResultSinkError {}

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

impl OtlpReceiverSource {
    pub fn new(
        receiver: OtlpReceiver,
        correlation_id: impl Into<String>,
        sample_attribute: impl Into<String>,
    ) -> Self {
        Self {
            receiver,
            correlation_id: correlation_id.into(),
            sample_attribute: sample_attribute.into(),
            poll_interval: Duration::from_millis(200),
            idle_timeout: Duration::from_secs(5),
            yielded_sample_ids: HashSet::new(),
            pending: VecDeque::new(),
        }
    }

    pub fn poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }

    pub fn idle_timeout(mut self, idle_timeout: Duration) -> Self {
        self.idle_timeout = idle_timeout;
        self
    }

    fn enqueue_new_samples(&mut self, grouped: HashMap<String, Vec<Span>>) {
        let mut sample_ids = grouped
            .keys()
            .filter(|sample_id| !self.yielded_sample_ids.contains(*sample_id))
            .cloned()
            .collect::<Vec<_>>();
        sample_ids.sort();

        for sample_id in sample_ids {
            let spans = grouped
                .get(&sample_id)
                .cloned()
                .expect("grouped spans should contain discovered sample ids");
            self.yielded_sample_ids.insert(sample_id.clone());
            self.pending.push_back(Sample {
                id: sample_id,
                input: spans,
                reference: None,
                metadata: HashMap::new(),
            });
        }
    }
}

#[allow(async_fn_in_trait)]
impl SampleSource<Vec<Span>> for OtlpReceiverSource {
    async fn next_sample(&mut self) -> Result<Option<Sample<Vec<Span>>>, ExecutorBoxError> {
        if let Some(sample) = self.pending.pop_front() {
            return Ok(Some(sample));
        }

        let deadline = Instant::now() + self.idle_timeout;

        loop {
            let grouped = self
                .receiver
                .fetch_spans(
                    &self.correlation_id,
                    &self.sample_attribute,
                    self.poll_interval,
                )
                .await
                .map_err(|err| Box::new(err) as ExecutorBoxError)?;
            self.enqueue_new_samples(grouped);

            if let Some(sample) = self.pending.pop_front() {
                return Ok(Some(sample));
            }

            if Instant::now() >= deadline {
                return Ok(None);
            }
        }
    }

    fn metadata(&self) -> HashMap<String, Value> {
        HashMap::from([
            (
                String::from("source.kind"),
                Value::String(String::from("otlp_receiver")),
            ),
            (
                String::from("source.correlation_id"),
                Value::String(self.correlation_id.clone()),
            ),
            (
                String::from("source.sample_attribute"),
                Value::String(self.sample_attribute.clone()),
            ),
        ])
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

fn score_json(score: &Score) -> Value {
    serde_json::to_value(score).unwrap_or(Value::Null)
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
    use evalkit::{Direction, RunMetadata, SampleResult, ScoreDefinition, TrialResult};
    use evalkit_runtime::SampleSource;
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

    #[test]
    fn otel_result_emitter_creates_run_and_sample_spans() {
        let emitter = OtelResultEmitter::new();
        let result = run_result_fixture();

        let spans = emitter.emit(&result);

        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].operation_name, "eval.run");
        assert_eq!(spans[0].attributes["evalkit.run_id"], json!("run-123"));
        assert_eq!(spans[1].operation_name, "eval.sample");
        assert_eq!(spans[1].parent_span_id.as_deref(), Some("run"));
        assert_eq!(spans[1].attributes["evalkit.sample_id"], json!("sample-1"));
    }

    #[test]
    fn otel_result_emitter_emits_score_events_for_successes_and_errors() {
        let emitter = OtelResultEmitter::new();
        let result = run_result_fixture();

        let spans = emitter.emit(&result);
        let sample_span = &spans[1];

        assert_eq!(sample_span.events.len(), 2);
        assert_eq!(sample_span.events[0].name, "eval.score");
        assert!(sample_span.events.iter().any(|event| {
            event.attributes.get("evalkit.score") == Some(&json!({"type":"binary","value":true}))
        }));
        assert!(
            sample_span.events.iter().any(|event| {
                event.attributes.get("evalkit.error") == Some(&json!("bad output"))
            })
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn otel_result_sink_collects_spans_on_finish() {
        let mut sink = OtelResultSink::new();
        let stored = sink.spans();
        let result = run_result_fixture();

        sink.finish(&result).await.unwrap();

        let spans = stored.lock().unwrap();
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].operation_name, "eval.run");
        assert_eq!(spans[1].operation_name, "eval.sample");
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
    async fn otlp_receiver_source_yields_new_samples_once() {
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
                            "name": "op-a",
                            "startTimeUnixNano": now_ns.to_string(),
                            "endTimeUnixNano": (now_ns + 1_000).to_string(),
                            "attributes": [
                                {"key": "eval.run_id", "value": {"stringValue": "run-source"}},
                                {"key": "eval.sample_id", "value": {"stringValue": "sample-b"}}
                            ]
                        },
                        {
                            "traceId": "t2",
                            "spanId": "s2",
                            "name": "op-b",
                            "startTimeUnixNano": now_ns.to_string(),
                            "endTimeUnixNano": (now_ns + 1_000).to_string(),
                            "attributes": [
                                {"key": "eval.run_id", "value": {"stringValue": "run-source"}},
                                {"key": "eval.sample_id", "value": {"stringValue": "sample-a"}}
                            ]
                        }
                    ]
                }]
            }]
        })
        .to_string();

        post_otlp(port, &body).await;

        let mut source = OtlpReceiverSource::new(receiver, "run-source", "eval.sample_id")
            .poll_interval(Duration::from_millis(10))
            .idle_timeout(Duration::from_millis(25));

        let first = source.next_sample().await.unwrap().unwrap();
        let second = source.next_sample().await.unwrap().unwrap();
        let done = source.next_sample().await.unwrap();

        assert_eq!(first.id, "sample-a");
        assert_eq!(second.id, "sample-b");
        assert_eq!(first.input.len(), 1);
        assert_eq!(second.input.len(), 1);
        assert!(done.is_none());
        assert_eq!(
            source.metadata().get("source.kind"),
            Some(&json!("otlp_receiver"))
        );
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

    fn run_result_fixture() -> RunResult {
        RunResult {
            metadata: RunMetadata {
                run_id: String::from("run-123"),
                seed: Some(7),
                dataset_fingerprint: String::from("dataset-abc"),
                scorer_fingerprint: String::from("scorers-abc"),
                code_commit: Some(String::from("abc123")),
                code_fingerprint: Some(String::from("tree:deadbeef")),
                judge_model_pins: vec![String::from("gpt-4o@2026-04-01")],
                started_at: parse_time("2026-04-03T10:00:00Z"),
                completed_at: parse_time("2026-04-03T10:00:05Z"),
                duration: Duration::from_secs(5),
                trial_count: 1,
                score_definitions: vec![ScoreDefinition {
                    name: String::from("accuracy"),
                    direction: Some(Direction::Maximize),
                }],
                source_mode: String::from("inline"),
            },
            samples: vec![SampleResult {
                sample_id: String::from("sample-1"),
                trials: vec![TrialResult {
                    scores: HashMap::from([
                        (String::from("accuracy"), Ok(Score::Binary(true))),
                        (
                            String::from("parser"),
                            Err(evalkit::ScorerError::internal(std::io::Error::other(
                                "bad output",
                            ))),
                        ),
                    ]),
                    duration: Duration::from_millis(25),
                    trial_index: 0,
                }],
                trial_count: 1,
                scored_count: 1,
                error_count: 1,
                token_usage: Default::default(),
                cost_usd: Some(0.002),
            }],
        }
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
