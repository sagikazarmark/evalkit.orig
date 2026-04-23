> **📦 Archived on 2026-04-23** — superseded by [evalkit Kernel Boundary Plan](../../evalkit-kernel-boundary-plan.md). Kept for historical reference.

# Evalkit vs. Verda: Final Evaluation Report

## Executive Summary

After comprehensive analysis of both evalkit and verda — including ecosystem research across 65+ eval tools, validation against 12 concrete use cases, and deep code-level inspection — the unanimous recommendation is to **continue developing evalkit and archive verda**. Evalkit aligns approximately 80% with the ideal abstraction specification versus verda's 45%, driven by three structural advantages that cannot be incrementally retrofitted: statistical rigor (Wilson CIs, Welch's t-test, Fisher's exact test), OTel-native trace observation, and typed score dispatch. However, verda contributes several critical capabilities — concurrent execution, Score/Metric separation, Dataset-as-trait, and sample tags — that must be migrated to evalkit before it can be considered production-ready. The highest-risk item is the concurrency retrofit, which should be validated with a time-boxed spike before committing to the full migration.

---

## 1. Use Cases for Validation

Twelve use cases were defined to objectively evaluate both libraries. These span AI-specific and domain-agnostic scenarios, validating the design principle that the library should be independent of AI terminology:

| # | Use Case | Key Requirements |
|---|----------|-----------------|
| UC-1 | RAG answer quality | Numeric scoring, multi-scorer (faithfulness + relevance + groundedness) |
| UC-2 | LLM judge consistency | Binary scoring, multi-trial, inter-rater agreement |
| UC-3 | Multi-trial agent evaluation | Concurrent execution, statistical significance across trials |
| UC-4 | Prompt A/B testing | Paired comparison, direction-aware significance |
| UC-5 | Brain dump agent (user's case) | Classification scoring (Label), tags for categories, lazy dataset |
| UC-6 | Code generation correctness | Binary pass/fail, sandboxed execution as acquisition |
| UC-7 | API regression testing | Latency metrics (p50/p95/p99), non-AI acquisition errors |
| UC-8 | Fuzz testing / large datasets | Lazy loading, concurrent execution, performance metrics |
| UC-9 | Document classification pipeline | Label scores, chi-squared comparison, confusion analysis |
| UC-10 | Translation quality | Numeric (BLEU), multi-scorer, cross-language slicing |
| UC-11 | A/B experiment analysis | Binary conversion, Fisher's exact test, segment tags |
| UC-12 | Content moderation tuning | Label + Binary hybrid, per-category (tag) analysis |

**Key finding**: 0 of 12 use cases require multiple scores from a single scorer, validating the single-score design. 4–5 of 12 require tag-based slicing. All AI use cases (UC-1 through UC-6) benefit from OTel trace observation. All use cases requiring comparison benefit from typed statistical dispatch.

---

## 2. Ideal Abstraction Specification

Based on the use cases and ecosystem survey, the ideal low-level eval library provides:

```
Dataset (trait)        → produces Samples with tags
  ↓
Acquisition (trait)    → transforms input → output (or observes traces)
  ↓
Scorer (trait)         → produces exactly one typed Score per ScoreDefinition
  ↓
Score (enum)           → Numeric(f64) | Binary(bool) | Label(String)
Metric (separate type) → latency, token count, etc. with percentile aggregation
  ↓
Run (orchestrator)     → concurrent trials, statistical aggregation
  ↓
Comparison             → direction-aware significance testing
```

**Design principles validated by ecosystem survey**:
- The Dataset → Acquisition → Scorer → Score pipeline is industry consensus
- Statistical significance in eval results is the critical ecosystem gap
- OTel-native evaluation is an emerging pattern (AgentEvals, EvalForge)
- Single-score-per-scorer with explicit composition (ScorerSet) is cleaner than multi-score return

---

## 3. Library Analysis

### Evalkit: Strengths

| Capability | Evidence |
|-----------|---------|
| **Statistical rigor** | ~613 lines: Wilson CIs, Welch's t-test, Fisher's exact, effect sizes, significance classification |
| **OTel observe mode** | 533-line module for trace-based acquisition — score existing agent runs without re-execution |
| **Typed scores** | `Numeric(f64) \| Binary(bool) \| Label(String)` enables correct statistical dispatch |
| **Mapper/ScorerSet** | Novel abstraction for shared expensive transforms (parse once, score N ways) |
| **Spec-driven foundation** | 21 iterations of documented design decisions with conformance tracking |
| **Typestate builder** | Compile-time validation of evaluation configuration |

### Evalkit: Weaknesses

| Issue | Severity | Resolution |
|-------|----------|-----------|
| **No concurrent execution** | Critical | Migrate verda's `buffer_unordered` pattern (P0) |
| **Score::Metric variant** | High | Liskov Substitution violation — metrics need percentiles (p50/p95/p99), scores need CIs. Separate into standalone `Metric` type (P1) |
| **`Vec<Sample>` dataset** | Medium | Replace with trait supporting lazy loading and `trial_index` (P1) |
| **No sample tags** | Medium | Add `Vec<String>` for sliced analysis (P1) |
| **`AcquisitionError` leaks OTel** | Medium | `TraceNotFound` in non-OTel paths is an abstraction leak. Generalize errors (P1) |
| **`'static` bounds on builder** | Low | Type-erasure cost (not concurrency cost); document trade-off |

### Verda: Strengths

| Capability | Evidence |
|-----------|---------|
| **Concurrent execution** | Production-ready `buffer_unordered` pattern without `'static` bounds |
| **Score/Metric separation** | Correctly treats latency/tokens as fundamentally different from quality scores |
| **Dataset trait** | `sample(index, trial_index)` enables lazy loading and per-trial stochastic variation |
| **Sample tags** | First-class `Vec<String>` for sliced analysis |
| **Ergonomic API** | `Evaluation::new(dataset, task, scorers)` — simple, discoverable |
| **Edge-case test suite** | Duplicate IDs, conflicting goals, non-finite values, blank names, cross-trial ID inconsistency |

### Verda: Weaknesses

| Issue | Severity | Why Not Fixable |
|-------|----------|----------------|
| **f64-only scores** | Critical | Encoding categorical data as floats is a statistical category error; changing it reshapes the entire data model |
| **No tracing infrastructure** | Critical | One `tracing::warn!` call; OTel integration would require ground-up work |
| **No statistical significance** | Critical | Mean/stddev only; adding Wilson CIs, Welch's t, Fisher's exact reshapes comparison logic |
| **`Vec<Score>` multi-return** | Medium | Prevents 1:1 ScoreDefinition binding and build-time validation |
| **5-parameter generic** | Low | `Evaluation<I,T,O,D,F>` — `D` and `F` as generics infect type signatures for negligible gain |

### Alignment Summary

| Ideal Spec Requirement | Evalkit | Verda |
|----------------------|---------|-------|
| Typed score dispatch | ✅ | ❌ (f64 only) |
| Statistical significance | ✅ | ❌ (mean/stddev) |
| OTel trace observation | ✅ | ❌ |
| Concurrent execution | ❌ | ✅ |
| Dataset as trait | ❌ | ✅ |
| Score/Metric separation | ❌ | ✅ |
| Sample tags | ❌ | ✅ |
| Spec-driven design | ✅ | ❌ |
| **Overall alignment** | **~80%** | **~45%** |

The critical distinction: evalkit's gaps are additive (port capabilities in), while verda's gaps are structural (would require a rewrite).

---

## 4. User's Use Case: Brain Dump Agent (UC-5)

The brain dump agent — which segments and classifies thoughts, with the goal of improving classification accuracy — maps cleanly to the recommended architecture:

- **Dataset**: Thought segments with tags (`"work"`, `"personal"`, `"creative"`, etc.) loaded lazily via the Dataset trait
- **Acquisition**: Agent classifies each thought → returns predicted label
- **Scorer**: `Label(String)` score comparing predicted vs. expected category
- **Statistical analysis**: Chi-squared test for category distribution comparison across agent versions; per-tag slicing to identify which categories improve/regress
- **Comparison**: Direction-aware significance testing between prompt/model versions

This use case specifically motivated two migration items: the Dataset trait (lazy loading of potentially large thought collections) and sample tags (per-category analysis). It also validates the `Label` score type — encoding "work" as `0.0` and "personal" as `1.0` would prevent meaningful statistical analysis.

---

## 5. Migration Plan

### P0 — Blocks Production Use

**Concurrent Execution** (Medium effort: 200–400 LOC, 2–3 days)

Follow verda's cooperative `buffer_unordered` pattern rather than spawn-based concurrency, to avoid tightening `'static` bounds. The core challenge is adding `+ Send` bounds to `TrialFuture<'a>`, `AcquisitionFuture<'a, O>`, `ScoreFuture`, and all four `RunExecutor` variants. Build logs from evalkit's iteration-010 show this was previously attempted and abandoned, making a time-boxed spike essential before committing.

**Implementation sketch**:
1. Add `+ Send` to all future type aliases
2. Pre-materialize work items as `Vec<(sample_index, trial_index, sample)>`
3. `stream::iter(items).map(|item| execute_trial(item)).buffer_unordered(concurrency)`
4. Sort results by `(sample_index, trial_index)` for deterministic output

### P1 — High Value (1 week total, parallelizable)

| Item | Effort | What Changes |
|------|--------|-------------|
| **Dataset trait** | Low-Medium | Replace `Vec<Sample>` with `trait Dataset { fn len(&self) -> usize; fn sample(&self, index: usize, trial_index: usize) -> Sample; }` |
| **Sample tags** | Low | Add `tags: Vec<String>` to `Sample`; add `filter_by_tag()` to result types |
| **Score/Metric separation** | Medium | Extract `Score::Metric` into standalone `Metric` type with `MetricSummary` (p50/p95/p99); update acquisition output, trial results, run results, stats, comparison, serialization |
| **Generic acquisition errors** | Low-Medium | Replace concrete `AcquisitionError` with `E: Error + Send + Sync + 'static`; move OTel variants behind feature gate |

### P2 — Polish

- `#[non_exhaustive]` audit on all public enums (`AcquisitionError`, `Change`, `RunResult`, etc.)
- `thiserror` for error types
- `Display` implementations on `RunResult`, `RunStats`, `Comparison` — cheap to implement, disproportionate impact on first impressions

### P3 — Backlog

- Port verda's edge-case test suite (ideally *before* P1 to serve as regression tests during refactoring)

### What NOT to Port

- **`compare.rs`** — evalkit's comparison is strictly superior (significance testing, CIs, effect sizes, `Change::Incomparable`)
- **`Vec<Score>` multi-score return** — inferior to 1:1 ScoreDefinition binding + ScorerSet
- **f64-only score model** — prevents correct statistical dispatch
- **5-parameter generic `Evaluation<I,T,O,D,F>`** — premature optimization that infects type signatures

---

## 6. Lessons Learned

1. **Simplicity is a feature.** Verda's `Evaluation::new(dataset, task, scorers)` is more discoverable than evalkit's 4-phantom-type builder. Every future API addition should be weighed against ergonomic cost.

2. **Separate what you measure from what you judge.** Score/Metric conflation was evalkit's most significant design error. Latency percentiles and quality confidence intervals have fundamentally different aggregation semantics.

3. **Design for concurrency from day one.** The `Send` bound cascade is the tax for deferring it. Future trait additions should include `Send` bounds proactively.

4. **Tags beat general-purpose metadata for the common case.** `tags.contains("work")` is dramatically more ergonomic than `metadata.get("category").and_then(|v| v.as_str())`.

5. **Spec-driven design compounds.** Evalkit's 21 iterations of documented decisions and conformance tracking mean every future extension can be evaluated against the spec. Verda's ad-hoc design made analysis harder and left design rationale implicit.

---

## 7. Risks and Reversal Triggers

### Risks

| Risk | Severity | Mitigation |
|------|----------|-----------|
| **`Send` bound cascade intractable** | High | Time-boxed 1-day spike before any other migration work. If >1000 LOC or requires trait hierarchy redesign, re-evaluate starting from verda with evalkit's stats/OTel ported in. |
| **Zero real users** | Medium | Seek early adopters aggressively. Every design decision is theoretical until validated by usage. |
| **Rust edition compatibility** | Medium | Evalkit's native async traits require edition 2024+. If edition 2021 support is needed, `async_trait` must be reintroduced. Requires explicit decision before first public release. |
| **Metric separation is cross-cutting** | Medium | Touches acquisition output, trial results, run results, stats, comparison, and serialization. Second-hardest migration item after concurrency. |
| **Ecosystem moves fast** | Low | A well-funded eval platform could open-source a Rust SDK that changes the landscape. Evalkit's spec-driven foundation is its hedge — it can adapt because its rationale is documented. |

### Reversal Triggers

| Trigger | Action |
|---------|--------|
| Concurrency spike exceeds 1000 LOC or requires redesign | Re-evaluate: port evalkit's stats + OTel into verda's architecture |
| <5% typed score adoption after 12 months | Simplify `Score` to `f64`-only |
| <5% OTel observe mode adoption after 12 months | Consider removing `Acquisition` trait in favor of simpler closures |

---

## 8. Recommended Next Steps

1. **Decide minimum Rust edition** — This gates whether native async traits stay or `async_trait` returns. Must be resolved before any concurrency work.

2. **Spike concurrency** — 1-day time-boxed attempt to add `Send` bounds across evalkit's future types. Success/failure determines whether the recommendation holds.

3. **Port verda's edge-case tests into evalkit** — Before making structural changes, establish a regression safety net.

4. **Implement P0 (concurrent execution)** — Using verda's `buffer_unordered` pattern as reference.

5. **Implement P1 items in parallel** — Dataset trait, sample tags, Score/Metric separation, and generic acquisition errors are independent work streams.

6. **Write the 12 use cases as integration tests** — Each use case becomes a concrete test exercising the library API, serving as both validation and documentation.

7. **Archive verda** — Add a README documenting which capabilities were migrated and pointing to evalkit.

8. **Ship P1 + P2 as a coordinated release** — Users should not face multiple sequential breaking changes. Bundle them into one pre-1.0 release.

---

## 9. Open Questions

- **Exact concurrency approach**: Cooperative (`buffer_unordered`) vs. spawn-based. Strong lean toward cooperative, but the spike will confirm feasibility with type-erased executors.
- **Label statistics scope**: Agreed on distribution + mode + chi-squared comparison. Whether confusion matrices belong at the library level remains open (current consensus: no).
- **Builder `'static` relaxation**: Possible via GATs or lifetime-parameterized builders in a future release. Not blocking for initial release.
- **Benchmarking**: Neither library has performance benchmarks. Once concurrency lands, throughput measurements at various concurrency levels should validate the implementation.
