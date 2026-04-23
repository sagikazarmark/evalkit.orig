> **📦 Archived on 2026-04-23** — superseded by [Evalkit vs. Verda: Final Evaluation Report](../../output.md). Kept for historical reference.

---
agent: agent-3
role: "Synthesizer"
phase: phase3
round: 1
seq: 028
timestamp: 2026-04-06T06:40:35Z
---

# Agent 3 — Synthesizer: Final Synthesis

## Recommendation

**Continue evalkit. Archive verda.**

This is the unanimous recommendation of all four agents after 4 rounds of deliberation, grounded in code-level analysis of both libraries, 12 validated use cases, and ecosystem survey of 65+ eval tools.

---

## Why Evalkit

Evalkit aligns ~80% with the ideal abstraction specification; verda aligns ~45%. The gap is structural, not incremental:

1. **Statistical rigor** — Wilson confidence intervals, Welch's t-test, Fisher's exact test, direction-aware comparison with significance classification. This shapes the data model and cannot be bolted on after the fact. (~613 lines of battle-tested stats code)

2. **OTel observe mode** — Trace-based acquisition (score an agent's existing traces rather than re-executing) is an emerging industry pattern validated by AgentEvals and EvalForge. Evalkit has a 533-line OTel module; verda has zero tracing infrastructure.

3. **Typed score system** — `Numeric(f64) | Binary(bool) | Label(String)` enables correct statistical dispatch: Welch's t for Numeric, Fisher's exact for Binary, chi-squared for Label. Verda's f64-only model makes encoding "pneumonia" as a float and running a t-test on it a statistical category error the type system can't prevent.

4. **Spec-driven foundation** — 21 iterations of documented design decisions, conformance tracking with 5 deviations + 2 omissions explicitly documented. This compounds: every future extension can be evaluated against the spec.

5. **Mapper/ScorerSet** — Novel abstraction for shared expensive transforms (e.g., parse JSON once, score faithfulness + relevance + groundedness). No equivalent in verda or most other frameworks.

---

## Areas of Full Consensus

Every contested decision was resolved with code-level evidence:

| Decision | Resolution | Key Evidence |
|----------|-----------|-------------|
| Single vs multi-score per scorer | **Single** (`Result<Score, ScorerError>`) | 1:1 `definition()→ScoreDefinition` binding enables build-time validation; 0/12 use cases require multi-score; ScorerSet handles shared computation |
| Score type system | **Typed enum** (Numeric, Binary, Label) | Different score types require different statistical tests — conflating them is a category error |
| Score::Metric | **Full separation** into standalone `Metric` type | Liskov Substitution violation: Metrics need percentiles (p50/p95/p99), Scores need CIs. Different aggregation semantics = different types |
| Dataset design | **Trait** with `sample(index, trial_index)` | Verda's tested impl validates: enables lazy loading and per-trial stochastic variation |
| Acquisition errors | **Generic** `E: Error + Send + Sync`; OTel variants behind feature gate | `AcquisitionError::TraceNotFound` surfacing in non-OTel usage is an abstraction leak |
| Builder pattern | **Keep typestate**; monitor DX feedback | Compile-time safety outweighs error message opacity risk |
| `'static` bounds | **Type-erasure cost**, NOT concurrency cost | Verda proves cooperative concurrency (`buffer_unordered`) works without `'static`; evalkit's `'static` comes from `Box<dyn>` erasure |

**No unresolved disagreements remain.** Every initial disagreement (Agent 3 R1 on multi-score, Agent 1 R1-R2 on Label, Agent 0 R2 on Metric separation, Agent 0 R2 on concurrency effort) was resolved through evidence and explicit concession.

---

## Migration Plan: What to Bring from Verda

### P0 — Blocks Production Use

**Concurrent Execution** — Medium effort (200-400 LOC, 2-3 days)
- Follow verda's `buffer_unordered` cooperative concurrency pattern (no `tokio::spawn`)
- Core challenge: `TrialFuture<'a>` and `AcquisitionFuture<'a, O>` lack `+ Send` bounds; adding them cascades through all 4 `RunExecutor` variants, `ScorerSet`'s `ScoreFuture`, and the `Scorer` trait
- Pre-materialize work items, `stream::iter().map().buffer_unordered(n)`, sort results by `(sample_index, trial_index)`
- Validate against: UC-3 (multi-trial agent), UC-7 (API regression), UC-8 (fuzz testing)

### P1 — High Value

| Item | Effort | Validation Use Cases |
|------|--------|---------------------|
| Dataset as trait with `sample(index, trial_index)` | Low-Medium | UC-5 (brain dump), UC-8 (large datasets) |
| Sample tags (`Vec<String>`) | Low | UC-5 (categories), UC-11 (conditions), UC-12 (content types) |
| Score/Metric separation + `MetricSummary` with p50/p95/p99 | Medium | UC-7 (latency), UC-8 (performance) |
| Generic acquisition errors (`E: Error + Send + Sync`) | Low-Medium | All non-AI use cases (UC-7 through UC-12) |

### P2 — Polish

| Item | Effort |
|------|--------|
| `#[non_exhaustive]` audit (missing from `AcquisitionError`, `Change`, `RunResult`, etc.) | Low |
| `thiserror` for error types | Low |
| `Display` impls on `RunResult`, `RunStats`, `Comparison` | Low |

### P3 — Backlog

| Item | Effort |
|------|--------|
| Port verda's edge-case tests (duplicate sample IDs, conflicting goals, non-finite values, blank score names, inconsistent cross-trial IDs) | Low |

---

## What NOT to Port from Verda

- **`compare.rs`** — evalkit's comparison is strictly superior (already has `Change::Incomparable` plus significance testing, CIs, effect sizes)
- **Non-fatal error recovery** — evalkit already has equivalent behavior (confirmed: `execute()` continues on acquisition failure, records per-scorer errors)
- **`Vec<Score>` multi-score return** — inferior to 1:1 ScoreDefinition binding + ScorerSet
- **f64-only score model** — prevents correct statistical dispatch

---

## Lessons from Verda

1. **Simplicity is a feature.** `Evaluation::new(dataset, task, scorers)` vs 4-phantom-type builder. Every future evalkit API addition should be weighed against ergonomic cost.
2. **Separate what you measure from what you judge.** Score/Metric conflation was evalkit's most significant design error.
3. **Design for concurrency from day 1.** The `Send` bound cascade is the tax for deferring it. Future trait additions should include `Send` bounds even if concurrent execution isn't yet implemented.
4. **Tags beat general-purpose metadata for the common case.** `tags.contains("work")` > `metadata.get("category").and_then(|v| v.as_str())`.
5. **`Display` impls are last-mile DX.** Cheap to implement, disproportionate impact on first impressions.

---

## Blind Spots & Risks

1. **Zero real users.** Both libraries are pre-launch. Every design decision is validated against hypothetical use cases and ecosystem patterns, not actual user feedback. The reversal triggers below are the safety net.

2. **Rust edition compatibility.** Evalkit uses native async traits (edition 2024). If the library needs to support edition 2021 users, the `Scorer` trait becomes unimplementable without `#[async_trait]`. This should be an explicit, documented decision.

3. **The concurrency retrofit is the highest-risk item.** If the `Send` bound cascade interacts poorly with the type-erased executor architecture, effort could balloon beyond the 200-400 LOC estimate. Recommend a time-boxed spike (1 day) before committing to the full implementation.

4. **Label stats scope creep.** The team agreed on minimal Label support (distribution + mode + chi-squared). Confusion matrices, per-class precision/recall, and other classification analysis tools are explicitly out of scope for the core library. This boundary should be documented to prevent scope creep.

5. **No benchmarking.** Neither library has performance benchmarks. Once concurrency is added, evalkit should benchmark throughput (samples/sec) at various concurrency levels to validate the implementation.

---

## Reversal Triggers

| Trigger | Action |
|---------|--------|
| Concurrency retrofit exceeds 1000 LOC or requires trait hierarchy redesign | Re-evaluate starting from verda with evalkit's stats/OTel modules ported in |
| <5% typed score adoption after 12 months | Simplify Score to f64-only |
| <5% OTel observe mode adoption after 12 months | Consider removing Acquisition trait in favor of simpler closures |

---

## Concrete Next Steps

1. **Spike**: Time-boxed 1-day spike on adding `Send` bounds to evalkit's future types — validate the concurrency migration is feasible within the 200-400 LOC estimate
2. **Implement P0**: Concurrent execution with `buffer_unordered`
3. **Implement P1 items** in parallel (they're independent): Dataset trait, Sample tags, Metric separation, Generic acquisition errors
4. **Archive verda** with a README pointing to evalkit and documenting what was migrated
5. **Document**: Minimum supported Rust edition, `'static` trade-off rationale, Label stats scope boundary

**DELIBERATION COMPLETE. All agents unanimous.**
