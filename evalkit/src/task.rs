//! Named adapter for closure-based active output sources.
//!
//! [`Task<I, O>`] wraps an async function behind a concrete, named type that
//! implements [`OutputSource`]. Closures already work through the blanket impl
//! on the trait; `Task` is for cases where you want to *name* the source value
//! — at config time, for reuse, or as part of a builder chain:
//!
//! ```rust,ignore
//! Eval::new(samples)
//!     .source(Task::from_fn(|input| async move { /* … */ }))
//!     .scorer(exact_match)
//! ```
//!
//! HTTP and subprocess plugins (`evalkit_providers::HttpSource`,
//! `evalkit_providers::SubprocessSource`) are first-class [`OutputSource`]
//! types in their own right — pass them directly to `.source(...)` rather than
//! wrapping them in `Task`. This crate intentionally has no dependency on
//! `evalkit-providers`.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::source::{OutputSource, OutputSourceError, SourceMetadata};

type ProduceFn<I, O> = Arc<
    dyn Fn(&I) -> Pin<Box<dyn Future<Output = Result<O, OutputSourceError>> + Send>>
        + Send
        + Sync,
>;

/// A named, concrete adapter that wraps an async function as an [`OutputSource`].
///
/// # Usage
///
/// ```rust,ignore
/// Eval::new(samples)
///     .source(Task::from_fn(|input| async move { /* … */ }))
///     .scorer(exact_match)
/// ```
///
/// For HTTP and subprocess plugins, use `evalkit_providers::HttpSource` and
/// `evalkit_providers::SubprocessSource` directly — they implement
/// [`OutputSource`] and need no wrapper.
pub struct Task<I, O> {
    produce: ProduceFn<I, O>,
    mode: &'static str,
}

impl<I, O> Task<I, O>
where
    I: Clone + Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    /// Wrap an async closure as a named task.
    ///
    /// The closure receives a shared reference to the input; `I: Clone` is required
    /// because the boxed future must be `'static`, so the input is cloned before the
    /// future is spawned.  This adds one clone per call relative to a raw closure —
    /// acceptable for an adapter type.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let task: Task<String, String> = Task::from_fn(|input: &String| {
    ///     let input = input.clone();
    ///     async move { Ok(format!("echo::{input}")) }
    /// });
    /// ```
    pub fn from_fn<F, Fut>(f: F) -> Self
    where
        F: Fn(&I) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<O, OutputSourceError>> + Send + 'static,
    {
        let f = Arc::new(f);
        Self {
            produce: Arc::new(move |input: &I| {
                let f = Arc::clone(&f);
                let owned = input.clone();
                Box::pin(async move { f(&owned).await })
            }),
            mode: "inline",
        }
    }

}

impl<I, O> OutputSource<I, O> for Task<I, O>
where
    I: Send + Sync,
    O: Send + Sync,
{
    async fn produce(&self, input: &I) -> Result<O, OutputSourceError> {
        (self.produce)(input).await
    }

    fn metadata(&self) -> SourceMetadata {
        SourceMetadata { mode: self.mode }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OutputSource;

    #[tokio::test(flavor = "current_thread")]
    async fn task_from_fn_implements_output_source() {
        let task: Task<String, String> = Task::from_fn(|input: &String| {
            let input = input.clone();
            async move { Ok(format!("echo::{input}")) }
        });

        let input = String::from("hello");
        let output = task.produce(&input).await.unwrap();
        assert_eq!(output, "echo::hello");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn task_from_fn_metadata_reports_inline_mode() {
        let task: Task<String, String> = Task::from_fn(|_input: &String| async move {
            Ok(String::from("ok"))
        });
        assert_eq!(task.metadata().mode, "inline");
    }
}
