//! Eval kernel crate bootstrap.
//!
//! This crate is the semver anchor for the workspace and hosts the batch eval
//! kernel: dataset, sample, scorer, run, result, comparison, and stats.
//! Runtime orchestration (executors, sources, sinks, samplers, sharding,
//! scrubbers, stream helpers) lives in the sibling `evalkit-runtime` crate
//! per `docs/root-crate-boundary-audit.md`.

pub mod acquisition;
mod comparison;
mod dataset;
mod eval;
mod jsonl;
mod mapper;
mod math;
mod run;
mod run_result;
mod sample;
mod sample_shapes;
pub mod schema;
mod score;
mod score_definition;
mod scorer;
mod scorer_context;
mod scorer_error;
mod scorer_ext;
mod scorer_set;
mod stats;

pub use acquisition::{
    AcquiredOutput, Acquisition, AcquisitionError, AcquisitionMetadata, AcquisitionSnapshot,
};
pub use comparison::{
    Change, CompareConfig, Comparison, SampleComparison, ScorerComparison, compare,
};
pub use dataset::Dataset;
pub use eval::{Eval, EvalRun, EvalTask};
pub use jsonl::{read_jsonl, write_jsonl};
pub use mapper::{MapError, Mapper};
pub use run::{Run, RunBuildError, RunError};
pub use run_result::{RunMetadata, RunResult, SampleResult, TokenUsage, TrialResult};
pub use sample::{Sample, SampleBuildError, SampleBuilder};
pub use sample_shapes::{
    ConversationSample, ConversationTurn, ToolCall, ToolResult, TrajectorySample, TrajectoryStep,
};
pub use schema::RUN_RESULT_SCHEMA_VERSION;
pub use score::Score;
pub use score_definition::{Direction, ScoreDefinition};
pub use scorer::{ScoreOutcome, Scorer, ScorerMetadata, ScorerResources};
pub use scorer_context::ScorerContext;
pub use scorer_error::ScorerError;
pub use scorer_ext::{
    AndScorer, IgnoreReferenceScorer, MapScoreScorer, NotScorer, OrScorer, ScorerExt, ThenScorer,
    TimeoutScorer, WeightedScorer, ignore_reference,
};
pub use scorer_set::ScorerSet;
pub use stats::{RunStats, ScorerStats};

pub mod prelude {
    pub use crate::{
        Acquisition, AcquisitionError, AcquisitionMetadata, AcquisitionSnapshot, AcquiredOutput,
        AndScorer, Change, CompareConfig, Comparison, Dataset, Direction, Eval, EvalRun, EvalTask,
        IgnoreReferenceScorer, MapError, MapScoreScorer, Mapper, NotScorer, OrScorer,
        RUN_RESULT_SCHEMA_VERSION, Run, RunBuildError, RunError, RunMetadata, RunResult, RunStats,
        Sample, SampleBuildError, SampleBuilder, SampleComparison,
        Score, ScoreDefinition, ScoreOutcome, Scorer, ScorerComparison,
        ScorerContext, ScorerError, ScorerExt, ScorerMetadata, ScorerResources, ScorerSet,
        ScorerStats, ThenScorer, TimeoutScorer, TokenUsage,
        ToolCall, ToolResult, TrajectorySample, TrajectoryStep, WeightedScorer, compare,
        ignore_reference, read_jsonl, write_jsonl,
    };
    pub use crate::{ConversationSample, ConversationTurn};
}
