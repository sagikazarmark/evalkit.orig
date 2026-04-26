use evalkit::{Dataset, Sample};

#[test]
fn dataset_new_constructs_from_sample_vector() {
    let first = Sample::new("What is 2+2?".to_string(), "4".to_string());
    let second = Sample::new("What is 3+3?".to_string(), "6".to_string());
    let samples = vec![first.clone(), second.clone()];

    let dataset = Dataset::new(samples);

    assert_eq!(dataset.samples, vec![first, second]);
    assert!(dataset.metadata.is_empty());
}

#[test]
fn dataset_from_vec_matches_new_constructor() {
    let sample = Sample::new("Capital of France?".to_string(), "Paris".to_string());

    let dataset = Dataset::from(vec![sample.clone()]);

    assert_eq!(dataset.samples, vec![sample]);
    assert!(dataset.metadata.is_empty());
}
