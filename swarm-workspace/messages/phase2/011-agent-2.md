---
agent: agent-2
role: ""
phase: phase2
round: 1
seq: 011
timestamp: 2026-04-06T06:18:05Z
---

Now I have a comprehensive view of both libraries. Let me produce my structured analysis.

---

# Agent 2 — Verda Deep Analysis (Round 1)

## Structured Evaluation of verda

### 1. Architecture & Type Design

**The 5-parameter generic `Evaluation<I, T, O, D, F>`:**

This is the most distinctive and most controversial design choice. Let me break down whether each parameter is justified:

| Param | Purpose | Justified? |
|-------|---------|------------|
| `I` (Input) | Task input type | ✅ Essential |
| `T` (Target/Reference) | Ground truth type | ✅ Necessary — reference can differ from input/output type |
| `O` (Output) | Task output | ✅ Essential |
| `D` (Dataset) | Dataset impl | ⚠️ **Questionable** — could be erased via `Box<dyn Dataset<I,T>>` |
| `F` (Function) | Task closure | ⚠️ **Questionable** — same as D; exists for monomorphization performance |

**My assessment**: `D` and `F` as generic parameters are a premature optimization. They infect the type signature and force users to write `Evaluation<String, String, String, Vec<Sample<String, String>>, impl Fn(...)>` anywhere they need to name the type (e.g., storing in a struct, passing between functions). In practice, the evaluation is constructed, run once, and discarded — monomorphization gains are negligible versus the ergonomic cost.

**However**: I'll note that verda somewhat mitigates this because `Evaluation::new()` infers all 5 parameters at the call site. The pain only hits when you try to abstract *over* evaluations. For the "run it once in a test" case, this is invisible.

**What would change my mind**: If profiling showed that `dyn Dataset` dynamic dispatch was a measurable overhead in a real eval workload. I strongly doubt it — the task execution (usually an LLM call) dominates by orders of magnitude.

### 2. API Ergonomics

**Strengths:**
- Builder pattern with `with_*` methods is idiomatic Rust
- `boxed_scorer()` helper reduces boilerplate
- `score()` and `scores()` convenience constructors are nice
- `From<O> for TaskOutput<O>` means simple tasks don't need to know about `TaskOutput`
- `Vec<Sample<I,T>>` implements `Dataset` — zero-boilerplate for the common case
- `RunConfig::new("name")` defaults are sensible (parallelism = available CPUs, trial_count = 1)

**Weaknesses:**
- Scorer requires `#[async_trait]` annotation — this is crate-level API leakage. Users must depend on `async_trait` crate themselves. **evalkit avoids this** with native async traits (Rust edition 2024)
- Scorer returns `Vec<Score>` — a single scorer can emit multiple named scores. This is powerful but subtle. The relationship between `Scorer::name()` (used for error reporting) and the `Score::name()` values it emits is indirect and confusing
- No built-in scorers at all. Every user writes their own from scratch. **evalkit provides `exact_match`, `contains`, `regex`, `json_schema`, `llm_judge`**

### 3. Score Model: Numeric-Only vs Typed Variants

**Critical difference from evalkit**: verda's `Score` is always `f64`. evalkit has `Score::Numeric(f64)`, `Score::Binary(bool)`, `Score::Label(String)`, `Score::Metric{...}`.

**My take**: verda's numeric-only approach is **more principled for a low-level library**. Binary scores are just `{0.0, 1.0}` with `Goal::Maximize`. Labels are a presentation concern. Mixing score types in an enum creates downstream complexity in aggregation and comparison (evalkit's `stats.rs` at 613 lines vs verda's inline aggregation). The simplicity of "everything is a number with a direction" is a genuine strength.

**However**, verda separates `Score` (scorer output) from `Metric` (task-attached performance data like latency). This separation is clean and well-motivated — scores measure quality, metrics measure performance. **evalkit conflates these** by putting `Metric` as a `Score` variant.

### 4. Comparison Engine

verda's comparison engine is solid but basic:
- Matches samples by normalized ID
- Classifies per-sample changes as `Improved | Regressed | Unchanged | Incomparable`
- Aggregates improvements/regressions counts and mean diffs
- Validates goal/unit consistency across runs

**What's missing vs evalkit**: No statistical significance testing. verda reports raw counts and diffs. evalkit computes p-values, confidence intervals, t-tests, Wilson intervals, and Fisher's exact tests. For a production eval workflow, significance testing is **not optional** — you need to know if a 0.02 improvement on 50 samples is signal or noise.

**What's better in verda**: The `Change` enum with `Incomparable` variant is cleaner than evalkit's 5-variant `Change` which adds `Insignificant` — significance is a separate axis from direction and should arguably be represented separately (a change can be both `Improved` and `Insignificant`).

### 5. Error Handling

**14 `RunError` variants** — is this justified?

After reading them, **yes, mostly**. Each variant represents a distinct validation failure with specific diagnostic fields (indices, names, values). This is significantly more helpful for debugging than a generic "validation failed" error. The variants decompose into:

- **Configuration errors** (3): MissingRunName, InvalidParallelism, InvalidTrialCount
- **Dataset integrity** (2): NoSamples, DuplicateSampleId
- **Cross-trial consistency** (1): InconsistentSampleId
- **Score integrity** (4): DuplicateScoreName, MissingScoreName, InvalidScoreValue, ConflictingScoreGoal
- **Metric integrity** (3): ConflictingMetricGoal, ConflictingMetricUnit, InvalidMetricValue
- **Serialization** (1): SampleResultEncoding

**The non-fatal error model is a genuine strength**: Task failures and scorer failures are recorded in `RunResult::errors` and per-sample `scorer_errors`, but the run continues. This is essential for real-world evals where some inputs may cause LLM errors. evalkit does the same, so this is parity.

**A valid critique**: The separate `EvaluationError` (construction-time) vs `RunError` (execution-time) split is good design, but having `RunError` cover both validation *and* data integrity is mixing concerns. Configuration validation could happen at build time like evalkit's `RunBuildError`.

### 6. Tracing Support

**This is verda's weakest point**. There is exactly ONE `tracing::warn!` call in the entire codebase, logging scorer failures. No span instrumentation of:
- Individual sample/trial execution
- Task invocation
- Score aggregation
- Comparison operations

The task statement says "first class tracing support is vital." Verda does not have it. evalkit has a feature-gated `otel` module (533 lines) with full span collection, Jaeger integration, correlation IDs, and an `Observe` acquisition mode that can score traces rather than direct outputs. This is a major gap.

**What verda *could* do easily**: The `run_work_item` function is the natural place to emit `tracing::instrument` spans. The architecture doesn't prevent it — it's just not done.

### 7. Persistence & Serialization

verda uses JSON (`serde_json`) with a typed→JSON→typed roundtrip via `SampleResult::decode::<I,T,O>()`. evalkit uses JSONL with a dedicated `jsonl.rs` module.

**verda's approach is cleaner in one respect**: The dual-layer (typed for execution, JSON for storage) is well-separated. `SampleResult` stores `serde_json::Value` internally, and `TypedSampleResult<I,T,O>` is materialized on demand. This avoids forcing all consumers to know the concrete types.

**But**: verda provides no file I/O. No `write_to_file()`, no JSONL support. Users must call `serde_json::to_string()` themselves. evalkit at least provides `write_jsonl`/`read_jsonl`.

### 8. Concurrency

verda has **built-in concurrent execution** via `buffer_unordered(parallelism)` using `futures-util`. This is production-ready. evalkit's execution is sequential (concurrency deferred per spec).

This is a significant verda advantage for real workloads where running 100 LLM calls sequentially is unacceptable.

### 9. Domain-Agnosticism

Both libraries are domain-agnostic in their core types. Neither has AI-specific terminology in the type system. verda's terminology (Sample, Scorer, Score, Evaluation, Dataset) is clean and general. The `tags` and `metadata` fields on `Sample` support arbitrary domain annotations.

### 10. What verda Does Better Than evalkit (Preserve List)

1. **Concurrent execution** — non-negotiable for production use
2. **Score/Metric separation** — cleaner than conflating them in a Score enum
3. **Numeric-only scores** — simpler aggregation, less special-casing
4. **Dataset trait with trial-aware materialization** — `sample(index, trial)` enables deterministic per-trial variation without external state
5. **Comparison engine** — basic but functional and serializable; evalkit's is more powerful but verda's `Change` enum is compositionally cleaner
6. **`pub(crate)` field encapsulation** with accessor methods — better API hygiene than public fields

### 11. What evalkit Does Better (Gaps in verda)

1. **Tracing/observability** — evalkit's OTel module is a major feature verda lacks entirely
2. **Statistical rigor** — CI, p-values, pass_at_k, significance classification
3. **Built-in scorers** — exact_match, contains, regex, json_schema, llm_judge
4. **Acquisition abstraction** — verda's task is just a closure; evalkit's `Acquisition` trait with blanket impls and observe-mode is more flexible
5. **Mapper abstraction** — type-safe output/reference transformations
6. **JSONL I/O** — built-in serialization format
7. **Spec-driven design** — evalkit has a formal spec with user stories, acceptance criteria, and documented deviations; verda has none
8. **Native async traits** — no `#[async_trait]` dependency (Rust 2024 edition)

### Critical Honest Assessment

Verda is a well-implemented, pragmatic library with clean Rust idioms and solid error handling. Its ad-hoc design process actually produced a reasonable API for the core evaluation loop. But it's missing two things the task statement calls vital: serious tracing support and statistical analysis. These are not small gaps.

The 5-parameter generic is defensible for the "write it inline in a test" workflow but would be painful for any framework that needs to abstract over evaluations. The `async_trait` dependency is a wart that will age poorly as Rust 2024 edition stabilizes.

**What evidence would change my mind**: If someone showed me a verda use case where the monomorphized `Evaluation<I,T,O,D,F>` meaningfully outperformed a `dyn`-erased version, I'd reconsider. And if the tracing gap is viewed as "add 50 lines of `#[instrument]` annotations" rather than a design issue, the gap closes fast.

---

I defer to Agent 0 on whether evalkit's spec conformance is genuine or superficial, to Agent 1 on whether the external landscape validates these design priorities, and to Agent 3 on the final library recommendation.

**CONVERGENCE**: Not yet. I've laid out verda's strengths and gaps. The key open questions are: (1) Does evalkit's spec-driven design actually produce better outcomes, or is it process theater? (2) Can verda's concurrency model be ported to evalkit easily? (3) Is the "continue one, migrate from the other" framing even right, or should key modules be cherry-picked?
