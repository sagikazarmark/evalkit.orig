use evalkit::Score;
use serde_json::json;

#[test]
fn score_numeric_serializes_as_tagged_json() {
    let score = Score::Numeric(0.85);

    let value = serde_json::to_value(&score).expect("numeric score should serialize");

    assert_eq!(value, json!({ "type": "numeric", "value": 0.85 }));
}

#[test]
fn score_binary_round_trips_through_json() {
    let score = Score::Binary(true);

    let value = serde_json::to_value(&score).expect("binary score should serialize");
    let decoded: Score =
        serde_json::from_value(value.clone()).expect("binary score should deserialize");

    assert_eq!(value, json!({ "type": "binary", "value": true }));
    assert_eq!(decoded, score);
}

#[test]
fn score_label_round_trips_through_json() {
    let score = Score::Label("supported".to_string());

    let value = serde_json::to_value(&score).expect("label score should serialize");
    let decoded: Score =
        serde_json::from_value(value.clone()).expect("label score should deserialize");

    assert_eq!(value, json!({ "type": "label", "value": "supported" }));
    assert_eq!(decoded, score);
}

#[test]
fn score_metric_round_trips_through_json() {
    let score = Score::Metric {
        name: "latency".to_string(),
        value: 123.4,
        unit: Some("ms".to_string()),
    };

    let value = serde_json::to_value(&score).expect("metric score should serialize");
    let decoded: Score =
        serde_json::from_value(value.clone()).expect("metric score should deserialize");

    assert_eq!(
        value,
        json!({
            "type": "metric",
            "name": "latency",
            "value": 123.4,
            "unit": "ms"
        })
    );
    assert_eq!(decoded, score);
}

#[test]
fn score_structured_round_trips_through_json() {
    let score = Score::Structured {
        score: 0.75,
        reasoning: "rubric matched expected answer".to_string(),
        metadata: json!({ "judge": "gpt-4.1", "criteria": ["correctness"] }),
    };

    let value = serde_json::to_value(&score).expect("structured score should serialize");
    let decoded: Score =
        serde_json::from_value(value.clone()).expect("structured score should deserialize");

    assert_eq!(
        value,
        json!({
            "type": "structured",
            "score": 0.75,
            "reasoning": "rubric matched expected answer",
            "metadata": {
                "judge": "gpt-4.1",
                "criteria": ["correctness"]
            }
        })
    );
    assert_eq!(decoded, score);
}
