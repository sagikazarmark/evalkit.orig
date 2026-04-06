# Brainstorm Session Transcript

**Date**: 2026-04-03
**Domain**: AI Evaluation
**Input**: Research corpus (9 streams + synthesis), Braindump corpus (5 documents + synthesis)

---

## Setup

The session began with `/resume` pointing to `@brainstorm.md`. All research and braindump documents were read before starting Phase 1.

**Documents read:**
- Research: 01-landscape.md, 02-domain-language.md, 03-user-workflows.md, 04-architecture.md, 05-pain-points.md, 06-ecosystem.md, 07-trajectory.md, 08-community.md, 09-failure-archaeology.md, synthesis.md
- Braindump: 00-record.md, 01-cross-reference.md, 02-discoveries.md, synthesis.md, convo.md

---

## Phase 1 — Connection Map

### Six clusters identified:

**Cluster 1: "The Eval Kernel"**
Core thesis: Evaluation is a generic computing primitive that deserves a proper library — the way `serde` is to serialization, this is to evaluation.

What feeds it: I-01 (generic Rust library), I-04 (flexible scoring), T-01 (AI-agnostic API), T-02 (Rust), H-01 (evaluation is generic), Discovery: scorer composition algebra, schema-validated scoring, pairwise comparison falls out naturally from generic scorer trait, Cross-reference: no Rust eval framework exists, Research: "evaluation-as-code" trend, Stream 9 validates framework flexibility wins. Size: 8 distinct ideas/opportunities. Tension: Generic purity vs. AI ergonomics — "trait bound soup" risk. Failure echo: Stream 9 Approach 1 (metrics-only library without platform failed) — but different, user is building a framework with extensibility.

**Cluster 2: "The Trace Grader"**
Core thesis: The best way to evaluate agents is to not execute them yourself — just grade their traces.

What feeds it: I-03 (OTel observation mode), I-02 (multi-mode), T-04 (OTel integration), A-03 (zero-code-change correlation via traceparent), Discovery #4 (evaluate agents you don't own), Discovery: eval replay, sidecar deployment, agentevals-dev as inspiration, Research Gap 4, Stream 6/7 on OTel standards. Size: 7 distinct ideas/opportunities. Tension: OTel insufficient for artifact-output agents, conventions still evolving. Failure echo: None — no OTel eval tool has failed.

**Cluster 3: "The Confident Eval"**
Core thesis: Every evaluation result should come with a confidence interval, not just a number.

What feeds it: I-05 (result comparison), Research Gap 1 (statistical rigor — CRITICAL), only Agentrial at 16 stars, Anthropic eval guide recommends multi-trial, Anti-Pattern 4 (single-trial evaluation), Discovery: serializable results enable "git for eval results". Size: 5 distinct ideas/opportunities. Tension: Do users actually want statistical rigor or just averages? Failure echo: None.

**Cluster 4: "The Provider-Neutral CLI"**
Core thesis: A fast, zero-dependency eval binary that fills the gap left by Promptfoo's OpenAI acquisition.

What feeds it: I-02 (binary mode), I-07 (multi-language via Rust), T-03 (binary+JSON protocol), Discovery: "cargo test for evals", Promptfoo vacuum. Size: 5 distinct ideas/opportunities. Tension: Configuration format design, needs built-in scorers to be useful. Failure echo: Barrier 3 "eval as feature."

**Cluster 5: "The Artifact Evaluator"**
Core thesis: Real agent evaluation often requires evaluating artifacts — rendered images, compiled code — not just text.

What feeds it: I-06 (transform pipeline), user's Excalidraw/GitHub issue use cases, Discovery #3 (transform pattern is universal). Size: 3 ideas. Tension: How often are transforms actually needed?

**Cluster 6: "The Self-Aware Evaluator"**
Core thesis: An eval framework that instruments itself with OTel enables cost tracking, profiling, and meta-evaluation automatically.

What feeds it: Discovery #1, Stream 5 #3 (eval cost pain point). Size: 3 ideas. Tension: Engineering elegance, not user demand.

### Orphan ideas noted:
- Unified quality + safety (user never mentioned safety/red-teaming)
- Error analysis → eval design pipeline (separate product type)
- Non-technical stakeholder eval (user building for developers)
- Deterministic LLM mocking (architecturally separate)
- Production feedback loops (requires platform, deferred)

### Assessment:
Clusters 1-4 recommended for exploration. Clusters 5-6 deferred as features, not directions.

---

> **User**: I agree with your recommendation, proceed

---

## Phase 2 — Scope

### Questions asked:

1. What does success look like in 3 months? In a year?
2. Is the core library the primary project, with OTel/stats/CLI as features?
3. What would make you look back and say this was a waste of time?
4. If you could only build ONE cluster, which one?
5. Working full-time, evenings/weekends, or in bursts?
6. Foundation crate that others build on, or a complete end-user tool?
7. Does the Promptfoo vacuum urgency matter to you?

### User answers:

1. "I'm using it daily, I achieve measurable improvements in my AI agents."
2. Since there are dependencies between them, even if they are equally important, the core library needs to be done first. Core library and OTEL are primary.
3. The project is designed to be flexible on purpose to support multiple use cases. If none or many of those use cases won't work, that flexibility would be useless.
4. "hm...kernel or the confident eval?"
5. Full time, might even be able to delegate to people.
6. Primarily a foundation. The low level part needs to be super stable. High level components are always more opinionated.
7. "My own needs for now"

### Revised shortlist:

Cluster 4 (CLI) dropped. Rationale: user is building a foundation crate, not an end-user tool. CLI is a higher-level, more opinionated component. Success metric (daily use on own agents) is served by library and OTel modes. Market timing doesn't matter.

Three remaining clusters have a natural dependency chain: Kernel first (dependency), then Trace Grader and Confident Eval (independent of each other, both depend on kernel).

---

> **User**: yes

---

## Phase 3 — Explore

### Cluster 1: The Eval Kernel

**3a — How It Works**: Rust crate providing typed evaluation primitives. `cargo add` it, import the scorer trait, write a scorer, run evaluations in-process. Core abstractions: Sample (input + optional reference), Scorer (trait: input + output + reference? → Score), Score (enum: Continuous, Binary, Label, Metric). Five lines to first evaluation. Concrete scenario: Mark changes blueprint-writer system prompt, runs 30 samples with two scorers (section coverage + LLM quality judge), gets results in 40 seconds.

**3b — What Makes It Hard**: Designing Scorer trait bounds in Rust — generic enough for arbitrary types but ergonomic for common cases. Generic `Scorer<I, O, R>` vs. trait objects `dyn Scorer`. Complexity multipliers: async scorers (LLM-as-judge needs network), serialization constraints, error handling (crash vs. low score). External deps: serde, tokio — both stable.

**3c — What Makes It Worth It**: Addresses fragmented tooling, lock-in concerns, user's personal pain. Delta: only Rust eval framework, only generic-core framework in any language, composable scorer algebra exists nowhere else. Strong compounding effect across user's diverse projects.

**3d — Failure Cross-Check**: Stream 9 Approach 1 (metrics-only library failed) — different because user is building a framework, not just metrics. RAGAS succeeded as a foundation (13.2k stars). Barrier 4 (metric commoditization) — differentiation is framework, not scorers. Barrier 5 (OSS sustainability) — not relevant yet, personal tool.

**3e — Distribution Sketch**: First 100 users via r/rust, Rust AI community, crates.io. Adoption trigger: "I need to evaluate my Rust AI agent and there's nothing." Pull for Rust AI niche, push for broader market.

**3f — Validation & Variants**: Cheapest test: write Scorer trait + 3 scorers + 3 evaluations (1 day). Variants: Minimal (core types, inline mode, 5 scorers), Core (+ async, composition, transforms, multi-trial, comparison), Full (+ OTel, Python bindings, CLI, config format).

---

> **User**: I think Core should be the initial goal.

---

### Cluster 2: The Trace Grader

**3a — How It Works**: Feature gate or companion crate adding OTel-based acquisition. Point at traces instead of calling functions. Two new abstractions: Acquisition (trait abstracting how outputs are obtained — inline vs. observe), SpanExtractor (trait extracting "output" from span tree). Concrete scenario: Mark points trace grader at last week's Jaeger traces, runs a new scorer against 50 historical runs in 3 seconds, discovers 60% of blueprints were missing risk assessments. Time-travel evaluation without re-execution cost.

**3b — What Makes It Hard**: Extracting structured output from raw OTel spans — which span IS "the agent's output"? GenAI conventions still evolving. Partially unknown — traceparent propagation needs end-to-end testing. Complexity: different trace backends, async trace collection, partial traces, non-HTTP agents.

**3c — What Makes It Worth It**: Eliminates re-execution cost (10-50x savings). Framework-agnostic via OTel. Delta over agentevals-dev: Rust performance, pluggable correlation (not locked to one strategy), extensible extraction, broader framework, persistent results.

**3d — Failure Cross-Check**: Barrier 1 risk (platform creep) — mitigated by user's explicit deferral of platform features. No OTel eval tool has failed.

**3e — Distribution Sketch**: OTel/CNCF community channels + blog "How I evaluate AI agents without re-running them." Mostly push — concept needs explaining.

**3f — Validation & Variants**: Cheapest test: OTel-instrumented HTTP agent + traceparent from Rust + query Jaeger (1 day). Variants: Minimal (one correlation, one backend, hardcoded extractor), Core (extensible extraction, multiple backends, trajectory), Full (+ embedded receiver, sidecar mode, eval replay).

---

> **User**: I'm not sure traceparent as a hard requirement is a good idea. It should be an implementation detail IMO.
>
> Once again, Core seems like the way to go for the first version

**Response**: Agreed — traceparent is one correlation strategy, not THE strategy. The abstraction should be a Correlator trait with multiple implementations. Core variant noted for Cluster 2.

---

### Cluster 3: The Confident Eval

**3a — How It Works**: Feature of Run builder — `.trials(10).confidence(0.95)`. Executes N times per sample, reports with statistical aggregation. Two additions: Trial (one execution of acquisition + scoring), Statistical aggregation (mean, CI, pass@k). Minimal API change, fundamental output quality change. Concrete scenario: Mark changes retrieval logic, sees quality drop 0.82 → 0.78 but confidence intervals overlap — "difference not significant at p < 0.05." Runs more trials to confirm.

**3b — What Makes It Hard**: Not the statistics (textbook math). Hard parts: UX of statistical output (developers don't think in CIs), cost multiplication (10 trials = 10x LLM cost), result comparison with CIs (statistical hypothesis testing). Known-hard, not unknown-hard.

**3c — What Makes It Worth It**: Addresses non-determinism (#2 pain point), statistical rigor gap (#7), anti-pattern #4 (single-trial). Only Agentrial (16 stars) in entire 65+ project landscape. The single most differentiated feature available.

**3d — Failure Cross-Check**: Nothing like this has been attempted and failed. Barrier 4 (metric commoditization) doesn't apply — this is methodology, not metrics.

**3e — Distribution Sketch**: Content-driven — "Why your AI eval scores are meaningless." Push — most teams don't know they have a statistics problem.

**3f — Validation & Variants**: Cheapest test: run same eval 10 times, plot distribution (1 hour). Variants: Minimal (trials + mean/std + pass@k), Core (+ Wilson CIs, significance testing, comparison, cost tracking), Full (+ adaptive trials, CUSUM drift, Fisher exact tests).

---

> **User**: Let's go with Core again

---

## Phase 4 — Provoke

### Provocations asked:

**Combination provocations:**
1. What if the core kernel exists primarily to serve OTel? Design observation mode first, extract generic primitives from what it needs?
2. What if statistical rigor is baked into Score types from the start? Every Score carries sample count and optional CI.

**Scale provocations:**
3. If you had 2-3 people, what would you parallelize?
4. The "foundation crate" ambition — is that about architecture or social contract?

**Inversion provocations:**
5. What specifically is muddy about the core?
6. What if Rust is wrong? Prototype in Python, ship in Rust?
7. What if AI-agnostic is wrong? When would you use this for something that isn't AI?

**Kill criteria** requested for each direction.

### User answers:

1. "No, design principles shouldn't be tailored to OTEL. OTEL may influence things, but the API should be independent: OTEL is not the only mode."
2. "Honestly, I don't know. It might make sense for numeric ones, but what about the rest?"
3. "I don't think I can parallelize in the beginning. We are building foundations. I might be able to get multiple people to try this while we are building it in different products."
4. "For now, foundation is definitely about architecture. I'm the only user. Once we expand, it may become a social contract."
5. "Case? Sample? scorer vs. grader vs. evaluator?"
6. "Sounds wasteful"
7. "If that's the case, we can always build a higher level layer and just use that."

Kill criteria:
- Cluster 1: The core API is ready, but doesn't work as expected (doesn't provide the features)
- Cluster 2: "I wouldn't. OTEL is the future."
- Cluster 3: Eval results don't help me improve agents

### Provocation Synthesis:

**What shifted:**
- Terminology muddiness was named — it's "Case? Sample? scorer vs. grader vs. evaluator?" — a naming problem, not architecture
- OTel conviction revealed as absolute — "I wouldn't stop. OTel is the future." Strongest signal in the conversation.
- Generic core reaffirmed with pragmatic escape hatch — "we can always build a higher level layer"
- Python prototyping firmly rejected — "Sounds wasteful"

**What didn't shift:**
- OTel conviction, Rust commitment, generic core as design principle

**Revised ranking:**
1. Trace Grader (moved up — deepest excitement and conviction)
2. Eval Kernel (essential foundation, muddiness resolved)
3. Confident Eval (stays third — correctness feature, least exciting)

**Emerging identity:** "An OTel-native evaluation system built on a generic foundation" — not "a generic eval library that also does OTel."

---

> **User**: I agree with the synthesis. The user also confirmed that terminology review is a separate pipeline step (domain.md), not something to settle in the brainstorm.

---

## Phase 5 — Converge

### Final Deliverables Written

Two files produced:

**brainstorm/directions.md** — Full direction documents for all three clusters:
- Direction 1: The Eval Kernel — Core variant (async, composition, transforms, multi-trial, comparison)
- Direction 2: The Trace Grader — Core variant (extensible extraction, multiple backends, trajectory, pluggable correlation)
- Direction 3: The Confident Eval — Core variant (Wilson CIs, significance testing, comparison, cost tracking)

Including: Non-goals, Goals, Validation plans, MVP scopes, Growth paths, Core technical decisions, User segments, Distribution, Competitive position, Sustainability, Risks, Kill criteria, Open questions, Connections, Go/No-Go cross-check, Dependency map, Recommended build order, Strategic notes, Domain triggers to watch.

**brainstorm/00-decisions-log.md** — All decisions, reactions, and new ideas that emerged during the session.

### Recommended Build Order
1. Eval Kernel (dependency — everything builds on it)
2. Trace Grader (user's deepest excitement and conviction)
3. Confident Eval (important correctness feature, can parallelize with #2)

---

## Post-Convergence Discussion

### Scorer trait design spike

After convergence, the user proposed implementing multiple variations of the Scorer trait and testing them against different workflows before committing to a design.

> **User**: I wonder if it would make sense to implement different variations of it and try them in different workflows.

**Response**: Agreed — the Scorer trait is the highest-risk decision. Three variations proposed:
1. Fully generic — `Scorer<I, O, R>` — maximum flexibility, trait bound risk
2. Associated types — `trait Scorer { type Input; type Output; type Ref; }` — cleaner per-scorer, harder to compose
3. Type-erased — `Scorer` with `serde_json::Value` — simple API, loses type checking

Test against four real workflows:
- Blueprint writer (text → text, LLM-as-judge)
- Excalidraw generator (text → JSON, needs transform) — hardest stress test
- GitHub issue agent (text → code, needs execution)
- Prompt tuning (text → text, exact match)

Success criterion: one variation handles all 4 workflows with <10 lines per scorer and no trait bound gymnastics.

This was saved to both directions.md (updated validation plan) and 00-decisions-log.md (as emerged idea).

---

> **User**: Does it make sense to save this somewhere?

Both files updated with the design spike approach.

---

## Key Takeaways

1. **The project is "OTel-native eval on a generic foundation"** — the generic core matters architecturally, but OTel is the soul
2. **Build order is Kernel → Trace Grader → Confident Eval** — dependency-driven, with 2 and 3 parallelizable
3. **All three directions are Core variant** — async, composition, extensible extraction, statistical rigor
4. **Terminology is explicitly deferred** to a dedicated review step (domain.md)
5. **The Scorer trait design is the single highest-risk decision** — validate with a design spike before committing
6. **Success = "I use it daily and measurably improve my agents"** — not adoption, not community, not revenue
7. **OTel conviction is absolute** — the user won't kill this direction under any conditions
8. **Correlation must be pluggable** — traceparent is one strategy, not baked into the API
9. **Foundation = architecture, not social contract (for now)** — layered and composable, but no external stability promises yet
10. **Generic core is a bet with a known escape hatch** — "we can always build a higher level layer"
