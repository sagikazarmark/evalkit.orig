//! Eval kernel crate bootstrap.

mod acquisition;
mod comparison;
mod dataset;
mod jsonl;
mod mapper;
mod math;
#[cfg(feature = "otel")]
mod otel;
mod run;
mod run_result;
mod sample;
mod score;
mod score_definition;
mod scorer;
mod scorer_context;
mod scorer_error;
mod scorer_set;
pub mod scorers;
mod stats;

pub use acquisition::{Acquisition, AcquisitionError};
pub use comparison::{
    Change, CompareConfig, Comparison, SampleComparison, ScorerComparison, compare,
};
pub use dataset::Dataset;
pub use jsonl::{read_jsonl, write_jsonl};
pub use mapper::{MapError, Mapper};
#[cfg(feature = "otel")]
pub use otel::{JaegerBackend, Observe, Span, SpanEvent, TraceBackend, TraceBackendError};
pub use run::{Run, RunBuildError, RunError};
pub use run_result::{RunMetadata, RunResult, SampleResult, TrialResult};
pub use sample::{Sample, SampleBuildError, SampleBuilder};
pub use score::Score;
pub use score_definition::{Direction, ScoreDefinition};
pub use scorer::Scorer;
pub use scorer_context::ScorerContext;
pub use scorer_error::ScorerError;
pub use scorer_set::ScorerSet;
#[cfg(feature = "llm-judge")]
pub use scorers::{LlmJudgeConfig, LlmJudgeScoreExtractor, llm_judge};
pub use scorers::{contains, exact_match, json_schema, regex};
pub use stats::{RunStats, ScorerStats};

pub mod prelude {
    pub use crate::{
        Acquisition, AcquisitionError, Change, CompareConfig, Comparison, Dataset, Direction,
        MapError, Mapper, Run, RunBuildError, RunError, RunMetadata, RunResult, RunStats, Sample,
        SampleBuildError, SampleBuilder, SampleComparison, Score, ScoreDefinition, ScorerComparison,
        ScorerContext, ScorerError, ScorerSet, ScorerStats, compare, read_jsonl, write_jsonl,
    };
    pub use crate::scorers::{contains, exact_match, json_schema, regex};
    #[cfg(feature = "llm-judge")]
    pub use crate::{LlmJudgeConfig, LlmJudgeScoreExtractor, llm_judge};
    #[cfg(feature = "otel")]
    pub use crate::{JaegerBackend, Observe, Span, SpanEvent, TraceBackend, TraceBackendError};
}
