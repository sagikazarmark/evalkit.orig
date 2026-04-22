use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Score {
    Numeric(f64),
    Binary(bool),
    Label(String),
    Structured {
        score: f64,
        reasoning: String,
        metadata: Value,
    },
    Metric {
        name: String,
        value: f64,
        unit: Option<String>,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ScoreSerde {
    Numeric {
        value: f64,
    },
    Binary {
        value: bool,
    },
    Label {
        value: String,
    },
    Structured {
        score: f64,
        reasoning: String,
        metadata: Value,
    },
    Metric {
        name: String,
        value: f64,
        unit: Option<String>,
    },
}

impl Serialize for Score {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = match self {
            Self::Numeric(value) => ScoreSerde::Numeric { value: *value },
            Self::Binary(value) => ScoreSerde::Binary { value: *value },
            Self::Label(value) => ScoreSerde::Label {
                value: value.clone(),
            },
            Self::Structured {
                score,
                reasoning,
                metadata,
            } => ScoreSerde::Structured {
                score: *score,
                reasoning: reasoning.clone(),
                metadata: metadata.clone(),
            },
            Self::Metric { name, value, unit } => ScoreSerde::Metric {
                name: name.clone(),
                value: *value,
                unit: unit.clone(),
            },
        };

        value.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Score {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match ScoreSerde::deserialize(deserializer)? {
            ScoreSerde::Numeric { value } => Self::Numeric(value),
            ScoreSerde::Binary { value } => Self::Binary(value),
            ScoreSerde::Label { value } => Self::Label(value),
            ScoreSerde::Structured {
                score,
                reasoning,
                metadata,
            } => Self::Structured {
                score,
                reasoning,
                metadata,
            },
            ScoreSerde::Metric { name, value, unit } => Self::Metric { name, value, unit },
        })
    }
}
