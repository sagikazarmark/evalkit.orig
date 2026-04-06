---
agent: agent-1
role: ""
phase: phase2
round: 3
seq: 018
timestamp: 2026-04-06T06:33:17Z
---

# Agent 1 — Eval Researcher: Round 3 (Final Position)

## Conceding on Score::Label — With a Constraint

Agents 0, 2, and 3 all argue to keep `Label(String)`. I concede. The argument that changed my mind is Agent 3's statistical dispatch point: **chi-squared comparison for labels vs. t-test for numerics is a genuine type-level distinction**, not just a presentation concern. Encoding "pneumonia" as 1.0 and running Welch's t-test is a statistical error the type system should prevent.

My Round 2 position ("Label is too niche") was wrong because I was thinking about aggregation (distribution + mode) rather than **comparison**. Comparing two runs' label distributions requires a different statistical test. That's the same structural argument that justifies Binary as a separate type from Numeric. I should have been consistent.

**The constraint I'd add**: Label stats support should be explicitly scoped to `{distribution, mode, chi_squared_comparison}`. No confusion matrix at the library level — that's a higher-layer concern that requires pairing predicted labels with reference labels across samples, which is a different data shape than per-sample scoring.

## Confirming Score::Metric Full Separation

I agreed with this in Round 2 and Agent 2's LSP argument in the same round sealed it. The Score enum becomes three variants: `Numeric(f64)`, `Binary(bool)`, `Label(String)`. Metrics flow through a parallel path. Clean.

## On Agent 0's `'static` Bound Issue

This is a real ergonomic concern that I can ground in ecosystem patterns. Looking at comparable Rust evaluation/testing frameworks:

- **criterion** (benchmarking): Uses `'static` bounds on benchmark functions but not on data. Allows borrowed reference data.
- **proptest**: Uses owned data throughout, similar to evalkit's constraint.
- **tokio::spawn**: Requires `'static` — the standard Rust pattern for async task spawning.

The `'static` bound is **inherent to the concurrent execution model evalkit needs to adopt**. `tokio::spawn` and `buffer_unordered` both require `'static` futures. So this isn't a fixable limitation — it's the price of concurrency in Rust. Verda's closure approach appears more flexible, but once you add `buffer_unordered` (which verda already uses), the same `'static` constraints apply via `Send + 'static` on the stream items.

**Implication**: This should be documented as a known trade-off, not a future fix target. The mitigation is `Arc<T>` for large shared reference data, which is idiomatic Rust for concurrent workloads.

## Final Ideal Abstraction Specification (Updated)

Incorporating all three rounds of deliberation, here is the final ideal spec that serves as the objective yardstick:

### Core Types
```
Sample<I, R>              — input + optional reference + stable ID + tags: Vec<String> + metadata
Dataset<I, R>             — TRAIT with sample(index, trial_index) → Sample; Vec<Sample> impl for convenience
Score                     — Enum: Numeric(f64) | Binary(bool) | Label(String)
ScoreDefinition           — name + Direction(Maximize|Minimize) + score_type hint; 1:1 with Scorer
Metric                    — Standalone type: name + value(f64) + unit + Goal
Scorer<I, O, R>           — async fn score(ctx) → Result<Score, ScorerError>; single score per scorer
ScorerSet<I, O, R>        — Grouped scorers with shared Mapper; mapper runs once per trial
Acquisition<I, O, E>      — Generic over error type E: Error + Send + Sync; blanket impl for closures
Mapper<I, O>              — Synchronous transform; output and/or reference adaptation
```

### Execution
```
Run::builder()
  .dataset(impl Dataset<I, R>)
  .acquisition(impl Acquisition<I, O, E> | closure)
  .scorer(impl Scorer<I, O, R>)        — additive, at least one
  .scorer_set(ScorerSet<I', O', R'>)    — optional, with mapper
  .trials(N)
  .concurrency(N)                       — real concurrent execution via buffer_unordered
  .build()? → Run
  .execute().await? → RunResult
```

### Results
```
RunResult                 — Per-sample, per-trial, per-scorer raw Score results + Vec<Metric> per trial
RunStats                  — Derived from RunResult:
                            Numeric → mean, stddev, CI (t-distribution), min, max
                            Binary  → pass_rate, pass_at_k, CI (Wilson), count
                            Label   → distribution, mode
MetricSummary             — mean, stddev, min, max, p50, p95, p99
Comparison                — Baseline vs candidate per-scorer:
                            Numeric → Welch's t-test, effect size, direction-aware Change
                            Binary  → Fisher's exact, direction-aware Change
                            Label   → chi-squared test on distributions
                            Per-sample deltas with Change enum (Improved|Regressed|Unchanged|Incomparable)
```

### Design Principles (ecosystem-grounded)
1. **Typed scores enable correct statistical dispatch** — validated by the chi-squared/Fisher's/Welch's distinction
2. **1:1 Scorer:Score with ScorerSet for composition** — validated by 0/12 use cases requiring multi-score
3. **Metrics ≠ Scores** — different aggregation semantics (percentiles vs. CIs)
4. **Acquisition generic over error type** — domain-agnostic, OTel errors behind feature gate
5. **Dataset as trait with trial awareness** — enables lazy loading and per-trial variation
6. **First-class tags for sliced analysis** — 4-5/12 use cases need them
7. **Non-fatal error recovery** — both libraries already have this (confirmed by Agent 3)
8. **OTel observe mode as first-class acquisition variant** — validated by AgentEvals, EvalForge ecosystem trend

### Evalkit Alignment Score: ~80%
Gaps: no concurrency (P0), concrete Dataset (P1), no tags (P1), Score::Metric conflation (P1), AcquisitionError not generic (P1)

### Verda Alignment Score: ~45%
Gaps: no typed scores, no statistical rigor, no OTel, no mapper, no significance testing, no observe mode

The 35-point gap confirms the recommendation quantitatively.

## What Would Still Change My Mind

| Position | Reversing evidence |
|----------|-------------------|
| Continue evalkit | The `Send` bound cascade for concurrency proves intractable (requires >1000 LOC or trait redesign) |
| Keep Label | Zero users request Label support AND chi-squared comparison proves unnecessary in practice |
| Ideal spec completeness | A real-world use case that none of my 12 cases cover AND that neither library can serve |

## CONVERGENCE STATEMENT

The team has reached substantive agreement on all major decisions: continue evalkit, migrate concurrency/Dataset-trait/tags/Metric-separation/generic-errors from verda, keep single-score-per-scorer with typed Score enum (Numeric/Binary/Label). The ideal abstraction specification is finalized and provides an objective yardstick for implementation. Remaining items are implementation-phase details. DELIBERATION COMPLETE
