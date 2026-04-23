//! Eval kernel crate bootstrap.

mod acquisition;
mod comparison;
mod dataset;
mod executor;
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
    current_sample_id,
};
pub use comparison::{
    Change, CompareConfig, Comparison, SampleComparison, ScorerComparison, compare,
};
pub use dataset::Dataset;
pub use executor::{
    AlwaysSampler, DatasetSource, ExecutionSink, Executor, ExecutorBoxError, ExecutorError,
    JsonlFileTailSource, NoopScrubber, NoopSink, PercentSampler, PullExecutor,
    RegexPiiScrubber, SampleSource, Sampler, SamplerBuildError, Scrubber, ShardBuildError,
    ShardSpec, ShardedSource, ShutdownMode, StringPrefixCheckpoint, StringStreamStage,
    TargetedSampler,
};
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
        Acquisition, AcquisitionError, AcquisitionMetadata, AndScorer, Change, CompareConfig,
        AcquisitionSnapshot, AcquiredOutput, Comparison, Dataset, DatasetSource, Direction,
        ExecutionSink, Executor, ExecutorBoxError, ExecutorError, IgnoreReferenceScorer,
        JsonlFileTailSource, MapError, MapScoreScorer, Mapper, NoopScrubber, NoopSink,
        NotScorer, OrScorer, PercentSampler, PullExecutor, RegexPiiScrubber,
        RUN_RESULT_SCHEMA_VERSION, Run, RunBuildError, RunError, RunMetadata, RunResult, RunStats,
        Sample, SampleBuildError, SampleBuilder, SampleComparison, SampleSource, Sampler,
        SamplerBuildError, Score, ScoreDefinition, ScoreOutcome, Scorer, ScorerComparison,
        ScorerContext, ScorerError, ScorerExt, ScorerMetadata, ScorerResources, ScorerSet,
        ScorerStats, Scrubber, ShardBuildError, ShardSpec, ShardedSource, ShutdownMode,
        StringPrefixCheckpoint, StringStreamStage, ThenScorer, TimeoutScorer, TokenUsage,
        ToolCall, ToolResult, TrajectorySample, TrajectoryStep, WeightedScorer, compare,
        current_sample_id, ignore_reference, read_jsonl, write_jsonl,
    };
    pub use crate::{AlwaysSampler, ConversationSample, ConversationTurn, TargetedSampler};
}
