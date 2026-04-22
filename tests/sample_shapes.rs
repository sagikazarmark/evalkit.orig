use evalkit::{
    ConversationSample, ConversationTurn, ToolCall, ToolResult, TrajectorySample, TrajectoryStep,
};
use serde_json::json;

#[test]
fn trajectory_sample_round_trips_tool_calls_and_reference() {
    let sample = TrajectorySample {
        input: String::from("prompt"),
        steps: vec![TrajectoryStep {
            role: String::from("assistant"),
            content: String::from("calling weather"),
            tool_calls: vec![ToolCall {
                call_id: String::from("call-1"),
                name: String::from("weather.lookup"),
                arguments: json!({ "city": "Paris" }),
            }],
            tool_results: vec![ToolResult {
                call_id: String::from("call-1"),
                content: json!({ "temperature_c": 18 }),
            }],
        }],
        reference: Some(String::from("18C")),
    };

    let decoded: TrajectorySample<String, String> =
        serde_json::from_str(&serde_json::to_string(&sample).unwrap()).unwrap();

    assert_eq!(decoded.input, "prompt");
    assert_eq!(decoded.reference.as_deref(), Some("18C"));
    assert_eq!(decoded.steps[0].tool_calls[0].call_id, "call-1");
    assert_eq!(
        decoded.steps[0].tool_results[0].content,
        json!({ "temperature_c": 18 })
    );
}

#[test]
fn conversation_sample_preserves_stable_turn_ids_and_order() {
    let sample = ConversationSample::<String, ()> {
        input: String::from("chat"),
        turns: vec![
            ConversationTurn {
                turn_id: String::from("turn-1"),
                role: String::from("user"),
                content: String::from("Hi"),
                tool_calls: Vec::new(),
                tool_results: Vec::new(),
            },
            ConversationTurn {
                turn_id: String::from("turn-2"),
                role: String::from("assistant"),
                content: String::from("Hello"),
                tool_calls: Vec::new(),
                tool_results: Vec::new(),
            },
        ],
        reference: None,
    };

    assert_eq!(sample.turns[0].turn_id, "turn-1");
    assert_eq!(sample.turns[1].turn_id, "turn-2");
    assert_eq!(sample.turns[1].content, "Hello");
}
