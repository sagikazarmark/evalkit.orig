use std::error::Error;
use std::fmt::{self, Display, Formatter};

use evalkit::{MapError, Mapper};

#[derive(Debug)]
struct TestError(&'static str);

impl Display for TestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl Error for TestError {}

#[test]
fn closure_mapper_maps_string_to_length() {
    let mapper = |input: &String| -> Result<usize, MapError> { Ok(input.len()) };

    let mapped = mapper
        .map(&"answer".to_string())
        .expect("closure mapper should be callable via the trait");

    assert_eq!(mapped, 6);
}

#[test]
fn mapper_trait_is_object_safe_for_shared_use() {
    let mapper = |input: &String| -> Result<String, MapError> { Ok(input.to_uppercase()) };
    let mapper: &dyn Mapper<String, String> = &mapper;

    let mapped = mapper
        .map(&"mixedCase".to_string())
        .expect("trait object mapper should execute");

    assert_eq!(mapped, "MIXEDCASE");
}

#[test]
fn mapper_supports_non_string_generics() {
    let mapper = |input: &Vec<i32>| -> Result<i32, MapError> { Ok(input.iter().sum()) };

    let mapped = mapper
        .map(&vec![1, 2, 3, 4])
        .expect("mapper should support arbitrary generic types");

    assert_eq!(mapped, 10);
}

#[test]
fn map_error_wraps_and_exposes_inner_error() {
    let mapper = |_input: &String| -> Result<String, MapError> {
        Err(MapError(Box::new(TestError("failed to extract output"))))
    };

    let err = mapper
        .map(&"trace payload".to_string())
        .expect_err("mapper should propagate mapping failures");

    assert_eq!(err.to_string(), "failed to extract output");
    assert_eq!(
        err.source()
            .expect("map errors should expose their source")
            .to_string(),
        "failed to extract output"
    );
}
