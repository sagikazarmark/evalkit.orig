use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::Sample;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Dataset<I, R = ()> {
    pub samples: Vec<Sample<I, R>>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl<I, R> Dataset<I, R> {
    pub fn new(samples: Vec<Sample<I, R>>) -> Self {
        Self {
            samples,
            metadata: HashMap::new(),
        }
    }
}

impl<I, R> From<Vec<Sample<I, R>>> for Dataset<I, R> {
    fn from(samples: Vec<Sample<I, R>>) -> Self {
        Self::new(samples)
    }
}
