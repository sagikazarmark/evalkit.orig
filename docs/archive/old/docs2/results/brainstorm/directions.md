> **📦 Archived on 2026-04-23** — superseded by [Project Directions](../../../docs/brainstorm/directions.md). Kept for historical reference.

# Project Directions

## Session Context
- **Domain**: AI Evaluation — tools, frameworks, and platforms for evaluating AI model outputs, agent behavior, correctness, and performance
- **Date**: 2026-04-03
- **Clusters explored**: The Eval Kernel, The Trace Grader, The Confident Eval
- **Clusters set aside**:
  - The Provider-Neutral CLI — delivery mechanism, not a foundation; build later as a higher-level component
  - The Artifact Evaluator — real need but small scope; an optional closure on the run builder, not a direction
  - The Self-Aware Evaluator — engineering elegance, not user demand; add as a feature gate once OTel integration exists

---

## Direction 1: The Eval Kernel

### What
A Rust crate providing typed evaluation primitives — Sample, Scorer trait, Score types, Result, Run — that are domain-agnostic at the core with AI-specific convenience layers on top. The foundation that all other directions build on. No equivalent exists in Rust, and no existing tool in any language takes a generic-core approach.

### Non-Goals
- NOT a platform — no dashboards, no cloud service, no user accounts
- NOT an observability tool — does not collect traces, does not replace Langfuse/Jaeger
- NOT a metrics catalog — differentiation is the framework, not pre-built scorers
- NOT a Python library — Rust-first; multi-language bindings are a future layer
- NOT a test framework — does not replace cargo test or pytest; complements them

### Goals
- Core types (Sample, Scorer, Score, Result, Run) that express any evaluation scenario without domain-specific assumptions
- Async scorer support for network-bound scorers (LLM-as-judge)
- Scorer composition operators (and, weighted, then) for declarative evaluation logic
- Optional transform step between output acquisition and scoring (for artifact evaluation)
- JSONL serialization for all result types
- 5-10 built-in scorers: exact match, contains, regex, JSON schema validation, LLM-as-judge
- Multi-trial runner with statistical aggregation (see Direction 3)
- Run-to-run result comparison from serialized JSONL files
- API design that doesn't prevent future PyO3 wrapping (avoid complex lifetimes in public API)

### Validation Plan
- **Cheapest test**: Implement 2-3 variations of the Scorer trait design and test each against real workflows. Variations: (1) fully generic `Scorer<I, O, R>`, (2) associated types, (3) type-erased with `serde_json::Value`. Test against: blueprint writer (text→text, LLM-as-judge), Excalidraw generator (text→JSON, needs transform), GitHub issue agent (text→code, needs execution), prompt tuning (text→text, exact match). **1-2 days.**
- **What it proves**: Which trait design balances genericity with ergonomics. The Excalidraw workflow is the hardest stress test — non-trivial output type plus transform step. If a variation handles that cleanly AND keeps text→text simple, it's the right design.
- **Success signal**: One variation handles all 4 workflows with <10 lines per scorer and no trait bound gymnastics
- **Failure signal**: Every variation requires awkward workarounds for at least one workflow, suggesting the core abstraction needs rethinking

### MVP Scope (Week 1)
- Sample, Scorer trait, Score enum (Continuous, Binary, Label, Metric), Result, Run
- Inline execution mode (call a function, score the output)
- Async scorer support
- exact_match, contains, regex scorers
- JSONL result output
- **OUT of MVP**: Scorer composition, transforms, multi-trial, comparison, LLM-as-judge scorer, OTel anything

### Growth Path
1. **Scorer composition** (and, weighted, then operators) — unlocks declarative evaluation logic — small
2. **LLM-as-judge scorer** — the minimum AI-specific scorer demonstrating the layered architecture — small
3. **Optional transform step** — enables artifact evaluation (Excalidraw, code gen) — small
4. **Multi-trial runner + statistical aggregation** (Direction 3) — the highest-leverage differentiator — medium
5. **Run-to-run comparison** from JSONL files — enables CI/CD gating and regression detection — small
6. **OTel observation mode** (Direction 2) — adds trace-based acquisition — medium
7. **Additional built-in scorers** (JSON schema, embedding similarity) — broadens immediate utility — small
8. **Python bindings via PyO3** — opens adoption beyond Rust — large

### Core Technical Decisions
- **Generic vs. concrete scorer trait**: Use generic trait `Scorer<I, O, R>` with AI-specific type aliases and convenience functions in a separate module. Reasoning: generic enables non-AI use and composability; aliases prevent verbosity for the common case.
- **Async**: All scorers async by default (sync scorers wrapped trivially). Reasoning: LLM-as-judge is the most important scorer class and it requires network calls. Making sync the default and async the exception creates two worlds.
- **Error handling in scorers**: Scorer returns `Result<Score, ScorerError>`, not `Score`. A scorer that fails (network error, parse error) is different from a scorer that returns a low score. Reasoning: conflating errors with low scores hides infrastructure problems.
- **Score type system**: Score is an enum (Continuous, Binary, Label, Metric), not a trait. Reasoning: finite set of score types enables exhaustive matching, serialization, and aggregation without trait object complexity.
- **Crate structure**: Single crate with feature gates (e.g., `otel`, `llm-judge`) vs. multiple crates. Recommend: start as single crate, split if compilation time or dependency weight justifies it.

### User Segments Served
- **Primary (beachhead)**: The user — a Rust developer building diverse AI agents who needs flexible, non-opinionated evaluation
- **Secondary**: Rust AI developer community (small but growing, completely underserved)
- **Long-term**: Anyone building evaluation tooling who wants a typed foundation crate to build on

### Distribution
- **First 100 users**: crates.io publication + r/rust post + Rust AI community channels + blog post showing real agent evaluation
- **Adoption trigger**: "I need to evaluate my Rust AI agent and there's nothing on crates.io"
- **Push vs. pull**: Pull for Rust AI niche (they're searching); push for broader eval market (they don't know they want Rust)

### Competitive Position
- **Replaces**: Ad-hoc scoring code in test suites; no direct tool replacement
- **Delta**: Only Rust eval framework. Only generic-core eval framework in any language. Composable scorer algebra exists nowhere else. Type-safe evaluation with compile-time guarantees.
- **Defensibility**: Rust + generic + composable requires unusual skill set intersection. Architecture is non-trivial to replicate. BUT: a Python-native tool could replicate the architecture with better Python ergonomics — the Rust advantage is performance and type safety, not the architecture itself.

### Sustainability
- **Model**: Open source side project initially. No revenue intent in the near term.
- **Cost structure**: Zero — no hosting, no infrastructure, no dependencies beyond Rust ecosystem
- **If commercial later**: Foundation crate stays open. Higher-level tools (CLI, platform, managed evaluation service) could be commercial.

### Risks
- **Generic types too verbose for common case** — Likelihood: medium — Mitigation: AI-specific type aliases and convenience functions; prototype early to validate ergonomics
- **Async complexity in scorer trait** — Likelihood: low — Mitigation: async traits are stabilizing in Rust; use `async_trait` crate as fallback
- **Nobody else uses it** — Likelihood: medium — Mitigation: user is the first customer and has multiple agents to evaluate; external adoption is a bonus, not a requirement
- **API design locks in wrong abstractions** — Likelihood: medium — Mitigation: keep the public API minimal; fewer types in v0.1 is better than more

### Kill Criteria
- The core API is built but doesn't provide the features needed for real evaluation scenarios (the user's own agents)
- Writing new evals feels like a chore — the anti-goal from the braindump
- The generic abstraction forces contortions for AI use cases that a direct AI-specific API wouldn't require, AND the higher-level layer workaround is insufficient

### Open Questions
- **Terminology**: Sample vs. Case vs. Example? Scorer vs. Grader vs. Evaluator? — Dedicated terminology review is scheduled as a separate pipeline step (domain.md)
- **Crate naming**: Generic name (evalkit, scored) vs. domain-leaning name (agenteval)?
- **Should Score::Error be a variant or should scorers return Result<Score, E>?**
- **How should scorer metadata (timing, cost) be attached to results?**
- **Should the Run own the execution (call the function) or accept pre-computed outputs?** Both patterns have uses — inline mode needs the former, observe mode needs the latter.

### Connections
- **Shares foundation with**: Direction 2 (Trace Grader), Direction 3 (Confident Eval)
- **Competes with**: Nothing — this is the foundation everything else depends on
- **Enables**: Direction 2, Direction 3, and all deferred directions (CLI, Python bindings, platform)

---

## Direction 2: The Trace Grader

### What
An OTel-based acquisition mode for the eval kernel. Instead of calling an agent function directly, point it at OTel traces — send a request with correlation context, collect traces from a backend, extract structured output from spans, and score it. Evaluate agents without executing them. Zero or minimal agent-side code changes.

### Non-Goals
- NOT an observability platform — does not replace Jaeger, Grafana Tempo, or Langfuse
- NOT a trace collector — does not replace the OTel Collector
- NOT tied to a single correlation mechanism — traceparent is one strategy, not the only one
- NOT limited to GenAI semantic conventions — should work with any OTel traces, with GenAI extraction as a convenience layer

### Goals
- Acquisition trait in the kernel that abstracts how outputs are obtained (inline vs. observe vs. other modes)
- Pluggable correlation strategies (traceparent injection, correlation ID in payload, env var, sequential timing, manual mapping)
- Pluggable span extraction (user-extensible SpanExtractor trait — extract "output" from a tree of spans)
- Support for multiple trace backends behind an abstraction (Jaeger API, Grafana Tempo API)
- Convenience extractors for common patterns: last LLM response, sequence of tool calls, full trajectory as structured data
- Eval replay: point at historical traces, score retroactively against new criteria
- Timeout and retry handling for trace collection

### Validation Plan
- **Cheapest test**: Set up an OTel-instrumented HTTP agent (one of the user's existing agents or a simple test app). Send a request with a traceparent header from Rust. Query Jaeger by trace ID. Verify spans contain expected data. **1 day.**
- **What it proves**: Whether trace correlation works end-to-end without agent code changes. This is the load-bearing assumption (A-03).
- **Success signal**: Trace ID propagates through the agent, spans appear in Jaeger with the correct parent trace, span content includes LLM inputs/outputs
- **Failure signal**: Trace ID doesn't propagate (agent framework strips it), or span content doesn't include the data needed for scoring

### MVP Scope (Week 1)
- Acquisition trait added to the kernel (abstraction over inline/observe)
- TraceAcquisition implementation: send HTTP request with correlation context → query trace backend → return spans
- One correlation strategy (traceparent or simplest viable option)
- One trace backend (Jaeger v2 API)
- One hardcoded extractor (last LLM response content)
- **OUT of MVP**: Multiple backends, custom extractors, embedded OTLP receiver, trajectory extraction, sidecar mode, eval replay

### Growth Path
1. **SpanExtractor trait** — user-extensible extraction from spans — small
2. **Trajectory extraction** — extract sequence of tool calls as structured output for trajectory scoring — medium
3. **Additional trace backends** — Grafana Tempo API — small
4. **Additional correlation strategies** — env var injection, correlation ID in payload, manual mapping — small
5. **Eval replay** — point at historical traces in a trace store, score retroactively — medium
6. **Embedded lightweight OTLP receiver** — for development without external Jaeger — medium
7. **Sidecar/continuous mode** — continuously evaluate a stream of traces from production — large

### Core Technical Decisions
- **Acquisition as a trait**: The kernel's Run accepts any Acquisition implementation. Inline mode is one implementation. Observe mode is another. This keeps the kernel ignorant of how outputs are obtained. Reasoning: this is the architectural insight from the braindump — the grading layer is shared, the acquisition layer is mode-specific.
- **Correlation is pluggable**: A Correlator trait maps (sample, request) → correlation context and (correlation context, trace backend) → spans. Traceparent is one implementation. Reasoning: the user's feedback — traceparent should be an implementation detail, not a hard requirement.
- **Extraction is pluggable**: A SpanExtractor trait takes a span tree and returns the "output" to score. Built-in extractors for common patterns (last LLM response, tool call sequence). Reasoning: "what the agent did" in a trace depends on the agent architecture — no single extraction logic works for all agents.
- **Trace backend is pluggable**: A TraceBackend trait abstracts querying spans by trace/correlation ID. Jaeger and Tempo are implementations. Reasoning: teams use different trace stores; lock-in to one is unacceptable.

### User Segments Served
- **Primary (beachhead)**: The user — evaluating their own OTel-instrumented Rust agents
- **Secondary**: Platform/infrastructure engineers (Stream 3, Segment 6) who manage eval infrastructure for multiple teams
- **Long-term**: Anyone with OTel-instrumented AI agents who wants cost-efficient evaluation

### Distribution
- **First 100 users**: OTel/CNCF community channels + blog post "How I evaluate AI agents without re-running them" + cross-post to agentevals-dev community
- **Adoption trigger**: "Evaluation is costing me as much as the agent itself" — re-execution cost drives search for trace-based alternatives
- **Push vs. pull**: Mostly push — "evaluate from traces" is not how most people think about evaluation yet

### Competitive Position
- **Replaces**: Re-execution-based evaluation workflows (run agent again just to score it)
- **Delta over agentevals-dev (112 stars)**: Rust (performance), pluggable correlation (not locked to one strategy), extensible extraction (not hardcoded), part of broader eval framework (not standalone), persistent results (not in-memory only)
- **Defensibility**: OTel-native + Rust + pluggable architecture. agentevals-dev could add these features but would require significant rearchitecture.

### Sustainability
- **Model**: Open source, part of the eval kernel crate (feature-gated)
- **Cost structure**: Zero beyond development time
- **If commercial later**: Managed trace-based evaluation service (hosted extraction + scoring) could be commercial

### Risks
- **OTel GenAI conventions change** — Likelihood: medium — Mitigation: abstract extraction behind SpanExtractor trait; update built-in extractors as conventions evolve
- **Traceparent doesn't propagate in practice** — Likelihood: low-medium — Mitigation: multiple correlation strategies; validate with prototype before committing
- **Span content insufficient for scoring** — Likelihood: low — Mitigation: extraction is user-extensible; users can extract whatever their agents emit
- **Trace backend query latency** — Likelihood: medium — Mitigation: configurable timeouts and retries; async execution

### Kill Criteria
- None. The user stated: "I wouldn't stop. OTel is the future." This direction survives regardless of obstacles — the question is how long it takes to get right, not whether to pursue it.

### Open Questions
- Which correlation strategy to implement first? Traceparent is cleanest for HTTP agents but may not be the user's most common case.
- Should the framework define its own OTel semantic conventions for evaluation results (e.g., `eval.run.id`, `eval.sample.id` span attributes)?
- How to handle partial traces — agent started but didn't finish, or trace backend hasn't ingested all spans yet?
- What's the right abstraction for "output" when it's extracted from spans? Raw span data? Structured GenAI types? Generic JSON?

### Connections
- **Shares foundation with**: Direction 1 (Kernel) — depends entirely on it
- **Competes with**: Nothing directly
- **Enables**: Eval replay (historical re-evaluation), sidecar mode (continuous production evaluation), third-party agent evaluation (evaluate agents you don't own)

---

## Direction 3: The Confident Eval

### What
Multi-trial evaluation with statistical aggregation as a first-class feature of the eval kernel. Run each sample N times, report results with confidence intervals, significance tests, and pass@k metrics. Turns evaluation from anecdote into evidence. Addresses the #1 gap identified in the domain research (synthesis.md, Gap 1: Critical unmet need).

### Non-Goals
- NOT a statistics library — implements specific, well-understood statistical tests for evaluation, not general-purpose stats
- NOT a replacement for domain expertise — statistical rigor tells you IF a change is real, not WHETHER it's good
- NOT only for AI — works with any non-deterministic system evaluated through the kernel

### Goals
- `.trials(N)` on the Run builder — minimal API surface change
- Per-sample aggregation: mean, standard deviation, confidence interval (Wilson for proportions, t-distribution for continuous)
- pass@k (at least one pass in k trials) and pass^k (all k pass) metrics
- Aggregate statistics across samples with proper confidence intervals
- Run-to-run comparison with significance testing: "are these two runs statistically different?"
- Cost tracking: total trials × tokens/cost per trial, so the user sees the cost of rigor
- Clear, non-overwhelming default output (mean ± CI) with detailed stats available on request

### Validation Plan
- **Cheapest test**: Take one of the user's agents. Run the same eval 10 times manually. Plot the score distribution. If scores vary by more than ±5%, the case makes itself. **1 hour.**
- **What it proves**: Whether the user's own agents are non-deterministic enough to benefit from multi-trial evaluation
- **Success signal**: Scores vary meaningfully across runs — the user sees that single-trial results are unreliable
- **Failure signal**: Scores are nearly identical across runs — the agent is effectively deterministic and multi-trial adds cost without value

### MVP Scope (Week 1)
- `.trials(N)` on Run builder
- Per-sample: mean + standard deviation + pass@k
- Aggregate: overall mean + pass rate
- Concurrent trial execution with configurable concurrency limit
- **OUT of MVP**: Confidence intervals, significance testing, run-to-run comparison, drift detection, adaptive trial counts, cost tracking

### Growth Path
1. **Wilson confidence intervals** for proportions + t-distribution CIs for continuous scores — small
2. **Run-to-run comparison** with significance testing (are two runs statistically different?) — medium
3. **Cost tracking** — total tokens and estimated cost across all trials — small
4. **Drift detection** (CUSUM/Page-Hinkley) across historical runs loaded from JSONL — medium
5. **Fisher exact tests** for step-level failure attribution (which step fails more than expected?) — medium
6. **Adaptive trial counts** — stop early when CI is narrow enough — medium

### Core Technical Decisions
- **Trial is a runner concept, not a type concept**: The Scorer trait doesn't change. A trial is one invocation of the acquisition + scoring pipeline. Multi-trial is managed by the Run, not the Scorer. Reasoning: scorers should be pure functions; trial management is orchestration.
- **Statistical output as optional detail**: Default output shows mean ± CI in a human-readable format. Detailed statistics (n, z, p-value, test type) available via `.detailed()` on the result. Reasoning: most developers don't think in statistics; overwhelming them with numbers reduces adoption.
- **Concurrent trials by default**: Trials run concurrently up to a configurable limit. Reasoning: 10 sequential trials on an agent that takes 5 seconds each = 50 seconds. Concurrent = 5-10 seconds. But concurrency must be configurable for agents with rate limits or shared state.
- **Fixed trial count first, adaptive later**: `.trials(10)` means exactly 10 trials. Adaptive ("run until CI < threshold") is a growth path feature. Reasoning: fixed is simpler, predictable, and sufficient for initial use.

### User Segments Served
- **Primary (beachhead)**: The user — evaluating non-deterministic AI agents and needing confidence in results
- **Secondary**: Any developer using CI/CD quality gates who's been burned by eval flakiness
- **Long-term**: Teams that need regulatory or compliance-grade evaluation evidence

### Distribution
- **First 100 users**: Content-driven — blog post "Why your AI eval scores are meaningless" with concrete before/after showing single-trial vs. multi-trial results
- **Adoption trigger**: "I made a change, my eval score dropped 3 points, I rolled back — but was it even real?"
- **Push vs. pull**: Push — most teams don't know they have a statistical rigor problem until shown

### Competitive Position
- **Replaces**: Manual multi-trial wrapper scripts (Stream 5, Workaround Pattern #5); single-trial evaluation workflows
- **Delta**: Only Agentrial (16 stars, Python) attempts this. No other tool in the 65+ project landscape offers statistical aggregation as a first-class feature. Integrated into a real eval framework rather than standalone.
- **Defensibility**: The AI eval community's ML/NLP background (deterministic test cases) creates a knowledge gap. Statistical testing of non-deterministic systems requires different expertise — this is a moat.

### Sustainability
- **Model**: Open source, part of the eval kernel crate
- **Cost structure**: Zero — pure computation, no external dependencies
- **If commercial later**: Statistical rigor as a differentiator for a managed evaluation service

### Risks
- **Users find statistical output confusing** — Likelihood: medium — Mitigation: simple defaults (mean ± CI), detailed stats opt-in
- **Cost multiplication** (10 trials = 10x LLM cost) — Likelihood: certain — Mitigation: make cost visible; support configurable trial counts; adaptive trial counts as growth path
- **Agents have concurrency issues** (shared state, rate limits) — Likelihood: medium — Mitigation: configurable concurrency limit, sequential mode available
- **Statistical rigor doesn't change decisions** — Likelihood: low — Mitigation: the cheapest test (run 10 times, see variance) proves or disproves value quickly

### Kill Criteria
- Eval results (with statistical rigor) don't help the user improve agents — the statistical output is technically correct but doesn't influence decisions
- The cost multiplication makes evaluation prohibitively expensive for the user's agents

### Open Questions
- Should confidence intervals be part of the Score type itself (always present, filled with n=1 for single trials) or a separate aggregation layer? User was unsure — this is a design question for prototyping.
- What's the right default trial count? 5? 10? 20? Depends on agent variance — may need a "calibration" run.
- How should results from different trial counts be compared? (Run A: 10 trials, Run B: 20 trials — are the CIs comparable?)

### Connections
- **Shares foundation with**: Direction 1 (Kernel) — depends entirely on it
- **Competes with**: Nothing
- **Enables**: Reliable CI/CD quality gates (confidence-based thresholds instead of raw scores), drift detection across releases

---

## Go/No-Go Cross-Check

| Direction | Go Signals Aligned | No-Go Signals Present | Failure Patterns Echoed | Verdict |
|-----------|-------------------|----------------------|------------------------|---------|
| Eval Kernel | No Rust eval framework exists; quality is #1 deployment barrier (32%); eval adoption gap (89% obs vs 52% eval); consolidation creates neutral-tool demand | Platform incumbents are well-funded; "eval as feature" commoditization | Metrics-only library without platform (Approach 1) — but user is building foundation, not product. Different. | **Proceed** |
| Trace Grader | OTel is emerging standard; cross-framework eval gap (HIGH priority); 89% have observability data already; agentevals-dev proved concept with 112 stars | OTel conventions still evolving | None — no OTel eval tool has failed | **Proceed** |
| Confident Eval | Statistical rigor is #1 gap (CRITICAL); only Agentrial at 16 stars; Anthropic recommends multi-trial; anti-pattern #4 (single-trial) is documented | None | None — nothing like this has been attempted and failed | **Proceed** |

## Dependency Map

- **Core Scorer Trait + Score Types + Result/Run** → enables Direction 1, Direction 2, Direction 3
  - Standalone value: Yes — usable as a Rust eval library immediately
  - Estimated effort: 1-2 weeks for Core variant

- **Acquisition Trait abstraction** → enables Direction 2 (observe mode), future CLI direction
  - Standalone value: Moderate — useful for separating "how to get output" from "how to score it"
  - Estimated effort: Days (design) + 1-2 weeks (OTel implementation)

- **Statistical Aggregation Functions** → enables Direction 3
  - Standalone value: Yes — reusable stats on any `Vec<Score>`
  - Estimated effort: Days for core stats, 1-2 weeks for full Core variant

- Direction 2 and Direction 3 are independent of each other — can be built in parallel once Direction 1 exists

## Recommended Build Order

1. **Direction 1: Eval Kernel** — why first: everything depends on it. The scorer trait is the keystone. If it's wrong, nothing else works. Get this right.
2. **Direction 2: Trace Grader** — why second: this is where the user's energy and deepest conviction are. OTel observation mode is the soul of the project. Building it immediately after the kernel validates the Acquisition trait abstraction.
3. **Direction 3: Confident Eval** — why third: important correctness feature, but the least exciting to the user. Build it when the kernel is stable and the user has run enough single-trial evals to feel the pain of non-determinism. Alternatively, build in parallel with Direction 2 since they're independent.

## Strategic Notes

### Cross-Cutting Observations

**The kernel is infrastructure, not product.** The user's success metric is "I use it daily to improve my agents" — not "I have a popular crate." This means API polish matters more than documentation breadth in the short term. The kernel should feel right to its one user before it's optimized for strangers.

**OTel is the conviction bet.** The user won't kill this direction under any conditions. This means OTel-related design decisions should be made with confidence — don't hedge excessively. Build for OTel working well, with graceful fallbacks if specific mechanisms (traceparent) don't work as expected.

**Statistical rigor is the market differentiator, not the user motivator.** The user finds it "least exciting" but acknowledges its importance. If this project ever needs to attract external users, confidence intervals are the headline feature (no other tool has them). But for the user's own workflow, it's a correctness feature, not an excitement feature.

**Terminology is deferred to a dedicated review step.** The brainstorm uses Sample/Scorer/Score/Result/Run as working terms. These may change after the terminology review (domain.md). The architecture should not depend on specific names.

**The generic core is a bet, not a certainty.** The user has a pragmatic escape hatch: "if AI-specific abstractions are actually needed at the core, we can always build a higher-level layer." This means the generic design should be attempted but not defended at all costs. If prototyping reveals that the generic types create friction for every AI use case, pivot to AI-specific core with generic as a lower layer.

### Domain Triggers to Watch

- **OTel GenAI semantic conventions reach v1.0 stable**: Locks in the span schema for extraction. Signal: OpenTelemetry blog announcement or spec version bump. → Finalize built-in SpanExtractors against the stable spec.

- **A Rust AI agent framework gains significant traction** (1k+ stars): Creates a natural beachhead for the eval kernel. Signal: r/rust or HN discussion about a Rust agent framework. → Prioritize integration/examples with that framework.

- **Another tool implements multi-trial statistical evaluation**: Narrows the differentiation window for Direction 3. Signal: DeepEval, Langfuse, or similar adds `.trials()` or confidence intervals. → Accelerate Direction 3 or find the next differentiator.

- **agentevals-dev gets acquired or gains significant traction** (1k+ stars): Validates the OTel-based eval approach but increases competition. Signal: GitHub star growth, acquisition announcement. → Differentiate on Rust performance, pluggable architecture, and integration with the kernel.

- **OpenAI/Anthropic release built-in evaluation with OTel export**: Would validate the approach AND create more OTel traces to evaluate. Signal: API docs update. → Ensure trace backend support covers their export format.
