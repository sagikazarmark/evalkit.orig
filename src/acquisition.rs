use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::time::Duration;

#[derive(Debug)]
pub enum AcquisitionError {
    ExecutionFailed(Box<dyn Error + Send + Sync>),
    TraceNotFound {
        correlation_id: String,
        sample_id: String,
    },
    BackendUnavailable(Box<dyn Error + Send + Sync>),
    Timeout(Duration),
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
        }
    }
}

impl Error for AcquisitionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ExecutionFailed(err) | Self::BackendUnavailable(err) => Some(err.as_ref()),
            Self::TraceNotFound { .. } | Self::Timeout(_) => None,
        }
    }
}

#[allow(async_fn_in_trait)]
pub trait Acquisition<I, O>: Send + Sync {
    async fn acquire(&self, input: &I) -> Result<O, AcquisitionError>;
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
