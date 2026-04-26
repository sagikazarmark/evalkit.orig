use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SampleBuildError {
    EmptyId,
}

impl fmt::Display for SampleBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId => write!(f, "sample id must not be empty"),
        }
    }
}

impl std::error::Error for SampleBuildError {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sample<I, R = ()> {
    pub id: String,
    pub input: I,
    pub reference: Option<R>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl<I: Hash, R: Hash> Sample<I, R> {
    /// Creates a sample with an auto-generated deterministic ID.
    pub fn new(input: I, reference: R) -> Self {
        Self {
            id: sample_id(&input, Some(&reference)),
            input,
            reference: Some(reference),
            metadata: HashMap::new(),
        }
    }
}

impl<I, R> Sample<I, R> {
    /// Builder for samples with an explicit ID or no reference.
    pub fn builder(input: I) -> SampleBuilder<I, R> {
        SampleBuilder {
            id: None,
            input,
            reference: None,
            metadata: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SampleBuilder<I, R = ()> {
    id: Option<String>,
    input: I,
    reference: Option<R>,
    metadata: HashMap<String, serde_json::Value>,
}

impl<I, R> SampleBuilder<I, R> {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn reference(mut self, reference: R) -> Self {
        self.reference = Some(reference);
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

impl<I: Hash, R: Hash> SampleBuilder<I, R> {
    pub fn build(self) -> Result<Sample<I, R>, SampleBuildError> {
        let id = match self.id {
            Some(id) if id.is_empty() => return Err(SampleBuildError::EmptyId),
            Some(id) => id,
            None => sample_id(&self.input, self.reference.as_ref()),
        };

        Ok(Sample {
            id,
            input: self.input,
            reference: self.reference,
            metadata: self.metadata,
        })
    }
}

// The standard library's default hashers intentionally vary; this keeps IDs
// stable for the same hashed content across process runs.
fn sample_id<I: Hash, R: Hash>(input: &I, reference: Option<&R>) -> String {
    let mut hasher = StableHasher::default();

    0_u8.hash(&mut hasher);
    input.hash(&mut hasher);
    1_u8.hash(&mut hasher);
    reference.is_some().hash(&mut hasher);

    if let Some(reference) = reference {
        2_u8.hash(&mut hasher);
        reference.hash(&mut hasher);
    }

    format!("{:016x}", hasher.finish())
}

#[derive(Default)]
struct StableHasher {
    state: u64,
}

impl Hasher for StableHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x0000_0100_0000_01b3;

        if self.state == 0 {
            self.state = OFFSET_BASIS;
        }

        for byte in bytes {
            self.state ^= u64::from(*byte);
            self.state = self.state.wrapping_mul(PRIME);
        }
    }
}
