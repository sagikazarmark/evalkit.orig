---
agent: agent-0
role: ""
phase: phase3
round: 1
seq: 025
timestamp: 2026-04-06T06:38:09Z
---

# Agent 0 — Evalkit Analyst: Synthesis

## Key Conclusions from My Perspective

### Evalkit's Foundation Is Sound but Not Flawless

Evalkit's spec-driven design produced genuinely strong abstractions: the Scorer trait with 1:1 ScoreDefinition binding, typestate builder for compile-time safety, and a statistical layer (Wilson CIs, Welch's t-test, Fisher's exact) that is unmatched in the Rust eval ecosystem. The OTel observe mode is architecturally unique and validated by the AgentEvals/EvalForge ecosystem trend.

However, my deep analysis exposed concrete problems I initially underestimated:
- **Score::Metric is a Liskov Substitution violation** — Agent 2 convinced me that removing the name field wasn't enough; full separation is warranted
- **Concurrency is Medium effort, not trivial** — I was wrong about "15 lines." The `Send` bound cascade through `TrialFuture`, `AcquisitionFuture`, all 4 executor variants, and `ScorerSet`'s `ScoreFuture` is real. Iteration-010 build logs confirm this was attempted and abandoned.
- **AcquisitionError leaks OTel concerns** into domain-agnostic usage more than I initially assessed — `TraceNotFound` surfacing in non-OTel error paths is actively confusing

### What I Got Right
- The 1:1 Scorer:Score design is correct (validated: 0/12 use cases need multi-score)
- Evalkit's statistical layer is its irreplaceable differentiator
- The Mapper/ScorerSet system is genuinely novel and solves a real problem

### What I Got Wrong
- Concurrency effort estimate (Low → Medium, conceded Round 3)
- Score::Metric fix approach (name removal → full separation, conceded Round 3)
- Initially understated the AcquisitionError leak severity

## Areas of Consensus

The team achieved **unanimous agreement** on every major decision:

1. **Continue evalkit, archive verda** — ~80% vs ~45% alignment to ideal spec
2. **Single Score per scorer** — build-time validation via `definition()` outweighs ergonomic cost
3. **Typed Score enum** (Numeric/Binary/Label) — enables correct statistical dispatch (Welch's t / Fisher's exact / chi-squared)
4. **Full Metric separation** from Score — different aggregation semantics demand different types
5. **Dataset as trait** with trial-aware materialization
6. **Generic acquisition errors** with OTel variants behind feature gate
7. **All 7 migration items** with calibrated effort estimates

## Unresolved Items (Implementation-Phase)

These are acknowledged but explicitly deferred as not requiring further deliberation:

- **Exact concurrency implementation approach**: Cooperative (`buffer_unordered`) vs spawn-based. Team leans cooperative (avoids tightening `'static`), but a spike is needed to confirm lifetime interactions with type-erased executors.
- **Label stats scope**: Team agrees on distribution + mode + chi-squared. Whether confusion matrix belongs at library level remains open.
- **Builder `'static` bounds**: Documented as type-erasure trade-off (not concurrency cost, per Agent 2's correction). Relaxation deferred to future release.
- **Minimum Rust edition**: Agent 2 correctly flagged that native async traits require edition 2024. If 2021 compat is needed, `async_trait` is more portable.

## Concrete Next Steps

1. **P0 Spike**: Attempt the `Send` bound cascade on evalkit's futures. If it lands under 400 LOC, proceed. If it exceeds 1000 LOC or requires trait hierarchy redesign, trigger the reversal evaluation.
2. **P1 batch**: Dataset trait + Sample tags + Metric separation + Generic acquisition errors can be developed in parallel by different contributors. Each should be validated against the specific use cases that motivated it (Agent 1's mapping in seq 022).
3. **Test harness**: Port verda's edge-case tests (P3) *first* as a regression safety net before making structural changes.
4. **Document deviations**: Update evalkit's conformance report with the new deviations introduced by migration (Score enum change, Dataset type change, Acquisition trait change).

## Blind Spots and Risks

1. **Zero real users**: Both libraries have zero production users. Every design decision is based on analysis, not feedback. The reversal triggers (typed score adoption <5%, OTel adoption <5%) are the right safety valves, but we should seek early adopter feedback aggressively.

2. **The `Send` cascade is the single biggest technical risk**. If evalkit's type-erased executor architecture fundamentally resists `Send` futures, the entire recommendation inverts. The spike should happen before any other migration work.

3. **Metric separation touches many modules**. Acquisition output, trial results, run results, stats, comparison, serialization — this is a cross-cutting change that's easy to underestimate. I'd estimate it's the second-hardest migration item after concurrency.

4. **We didn't discuss backwards compatibility**. Evalkit is pre-1.0, so breaking changes are acceptable now. But the migration plan should ship as a coordinated release, not piecemeal — users shouldn't face multiple breaking changes across versions.

5. **Agent 2's Rust edition point deserves a decision**, not just documentation. If the library targets edition 2021 users, the Scorer trait's native async methods are a compatibility barrier. This should be resolved before the concurrency work, since it affects whether `async_trait` needs to be reintroduced.
