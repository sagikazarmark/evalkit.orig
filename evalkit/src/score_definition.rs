use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Maximize,
    Minimize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreDefinition {
    pub name: String,
    pub direction: Option<Direction>,
}

impl ScoreDefinition {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            direction: None,
        }
    }

    pub fn maximize(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            direction: Some(Direction::Maximize),
        }
    }

    pub fn minimize(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            direction: Some(Direction::Minimize),
        }
    }
}
