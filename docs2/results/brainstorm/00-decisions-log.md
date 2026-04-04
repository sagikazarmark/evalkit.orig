# Brainstorm Decisions Log

## Session: 2026-04-03

---

## Clusters Formed

### Phase 1 — Six clusters identified:
1. **The Eval Kernel** — generic Rust eval library with layered architecture
2. **The Trace Grader** — OTel-native evaluation without execution
3. **The Confident Eval** — statistical rigor for non-deterministic evaluation
4. **The Provider-Neutral CLI** — fast, zero-dependency eval binary
5. **The Artifact Evaluator** — transform pipeline for non-text outputs
6. **The Self-Aware Evaluator** — self-instrumentation with OTel

### User selection:
Clusters 1-4 taken forward. Clusters 5-6 deferred as features, not directions.

**Rationale**: Clusters 5-6 are small-scope features (optional closure, feature gate) rather than strategic directions. The user agreed with the assessment.

---

## Scoping Decisions (Phase 2)

### What the user prioritized:
- **Success metric**: "I'm using it daily, I achieve measurable improvements in my AI agents" — personal productivity, not adoption
- **Primary priorities**: Core library + OTel (co-primary)
- **Architecture intent**: Foundation crate — layered, composable. "The low level part needs to be super stable."
- **Work capacity**: Full-time, potential to delegate

### What the user deprioritized:
- External adoption and market timing ("My own needs for now")
- Python bindings (deferred — building for Rust first)
- CLI (dropped from exploration — delivery mechanism, not foundation)
- Platform features (explicitly deferred since braindump)

### Key insight from scoping:
The "foundation crate" ambition is about **architecture**, not **social contract**. The user doesn't want to maintain a stable public API for strangers yet — they want the internal architecture to be layered and composable. This changes how much API polish to invest in v0.1.

### CLI dropped:
**Rationale**: User is building a foundation, not an end-user tool. CLI is a higher-level, more opinionated component. Success metric (daily use on own agents) is served by library mode and OTel mode, not CLI. Market timing (Promptfoo vacuum) doesn't matter since user isn't targeting the market.

---

## Key Reactions from Exploration (Phase 3)

### Cluster 1 (Kernel):
- User chose **Core variant** (async scorers, composition, transforms, multi-trial, comparison)
- No pushback on the architecture or type design approach
- The "muddy" feeling was later identified as terminology, not architecture

### Cluster 2 (Trace Grader):
- User chose **Core variant** (extensible extraction, multiple backends, trajectory extraction)
- **Important pushback**: "I'm not sure traceparent as a hard requirement is a good idea. It should be an implementation detail." → Correlation mechanism must be pluggable, not baked in. This is a significant design constraint.
- User acknowledged they "don't quite see yet how it's gonna work" — excited but uncertain on mechanics

### Cluster 3 (Confident Eval):
- User chose **Core variant** (Wilson CIs, significance testing, cost tracking)
- No pushback on the approach
- Acknowledged as "least exciting" but "very important correctness feature"

---

## Provocation Outcomes (Phase 4)

### What shifted thinking:

**Provocation 1 (design for OTel first)**: User rejected — "design principles shouldn't be tailored to OTEL." OTel influences but doesn't drive the core API. **This reinforced the generic core conviction.**

**Provocation 2 (bake CIs into score types)**: User was genuinely uncertain — "I don't know. It might make sense for numeric ones, but what about the rest?" **This remains an open design question for prototyping.**

**Provocation 5 (what's muddy?)**: User named it — "Case? Sample? scorer vs. grader vs. evaluator?" **This unlocked the realization that the muddiness is terminology, not architecture.** Terminology review is scheduled as a separate pipeline step.

**Provocation 6 (prototype in Python?)**: User rejected firmly — "Sounds wasteful." **Confirms Rust-only development path.**

**Provocation 7 (what if AI-agnostic is wrong?)**: User was pragmatic — "If that's the case, we can always build a higher level layer and just use that." **Confirms generic core is a bet with a known escape hatch, not a dogma.**

### What didn't shift:
- OTel conviction — absolute ("I wouldn't stop. OTel is the future.")
- Rust commitment — no wavering
- Generic core as design principle — reaffirmed with pragmatic fallback

### Revised ranking after provocations:
1. Trace Grader (moved up — deepest excitement and conviction)
2. Eval Kernel (stays essential — muddiness resolved as terminology)
3. Confident Eval (stays third — correctness feature, least exciting)

Build order remains Kernel first (dependency), but the project's identity shifted: "an OTel-native evaluation system built on a generic foundation" rather than "a generic eval library that also does OTel."

---

## Kill Criteria Agreed

| Direction | Kill Condition |
|-----------|---------------|
| Eval Kernel | Core API doesn't provide the features needed for real evaluation scenarios |
| Trace Grader | Never — "OTel is the future" (user won't kill this direction) |
| Confident Eval | Eval results don't help the user improve agents (statistical rigor doesn't change decisions) |

---

## New Ideas That Emerged During Session

- **Acquisition trait as the architectural seam**: The insight that the kernel should have an `Acquisition` trait separating "how to get the output" from "how to score it" emerged during Cluster 2 exploration. This is the key architectural concept enabling multi-mode support.
- **Correlation as pluggable strategy**: Emerged from user's pushback on traceparent. The `Correlator` trait concept was generated during the session, not present in the braindump.
- **SpanExtractor as the hard problem**: The exploration clarified that extracting structured output from OTel spans is the genuinely hard part of the trace grader — not correlation, not scoring. This wasn't surfaced in the braindump.
- **CI baking as design question**: Provocation 2 surfaced an unresolved design question — should statistical information be part of the Score type or a separate aggregation layer? The user didn't have an answer, leaving it for prototyping.
- **Project identity reframe**: The provocations revealed the project is "OTel-native eval on a generic foundation" not "generic eval library with OTel." This reframing was not in the braindump.
- **Scorer trait design spike**: Post-convergence, the user proposed implementing 2-3 variations of the Scorer trait and testing them against real workflows before committing. Variations to try: (1) fully generic `Scorer<I, O, R>`, (2) associated types, (3) type-erased with `serde_json::Value`. Test against: blueprint writer (text→text), Excalidraw generator (text→JSON+transform), GitHub issue agent (text→code+execution), prompt tuning (text→text). The Excalidraw workflow is the hardest stress test — if a variation handles that cleanly AND keeps text→text simple, it's the right design. This should be a formal design spike in the specification step.
