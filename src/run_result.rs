use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{Score, ScoreDefinition, ScorerError};

#[derive(Debug, Serialize, Deserialize)]
pub struct TrialResult {
    #[serde(with = "score_results_serde")]
    pub scores: HashMap<String, Result<Score, ScorerError>>,
    pub duration: Duration,
    pub trial_index: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SampleResult {
    pub sample_id: String,
    pub trials: Vec<TrialResult>,
    pub trial_count: usize,
    pub scored_count: usize,
    pub error_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunMetadata {
    pub run_id: String,
    pub seed: Option<u64>,
    pub dataset_fingerprint: String,
    pub scorer_fingerprint: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration: Duration,
    pub trial_count: usize,
    pub score_definitions: Vec<ScoreDefinition>,
    pub acquisition_mode: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunResult {
    pub metadata: RunMetadata,
    pub samples: Vec<SampleResult>,
}

mod score_results_serde {
    use super::*;

    #[derive(Serialize, Deserialize)]
    enum ScoreResultOwned {
        Ok(Score),
        Err(String),
    }

    #[derive(Debug)]
    struct SerializedScorerError(String);

    impl Display for SerializedScorerError {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            f.write_str(&self.0)
        }
    }

    impl Error for SerializedScorerError {}

    pub fn serialize<S>(
        scores: &HashMap<String, Result<Score, ScorerError>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut entries: Vec<_> = scores.iter().collect();
        entries.sort_by(|(left_name, _), (right_name, _)| left_name.cmp(right_name));

        let mut map = serializer.serialize_map(Some(entries.len()))?;

        for (name, result) in entries {
            let value = match result {
                Ok(score) => ScoreResultOwned::Ok(score.clone()),
                Err(error) => ScoreResultOwned::Err(error.to_string()),
            };

            map.serialize_entry(name, &value)?;
        }

        map.end()
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<String, Result<Score, ScorerError>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = HashMap::<String, ScoreResultOwned>::deserialize(deserializer)?;

        Ok(raw
            .into_iter()
            .map(|(name, result)| {
                let value = match result {
                    ScoreResultOwned::Ok(score) => Ok(score),
                    ScoreResultOwned::Err(message) => {
                        Err(ScorerError::internal(SerializedScorerError(message)))
                    }
                };

                (name, value)
            })
            .collect())
    }
}
