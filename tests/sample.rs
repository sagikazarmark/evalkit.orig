use evalkit::Sample;
use serde_json::json;

#[test]
fn sample_new_constructs_input_and_reference() {
    let sample = Sample::new("What is 2+2?".to_string(), "4".to_string());

    assert!(!sample.id.is_empty());
    assert_eq!(sample.input, "What is 2+2?");
    assert_eq!(sample.reference.as_deref(), Some("4"));
    assert!(sample.metadata.is_empty());
}

#[test]
fn sample_new_generates_deterministic_ids_from_content() {
    let first = Sample::new("Capital of France?".to_string(), "Paris".to_string());
    let second = Sample::new("Capital of France?".to_string(), "Paris".to_string());
    let changed = Sample::new("Capital of France?".to_string(), "Lyon".to_string());

    assert_eq!(first.id, second.id);
    assert_ne!(first.id, changed.id);
}

#[test]
fn sample_builder_supports_explicit_id_optional_reference_and_metadata() {
    let sample = Sample::<_, ()>::builder("prompt".to_string())
        .id("sample-42")
        .metadata("difficulty", json!("easy"))
        .build();

    assert_eq!(sample.id, "sample-42");
    assert_eq!(sample.input, "prompt");
    assert_eq!(sample.reference, None);
    assert_eq!(sample.metadata.get("difficulty"), Some(&json!("easy")));
}

#[test]
fn sample_builder_generates_id_when_reference_is_present() {
    let built = Sample::builder("hello".to_string())
        .reference("world".to_string())
        .build();
    let direct = Sample::new("hello".to_string(), "world".to_string());

    assert_eq!(built.id, direct.id);
    assert_eq!(built.reference.as_deref(), Some("world"));
}
