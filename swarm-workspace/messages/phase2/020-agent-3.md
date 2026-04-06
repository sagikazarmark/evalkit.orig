---
agent: agent-3
role: "Synthesizer"
phase: phase2
round: 3
seq: 020
timestamp: 2026-04-06T06:35:41Z
---

Good — evalkit already has `Change::Incomparable`. Agent 2 is correct: verda's comparison adds nothing evalkit doesn't have. And evalkit has no `Display` impls on result types — confirmed.

Now I have everything needed for the final deliverable.

---

# Agent 3 (Synthesizer) — Round 3: FINAL CONSOLIDATED DELIVERABLE

## Executive Summary

**Recommendation: Continue evalkit. Archive verda.** Migrate 7 specific capabilities from verda into evalkit.

This recommendation is grounded in 12 validated use cases, ecosystem analysis of 65+ tools, and detailed code-level analysis of both libraries. All four agents agree.

## Decision Register — All Resolved Items

| Decision | Resolution | Dissent | Evidence |
|----------|-----------|---------|----------|
| **Which library to continue** | evalkit | None | Statistical rigor (Wilson CI, Welch's t, Fisher's exact) + OTel observe mode are structurally irreplaceable; verda alignment to ideal spec: ~45% vs evalkit's ~80% |
| **Single vs multi-score per scorer** | Single (`Result<Score, ScorerError>`) | None (Agent 3 R1 initially disagreed, revised R2) | 1:1 `definition()→ScoreDefinition` enables build-time validation; 0/12 use cases require multi-score; ScorerSet handles shared computation |
| **Score type system** | Typed enum: `Numeric(f64) \| Binary(bool) \| Label(String)` | Agent 1 initially proposed dropping Label; conceded R3 | Typed dispatch enables correct statistics: Welch's t for Numeric, Fisher's exact for Binary, chi-squared for Label. Encoding labels as f64 is a statistical category error. |
| **Score::Metric** | Full separation into standalone `Metric` type | Agent 0 initially proposed keeping in enum with name removed; conceded R3 | Liskov Substitution violation: Metrics need percentile aggregation (p50/p95/p99), Scores need CI-based significance testing. Different aggregation semantics = different types. |
| **Dataset** | Trait with `sample(index, trial_index)` | None | Verda's tested impl validates: enables lazy loading, per-trial variation. `Vec<Sample>` blanket impl preserves backward compat. |
| **Acquisition errors** | Generic `E: Error + Send + Sync`; OTel variants behind feature gate | None | `AcquisitionError::TraceNotFound` surfacing in non-OTel usage is an abstraction leak |
| **Builder pattern** | Keep typestate; monitor DX feedback | None | Compile-time safety (prevents wrong-type scorers, double-mapping) outweighs error message opacity risk |
| **`'static` bounds** | Document as type-erasure trade-off, NOT inherent to concurrency | Agent 1 initially claimed inherent; Agent 2 corrected R3 | Verda proves cooperative concurrency (`buffer_unordered`) works without `'static`. Evalkit's `'static` comes from `Box<dyn>` erasure. |
| **Verda's `compare.rs`** | Do NOT port; evalkit's comparison is strictly superior | None | Evalkit already has `Change::Incomparable`, plus significance testing, CIs, effect sizes |

## Consolidated Migration Plan

### P0 — Must Have (Blocks Production Use)

**1. Concurrent Execution**
- **Source**: verda's `buffer_unordered` pattern in `run_work_items`
- **Effort**: Medium (200-400 LOC, 2-3 days)
- **Why Medium, not Low**: `TrialFuture<'a>` and `AcquisitionFuture<'a, O>` lack `+ Send` bounds. Adding `Send` cascades through all 4 `RunExecutor` variants, `ScorerSet`'s `ScoreFuture`, and the user-facing `Scorer` trait's async method. Iteration-010 build logs confirm this cascade was attempted and abandoned during development.
- **Approach**: Follow verda's pattern — pre-materialize work items, `stream::iter().map().buffer_unordered(n)`, sort results by `(sample_index, trial_index)`. Use cooperative concurrency (no `tokio::spawn`) to avoid tightening `'static` requirements beyond what type-erasure already demands.
- **Risk**: If `Send` bound cascade proves intractable with the current trait hierarchy, may require `Arc`-wrapping internal executor state — adding ~100 LOC.

### P1 — High Value (Should Ship Soon After P0)

**2. Dataset as Trait**
- **Source**: verda's `trait Dataset<I, T>` with `sample(index, trial_index) → Sample`
- **Effort**: Low-Medium
- **Spec**: Replace `pub struct Dataset<I, R>` with `pub trait Dataset<I, R>: Send + Sync { fn len(&self) -> usize; fn sample(&self, index: usize, trial_index: usize) -> Sample<I, R>; }`. Add `impl<I, R> Dataset<I, R> for Vec<Sample<I, R>>` returning `self[index].clone()` (ignoring trial_index for backward compat). Migrate `metadata: HashMap<String, Value>` to a separate `DatasetMeta` or add to trait as default method.
- **Validation**: Port verda's `TrialDataset` and `UnstableTrialIdDataset` tests.

**3. Sample Tags**
- **Source**: verda's `tags: Vec<String>` on `Sample`
- **Effort**: Low
- **Spec**: Add `pub tags: Vec<String>` to `Sample<I, R>`. Add `Sample::with_tag(tag: impl Into<String>)` builder method. No changes to execution — tags are metadata for downstream sliced analysis.
- **Use cases**: UC-5 (brain dump categories), UC-8 (input complexity tiers), UC-11 (condition types), UC-12 (content categories).

**4. Score/Metric Separation**
- **Source**: verda's standalone `Metric` type + `MetricSummary`
- **Effort**: Medium
- **Spec**: 
  - Add `Metric { name: String, value: f64, unit: Option<String>, goal: Direction }`
  - Modify acquisition output to carry `Vec<Metric>` alongside the primary output (via a `TaskOutput<O>` wrapper or parallel field in `TrialResult`)
  - Add `MetricSummary { mean, stddev, min, max, p50, p95, p99 }` to `RunResult`
  - Remove `Score::Metric` variant. Score becomes: `Numeric(f64) | Binary(bool) | Label(String)`
  - Update `stats.rs` to handle Label: distribution + mode (minimal)
  - Update `comparison.rs` to handle Label: chi-squared test on distributions

**5. Generic Acquisition Errors**
- **Source**: verda's generic `E: Error` on task function
- **Effort**: Low-Medium
- **Spec**: Change `Acquisition<I, O>` to `Acquisition<I, O, E = AcquisitionError>` where `E: Error + Send + Sync + 'static`. Blanket impl for closures returns user-defined error type. `AcquisitionError` becomes the default for OTel observe mode only. Move `TraceNotFound` and `BackendUnavailable` behind `#[cfg(feature = "otel")]`.

### P2 — Nice to Have (Polish)

**6. `#[non_exhaustive]` Audit**
- **Effort**: Low. Audit all public enums and structs; add `#[non_exhaustive]` where missing.

**7. `thiserror` for Error Types**
- **Effort**: Low. Replace manual `impl Error` + `impl Display` with `#[derive(thiserror::Error)]`.

**8. `Display` Impls on Result Types**
- **Source**: verda's formatted table output for `RunResult` and `ComparisonResult`
- **Effort**: Low. Implement `Display` for `RunResult`, `RunStats`, `Comparison`.

**9. MetricSummary Percentiles**
- Included in P1 item 4 above; listed separately for tracking. Port verda's percentile computation (p50/p95/p99).

### P3 — Backlog

**10. Port Verda's Edge-Case Tests**
- **Effort**: Low. Verify evalkit handles: duplicate sample IDs, conflicting scorer goals, blank score names, non-finite score values, inconsistent cross-trial sample IDs.

## Lessons from Verda for Evalkit's Ongoing Development

1. **Simplicity is a feature.** Verda's `Evaluation::new(dataset, task, scorers)` vs evalkit's 4-phantom-type builder illustrates the cost of type-level safety. The safety is worth it — but every future API addition should be weighed against ergonomic cost.

2. **Separate what you measure from what you judge.** Verda's Score/Metric split is the correct conceptual model. Conflating them in an enum was evalkit's most significant design error.

3. **Design for concurrency from day 1.** The `Send` bound cascade is the tax for deferring concurrency. Future trait additions should include `Send` bounds even if concurrent execution isn't implemented yet.

4. **First-class tags beat general-purpose metadata for the common case.** Users reaching for `metadata.get("category").and_then(|v| v.as_str())` when `tags.contains("category:work")` would suffice is unnecessary friction.

5. **`Display` impls are last-mile DX.** Users will `println!("{}", result)` before they write custom formatters. This is cheap to implement and disproportionately improves first impressions.

## What Would Reverse This Recommendation

| Trigger | Action |
|---------|--------|
| Concurrency retrofit exceeds 1000 LOC or requires redesigning the trait hierarchy | Re-evaluate starting from verda with evalkit's stats/OTel modules ported in |
| Real-world usage shows <5% of evaluations use typed scores (Binary/Label) | Simplify Score to verda's f64-only model |
| OTel observe mode has <5% adoption after 12 months | Consider removing the Acquisition trait in favor of verda's simpler closure approach |

## CONVERGENCE STATEMENT

The team has reached full agreement on the core recommendation (continue evalkit), the migration plan (7 items from verda, prioritized P0-P3 with calibrated effort estimates), and all contested design decisions (single-score, typed Score enum with Label, full Metric separation, Dataset trait, generic acquisition errors). The ideal abstraction specification provides an objective yardstick for implementation. No further deliberation rounds would add significant value.

DELIBERATION COMPLETE
