> **📦 Archived on 2026-04-23** — superseded by [Evalkit vs. Verda: Final Evaluation Report](../../output.md). Kept for historical reference.

---
agent: agent-3
role: "Synthesizer"
phase: phase2
round: 1
seq: 012
timestamp: 2026-04-06T06:21:20Z
---

Now I have enough code-level evidence to form an independent synthesis. Here's my analysis:

---

## Agent 3 (Synthesizer) — Round 1: Independent Assessment

### Untested Assumptions I Want to Flag

Before diving in, three assumptions in the task description need scrutiny:

1. **"evalkit is the result of a better planning process"** — The extensive docs confirm a thorough *process*, but process quality ≠ API quality. I'll check whether the spec-driven approach produced a better or merely more complex API.
2. **"First-class tracing support is vital"** — This needs grounding in use cases. Vital for *what*? Observing production agents? Debugging scorer logic? The two libraries interpret "tracing" very differently.
3. **"Stable abstraction that works for various use cases"** — Neither library has been validated against diverse non-AI use cases. Both are untested against real-world usage.

### Head-to-Head: Concrete Design Differences That Matter

| Dimension | evalkit | verda | Verdict |
|-----------|---------|-------|---------|
| **Score model** | Enum: `Numeric(f64)`, `Binary(bool)`, `Label(String)`, `Metric{...}` — scorer returns ONE `Score` | Always `f64` with name+goal — scorer returns `Vec<Score>` | **Split.** evalkit's typed scores are semantically richer, but verda's multi-score-per-scorer is more practical (one scorer can emit accuracy + latency). |
| **Acquisition/Task** | Formal `Acquisition<I,O>` trait with OTel `Observe` variant | Bare `Fn(I) -> Fut<Result<R,E>>` with `R: Into<TaskOutput<O>>` | **evalkit** wins on tracing. But verda's generic error type is friendlier — evalkit forces `AcquisitionError` specifically. |
| **Concurrency** | Sequential only (deferred) | Real `buffer_unordered` parallelism | **verda** — this is table stakes for any eval framework. |
| **Dataset** | Wrapper over `Vec<Sample>` | Trait with `sample(index, trial_index)` — enables per-trial variation | **verda** — the trial_index parameter is a genuinely useful abstraction for stochastic datasets. |
| **Mapper** | Dedicated `Mapper<I,O>` trait, per-run and per-ScorerSet | None | **evalkit** — useful for normalizing outputs before scoring. |
| **Tags** | None on samples | `Vec<String>` tags on samples | **verda** — tags enable sliced analysis (e.g., "accuracy on category X"). |
| **Task Metrics** | Conflated into `Score::Metric` variant | Separate `Metric` type + `MetricSummary` with percentiles | **verda** — separating metrics from scores is correct. Latency is not a "score." |
| **Tracing/OTel** | First-class: `TraceBackend`, `Observe`, `JaegerBackend`, task-local correlation | One `tracing::warn!` call | **evalkit** by a mile. |
| **Statistics** | Wilson CI, t-distribution, significance testing, `pass_at_k` | mean/median/stddev/min/max + p50/p95/p99 | **evalkit** for rigor. verda's percentiles on metrics are useful though. |
| **Builder complexity** | Type-state builder with 4 phantom type params | `Evaluation::new(dataset, task, scorers)` | **verda** — evalkit's builder is over-engineered for the gain. |
| **Built-in scorers** | 5 (exact_match, contains, regex, json_schema, llm_judge) | Zero | **evalkit** — practical value. |
| **Error modeling** | Custom error types, manual `impl Error` | `thiserror` derive, cleaner | **verda** — more idiomatic Rust. |

### My Preliminary Recommendation

**Continue evalkit. Migrate specific verda wins into it.**

Rationale:
- The task explicitly states "first-class tracing support is vital." evalkit has this; verda fundamentally doesn't. Adding OTel to verda would be a larger refactor than simplifying evalkit's API.
- evalkit's spec + conformance + research corpus is an asset that compounds over time. verda's ad-hoc design means future extensions are higher-risk.
- evalkit's statistical rigor (significance testing, CIs) is genuinely hard to add after the fact — it shapes the data model.

**But evalkit has real problems that verda solves better:**

### Migration Plan (Verda → Evalkit)

| Priority | What to Migrate | Why | Effort |
|----------|----------------|-----|--------|
| **P0** | Concurrent execution (`buffer_unordered`) | Blocking. Sequential-only is disqualifying for real workloads. | Medium — requires reworking `run.rs` execute loop |
| **P0** | Multi-score per scorer (`Vec<Score>`) | Evalkit's single-Score-per-scorer is a design mistake. A classification scorer needs to emit precision, recall, F1 in one pass. | Medium — changes `Scorer` trait signature, `ScorerContext` flow |
| **P1** | Separate Metric type from Score | Score::Metric conflates measurement with evaluation. Latency is not a scorer judgment. | Low — add `Metric` type, adjust `TaskOutput`/`RunResult` |
| **P1** | Sample tags | Essential for sliced analysis ("how does the agent perform on category X?"). | Low — add field to `Sample` |
| **P1** | Dataset trait with `trial_index` | Enables stochastic datasets and per-trial input variation. | Low — change `Dataset` from struct to trait |
| **P2** | Simplify builder API | Type-state with 4 phantom params is excessive. Consider verda's `Evaluation::new()` approach or at least reduce phantom complexity. | Medium |
| **P2** | Use `thiserror` for error types | More idiomatic, less boilerplate. | Low |
| **P2** | Task error generics | Allow `Fn(I) -> Result<O, E>` where E: Error, not just `AcquisitionError`. | Low |

### What I'd Preserve from Verda Before Discarding

1. **The `compare.rs` module** — verda's per-sample improvement/regression tracking with `Change` enum and `SampleDiff` is well-structured. Cross-check with evalkit's comparison module to see if evalkit's is strictly better or if verda's sample-level diff granularity should be ported.
2. **`MetricSummary` with percentiles** (p50/p95/p99) — evalkit's stats module may not have this for metrics.
3. **The test suite** — verda's tests for edge cases (duplicate sample IDs, conflicting goals, blank score names, non-finite values) represent validated invariants that should be added to evalkit's test suite if not already present.

### What Would Change My Mind

- If Agent 0 shows evalkit's type-state builder enables compile-time safety that catches real bugs users would hit, I'd soften the "simplify builder" recommendation.
- If Agent 2 shows verda's comparison module handles multi-trial aggregation better than evalkit's, that would elevate the comparison migration priority.
- If Agent 1's use-case analysis reveals that most use cases don't need multi-score-per-scorer, I'd reconsider that P0.
- If the "brain dump agent" use case requires tagging + sliced analysis heavily, tags become P0.

### Open Questions for Other Agents

1. **Agent 0**: Does evalkit's conformance report flag the missing concurrency as a known gap? Is there a design reason it was deferred beyond "post-MVP"?
2. **Agent 1**: In your use case list, how many require multi-score-per-scorer? How many need sample tags for sliced reporting?
3. **Agent 2**: How does verda handle the case where trial_count > 1 and dataset.sample() returns different inputs per trial? Is that actually used/tested, or is it a theoretical capability?

---

This is my independent assessment. I expect disagreement on the builder complexity question and possibly on whether Score::Metric should be separated. I'm confident on the core recommendation (continue evalkit) but hold it loosely — the concurrency gap is serious enough that if evalkit's architecture makes concurrent execution structurally difficult, the calculus changes.
