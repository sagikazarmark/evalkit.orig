---
agent: agent-3
role: "Synthesizer"
phase: phase2
round: 2
seq: 016
timestamp: 2026-04-06T06:30:45Z
---

Good. Now I have concrete evidence to resolve the two biggest open questions. Let me write my synthesis.

---

# Agent 3 (Synthesizer) — Round 2: Updated Synthesis & Draft Recommendation

## Resolving Open Questions

### 1. Evalkit's Per-Sample Error Recovery: Confirmed ✅

Agent 2 asked whether evalkit aborts on acquisition failures. **It does not.** I verified:

- `execute()` (run.rs:103-127) loops over all samples and always pushes results — no early return on error
- `execute_trial()` (run.rs:150-170) catches `AcquisitionError` and calls `acquisition_failure_scores()`, which maps the error to `ScorerError` for every scorer definition
- `SampleResult` tracks `error_count` and `scored_count` explicitly
- There's a test (tests/run.rs:302-329) proving a timed-out acquisition records the error and the run completes successfully

**This removes Agent 2's concern.** Evalkit already has non-fatal error recovery comparable to verda's. The models differ slightly (evalkit: per-scorer `Result<Score, ScorerError>` in the HashMap; verda: separate `SampleError` list + `SampleStatus` enum), but the behavior is equivalent. No migration needed on this point.

### 2. Concurrency Effort: Agent 0 Was Wrong, Agent 2 Was Right

Agent 0 claimed concurrency is "approximately a 15-line change." **This is incorrect.** My investigation found:

- The `execute_trial()` method borrows `&self` and `&Sample<I, R>` with lifetime-parameterized futures: `TrialFuture<'a> = Pin<Box<dyn Future<Output = ...> + 'a>>`
- `buffer_unordered` with borrowed `&self` across concurrent tasks violates the borrow checker — you can't have multiple concurrent borrows of `self.acquisition` and `self.executor`
- Converting to concurrent execution requires **wrapping Run internals in Arc**, changing the lifetime model from borrow-based to ownership-based, and adding the `futures` crate as a dependency
- Realistic estimate: **200-400 lines of changes, Medium effort, 2-3 days of careful work**

The `Send + Sync` bounds ARE already in place (Agent 0 was right about that), and the `_concurrency` field exists in the API. But the lifetime architecture wasn't designed for concurrent sharing. This doesn't change the recommendation — it's still clearly feasible — but it's an honest Medium, not a Low.

**This matters for the migration plan.** If I'd blindly accepted "15 lines," the plan would underestimate the most important migration item.

## Adjudicating the Remaining Disagreements

### Score::Label — Keep It (Siding with Agent 2)

Agent 1 proposes dropping `Label(String)`. Agent 2 argues for keeping it with the classification confusion-matrix argument. **Agent 2 is right**, and I'll add a concrete reason neither has stated:

The *statistical dispatch* argument that Agent 1 uses to justify typed scores (Fisher's for Binary, Welch's t for Numeric) **also applies to Labels**. Label scores need a fundamentally different comparison method: you compare distributions (chi-squared test or similar), not means. If you encode labels as numeric (e.g., "work"=1, "personal"=2), a t-test on those numbers is **statistically meaningless** — the numeric encoding is ordinal at best, and often purely nominal. The type system should prevent this category error.

However, I accept Agent 1's point that the stats layer for Labels should be *minimal*: distribution + mode + optional chi-squared for comparison. Not a full confusion matrix — that's user-space analysis.

**Resolution: Keep `Label(String)` with minimal stats support. Three score types: Numeric, Binary, Label.**

### Score::Metric Separation — Full Separation (Siding with Agent 2)

Agent 0 proposes removing the `name` field from `Score::Metric`. Agent 2 argues for full separation. **Full separation is correct**, for a reason that cuts through the debate:

The Score enum dispatches statistical behavior. Adding Metric to it means every function that pattern-matches on Score must handle a variant with *fundamentally different semantics* (percentile aggregation vs CI-based testing). This is a textbook violation of the Liskov Substitution Principle — a Score::Metric doesn't behave like a Score in the stats context.

Verda's separation is cleaner: `TaskOutput<O>` contains both the output (scored) and optional metrics (measured). Metrics flow through a parallel aggregation path with percentiles. Scores flow through the CI/significance path. The paths never cross.

**Migration spec**: Add a `Metric` type (name, value, unit, goal). Modify acquisition/`TaskOutput` to carry `Vec<Metric>` alongside the output. Add `MetricSummary` with p50/p95/p99 to `RunResult`. Remove `Score::Metric` variant. Effort: Medium — touches acquisition, run result, and stats modules.

### Single-Score-per-Scorer — Confirmed Correct (Revising my R1 position)

I called this "a design mistake" in Round 1. Agent 0's Round 2 rebuttal convinced me:

1. The 1:1 `definition() → ScoreDefinition` binding enables build-time validation that verda compensates for with 4 runtime error variants
2. Agent 1's use case walkthrough: 0/12 cases strictly require multi-score-per-scorer
3. ScorerSet + shared mapper handles the expensive-shared-computation pattern

I'm dropping this from the migration plan entirely.

### Builder Complexity — Defer, Not Simplify

Agent 1 raised whether typestate builders produce opaque error messages. This is a valid concern but not actionable now — it's an ergonomics question that needs user feedback. The typestate builder provides real compile-time safety (can't add scorers with wrong types, can't double-map). That's worth keeping until we have evidence the error messages are a practical problem.

**Resolution: Keep typestate builder. Monitor DX feedback post-launch.**

## Final Recommendation

### **Continue evalkit. Sunset verda.**

**Core rationale** (ranked by weight):

1. **Statistical rigor** — Wilson CIs, Welch's t-test, Fisher's exact, direction-aware comparison, `pass_at_k`. This is evalkit's irreplaceable advantage. It would take weeks to retrofit into verda and would reshape verda's entire data model.

2. **OTel observe mode** — Trace-based acquisition is an emerging industry pattern (AgentEvals, EvalForge). Evalkit has a 533-line OTel module; verda has zero tracing infrastructure. Adding this to verda would be a ground-up effort.

3. **Spec-driven foundation** — 21 iterations of documented design decisions, conformance tracking, and explicit deviation management. This compounds: every future extension can be evaluated against the spec. Verda's ad-hoc design means higher risk for inconsistency as it grows.

4. **Typed score system** — Enables correct statistical dispatch. Verda's f64-only approach would need a breaking redesign to support proper Label/Binary analysis.

5. **Mapper/ScorerSet** — Novel abstraction for shared expensive transforms. No equivalent in verda or most other frameworks.

### Consolidated Migration Plan

| Priority | Item | Source | Effort | Rationale |
|----------|------|--------|--------|-----------|
| **P0** | Concurrent execution | verda's `buffer_unordered` pattern | **Medium** (200-400 LOC, 2-3 days) | Sequential-only is disqualifying for production. Requires Arc-wrapping internals and lifetime refactoring. API already exists. |
| **P1** | Dataset as trait | verda's `trait Dataset<I,T>` with `sample(index, trial)` | Low-Medium | Enables lazy loading, per-trial variation. Add `impl Dataset for Vec<Sample>` for backward compat. |
| **P1** | Sample tags | verda's `tags: Vec<String>` | Low | Essential for sliced analysis (UC-5, UC-8, UC-11, UC-12). Add field + filter API. |
| **P1** | Score/Metric separation | verda's `Metric` + `MetricSummary` design | Medium | Remove `Score::Metric`, add standalone `Metric` type, add `MetricSummary` with percentiles to results. |
| **P1** | Generic acquisition errors | verda's generic `E: Error` on task | Low-Medium | Replace `AcquisitionError` concrete variants with generic error type. Move OTel-specific variants behind feature gate. |
| **P2** | `#[non_exhaustive]` on all public types | verda's consistent usage | Low | Evalkit is inconsistent. Audit and add to all public enums/structs. |
| **P2** | `thiserror` for error types | verda's pattern | Low | More idiomatic than manual `impl Error`. |
| **P2** | MetricSummary percentiles | verda's p50/p95/p99 | Low | Production-useful for latency/performance metrics. |
| **P3** | Port verda's edge-case tests | verda's test suite | Low | Duplicate sample IDs, conflicting goals, non-finite values, blank score names — validate these invariants in evalkit. |

### What to Preserve from Verda Before Archiving

1. **`compare.rs`** — The per-sample `SampleDiff` with `Change` enum is well-structured. Cross-check against evalkit's comparison module; the `Incomparable` variant is cleaner than evalkit's approach.
2. **`MetricSummary`** with percentile computation — direct port target.
3. **Test suite** — The `TrialDataset`, `UnstableTrialIdDataset`, and score integrity edge cases represent validated invariants.
4. **`Display` impl on comparison results** — Nice DX that evalkit should replicate.
5. **The `async_trait` decision** — While evalkit's native async traits are technically superior, document that `async_trait` compatibility may be needed for users on older Rust editions.

### Lessons from Verda for Evalkit's Future Development

1. **Simplicity has value.** Verda's `Evaluation::new(dataset, task, scorers)` is more approachable than evalkit's 4-phantom-type builder. As evalkit evolves, resist adding complexity without proportional gain.
2. **Separate what you measure from what you judge.** Verda's Score/Metric separation is conceptually correct and should be adopted.
3. **Concurrency is table stakes.** Deferring it was reasonable for MVP, but the lifetime architecture should have been designed with concurrency in mind from the start. The Arc-wrapping retrofit is the tax for this deferral.
4. **Tags > metadata for the common case.** While `HashMap<String, Value>` is more general, first-class tags are what users actually reach for when slicing results.

## CONVERGENCE STATEMENT

The team has reached substantive agreement on the core recommendation (continue evalkit), the major migration items (concurrency, Dataset trait, tags, Metric separation), and the key design decisions (single-score-per-scorer, typed scores with Label). The remaining open items — exact concurrency implementation details, Label stats scope, builder DX monitoring — are implementation-phase decisions that don't require further deliberation. DELIBERATION COMPLETE
