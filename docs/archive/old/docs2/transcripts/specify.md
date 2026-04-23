> **📦 Archived on 2026-04-23** — superseded by no successor — this work was abandoned. Kept for historical reference.

# Specification Session Transcript

**Date:** 2026-04-03
**Scope:** Unified spec — Eval Kernel + Trace Grader + Confident Eval
**Output:** `spec/eval-kernel.md`, `spec/decisions-log.md`

---

## Phase 1 — Anchor

**Assistant** presented the anchor: scope summary covering all three directions as a single Rust crate, non-goals (NOT a platform, NOT observability, NOT a CLI, etc.), 12 core technical decisions already made from brainstorm, 12 open questions carried forward, and asked for new constraints.

**User:** "Accurate. No new. No questions resolved."

---

## Phase 2, Round 0 — Terminology Gate

**Assistant** presented a 30-term term sheet cross-referenced against `02-domain-language.md`, `01-landscape.md`, and `06-ecosystem.md`. Key choices: Sample (not Task/Test Case), Dataset, Scorer (not Grader/Evaluator), Score, Numeric (not Continuous), Binary, Label, Metric, SampleResult (not Result — Rust collision), Run, RunReport, Trial, Acquisition (novel), inline/observe (novel), Correlation/Correlator (novel, later removed), SpanExtractor (novel, later removed).

**User:** "Generally, yes. I'd like to throw in one question: doesn't score suggest a numeric nature thing? Whereas it can be boolean or string label. Also, OpenAI also calls these Graders."

**Assistant** analyzed: Score has been stretched beyond numeric in the eval domain (Inspect AI's Scorer returns Score with string/number/boolean values). Grader has momentum from Anthropic + OpenAI but implies teacher-student authority. Scorer pairs cleanly with Score. Recommended staying with Scorer/Score.

**User:** "Let's stick with Scorer/Score."

---

## Phase 2, Round 1 — User Stories

**Assistant** presented 13 user stories (US-01 through US-13) with acceptance criteria covering all three directions: basic scoring (US-01), dataset runs (US-02), serialization/comparison (US-03), scorer composition (US-04), LLM-as-judge (US-05), transforms (US-06), OTel observe (US-07), custom extraction (US-08), correlation strategies (US-09), multi-trial (US-10), significance testing (US-11), historical traces (US-12), scorer failure distinction (US-13).

**User** proposed a major redesign of OTel observe mode: "Maybe something else calls the agent. The framework just grabs spans matching a correlation ID. The difference: the agent is not executed by the framework."

**Assistant** recognized this as a significant simplification: framework doesn't need to know how to call the agent, truly agent-agnostic, US-07 and US-12 collapse into the same operation. Rewrote US-07, simplified US-09, dropped US-12.

**User** further refined: "Not trace ID. Some sort of correlation ID. Maybe custom baggage info, like an execution/run_id combined with a sample ID."

**Assistant** updated: correlation is a domain-level concept (run_id + sample_id as OTel baggage), not an OTel-level concept (trace ID). Correlator trait removed — correlation is just a query contract on TraceBackend. Future OTLP receiver makes it even simpler.

**User:** "Sounds good."

---

## Phase 2, Round 2 — API Surface

**Assistant** presented full API: Sample<I, E>, Score enum, ScorerError, Scorer trait with score(input, output, expected), built-in scorers, composition operators, Acquisition trait, SpanExtractor, TraceBackend, Run builder, RunReport, serialization, comparison. Included hello world example.

**User:** "Does it accept multiple scorers?"

**Assistant:** Yes, `.scorer()` is repeatable. Results keyed by scorer name. Single run produces samples × scorers × trials matrix.

**User:** "It looks okay, but it's kinda your job to find ways to validate the API and make sure it's the best."

**Assistant** stress-tested against 5 real workflows from brainstorm validation plan:
1. Blueprint writer (text→text) — works
2. Excalidraw (text→JSON, needs transform) — **PROBLEM: transform is per-Run but different scorers need different views**. Fixed: moved transform to per-scorer `with_transform()` combinator.
3. GitHub issue agent (text→code) — works with per-scorer transform
4. Prompt tuning (text→text, multi-trial) — works
5. OTel observe — **QUESTION: how does sample match span attribute?** Fixed: added `id: String` to Sample.

**User:** "Yeah, they sound good. But let's test it with a few more common agentic use cases."

**Assistant** stress-tested 4 more agentic workflows:
6. RAG agent (answer + context) — verbose but correct with per-scorer transforms
7. Multi-turn conversation — works, generics handle complex types
8. Tool-calling agent (trajectory) — **PROBLEM: no way to transform the expected/reference value**. Fixed: added `with_expected()` combinator.
9. OTel with tool-calling — solved by rich extractor + with_transform pattern

**User** proposed 5 ideas that reshaped the API:
1. ScorerSet with shared transformer → **Accepted**
2. Rename `expected` → `reference` → **Accepted**
3. Transformer as a trait with closure blanket impl → **Accepted**
4. Remove SpanExtractor, Observe returns everything → **Accepted** (SpanExtractor eliminated)
5. Global transformer support → **Accepted** (two-level pipeline)

**User:** "Transform vs reference: they are actually the same thing. Yet called different things."

**Assistant** unified under `Mapper` trait with `.map_output()` and `.map_reference()` builder methods.

**User:** "Is Map the right name? Maybe brainstorm a few?"

**Assistant** brainstormed: Mapper, Transform, Converter, Selector, Lens, Extractor, Adapter, Projection, Into. Recommended `Mapper` — familiar from Iterator::map, unifies both use cases.

**User:** "ScorerInput: How about ScorerContext?"

**Assistant** brainstormed: ScorerInput, ScorerContext, ScorerArgs, Observation, Evidence, ScorerView, EvalContext. Recommended `ScorerContext` — signals extensibility, avoids ambiguity with sample input.

**User:** "Yeah, sounds good, but let's do one more stress test. Also, make sure all decisions are recorded."

**Assistant** ran stress test round 3 (tests 10-15):
10. Heterogeneous score types — **PROBLEM: aggregate stats don't work for all types**. Fixed: `ScorerStats` enum with per-variant stats.
11. Scorer composition with ScorerContext — works, ctx is shared reference
12. Partial failures in multi-trial — **Decision: errored trials excluded from stats denominator**
13. Sample ID auto-gen vs observe mode — **Fixed: observe validates explicit IDs at build time**
14. Comparison with different scorers — compare shared scorers, report unique
15. Builder ordering — type mismatch gives compile error, document ordering

Saved `spec/decisions-log.md`.

---

## Phase 2, Round 3 — Data Model

**Assistant** presented full data model: Sample, Dataset, Score (with serialization format), ScorerError, TrialResult, SampleReport, AggregateStats, RunMetadata, RunReport, Comparison, ScorerComparison, SampleComparison, ComparisonDirection, persistence strategy (JSONL).

**User** raised 7 questions:

1. **Multiple labels?** → Deferred. Score enum addable later.
2. **InvalidInput variant?** → Simplified ScorerError to single struct wrapping `Box<dyn Error>`.
3. **Raw results vs aggregation?** → Split: RunResult is raw data, stats computed separately via `.stats()`.
4. **Acquisition enum or trait?** → Trait with Fn blanket impl. Closures just work.
5. **run_a/run_b naming?** → Brainstormed. Chose baseline/candidate.
6. **ComparisonDirection?** → Renamed to `Change`. Added `Incomparable`.
7. **Persistence coupling?** → Decoupled from core. Serde derives + JSONL convenience.

**User:** "I'm not sure I understand: direction must be persisted in RunResult."

**Assistant** reconsidered: direction could be passed at comparison time instead of stored. But concluded direction genuinely belongs to the scorer's definition.

**User:** "Doesn't score suggest a numeric nature thing? What about pass/fail? Also, OpenAI uses Graders."

*(This was the terminology discussion from Round 0, referenced here for context.)*

**User:** "On Direction enum names Higher/LowerIsBetter: can we brainstorm?"

**Assistant** brainstormed: HigherIsBetter/LowerIsBetter, Maximize/Minimize, Ascending/Descending, Higher/Lower, Up/Down, More/Less, Positive/Negative. Chose **Maximize/Minimize** — concise, mathematical, standard in optimization.

**User:** "I agree direction belongs to the scorer. But what about neutral directions for binary/label?"

**Assistant** analyzed: Direction is meaningless for Binary (always want passes) and Label (no ordering). `Option<Direction>` with only Maximize/Minimize — `None` means direction doesn't apply. No Neutral variant.

**User:** "How about just `::new` for the constructor?"

**Assistant:** Simple and right. `ScoreDefinition::new("name")` for no direction, `::maximize()` and `::minimize()` for directed scores.

**User** raised measurement analogy: "There is a measurer, a method of measurement, and a measurement." This led to formalizing `ScoreDefinition` as a first-class type replacing separate `name()` and `direction()` methods on the Scorer trait.

---

## Phase 2, Round 4 — System Architecture

**Assistant** presented: component overview diagram, data flow (inline + OTel + multi-trial), boundaries (process, network, feature gates), concurrency model, extension points, architectural patterns (adopted and rejected with research references), failure archaeology cross-check, crate structure.

**User** suggested:
- No `mod.rs`, use file-based modules → **Accepted**
- Scorer composition: defer → **Accepted**
- Consolidate scoring types into `scoring.rs` → **Accepted**
- Move core to crate root → **Accepted**
- OTel under acquisition module → **Accepted**
- Stats and compare: keep separate → **Accepted**
- Concurrency: start sequential → **Accepted**

Revised crate structure to flat, idiomatic Rust layout.

---

## Phase 2, Round 5 — Integration Surface

**Assistant** presented 5 integration points: Jaeger v2 API, LLM Provider APIs, OTel-instrumented agents (indirect), serde ecosystem, tokio. Ecosystem alignment check against `06-ecosystem.md`.

**User:** "LLM judge: that's just an implementation. Later we can add a client. The rest sounds good."

---

## Phase 2, Round 6 — Error Handling

**Assistant** presented error categories: BuildError (6 variants), AcquisitionError (4 variants), ScorerError, RunError (AllSamplesFailed, ThresholdExceeded, Internal). Error propagation rules, interrupted operation handling.

**User:** "Would it make sense to call it RunBuildError?"

**Assistant:** Yes — more specific. BuildError too generic.

**Assistant** launched two subagents researching: (1) failure threshold patterns in batch processing, async panic handling in Rust; (2) error handling in DeepEval, Inspect AI, Promptfoo, agentevals-dev.

Key findings:
- Inspect AI has the best failure threshold model (`fail_on_error` accepts bool/f64/usize)
- Per-sample timeout is critical for sequential execution
- Score validation needed (NaN, empty labels)
- `tokio::spawn` + `JoinHandle` is the right panic pattern
- Every major framework distinguishes errors from low scores

Added: `FailureThreshold` enum, per-sample timeout, score validation, content hash fix for auto-IDs.

**User:** "I wonder if AllSamplesFailed is useful. You need to check for failed samples anyway."

**Assistant:** Agreed. Dropped `AllSamplesFailed`. Caller inspects RunResult.

**User:** "I wonder if we should delay ThresholdExceeded too. You get a run result anyway."

**Assistant:** Agreed. `execute()` always returns `Ok(RunResult)`. `RunError` simplified to `Build` + `Internal` only. FailureThreshold deferred.

---

## Phase 2, Round 7 — Constraints & Quality Attributes

**Assistant** presented measurable constraints: performance (scorer overhead < 1ms, serialization < 100ms), correctness (stats match R/SciPy to 1e-6, score validation, deterministic IDs), security (no API key leakage, no network without feature gates), compatibility (Rust 2024, tokio, Linux/macOS/Windows), resource limits (< 10MB overhead, < 30 crate deps), API stability (#[non_exhaustive], pre-1.0).

**User:** "Yep, sounds good."

---

## Phase 3 — Validate

**Assistant** performed coverage check: every goal → user story → API surface. Non-goal leak check: nothing out of scope is achievable. Failure archaeology cross-check: one acknowledged echo (metrics-only library pattern — accepted risk). Ecosystem alignment: no unintentional deviations. Growth path compatibility: no breaking changes required for any future capability.

**User:** "Everything looks good."

---

## Phase 4 — Write

**Assistant** wrote the full specification to `spec/eval-kernel.md` (14 sections) and updated `spec/decisions-log.md`.

---

## Final Review

**Assistant** launched code-reviewer agent to check internal consistency.

**Findings (HIGH):**
- Duplicate section 8.2 numbering → fixed
- Stale "Transform" terminology → fixed to "Mapper"
- `MapError`, `TraceBackendError`, `AcquisitionError`, `RunBuildError`, `RunError` used but never defined → added Section 5.4
- `.concurrency(N)` missing from Run builder → added
- `ThresholdExceeded` inconsistency between spec and decisions log → reconciled

**Findings (MEDIUM):**
- ScorerContext dependency graph wrong → fixed
- `Span` type undefined → added full definition
- `Dataset` type undefined → added
- `SampleBuilder` undefined → documented methods
- `AggregateStats` undefined → defined via `RunStats` in new Section 5.9
- `stats()` and comparison API unspecified → added Sections 5.9, 5.10, 5.11
- `#[non_exhaustive]` not shown on RunError/RunBuildError → now in code
- `error_count` documentation misleading → clarified
- `regex` scorer type difference → documented
- `LlmJudgeConfig` undefined → acknowledged as implementation detail
- `aggregate_delta` semantics for non-numeric → documented

**Findings (LOW):**
- US-08/US-12 gap in numbering → noted in spec
- `evalkit` crate name used before OQ-01 resolved → marked as TBD
- Missing glossary entries → added 6 entries
- OQ-05 / AC-11.5 contradiction → resolved
- `ScorerComparison.aggregate_delta` for Label scores → documented

All issues fixed. Spec is clean.

---

## Final Notes

**Assistant's closing guidance:**
- OQ-06 (scorer trait design spike) is the most important open question — implement variations before committing
- OQ-01 (crate name) doesn't block implementation
- Metrics-only library pattern is the one acknowledged failure archaeology echo — mitigated by building for one user
- Spec and decisions log saved to `spec/eval-kernel.md` and `spec/decisions-log.md`
