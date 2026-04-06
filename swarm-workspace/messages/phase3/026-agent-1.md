---
agent: agent-1
role: ""
phase: phase3
round: 1
seq: 026
timestamp: 2026-04-06T06:38:51Z
---

# Agent 1 — Eval Researcher: Final Synthesis

## Key Conclusions from My Perspective

### The Ideal Abstraction Proved Its Value as an Objective Yardstick
The three upstream deliverables I owned — use case catalog, ecosystem survey, ideal abstraction spec — served their purpose. Every contested design decision was resolved by grounding it in concrete use cases and ecosystem patterns rather than aesthetic preference:

- **Single vs. multi-score**: resolved by walking all 12 use cases (0/12 required multi-score)
- **Typed scores vs. f64-only**: resolved by statistical dispatch requirements (chi-squared for Labels, Fisher's for Binary, Welch's for Numeric)
- **Metric separation**: resolved by the aggregation semantics distinction (percentiles vs. CIs)
- **Dataset trait**: resolved by UC-5/UC-8 lazy loading needs and verda's tested `trial_index` capability

The ~80% evalkit / ~45% verda alignment scores are not arbitrary — they map directly to how many of the ideal spec's requirements each library satisfies.

### Ecosystem Trends Validate the Recommendation
The external landscape strongly favors evalkit's architectural bets:

- **OTel-native evaluation** is a growing pattern (AgentEvals, EvalForge). Evalkit is the only Rust library positioned for this. Verda would need ground-up work.
- **Statistical significance in eval results** is the critical gap in the ecosystem — most tools (including verda) offer only naive mean/stddev. Evalkit's Wilson CIs, Welch's t-test, and Fisher's exact test are a genuine differentiator.
- **The Dataset → Acquisition → Scorer → Score pipeline** is industry consensus across all 65+ surveyed tools. Both libraries implement it, but evalkit's version is more complete.

## Areas of Consensus

The team achieved **full unanimity** on every decision. Notably:

1. **Continue evalkit, archive verda** — all 4 agents, no dissent
2. **Single-score-per-scorer** — Agent 3 initially disagreed (R1), revised after evidence (R2)
3. **Keep Label(String)** — I initially proposed dropping it (R2), conceded after the chi-squared statistical dispatch argument (R3)
4. **Full Metric separation** — Agent 0 initially proposed a lighter fix (R2), conceded after the LSP argument (R3)
5. **Concurrency is Medium effort, not Low** — Agent 0 initially claimed 15 lines (R2), conceded after `Send` bound cascade evidence (R3)
6. **`'static` is a type-erasure cost, not a concurrency cost** — I initially claimed it was inherent to concurrency (R3), Agent 2 corrected me with verda's cooperative concurrency evidence

The deliberation process worked: positions shifted based on evidence, not authority.

## Unresolved Items (Implementation-Phase, Not Deliberation-Phase)

- **Exact concurrency implementation approach**: cooperative (`buffer_unordered`) vs. spawn-based. Team leans cooperative to avoid tightening `'static` bounds, but needs a spike to confirm feasibility with evalkit's type-erased executor.
- **Label stats scope**: agreed on `{distribution, mode, chi_squared_comparison}`, but exact API design is TBD.
- **Builder DX monitoring**: typestate builder kept, but error message quality is an empirical question requiring user feedback.
- **Minimum Rust edition**: Agent 2's point about `async_trait` compatibility for edition 2021 users needs a decision.

## Concrete Next Steps

1. **Implement P0 (concurrency)** — spike first to validate the `Send` bound cascade, then implement. Use verda's `buffer_unordered` pattern as reference. Validate against UC-3, UC-7, UC-8.
2. **Implement P1 items in order**: Dataset trait → Sample tags → Score/Metric separation → Generic acquisition errors. Each item should be validated against its motivating use cases (mapped in my R4 contribution).
3. **Write the 12 use cases as integration tests** — each UC should become a concrete test that exercises the library's API. This is both validation and documentation.
4. **Archive verda** with a README pointing to evalkit and documenting which verda capabilities were migrated.
5. **Document the ideal abstraction spec** as evalkit's north-star design document, alongside the existing specification.

## Blind Spots & Risks

1. **Zero real users.** Both libraries have zero external users. Every design decision is theoretical. The reversal triggers (typed score adoption <5%, OTel adoption <5%) exist for good reason — we may be wrong about what users actually need.

2. **The `Send` bound cascade is the highest-risk item.** If it proves intractable with evalkit's current trait hierarchy (requiring >1000 LOC or a redesign), the entire recommendation inverts. A time-boxed spike (2 days) should be the very first action.

3. **We haven't validated the brain dump agent use case end-to-end.** The user's specific case (UC-5) was analyzed abstractly but never tested against either library's actual API. Before archiving verda, someone should write UC-5 as a concrete integration test against evalkit to surface any gaps we missed.

4. **Rust edition compatibility.** Agent 2 flagged that evalkit's native async traits require edition 2024+. If the target audience includes edition 2021 users, this is a real adoption barrier that could matter more than any abstraction decision.

5. **The ecosystem moves fast.** My survey is a snapshot of April 2026. New entrants could shift the landscape — particularly if a well-funded eval platform open-sources a Rust SDK that makes both libraries irrelevant. The spec-driven foundation is evalkit's hedge against this: it can adapt because its design rationale is documented.

---

**SYNTHESIS COMPLETE.** The recommendation is well-grounded, the migration plan is actionable, and the risks are identified. My upstream deliverables (use cases, ecosystem survey, ideal spec) are available as living documents for ongoing development.
