use evalkit::{Direction, ScoreDefinition};

#[test]
fn score_definition_new_defaults_to_no_direction() {
    let definition = ScoreDefinition::new("exact_match");

    assert_eq!(
        definition,
        ScoreDefinition {
            name: "exact_match".to_string(),
            direction: None,
        }
    );
}

#[test]
fn score_definition_maximize_sets_maximize_direction() {
    let definition = ScoreDefinition::maximize("accuracy");

    assert_eq!(
        definition,
        ScoreDefinition {
            name: "accuracy".to_string(),
            direction: Some(Direction::Maximize),
        }
    );
}

#[test]
fn score_definition_minimize_sets_minimize_direction() {
    let definition = ScoreDefinition::minimize("latency_ms");

    assert_eq!(
        definition,
        ScoreDefinition {
            name: "latency_ms".to_string(),
            direction: Some(Direction::Minimize),
        }
    );
}

#[test]
fn score_definition_round_trips_through_json() {
    let definition = ScoreDefinition::maximize("accuracy");

    let json = serde_json::to_string(&definition).unwrap();
    let round_trip: ScoreDefinition = serde_json::from_str(&json).unwrap();

    assert_eq!(round_trip, definition);
}
