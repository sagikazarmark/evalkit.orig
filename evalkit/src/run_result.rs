use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::{ResourceUsage, Score, ScoreDefinition, ScorerError};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScoredEntry {
    #[serde(with = "score_result_serde")]
    pub result: Result<Score, ScorerError>,
    #[serde(default)]
    pub reasoning: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrialResult {
    pub scores: HashMap<String, ScoredEntry>,
    pub duration: Duration,
    pub trial_index: usize,
    #[serde(default)]
    pub source_metadata: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SampleResult {
    pub sample_id: String,
    pub trials: Vec<TrialResult>,
    pub trial_count: usize,
    pub scored_count: usize,
    pub error_count: usize,
    #[serde(default)]
    pub token_usage: TokenUsage,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub source_resources: ResourceUsage,
    #[serde(default)]
    pub scorer_resources: ResourceUsage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunMetadata {
    pub run_id: String,
    pub seed: Option<u64>,
    pub dataset_fingerprint: String,
    pub scorer_fingerprint: String,
    #[serde(default)]
    pub code_commit: Option<String>,
    #[serde(default)]
    pub code_fingerprint: Option<String>,
    #[serde(default)]
    pub judge_model_pins: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration: Duration,
    pub trial_count: usize,
    pub score_definitions: Vec<ScoreDefinition>,
    pub source_mode: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunResult {
    pub metadata: RunMetadata,
    pub samples: Vec<SampleResult>,
}

mod score_result_serde {
    use super::*;

    #[derive(Serialize, Deserialize)]
    enum ScoreResultOwned {
        Ok(Score),
        Err(String),
    }

    pub fn serialize<S>(
        result: &Result<Score, ScorerError>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = match result {
            Ok(score) => ScoreResultOwned::Ok(score.clone()),
            Err(err) => ScoreResultOwned::Err(err.to_string()),
        };
        value.serialize(serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Result<Score, ScorerError>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = ScoreResultOwned::deserialize(deserializer)?;
        Ok(match raw {
            ScoreResultOwned::Ok(score) => Ok(score),
            ScoreResultOwned::Err(message) => {
                Err(ScorerError::internal(SerializedScorerError(message)))
            }
        })
    }

    #[derive(Debug)]
    struct SerializedScorerError(String);

    impl Display for SerializedScorerError {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            f.write_str(&self.0)
        }
    }

    impl Error for SerializedScorerError {}
}

#[cfg(test)]
mod sample_result_tests {
    use super::*;

    #[test]
    fn sample_result_default_resources_are_zero() {
        let sr = SampleResult {
            sample_id: "s1".to_string(),
            trials: vec![],
            trial_count: 0,
            scored_count: 0,
            error_count: 0,
            token_usage: TokenUsage::default(),
            cost_usd: None,
            source_resources: ResourceUsage::default(),
            scorer_resources: ResourceUsage::default(),
        };
        assert_eq!(sr.source_resources, ResourceUsage::default());
    }
}

#[cfg(test)]
mod entry_tests {
    use super::*;

    #[test]
    fn scored_entry_serializes_and_deserializes() {
        use serde_json::json;
        let entry = ScoredEntry {
            result: Ok(Score::Binary(true)),
            reasoning: Some("matches".to_string()),
            metadata: HashMap::from([("k".to_string(), json!("v"))]),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let round_trip: ScoredEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(round_trip.reasoning.as_deref(), Some("matches"));
    }
}
