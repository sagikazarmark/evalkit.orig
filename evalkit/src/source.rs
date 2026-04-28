//! Output source abstraction.
//!
//! `OutputSource` is the kernel umbrella for "produce evaluation output for a sample."
//! Most evals use `Task::from_fn` or a closure (active). To evaluate an already-instrumented
//! system, use a passive source from an adapter crate (e.g., `evalkit-otel::OtelObserver`).

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputSnapshot<O> {
    pub label: String,
    pub output: O,
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

impl<O> OutputSnapshot<O> {
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
pub struct SourceOutput<O> {
    pub output: O,
    #[serde(default)]
    pub snapshots: Vec<OutputSnapshot<O>>,
}

impl<O> SourceOutput<O> {
    pub fn new(output: O) -> Self {
        Self {
            output,
            snapshots: Vec::new(),
        }
    }

    pub fn with_snapshot(mut self, snapshot: OutputSnapshot<O>) -> Self {
        self.snapshots.push(snapshot);
        self
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum OutputSourceError {
    ExecutionFailed(Box<dyn Error + Send + Sync>),
    TraceNotFound {
        correlation_id: String,
        sample_id: String,
    },
    BackendUnavailable(Box<dyn Error + Send + Sync>),
    Timeout(Duration),
    Panicked(String),
}

impl OutputSourceError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::BackendUnavailable(_) | Self::Timeout(_))
    }
}

impl Display for OutputSourceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExecutionFailed(err) => write!(f, "output source execution failed: {err}"),
            Self::TraceNotFound {
                correlation_id,
                sample_id,
            } => write!(
                f,
                "no spans found for correlation_id `{correlation_id}` and sample_id `{sample_id}`"
            ),
            Self::BackendUnavailable(err) => write!(f, "trace backend unavailable: {err}"),
            Self::Timeout(duration) => write!(f, "output source timed out after {duration:?}"),
            Self::Panicked(message) => write!(f, "output source panicked: {message}"),
        }
    }
}

impl Error for OutputSourceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ExecutionFailed(err) | Self::BackendUnavailable(err) => Some(err.as_ref()),
            Self::TraceNotFound { .. } | Self::Timeout(_) | Self::Panicked(_) => None,
        }
    }
}

#[allow(async_fn_in_trait)]
pub trait OutputSource<I, O>: Send + Sync {
    async fn produce(&self, input: &I) -> Result<O, OutputSourceError>;

    async fn produce_with_snapshots(&self, input: &I) -> Result<SourceOutput<O>, OutputSourceError> {
        self.produce(input).await.map(SourceOutput::new)
    }

    fn metadata_mode(&self) -> &'static str { "inline" }
}

impl<I, O, F, Fut> OutputSource<I, O> for F
where
    F: Fn(&I) -> Fut + Send + Sync,
    Fut: Future<Output = Result<O, OutputSourceError>> + Send,
{
    async fn produce(&self, input: &I) -> Result<O, OutputSourceError> {
        self(input).await
    }
}

pub fn current_sample_id() -> Option<String> {
    CURRENT_SAMPLE_ID.try_with(Clone::clone).ok()
}

pub async fn with_current_sample_id<Fut>(sample_id: &str, future: Fut) -> Fut::Output
where
    Fut: Future,
{
    CURRENT_SAMPLE_ID.scope(sample_id.to_string(), future).await
}

#[cfg(test)]
mod tests {
    use super::{OutputSource, OutputSourceError};
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

    struct PrefixSource;

    impl OutputSource<String, String> for PrefixSource {
        async fn produce(&self, input: &String) -> Result<String, OutputSourceError> {
            Ok(format!("agent::{input}"))
        }
    }

    fn assert_send_sync<T: Send + Sync>() {}

    #[tokio::test(flavor = "current_thread")]
    async fn output_source_trait_supports_custom_implementations() {
        assert_send_sync::<PrefixSource>();

        let source = PrefixSource;
        let input = String::from("prompt");

        let output = source.produce(&input).await.unwrap();

        assert_eq!(output, "agent::prompt");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn output_source_blanket_impl_supports_async_closures() {
        let source = |input: &String| {
            let output = format!("{input} -> completion");
            async move { Ok::<_, OutputSourceError>(output) }
        };
        let input = String::from("question");

        let output = source.produce(&input).await.unwrap();

        assert_eq!(output, "question -> completion");
    }

    #[test]
    fn output_source_error_wrapped_variants_preserve_sources() {
        let execution_failed = OutputSourceError::ExecutionFailed(Box::new(TestError("agent down")));
        let backend_unavailable =
            OutputSourceError::BackendUnavailable(Box::new(TestError("jaeger offline")));

        assert_eq!(
            execution_failed.to_string(),
            "output source execution failed: agent down"
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
    fn output_source_error_value_variants_are_distinct_from_wrapped_failures() {
        let trace_not_found = OutputSourceError::TraceNotFound {
            correlation_id: String::from("run-123"),
            sample_id: String::from("sample-7"),
        };
        let timeout = OutputSourceError::Timeout(Duration::from_secs(3));

        assert_eq!(
            trace_not_found.to_string(),
            "no spans found for correlation_id `run-123` and sample_id `sample-7`"
        );
        assert_eq!(timeout.to_string(), "output source timed out after 3s");
        assert!(trace_not_found.source().is_none());
        assert!(timeout.source().is_none());
    }

    #[test]
    fn is_retryable_classifies_known_variants() {
        use std::time::Duration;
        let backend = OutputSourceError::BackendUnavailable(Box::new(TestError("down")));
        let timeout = OutputSourceError::Timeout(Duration::from_secs(1));
        let exec = OutputSourceError::ExecutionFailed(Box::new(TestError("bad")));
        let panicked = OutputSourceError::Panicked("boom".to_string());

        assert!(backend.is_retryable());
        assert!(timeout.is_retryable());
        assert!(!exec.is_retryable());
        assert!(!panicked.is_retryable());
    }

    #[test]
    fn panicked_carries_message() {
        let err = OutputSourceError::Panicked("agent shim crashed".to_string());
        assert_eq!(
            err.to_string(),
            "output source panicked: agent shim crashed"
        );
    }

    #[test]
    fn metadata_mode_default_is_inline() {
        struct Bare;
        impl OutputSource<String, String> for Bare {
            async fn produce(&self, _input: &String) -> Result<String, OutputSourceError> {
                Ok(String::new())
            }
        }
        let bare = Bare;
        assert_eq!(bare.metadata_mode(), "inline");
    }
}
