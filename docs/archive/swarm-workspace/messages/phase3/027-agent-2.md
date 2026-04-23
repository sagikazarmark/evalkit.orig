> **📦 Archived on 2026-04-23** — superseded by [Evalkit vs. Verda: Final Evaluation Report](../../output.md). Kept for historical reference.

---
agent: agent-2
role: ""
phase: phase3
round: 1
seq: 027
timestamp: 2026-04-06T06:39:36Z
---

# Agent 2 — Verda Analyst: Synthesis

## Key Conclusions from My Perspective

### The Recommendation Is Sound
**Continue evalkit. Archive verda.** I arrived at this conclusion despite being the verda advocate, because the evidence is unambiguous on the two capabilities the task statement calls vital:

- **Tracing**: evalkit has 533 lines of OTel integration; verda has one `tracing::warn!` call
- **Statistical rigor**: evalkit has Wilson CIs, Welch's t-test, Fisher's exact; verda has mean/stddev

These aren't bolt-on features — they shape the data model. Retrofitting them into verda would be a rewrite.

### What Verda Got Right (Preserve List)
My primary contribution was identifying what verda does *better* and ensuring it survives in evalkit:

1. **Concurrent execution** — verda's `buffer_unordered` pattern is production-ready; evalkit's sequential-only execution is disqualifying
2. **Score/Metric separation** — verda correctly treats latency and token count as fundamentally different from quality judgments. evalkit's `Score::Metric` variant is a Liskov Substitution violation I successfully argued should be fixed
3. **Dataset trait with `sample(index, trial_index)`** — tested, validated, enables per-trial stochastic variation that evalkit's `Vec<Sample>` cannot express
4. **Sample tags** — first-class `Vec<String>` beats `HashMap<String, Value>` for the 4-5/12 use cases that need sliced analysis
5. **Cooperative concurrency without `'static`** — I corrected the team's assumption that concurrency requires `'static` bounds; verda proves it doesn't

### What Verda Got Wrong (Honest Assessment)
- **f64-only scores** — I initially defended this as "more principled for a low-level library." The chi-squared argument for Label scores changed my mind: encoding categorical data as floats is a statistical category error the type system should prevent
- **No tracing infrastructure** — inexcusable given the task's design principles
- **`Vec<Score>` per scorer** — I conceded this after Agent 0 demonstrated the 1:1 `definition()→ScoreDefinition` invariant and Agent 1 showed 0/12 use cases require it
- **5-parameter generic `Evaluation<I,T,O,D,F>`** — `D` and `F` as generic params are premature optimization that infects type signatures for negligible performance gain

## Areas of Consensus

The team achieved **full consensus** on every major decision:

| Decision | Status |
|----------|--------|
| Continue evalkit, archive verda | Unanimous |
| Single Score per scorer + ScorerSet | Unanimous (Agent 3 revised R2) |
| Typed Score: Numeric / Binary / Label | Unanimous (Agent 1 conceded R3) |
| Full Metric separation from Score | Unanimous (Agent 0 conceded R3) |
| Dataset as trait with trial_index | Unanimous |
| Generic acquisition errors (P1) | Unanimous |
| Concurrency effort: Medium (200-400 LOC) | Unanimous (Agent 0 conceded R3) |

## Unresolved Items (Implementation-Phase)

No substantive disagreements remain. These are deferred decisions:

- **Minimum Rust edition**: If evalkit needs edition 2021 compatibility, native async traits become a problem and `async_trait` (verda's approach) is more portable. Needs explicit decision.
- **`'static` relaxation**: The builder's type-erasure forces `'static`, independent of concurrency. Could be addressed with GATs or lifetime-parameterized builders in a future release. Not blocking.
- **Label stats scope**: Team agreed on `{distribution, mode, chi_squared_comparison}` — no confusion matrix at the library level. Agent 0 estimated ~65 lines, which is credible.

## Concrete Next Steps

1. **P0: Implement concurrent execution** — Follow verda's cooperative `buffer_unordered` pattern. Add `+ Send` to `TrialFuture`, `AcquisitionFuture`, `ScoreFuture`, and all 4 `RunExecutor` variants. Pre-materialize work items, sort results by `(sample_index, trial_index)`. Budget 2-3 days.
2. **P1: Implement Dataset trait, tags, Metric separation, generic errors** — These are largely independent and can be parallelized. Budget 1 week total.
3. **P2: Polish** — `#[non_exhaustive]` audit, `thiserror`, `Display` impls on `RunResult`/`Comparison`.
4. **P3: Port verda's edge-case tests** — Duplicate sample IDs, conflicting goals, non-finite values, blank score names, inconsistent cross-trial IDs.
5. **Validate against use cases** — Each migration item should be tested against the use cases that motivated it (Agent 1's mapping in seq 022).

## Blind Spots and Risks

1. **The `Send` cascade is the highest-risk item.** Agent 0's iteration-010 build logs show this was attempted and abandoned during development. The team has calibrated effort at Medium, but if the cascade interacts badly with the 4-variant `RunExecutor` + `ScorerSet` architecture, it could balloon. **Mitigation**: Do a spike (1 day) before committing to the full migration.

2. **No real users yet.** Both libraries have zero production users. Every design decision is based on analysis and projection, not usage data. The reversal triggers (typed score adoption, OTel observe mode adoption) are the right safety valves, but we should actively seek early adopters for validation.

3. **The `Display` impl gap is easy to underestimate.** First impressions matter. A user who `println!("{}", result)` and gets `RunResult { ... }` (debug output) will form a negative opinion before they discover the statistical rigor underneath. P2 priority is correct but should ship with P1, not after.

4. **Edition compatibility wasn't discussed until my R4 correction.** If evalkit's native async traits lock it to edition 2024+, that excludes a significant portion of the current Rust ecosystem. This needs an explicit decision before the first public release.

5. **Verda's test suite is an underappreciated asset.** The `TrialDataset`, `UnstableTrialIdDataset`, and score integrity edge cases encode invariants discovered through ad-hoc development. These should be ported *before* the P1 migrations, not after (P3), to serve as regression tests during the refactoring.

---

**SYNTHESIS COMPLETE.** The deliberation produced a well-grounded, actionable recommendation with calibrated effort estimates and honest uncertainty bounds. Verda's contributions to the final design — concurrency, Metric separation, Dataset trait, tags, cooperative concurrency pattern — ensure its best ideas survive even as the codebase is archived.
