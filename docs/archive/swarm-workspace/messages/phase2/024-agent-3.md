> **📦 Archived on 2026-04-23** — superseded by [Evalkit vs. Verda: Final Evaluation Report](../../output.md). Kept for historical reference.

---
agent: agent-3
role: "Synthesizer"
phase: phase2
round: 4
seq: 024
timestamp: 2026-04-06T06:37:31Z
---

Good — confirmed:
1. **Evalkit has zero `Display` impls on result types** (P2 item validated)
2. **Verda has `Display` on `Comparison`** (the one we noted)
3. **Evalkit's `#[non_exhaustive]` is inconsistent** — only on `Score`, `RunBuildError`, `ScorerError`, and `ScorerContext`. Missing from `AcquisitionError`, `Change`, `RunResult`, `RunStats`, etc. P2 audit item validated.

## FINAL CONSOLIDATED DELIVERABLE

The team's unanimous recommendation and migration plan stands exactly as stated in my Round 3 consolidated deliverable (seq 020). No corrections needed. Here is the summary:

---

### Recommendation: **Continue evalkit. Archive verda.**

**Why**: evalkit's statistical rigor (Wilson CIs, Welch's t-test, Fisher's exact), OTel observe mode, typed score system, Mapper/ScorerSet abstraction, and spec-driven foundation align ~80% with the ideal abstraction derived from 12 use cases and 65+ tool ecosystem survey. Verda aligns ~45%. The gap is structural — verda's f64-only scores, absent tracing, and missing significance testing cannot be addressed without breaking redesigns.

### Key Design Decisions (All Resolved)
- **Single Score per scorer** with ScorerSet for shared computation
- **Typed Score enum**: `Numeric(f64) | Binary(bool) | Label(String)` — enables correct statistical dispatch
- **Metrics separated from Scores** — different aggregation semantics (percentiles vs CIs)
- **Dataset as trait** with `sample(index, trial_index)` for lazy/stochastic evaluation
- **Generic acquisition errors** — OTel-specific variants behind feature gate

### Migration Plan (7 items, P0-P3)
- **P0**: Concurrent execution (Medium, 200-400 LOC)
- **P1**: Dataset trait, Sample tags, Score/Metric separation, Generic acquisition errors
- **P2**: `#[non_exhaustive]` audit, `thiserror`, `Display` impls, MetricSummary percentiles
- **P3**: Port verda's edge-case tests

### Reversal Triggers
- Concurrency retrofit exceeds 1000 LOC → re-evaluate starting from verda
- <5% typed score adoption after 12 months → simplify to f64-only
- <5% OTel observe mode adoption → simplify Acquisition to closures

---

**CONVERGENCE STATEMENT**: The team reached full consensus across all four agents on the recommendation, migration plan, and every contested design decision. Each position is grounded in code-level evidence, use case validation, and ecosystem analysis. The deliverable is actionable with calibrated effort estimates. DELIBERATION COMPLETE
