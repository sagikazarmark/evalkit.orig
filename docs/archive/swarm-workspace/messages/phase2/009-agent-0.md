> **📦 Archived on 2026-04-23** — superseded by [Evalkit vs. Verda: Final Evaluation Report](../../output.md). Kept for historical reference.

---
agent: agent-0
role: ""
phase: phase2
round: 1
seq: 009
timestamp: 2026-04-06T06:10:50Z
---

Now I have a thorough understanding. Here is my independent analysis:

---

# Agent 0 — Evalkit Deep Analysis (Round 1, Blind)

## 1. Architecture & Spec Fidelity Summary

Evalkit is a ~4,200-line Rust library built through 21 iterations of spec-driven implementation, reaching ~95% conformance against a 1,150-line specification. It implements a **typestate builder pattern** for run construction, a **generic trait triplet** (Scorer, Acquisition, Mapper), statistical aggregation with proper significance testing, and OTel-based trace observation.

**The documented deviations are 5, not 4** — DEV-01 through DEV-05 as listed in the conformance report. Additionally, the missing `prelude` module (D-01 in conformance) and missing Serialize/Deserialize on Span types (D-02) are noted. This is a minor discrepancy worth flagging for accuracy.

## 2. Domain-Agnosticism: Where the Abstraction Leaks

This is my central challenge. Evalkit **claims** domain-agnosticism but I identify several places where AI assumptions seep through:

### 2.1 AcquisitionError Variants

```rust
pub enum AcquisitionError {
    ExecutionFailed(Box<dyn Error + Send + Sync>),
    TraceNotFound { correlation_id: String, sample_id: String },
    BackendUnavailable(Box<dyn Error + Send + Sync>),
    Timeout(Duration),
}
```

`TraceNotFound` and `BackendUnavailable` are **OTel-specific concerns** baked into the core error type. A truly domain-agnostic acquisition error should only need `ExecutionFailed` and `Timeout`. The trace-specific variants belong behind the `otel` feature gate, not in the base enum. This is a concrete abstraction leak — if I'm using evalkit to evaluate, say, a compiler optimization pipeline, I'm carrying dead error variants.

**Counterargument**: `#[non_exhaustive]` means users already must handle unknown variants, so extra variants don't break compatibility. But they pollute the mental model.

### 2.2 Built-in Scorers Are String-Only

```rust
pub fn exact_match() -> impl Scorer<String, String, String>
pub fn contains() -> impl Scorer<String, String, String>
pub fn regex(pattern: &str) -> impl Scorer<String, String>
```

Every built-in scorer is `Scorer<String, String, ...>`. This is fine as a convenience layer, but it subtly signals that the library's "natural habitat" is text-in/text-out evaluation. For a domain-agnostic library, I'd expect at least one generic built-in (e.g., `exact_match<T: PartialEq>()` returning `Scorer<T, T, T>`).

**Impact**: Low — these are convenience functions, not core abstractions. But they shape first impressions.

### 2.3 Score::Metric Variant

The `Score::Metric { name, value, unit }` variant overlaps with `Score::Numeric(f64)` in a way that suggests it was designed for AI latency/token-count use cases. In a generic evaluation framework, a metric IS a score — the distinction between "the thing I'm evaluating" and "a side-channel measurement" is domain-specific.

**What would change my mind**: If there's a compelling use case where Metric and Numeric need fundamentally different statistical treatment. Currently, stats.rs treats them identically (mean/stddev/CI).

### 2.4 LLM Judge as a Feature

The `llm-judge` feature gate is well-isolated, but its mere presence in the core library (rather than a separate crate) signals an AI-first identity. This is a packaging choice, not an abstraction leak, but worth noting.

## 3. Abstraction Quality: What's Genuinely Good

### 3.1 Scorer<I, O, R = ()> — Clean Design

The Scorer trait is genuinely well-designed:
- Returns `Result<Score, ScorerError>` — infrastructure failures are distinct from low scores. This is **critical** and many eval frameworks get it wrong.
- Default `R = ()` makes reference-free scoring ergonomic without losing generality.
- `definition()` provides metadata (name + direction) separate from scoring logic — enables the stats/comparison layer to work without calling scorers.

### 3.2 Mapper Trait — Elegant Simplification

The `Mapper<I, O>` trait with closure blanket impl is clean and solves a real problem: transforming acquisition outputs before scoring without running the transformation N times per scorer. The ScorerSet ensures mappers run once per trial.

### 3.3 Typestate Builder — Compile-Time Safety

The Run builder uses typestate (Unmapped/Mapped phantom types) to prevent double-mapping and ensure scorers match mapped types. This is genuinely excellent Rust — it catches configuration errors at compile time. However, it produces **4 separate RunExecutor implementations** (Raw, OutputMapped, ReferenceMapped, FullyMapped), which is a maintainability concern.

### 3.4 Statistical Layer — Rigorous

Wilson score intervals for binary, Welch's t-test for numeric, Fisher's exact for comparison. This is **significantly above average** for eval libraries, most of which use naive mean/stddev.

## 4. What's Missing or Problematic

### 4.1 No Concurrent Execution (MVP Decision, but Consequential)

The `.concurrency(N)` API exists but does nothing. Samples execute sequentially. For production evals with expensive acquisition (LLM calls, agent runs), this is a real limitation. The API promises something it doesn't deliver.

**Mitigation**: The spec explicitly defers this, and the API is designed for future addition. But a user who writes `.concurrency(8)` today gets silently ignored, which is worse than not offering the API.

### 4.2 Dataset is a Concrete Type, Not a Trait

```rust
pub struct Dataset<I, R = ()> {
    pub samples: Vec<Sample<I, R>>,
    pub metadata: HashMap<String, serde_json::Value>,
}
```

Evalkit's Dataset is a simple `Vec<Sample>` wrapper. Compare with verda's `trait Dataset<I, T>` which allows lazy/index-based materialization with per-trial variation. Verda's approach is strictly more flexible — it enables:
- Streaming/lazy datasets
- Deterministic per-trial variation (e.g., different seeds)
- Custom loading strategies

Evalkit requires all samples materialized in memory upfront. For large evaluation suites, this is limiting.

### 4.3 Scorer Returns Single Score, Not Vec<Score>

Evalkit's `Scorer::score()` returns `Result<Score, ScorerError>` — a single score. Verda's returns `Result<Vec<Score>, ScorerError>` — multiple scores per scorer invocation.

**Consequence**: In evalkit, if you want to emit multiple scores from one scorer, you must use ScorerSet (which adds a mapper layer). In verda, a single scorer can emit multiple named scores directly. Verda's approach is simpler for common cases like "accuracy + latency + token_count from one evaluation call."

**Evalkit's design rationale**: The single-score-per-scorer constraint simplifies stats aggregation and ensures a 1:1 mapping between ScoreDefinition and score values. But it pushes complexity to the user.

### 4.4 `'static` Requirement on Builder Types

DEV-02 and DEV-04 require `'static` bounds on all generic types passed to the builder. This means borrowed data can't flow through the pipeline — everything must be owned or `Arc`'d. For expensive reference data (large test fixtures), this forces unnecessary cloning.

### 4.5 No Sample Tags / Filtering

Verda has `tags: Vec<String>` on samples for grouping/filtering. Evalkit has only `metadata: HashMap<String, Value>`. While metadata is more general, tags enable a very common eval workflow: "run only the 'regression' tagged samples" or "compare scores across difficulty tiers." Without first-class tag support, this requires custom metadata parsing.

## 5. Comparison with Verda's Key Abstractions

| Aspect | Evalkit | Verda | Assessment |
|--------|---------|-------|------------|
| Scorer return type | Single `Score` | `Vec<Score>` | **Verda wins** — more natural for multi-metric scorers |
| Score type | Enum (Numeric/Binary/Label/Metric) | Struct (name + f64 + goal) | **Evalkit richer**, but Metric/Numeric overlap is concerning |
| Dataset | Concrete `Vec<Sample>` | Trait with index-based materialization | **Verda wins** — more flexible, supports per-trial variation |
| Acquisition | Dedicated trait | Task function `Fn(I) -> Fut` | **Comparable** — evalkit's trait is slightly more structured |
| Error handling | ScorerError vs. Score | Same pattern | **Evalkit slightly better** — typed AcquisitionError |
| Stats | Wilson CI, Welch's t, Fisher's exact | Basic aggregation | **Evalkit substantially better** |
| Comparison | Full significance testing | Aggregate diffs | **Evalkit substantially better** |
| Mapper/Transform | First-class Mapper trait, ScorerSet | None (inline in scorer) | **Evalkit unique** — valuable for expensive transforms |
| Tracing | OTel Observe mode | None | **Evalkit unique** |
| Sample metadata | HashMap<String, Value> | Tags + Metadata | **Verda slightly better** — tags are first-class |
| Builder safety | Typestate (compile-time) | Runtime validation | **Evalkit better** for correctness |
| Concurrency | API exists, sequential impl | Configurable parallelism | **Verda better** — actually works |

## 6. Use Case Fitness (Preliminary)

For the brain dump agent use case (segment and classify thoughts, improve classification):

- **Evalkit can handle it**: Define `Scorer<Thought, Classification, Category>`, use reference-based scoring with exact_match or a custom multi-label scorer. Multi-trial with stats gives statistical rigor.
- **Gap**: If the agent emits both a classification AND a confidence score, you need either two scorers or a ScorerSet. Verda would let a single scorer emit both.
- **Gap**: No concurrent execution means evaluating many thoughts is slow if acquisition is expensive.

I'll defer detailed use case analysis to Agent 1 but flag that evalkit's single-score-per-scorer design creates friction for multi-output evaluation.

## 7. Critical Observations for the Team

1. **Evalkit's statistical layer is its strongest differentiator**. Wilson CIs, proper significance testing, direction-aware comparison — this is production-grade and hard to replicate.

2. **Evalkit's Mapper/ScorerSet system is genuinely novel** among eval frameworks. Shared mappers that run once per trial with multiple scorers is an elegant solution to the "expensive transform" problem.

3. **However**: The Dataset-as-concrete-type and single-Score-return-type are **less flexible than verda's equivalents**. These are not fatal but they push complexity to the user in common cases.

4. **The AcquisitionError leak is real but minor** — it pollutes domain-agnostic usage without causing actual breakage.

5. **What would change my mind**: If someone showed me a production use case where single-score-per-scorer is strictly better than Vec<Score> in a way that outweighs the ergonomic cost, I'd revise my assessment on point 3.

---

I'm **not yet ready** to make a recommendation on which library to continue — that depends on Agent 2's verda deep-dive and Agent 1's ideal abstraction. But my current lean is: **evalkit's statistical and comparison layer should be preserved regardless**, and the Dataset/Scorer flexibility from verda is worth incorporating.

Open questions for the team:
- Does verda have anything comparable to evalkit's OTel observe mode?
- How does verda handle the "mapper runs once, shared by multiple scorers" pattern?
- What is verda's actual concurrency implementation quality?
