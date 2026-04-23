> **📦 Archived on 2026-04-23** — superseded by [Specification Decisions Log](../../../docs/spec/decisions-log.md). Kept for historical reference.

# Specification Decisions Log

## Session: 2026-04-03
## Scope: Unified spec — Eval Kernel + Trace Grader + Confident Eval

---

## Phase 1 — Anchor

- **Scope confirmed**: All three directions as a single Rust crate
- **Non-goals confirmed** with two additions: NOT a CLI tool, NOT a storage system
- **12 open questions carried forward**, none resolved by user prior to session
- **No new constraints** from user

---

## Round 0 — Terminology

### Decisions Made

| Concept | Decision | Rationale |
|---------|----------|-----------|
| Single test data point | `Sample` | Domain fractured. "Task" overloaded in Rust. "Sample" is neutral, used by RAGAS. |
| Collection of samples | `Dataset` | Settled domain term. |
| Expected/comparison data | `expected` → **later changed to `reference`** | See Round 2 evolution below. |
| Component that scores | `Scorer` | Most neutral. Pairs with Score. Recommended by cross-reference analysis. |
| Output of a scorer | `Score` | Universal pairing with Scorer. |
| Continuous score variant | `Numeric` | More self-explanatory than "Continuous" for developers. |
| Binary score variant | `Binary` | Generic, fits domain-agnostic core. |
| Categorical score variant | `Label` | 🆕 NOVEL. Self-explanatory. |
| Named measurement variant | `Metric` | Aligned with domain. ⚠️ Must document: Metric is a Score variant, not a Scorer. |
| Result of scoring one sample | `SampleResult` → **later `SampleReport`** | Avoids collision with `std::result::Result`. |
| Complete evaluation execution | `Run` | Most common informal term. |
| Aggregate output of a Run | `RunReport` | 🆕 NOVEL. Signals summary artifact. |
| One execution attempt | `Trial` | Anthropic + Agentrial convention. |
| How outputs are obtained | `Acquisition` | 🆕 NOVEL. No domain equivalent. |
| Inline execution mode | `inline` | 🆕 NOVEL. Descriptive. |
| OTel observation mode | `observe` | 🆕 NOVEL. Clear contrast with inline. |
| OTel execution record | `Trace` | OTel standard. |
| Individual operation in trace | `Span` | OTel standard. |
| Agent step sequence (eval concept) | `Trajectory` | Domain convention per 02-domain-language.md. Distinct from Trace. |
| Multi-trial execution | `trials` (API: `.trials(N)`) | Anthropic convention. |
| At least one pass in k | `pass_at_k` | Settled domain term (HumanEval paper). |
| All k pass | `pass_all_k` | Anthropic convention. More readable than pass^k. |
| Verb: produce a score | `score` | Consistent with Scorer. |
| Verb: run evaluation | `execute` (method) | Avoids `run.run()`. |
| Composing scorers | `composition` (.and, .weighted, .then) | 🆕 NOVEL. No other tool has this. |

### User Discussion: Score vs Grade, Scorer vs Evaluator vs Grader

- User questioned whether "Score" implies numeric nature, since it can be boolean or label
- Reviewed domain usage: Inspect AI Scorer returns Score with string/number/boolean values — domain has already stretched the term beyond numeric
- User noted OpenAI also uses "Grader" (Anthropic convention)
- Discussed tradeoffs: Scorer/Score pairs cleanly, Grader/Grade has provider momentum, Evaluator creates vocabulary mismatch (evaluator.evaluate() → Score?)
- **Decision: Stay with Scorer/Score.** Clean pairing, neutral for generic core.

---

## Round 1 — User Stories

### Stories Defined (US-01 through US-13)

- US-01: Score text output against expected value (built-in scorers)
- US-02: Run evaluation across a dataset
- US-03: Serialize and compare results (CI/CD)
- US-04: Compose scorers declaratively
- US-05: LLM-as-judge scorer
- US-06: Transform output before scoring → **later redesigned, see Round 2**
- US-07: Evaluate from OTel traces → **significantly redesigned**
- US-08: Custom span extraction → **later removed, see Round 2**
- US-09: Different correlation strategies → **redesigned as different trace sources**
- US-10: Multi-trial with statistical aggregation
- US-11: Compare runs with significance testing
- US-12: Evaluate historical traces → **merged into US-07**
- US-13: Scorer failure is not a low score

### OTel Observe Mode Redesign (user-initiated)

**Original design (US-07):** Framework sends HTTP request to agent, injects traceparent, collects traces, extracts, scores.

**User's insight:** "Maybe something else calls the agent. The framework just grabs spans matching a correlation ID."

**Rationale:**
1. Huge simplification — framework doesn't need to know how to call the agent
2. Truly agent-agnostic — any agent with OTel works
3. US-07 and US-12 collapse into the same operation
4. Fits the "grading layer is shared" architecture

**New design:** Framework accepts a correlation ID (e.g., run_id + sample_id as OTel baggage/attributes), queries trace backend, groups spans by sample attribute, extracts, scores.

**Impact:**
- US-07 rewritten — no HTTP request sending
- US-09 simplified — Correlator trait removed, correlation is a query contract on TraceBackend
- US-12 dropped — absorbed into US-07 (old traces = same operation as new traces)
- SpanExtractor initially kept, later removed (see Round 2)

**User's further refinement:** Correlation should be a domain-level concept (run_id + sample_id as OTel baggage), not an OTel-level concept (trace ID). Agent might span multiple traces. Future OTLP receiver mode makes this even simpler (filter received data).

---

## Round 2 — API Surface

### Initial API Design

- Sample<I, E> with input, expected, metadata
- Scorer<I, O, E> trait with score(input, output, expected) method
- Acquisition<I, O> trait (inline vs observe)
- SpanExtractor<O> trait
- TraceBackend trait
- Run builder with .dataset(), .acquisition(), .scorer(), .transform(), .trials()
- RunReport with SampleResult, TrialResult, aggregate stats

### Stress Test Round 1 (4 workflows from validation plan + OTel)

**Issues found:**

1. **Transform is per-Run, but different scorers need different views.**
   - Excalidraw: json_schema needs raw JSON, visual_similarity needs PNG
   - Resolution: Moved transform from Run builder to per-scorer `with_transform()` combinator

2. **Sample needs a first-class `id` field.**
   - Required for observe-mode correlation, result keying, cross-run comparison
   - Added `id: String` to Sample

### Stress Test Round 2 (5 more agentic workflows)

**Issues found:**

3. **No way to transform the reference/expected value.**
   - Tool-calling agent: expected is ExpectedBehavior struct, but exact_match needs String
   - Resolution: Added `with_expected()` combinator alongside `with_transform()`

4. **OTel multi-extraction solved by rich extractor + with_transform** — consistent pattern, no special API needed.

### User-Initiated API Redesign

User proposed 5 ideas that reshaped the API:

**Idea 1: ScorerSet with shared transform**
- Group scorers that share a transform; transform runs once
- **Accepted.** Reduces redundancy, logical grouping.

**Idea 2: Rename `expected` → `reference`**
- "Reference" is more domain-appropriate (RAGAS uses it), neutral about correctness
- Rust `&T` collision is a non-issue for field names
- **Accepted.**

**Idea 3: Transform as a trait with closure blanket impl**
- Standard Rust pattern. Closures for simple cases, trait impl for complex.
- **Accepted.**

**Idea 4: Remove SpanExtractor, Observe returns all spans**
- SpanExtractor is redundant — it's just a transform on Vec<Span>
- Returning all spans isn't wasteful (10-100 spans typical)
- Extraction becomes a transform, unifying concepts
- **Accepted.** SpanExtractor removed.

**Idea 5: Global transform support on Run builder**
- Two-level pipeline: global (acquisition output → domain type) + per-set (domain type → specific field)
- Essential for OTel: global converts Vec<Span> → AgentOutput, per-set extracts fields
- **Accepted.**

### Naming Discussions

**Transform vs Transformer vs Mapper:**
- User asked to brainstorm alternatives
- Considered: Transform, Transformer, Converter, Selector, Lens, Extractor, Adapter, Projection
- **Decision: `Mapper`** — unifies output transform and reference map under one concept. Familiar from Iterator::map. Builder methods: `.map_output()`, `.map_reference()`.

**ScorerInput vs ScorerContext:**
- User suggested ScorerContext
- Considered: ScorerInput (ambiguous with sample input), ScorerContext (signals extensibility), ScorerArgs (too implementation-focused), Observation (collides with OTel), Evidence (too metaphorical)
- **Decision: `ScorerContext`** with `#[non_exhaustive]` for future-proof extensibility.

### Stress Test Round 3

**Issues found:**

5. **Aggregate stats don't work for all score types.**
   - Mean is meaningless for Label scores. pass_at_k only for Binary.
   - Resolution: `ScorerStats` enum with per-variant appropriate stats (Numeric, Binary, Label, Metric, Errored)

6. **Errored trials in stats.**
   - Should errors be excluded from denominator or counted as failures?
   - Decision: Excluded. Error ≠ failure. Report `scored_count` + `error_count` separately.

7. **Sample ID auto-generation vs observe mode.**
   - Auto IDs don't match span attributes.
   - Decision: Observe mode validates explicit IDs at build time. Inline auto-generates if not provided.

8. **Comparison across runs with different scorers.**
   - Only compare shared scorers. Report unique scorers per run.

9. **Builder ordering (mappers before scorers).**
   - Type mismatch gives compile error. Document ordering. Defer typestate phases if needed.

### Concepts Removed During Round 2

| Concept | Why Removed |
|---------|-------------|
| SpanExtractor trait | Redundant — Mapper<Vec<Span>, O> covers this |
| Correlator trait | Absorbed into TraceBackend query contract |
| `with_transform()` combinator | Replaced by ScorerSet + `.map_output()` |
| `with_expected()` combinator | Replaced by ScorerSet + `.map_reference()` |
| US-08 (Custom span extraction) | Absorbed — custom extraction is a Mapper impl |
| US-12 (Historical trace eval) | Merged into US-07 — same operation |

### Final API Shape (post Round 2)

```
Mapper trait (unified: output mapping + reference mapping)
  ↓
ScorerContext struct (#[non_exhaustive], future-proof)
  ↓
Scorer trait (receives ScorerContext)
  ↓
ScorerSet (groups scorers + optional map_output + optional map_reference)
  ↓
Acquisition trait (Inline | Observe)
  ↓
Run builder:
  .dataset() → .acquisition() → .map_output() → .map_reference()
  → .scorer() / .scorer_set() → .trials() → .concurrency() → .build() → .execute()
  ↓
RunReport → SampleReport → TrialResult → ScorerStats (enum per score type)
```

### Term Sheet Updates

| Original Term | Current Term | When Changed | Why |
|---------------|-------------|-------------|-----|
| `expected` (field) | `reference` | Round 2, user suggestion | More domain-appropriate, neutral about correctness |
| `E` (type param) | `R` | Round 2 | Follows from expected→reference |
| `Transform` (trait) | `Mapper` | Round 2, naming discussion | Unifies output and reference mapping |
| `.transform()` | `.map_output()` | Round 2 | Parallel with .map_reference() |
| `.reference_map()` | `.map_reference()` | Round 2 | Parallel with .map_output() |
| `TransformError` | `MapError` | Round 2 | Follows from Mapper |
| `ScorerInput` | `ScorerContext` | Round 2, user suggestion | Signals extensibility, avoids ambiguity with sample input |
| `SampleResult` | `SampleReport` | Round 2 | Consistent with RunReport |
| `SpanExtractor` | (removed) | Round 2, user insight | Redundant — Mapper covers this |
| `Correlator` | (removed) | Round 1, OTel redesign | Absorbed into TraceBackend |

---

## Round 3 — Data Model

### Decisions Made

**Score::Labels (multi-label) — Deferred.**
- Use case: classify output as both "helpful" AND "verbose" AND "safe"
- Workaround: multiple Binary scorers, one per label dimension
- Score is an enum — adding a Labels variant later is non-breaking (minor version bump)
- Deferred because aggregation for label sets is an unsolved design question

**ScorerError simplified to single struct.**
- Original: enum with `Execution` and `InvalidInput` variants
- Problem: `InvalidInput` is redundant — wrong types are compile errors in Rust generics, bad deserialization is an Execution error
- New: `pub struct ScorerError(pub Box<dyn std::error::Error + Send + Sync>)`
- Inner error can be downcast later if programmatic classification needed

**RunReport → RunResult, aggregation decoupled.**
- User insight: "sample report should return raw results only, aggregation should be a different step"
- Rationale: decouples "what happened" from "what it means", enables custom aggregation
- RunResult contains raw trials only. Stats computed via `.stats()` convenience or custom functions.
- SampleReport → SampleResult (follows from rename)

**Acquisition: trait with shorthand.**
- Acquisition is a trait (not enum) — users can implement custom acquisition
- `.acquire(closure)` shorthand added for implicit Inline (most common case)
- `.acquisition(impl Acquisition)` for custom implementations

**Comparison naming: baseline/candidate.**
- run_a/run_b → baseline/candidate
- Clearer semantics: baseline = known-good, candidate = under test

**ComparisonDirection → Change with Incomparable.**
- Renamed to `Change` (cleaner)
- Added `Incomparable` variant for safety (different score types, missing samples, insufficient data)

**Persistence: decoupled from core, convenience provided.**
- Core library: `Run::execute()` → `RunResult`. Core API unaffected by persistence.
- All public types: `serde::Serialize + Deserialize`.
- Convenience JSONL reader/writer provided (no external deps beyond serde) — like stats, a good default near the core.
- No database drivers. Persistence strategy is the caller's choice.

**Score direction: declared by Scorer via ScoreDefinition, persisted in RunResult.**
- Scorer trait has `fn definition(&self) -> ScoreDefinition` (replaces separate `name()` + `direction()`)
- `ScoreDefinition { name: String, direction: Option<Direction> }`
- `Direction` enum: `Maximize`, `Minimize` (only two variants — no Neutral)
- `direction: None` for Binary/Label scorers (direction concept doesn't apply to classifications)
- Constructors: `ScoreDefinition::new("name")` (no direction), `::maximize("name")`, `::minimize("name")`
- Direction stored in `RunMetadata` per scorer
- At comparison time: if baseline and candidate disagree on direction for same scorer → `Change::Incomparable`
- Direction only matters for comparison interpretation, not aggregation

**Sample ID: deterministic by default.**
- Auto-generated IDs are content-hashed (hash of input + reference) → same content = same ID across runs
- Prevents footgun: code-defined samples without explicit IDs still compare correctly across runs
- User can override with `.id("custom")` via builder
- Types that don't implement Hash require explicit ID via builder

**Acquisition: blanket impl for closures.**
- `impl Acquisition<I, O> for F where F: Fn(&I) -> Fut` — closures are Acquisitions directly
- No `Inline` wrapper needed for the common case
- `.acquisition(|input| async { ... })` just works
- Custom implementations use the trait directly

**Comparison naming: baseline / candidate.**
- Confirmed against benchmark tools: criterion uses "baseline", benchstat uses "base", hyperfine uses "reference"
- "candidate" for the new/under-test side — standard in A/B testing and release engineering
- Most benchmark tools don't name the second side; "candidate" is more explicit

**Change enum (was ComparisonDirection):**
- Renamed to `Change`
- Added `Incomparable` variant (different score types, direction mismatch, missing data)

**Score::Labels (multi-label): Deferred.**
- Workaround: multiple Binary scorers
- Adding enum variant later is non-breaking

---

## Round 4 — System Architecture

### Decisions Made

**Crate structure: flat, idiomatic Rust.**
- No `mod.rs` — file-based modules (modern Rust convention)
- No `core/` submodule — types live at crate root (the crate IS the core)
- One file per cohesive concept, types in logical reading order within each file
- `scoring.rs` is the central file: Scorer trait, Score, ScorerContext, ScoreDefinition, Direction, ScorerError, ScorerSet
- Subdirectories only for feature-gated boundaries: `acquisition/observe.rs`, `scorers/llm_judge.rs`

**Scorer composition: Deferred.**
- `.and()`, `.weighted()`, `.then()` operators deferred to post-MVP
- Pure library code, can be added without changing other components
- No impact on trait design — composition wraps scorers, doesn't change them

**Concurrency: Start sequential.**
- MVP: samples, trials, scorers all processed sequentially
- Simple, debuggable, predictable
- `.concurrency(N)` stays in the Run builder API (already designed), defaults to 1
- Concurrent executor added later — RunResult identical regardless of execution mode
- Non-breaking addition: implementation detail behind the same API

**OTel under acquisition module.**
- Observe is an acquisition strategy, not a separate system
- `acquisition/observe.rs` alongside the Acquisition trait
- TraceBackend, JaegerBackend, Span types all in the acquisition boundary

**Stats and compare: separate files.**
- Stats = summarizing one run. Compare = relationship between two runs.
- Related but different concerns. Both at crate root level.

### Architectural Patterns

**Adopted from research:**
- Library/Framework pattern (04-architecture.md Pattern 1) — composable library, not platform
- OTel-Native pattern (Pattern 4) — observe mode via trace backend queries
- Layered evaluation (Anthropic guide) — cheap scorers first, expensive scorers via composition (deferred)

**Explicitly rejected:**
- Platform/SaaS (Pattern 2) — non-goal
- Observability-first (Pattern 3) — we're eval-first, OTel is acquisition mode
- Evaluator-as-Agent (Pattern 5) — too expensive, non-deterministic
- Declarative/Config-driven (Pattern 6) — no CLI, library-first
- Research/Sandbox (Pattern 7) — no sandboxing, caller's responsibility

**Failure archaeology awareness:**
- Metrics-only library risk (Approach 1) acknowledged — building for one user first, not market
- Log10 differentiation failure avoided — four axes of differentiation (Rust, generic core, OTel observe, statistical rigor)
- Single-provider coupling avoided — provider-agnostic by design

---

## Round 5 — Integration Surface

No significant decisions. Integrations confirmed:
- Jaeger v2 API (otel feature) — HTTP/JSON, configurable timeout/retry
- LLM Provider APIs (llm-judge feature) — implementation detail, no client bundled initially
- OTel-instrumented agents (indirect) — convention-based baggage attributes
- serde ecosystem — Serialize/Deserialize on all public types
- tokio async runtime — async execution foundation

---

## Round 6 — Error Handling & Edge Cases

### Research-Informed Decisions

**Subagent research covered:** DeepEval, Inspect AI, Promptfoo, agentevals-dev error handling patterns; batch processing failure thresholds (Spark, pytest, cargo test); async panic handling in tokio.

**FailureThreshold: researched Inspect AI pattern, deferred.**
- Inspect AI has the best model: `fail_on_error` accepts `bool | f64 | usize`
- Deferred for now: `execute()` always returns `Ok(RunResult)`. Errors are in the result, not the return type.
- `RunError` simplified to `Build(RunBuildError)` + `Internal(...)` only.
- `Err` = couldn't run at all. `Ok` = ran, check the results. Clean separation.
- `AllSamplesFailed` also dropped — caller inspects RunResult, no special error variant.
- FailureThreshold + ThresholdExceeded can be added later (non-breaking) when large datasets make early abort valuable.

**Per-sample timeout added.**
- `.sample_timeout(Duration)` on Run builder
- Essential for sequential execution — prevents one hanging sample from blocking the entire run
- Implementation: `tokio::time::timeout()` wrapping each sample execution
- Timeout → `AcquisitionError::Timeout`

**Score validation after every scorer call.**
- `Score::Numeric(v)`: v must be finite (not NaN, not Infinity)
- `Score::Label(s)`: s must be non-empty
- `Score::Metric { name, value, .. }`: name non-empty, value finite
- Invalid scores become ScorerError — scorer has a bug, not a finding

**BuildError → RunBuildError.**
- User suggestion: more specific about what's being built
- `BuildError` too generic in a crate with multiple builders

**Partial results on threshold abort — DEFERRED with ThresholdExceeded.**
- Original design: no partial results on any failure
- Intermediate design: `RunError::ThresholdExceeded` includes partial `RunResult`
- Final decision: ThresholdExceeded deferred along with FailureThreshold. `execute()` always returns `Ok(RunResult)`.
- Ctrl+C / SIGKILL: no partial results in MVP

**Idempotent scorers: documentation requirement.**
- Scorers should be idempotent (safe to retry, safe for multi-trial)
- Documented, not enforced by the framework

**Content hash for auto-ID must include both input AND reference.**
- Prevents collision when two samples have same input but different references

**Panic handling: tokio::spawn pattern.**
- `tokio::spawn` + `JoinHandle` catches panics automatically
- Panics converted to `AcquisitionError` or `ScorerError`
- Combined with `tokio::time::timeout` for both isolation and timeout enforcement
