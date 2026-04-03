use std::error::Error;
use std::fmt::{self, Display, Formatter};

use evalkit::ScorerError;

#[derive(Debug)]
struct TestError(&'static str);

impl Display for TestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl Error for TestError {}

#[test]
fn scorer_error_wraps_and_displays_inner_error() {
    let err = ScorerError(Box::new(TestError("invalid regex pattern")));

    assert_eq!(err.to_string(), "invalid regex pattern");
}

#[test]
fn scorer_error_exposes_inner_error_as_source() {
    let err = ScorerError(Box::new(TestError("network failure")));

    let source = err.source().expect("wrapped errors should expose a source");

    assert_eq!(source.to_string(), "network failure");
}

#[test]
fn scorer_error_implements_error_send_and_sync() {
    fn assert_error_send_sync<T: Error + Send + Sync>() {}

    assert_error_send_sync::<ScorerError>();
}

#[test]
fn scorer_error_debug_includes_wrapper_type() {
    let err = ScorerError(Box::new(TestError("mapper failure")));

    let debug = format!("{err:?}");

    assert!(debug.contains("ScorerError"));
    assert!(debug.contains("mapper failure"));
}
