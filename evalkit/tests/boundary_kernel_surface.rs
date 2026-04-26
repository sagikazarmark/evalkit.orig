//! Boundary contract test: the classified-KEEP kernel surface must stay
//! reachable from `evalkit`. If one of these `use` lines fails to compile,
//! the root crate shed something it was supposed to keep.
//!
//! Paired with the `compile_fail` doc tests in `src/lib.rs` that assert the
//! MOVE surface is *not* reachable. Together they pin the boundary.

#![allow(unused_imports, dead_code)]

use evalkit::{
    SourceOutput, OutputSource, OutputSourceError, SourceMetadata, OutputSnapshot,
    AndScorer, Change, CompareConfig, Comparison, ConversationSample, ConversationTurn, Dataset,
    Direction, Eval, EvalRun, EvalTask, IgnoreReferenceScorer, MapError, MapScoreScorer, Mapper,
    NotScorer, OrScorer, RUN_RESULT_SCHEMA_VERSION, Run, RunBuildError, RunError, RunMetadata,
    RunResult, RunStats, Sample, SampleBuildError, SampleBuilder, SampleComparison, SampleResult,
    Score, ScoreDefinition, ScoreOutcome, Scorer, ScorerComparison, ScorerContext, ScorerError,
    ScorerExt, ScorerMetadata, ScorerResources, ScorerSet, ScorerStats, ThenScorer, TimeoutScorer,
    TokenUsage, ToolCall, ToolResult, TrajectorySample, TrajectoryStep, TrialResult,
    WeightedScorer, compare, ignore_reference, read_jsonl, write_jsonl,
};

#[test]
fn kernel_keep_surface_compiles() {
    // Just touching the symbols would count, but exercise a couple of the
    // non-trivial ones so the test also doubles as a smoke check.
    let _ = RUN_RESULT_SCHEMA_VERSION;
    let _ = ScoreDefinition::maximize("keep_check");
    let _ = Direction::Maximize;
}
