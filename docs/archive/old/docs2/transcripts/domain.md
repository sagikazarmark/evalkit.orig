> **📦 Archived on 2026-04-23** — superseded by no successor — this work was abandoned. Kept for historical reference.

# Domain Language Review Transcript

**Date**: 2026-04-03
**Task**: Final domain language review of `spec/eval-kernel.md` using `spec-domain-review.md` methodology
**Spec**: The Eval Kernel — a Rust crate for typed evaluation primitives with OTel trace observation and multi-trial statistical aggregation

---

## Context

User requested a "final final domain review" before moving to implementation. The review followed the 5-phase `spec-domain-review.md` methodology: Extract → Cross-Reference → Gap Analysis → Recommendations → Glossary Rewrite.

### Documents Read

- `spec/eval-kernel.md` — the specification under review
- `spec/decisions-log.md` — terminology decisions from specification rounds
- `research/02-domain-language.md` — primary terminology reference
- `research/01-landscape.md` — 65+ existing projects and their naming
- `research/03-user-workflows.md` — how users describe evaluation tasks
- `research/04-architecture.md` — technical patterns and their names
- `research/06-ecosystem.md` — standards, protocols, and their vocabulary
- `brainstorm/directions.md` — the direction this spec implements

---

## Phase 1 — Extract

93 domain terms extracted across 14 sections of the specification:

**API Surface terms (27):** Sample, Dataset, Score, ScoreDefinition, Direction, Scorer, ScorerContext, ScorerError, ScorerSet, Mapper, Acquisition, Observe, TraceBackend, JaegerBackend, Run, RunResult, exact_match, contains, regex, json_schema, llm_judge, compare, write_jsonl/read_jsonl, .trials(N), .execute(), .stats(), .map_output()/.map_reference()

**Data Model terms (32):** input, reference, id, Numeric, Binary, Label, Metric (Score variants), TrialResult, SampleResult, RunMetadata, RunResult, ScorerStats, RunStats, Comparison, ScorerComparison, SampleComparison, Change (Improved/Regressed/Unchanged/Insignificant/Incomparable), baseline, candidate, Span, SpanEvent, pass_at_k, pass_all_k, pass_rate, correlation_id, sample_attribute, acquisition_mode, trial_count, scored_count, error_count

**Error & Status terms (17):** RunBuildError (NoDataset, NoAcquisition, NoScorer, EmptyDataset, DuplicateSampleIds, DuplicateScorerNames, MissingSampleIds), RunError (Build, Internal), AcquisitionError (ExecutionFailed, TraceNotFound, BackendUnavailable, Timeout), ScorerError, MapError, TraceBackendError

**Config & Integration terms (12):** otel, llm-judge (feature gates), LlmJudgeConfig, CompareConfig, JSONL, Jaeger v2 API, OTLP, eval.sample_id, eval.run_id, correlation_id, sample_attribute, confidence_level

**Conceptual terms (27):** evaluation/eval, scorer, score, sample, dataset, reference, trial, run, acquisition, inline, observe, mapper, trace, span, trajectory, LLM-as-judge, multi-trial, confidence interval, significance testing, scorer composition, scorer set, score definition, direction, regression, benchmark, pass_at_k, pass_all_k

**Domain identified**: AI/LLM Evaluation, intersecting with OpenTelemetry observability and applied statistics.

User confirmed. Proceeded to Phase 2.

---

## Phase 2 — Cross-Reference Against Research

Every extracted term was checked against:
1. `02-domain-language.md` (primary reference)
2. `01-landscape.md` (existing project naming)
3. `03-user-workflows.md` (user vocabulary)
4. `06-ecosystem.md` (standards vocabulary)
5. Spec's own glossary (Section 14)

### Key Findings

**Well-aligned terms (25):** Dataset, Score, evaluation/eval, Trial, Run, Trace, Span, Trajectory, pass_at_k, pass_all_k, baseline, candidate, Binary, JSONL, pass_rate, ScorerError (concept), regression, confidence interval, significance testing, JaegerBackend, OTLP, SpanEvent

**Deliberate choices in fractured domains (3):** Sample (follows RAGAS over Inspect AI's "Task" and DeepEval's "Test Case"), Scorer (follows Inspect AI/Braintrust over Anthropic's "Grader" and LangChain's "Evaluator"), reference (follows RAGAS over Microsoft's "Ground Truth" and Inspect AI's "Target"). All documented in glossary with rationale.

**Novel terms (23):** Acquisition, inline, observe, Mapper, ScorerSet, ScorerContext, ScoreDefinition, Direction, RunResult, RunStats, ScorerStats, Change, TraceBackend, Comparison, correlation_id, sample_attribute, Numeric, Label, MapError, AcquisitionError, RunBuildError, RunError, TraceBackendError. All represent genuinely new abstractions.

**Drift items (2):**
- `Metric` Score variant — collision with DeepEval/RAGAS where "Metric" means the scorer component
- `llm_judge`/`llm-judge` — shortened from canonical "LLM-as-a-Judge"

**Glossary gaps (10):** Terms used in spec body but absent from Section 14:
1. LLM-as-a-Judge (core domain concept)
2. TrialResult (data model)
3. SampleResult (data model)
4. RunMetadata (data model)
5. ScorerComparison (data model)
6. SampleComparison (data model)
7. TraceBackendError (error type)
8. SpanEvent (OTel type)
9. correlation_id (observe mode concept)
10. Observe (struct type)

User confirmed. Proceeded to Phase 3.

---

## Phase 3 — Gap Analysis

| Status | Count | High | Medium | Low |
|--------|-------|------|--------|-----|
| ALIGNED | 25 | 0 | 0 | 0 |
| DRIFT | 2 | 0 | 1 | 1 |
| MISNAMED | 0 | 0 | 0 | 0 |
| INCONSISTENCY | 0 | 0 | 0 | 0 |
| MODELING ISSUE | 0 | 0 | 0 | 0 |
| NOVEL TERM | 23 | 0 | 0 | 23 |
| TERM SHEET DRIFT | 7 | 0 | 3 | 4 |
| **TOTAL** | **57** | **0** | **4** | **28** |

**Assessment**: Exceptionally clean spec. Zero high-severity issues. Zero misnamed terms. Zero internal inconsistencies. Zero modeling issues. The Round 0 terminology gate was highly effective.

**Medium-severity items (4):**
1. `Metric` Score variant — domain collision with DeepEval/RAGAS naming
2. `LLM-as-a-Judge` — missing glossary entry for core domain concept
3. `TrialResult`/`SampleResult` — missing glossary entries despite RunResult being present
4. `correlation_id` — central observe-mode concept with no glossary entry

User confirmed. Proceeded to Phase 4.

---

## Phase 4 — Recommendations

### No renames needed

Every term is either domain-aligned, a justified novel term, or a documented deliberate choice.

### Documentation fixes

1. **Strengthen Metric collision warning** in glossary — change from "Potential confusion" to explicit domain collision note with concrete mapping (DeepEval `Metric` = this project's `Scorer`; DeepEval metric output = this project's `Score::Metric`)

2. **Standardize prose** — use canonical "LLM-as-a-Judge" (with article) in all prose; keep `llm_judge` (snake_case) and `llm-judge` (kebab-case) for code/feature gates

3. **Add 10 glossary entries**: LLM-as-a-Judge, TrialResult, SampleResult, RunMetadata, ScorerComparison, SampleComparison, TraceBackendError, SpanEvent, correlation_id, sample_attribute

### All 23 novel terms validated as justified

Each names a genuinely new concept with no domain equivalent. Glossary definitions assessed as adequate, with minor improvement suggestions for Acquisition (add example of both modes) and ScoreDefinition (add metrology analogy).

User confirmed. Proceeded to Phase 5.

---

## Phase 5 — Glossary Rewrite

Produced replacement for Section 14 with:
- 49 entries (up from 37) organized into 9 subsections: Core Concepts, Scoring, Mapping & Transformation, Acquisition, OTel/Observability, Evaluation Execution, Results, Statistics, Comparison, Errors
- Every domain term used anywhere in the spec has an entry
- Stronger Metric collision warning with concrete DeepEval/RAGAS mapping
- New LLM-as-a-Judge entry with casing convention notes
- New entries for all identified glossary gaps

### Deliverables saved

- `spec/domain-language-review.md` — full Phases 1–4 analysis
- `spec/glossary-rewrite.md` — standalone glossary document

---

## Changes Applied to Spec

User said "do it" — all recommendations applied directly to `spec/eval-kernel.md`:

1. **Section 14 (Glossary) replaced** — expanded from 37 to 49 entries across 9 organized subsections. New entries: evaluation, LLM-as-a-Judge, TrialResult, SampleResult, RunMetadata, correlation_id, sample_attribute, ScorerComparison, SampleComparison, TraceBackendError, SpanEvent, Observe. Metric collision warning strengthened.

2. **Prose standardized** — 8 instances of "LLM-judge"/"LLM-as-judge" in prose changed to "LLM-as-a-Judge". Feature gate names (`llm-judge`) and code (`llm_judge`) left unchanged. Affected sections: §2, §3, §4.1, §4.3, §7, §8, §11.

### Files modified
- `spec/eval-kernel.md` — glossary replacement + prose fixes

### Files created
- `spec/domain-language-review.md` — full analysis
- `spec/glossary-rewrite.md` — standalone glossary
- `transcripts/domain.md` — this transcript
