use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::task_local;

task_local! {
    static CURRENT_SAMPLE_ID: String;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct AcquisitionMetadata {
    pub mode: &'static str,
}

impl Default for AcquisitionMetadata {
    fn default() -> Self {
        Self { mode: "inline" }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AcquisitionSnapshot<O> {
    pub label: String,
    pub output: O,
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

impl<O> AcquisitionSnapshot<O> {
    pub fn new(label: impl Into<String>, output: O) -> Self {
        Self {
            label: label.into(),
            output,
            metadata: HashMap::new(),
        }
    }

    pub fn metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AcquiredOutput<O> {
    pub output: O,
    #[serde(default)]
    pub snapshots: Vec<AcquisitionSnapshot<O>>,
}

impl<O> AcquiredOutput<O> {
    pub fn new(output: O) -> Self {
        Self {
            output,
            snapshots: Vec::new(),
        }
    }

    pub fn with_snapshot(mut self, snapshot: AcquisitionSnapshot<O>) -> Self {
        self.snapshots.push(snapshot);
        self
    }
}

impl AcquisitionMetadata {
    pub fn mode(mut self, mode: &'static str) -> Self {
        self.mode = mode;
        self
    }
}

#[derive(Debug)]
pub enum AcquisitionError {
    ExecutionFailed(Box<dyn Error + Send + Sync>),
    TraceNotFound {
        correlation_id: String,
        sample_id: String,
    },
    BackendUnavailable(Box<dyn Error + Send + Sync>),
    Timeout(Duration),
    Panicked,
}

impl Display for AcquisitionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExecutionFailed(err) => write!(f, "acquisition execution failed: {err}"),
            Self::TraceNotFound {
                correlation_id,
                sample_id,
            } => write!(
                f,
                "no spans found for correlation_id `{correlation_id}` and sample_id `{sample_id}`"
            ),
            Self::BackendUnavailable(err) => write!(f, "trace backend unavailable: {err}"),
            Self::Timeout(duration) => write!(f, "acquisition timed out after {duration:?}"),
            Self::Panicked => write!(f, "acquisition panicked"),
        }
    }
}

impl Error for AcquisitionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ExecutionFailed(err) | Self::BackendUnavailable(err) => Some(err.as_ref()),
            Self::TraceNotFound { .. } | Self::Timeout(_) | Self::Panicked => None,
        }
    }
}

#[allow(async_fn_in_trait)]
pub trait Acquisition<I, O>: Send + Sync {
    async fn acquire(&self, input: &I) -> Result<O, AcquisitionError>;

    async fn acquire_with_snapshots(&self, input: &I) -> Result<AcquiredOutput<O>, AcquisitionError> {
        self.acquire(input).await.map(AcquiredOutput::new)
    }

    fn metadata(&self) -> AcquisitionMetadata {
        AcquisitionMetadata::default()
    }
}

impl<I, O, F, Fut> Acquisition<I, O> for F
where
    F: Fn(&I) -> Fut + Send + Sync,
    Fut: Future<Output = Result<O, AcquisitionError>> + Send,
{
    async fn acquire(&self, input: &I) -> Result<O, AcquisitionError> {
        self(input).await
    }
}

pub fn current_sample_id() -> Option<String> {
    CURRENT_SAMPLE_ID.try_with(Clone::clone).ok()
}

pub(crate) async fn with_current_sample_id<Fut>(sample_id: &str, future: Fut) -> Fut::Output
where
    Fut: Future,
{
    CURRENT_SAMPLE_ID.scope(sample_id.to_string(), future).await
}

#[cfg(test)]
mod tests {
    use super::{Acquisition, AcquisitionError};
    use std::error::Error;
    use std::fmt::{self, Display, Formatter};
    use std::time::Duration;

    #[derive(Debug)]
    struct TestError(&'static str);

    impl Display for TestError {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            f.write_str(self.0)
        }
    }

    impl Error for TestError {}

    struct PrefixAcquisition;

    impl Acquisition<String, String> for PrefixAcquisition {
        async fn acquire(&self, input: &String) -> Result<String, AcquisitionError> {
            Ok(format!("agent::{input}"))
        }
    }

    fn assert_send_sync<T: Send + Sync>() {}

    #[tokio::test(flavor = "current_thread")]
    async fn acquisition_trait_supports_custom_implementations() {
        assert_send_sync::<PrefixAcquisition>();

        let acquisition = PrefixAcquisition;
        let input = String::from("prompt");

        let output = acquisition.acquire(&input).await.unwrap();

        assert_eq!(output, "agent::prompt");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn acquisition_blanket_impl_supports_async_closures() {
        let acquisition = |input: &String| {
            let output = format!("{input} -> completion");
            async move { Ok::<_, AcquisitionError>(output) }
        };
        let input = String::from("question");

        let output = acquisition.acquire(&input).await.unwrap();

        assert_eq!(output, "question -> completion");
    }

    #[test]
    fn acquisition_error_wrapped_variants_preserve_sources() {
        let execution_failed = AcquisitionError::ExecutionFailed(Box::new(TestError("agent down")));
        let backend_unavailable =
            AcquisitionError::BackendUnavailable(Box::new(TestError("jaeger offline")));

        assert_eq!(
            execution_failed.to_string(),
            "acquisition execution failed: agent down"
        );
        assert_eq!(
            backend_unavailable.to_string(),
            "trace backend unavailable: jaeger offline"
        );
        assert_eq!(
            execution_failed
                .source()
                .map(ToString::to_string)
                .as_deref(),
            Some("agent down")
        );
        assert_eq!(
            backend_unavailable
                .source()
                .map(ToString::to_string)
                .as_deref(),
            Some("jaeger offline")
        );
    }

    #[test]
    fn acquisition_error_value_variants_are_distinct_from_wrapped_failures() {
        let trace_not_found = AcquisitionError::TraceNotFound {
            correlation_id: String::from("run-123"),
            sample_id: String::from("sample-7"),
        };
        let timeout = AcquisitionError::Timeout(Duration::from_secs(3));

        assert_eq!(
            trace_not_found.to_string(),
            "no spans found for correlation_id `run-123` and sample_id `sample-7`"
        );
        assert_eq!(timeout.to_string(), "acquisition timed out after 3s");
        assert!(trace_not_found.source().is_none());
        assert!(timeout.source().is_none());
    }
}
