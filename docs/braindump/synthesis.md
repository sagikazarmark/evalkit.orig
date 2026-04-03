# Domain Exploration Synthesis

## Executive Summary

The user is exploring a **generic, layered evaluation framework in Rust** — domain-agnostic at the core, with AI-specific layers on top. The domain (AI evaluation) is large, fragmented, and growing, with 65+ tools and no single dominant solution. The user's angle is genuinely novel: no Rust eval framework exists, no tool takes a generic-core approach, and no tool supports all execution modes (inline, subprocess, OTel observation, agent harness) from a single codebase.

The strongest opportunity is a Rust evaluation library with a clean scorer trait system, multiple execution modes, and OTel-native observation — differentiated by flexibility, performance, and statistical rigor (multi-trial evaluation with confidence intervals). The primary risk is adoption in a Python-dominated ecosystem, mitigated by the user building for himself first and deferring multi-language support.

Top areas worth pursuing: **(1)** Core eval library in Rust with generic scorer trait, **(2)** OTel observation mode with traceparent correlation, **(3)** Statistical multi-trial evaluation.

---

## Exploration Areas

### Area 1: Core Eval Library — Generic Scorer Framework in Rust

- **What**: A Rust library providing typed evaluation primitives — Sample, Scorer trait, Score types, Result, Run — that are domain-agnostic. The scorer is a function `(input, output, reference?) → Score`. Score types include continuous (0-1), binary (pass/fail), label, and metric. AI-specific types and convenience functions live in a separate layer.
- **Shape**: Library (Rust crate)
- **Rooted in**: I-01, I-04, T-01, T-02, H-01
- **Domain support**: No Rust eval framework exists (web search confirmed only `adk-eval` at 1k downloads, tied to its framework). The generic-core approach is unique — every existing tool bakes AI into the core API. Stream 9 validates that framework flexibility wins over opinionated approaches (Approach 5 failure).
- **Existing solution quality**: Nothing comparable exists in Rust. In Python, DeepEval (14.4k stars) is the closest as a library but is AI-opinionated, Python-only, and coupled to Confident AI cloud. Inspect AI's Solver/Scorer model is well-designed but research-focused and Python-only.
- **Core technical problem**: Designing Rust trait bounds that are generic enough for arbitrary input/output types but ergonomic for the common case (string input, string output, continuous score). Rust's type system is powerful but complex — the `Scorer<I, O, R>` trait needs careful design to avoid "trait bound soup."
- **Smallest useful version**: `Sample<I, R>`, `Scorer` trait, `Score` enum (Continuous/Binary/Label/Metric), `Result`, `Run`, inline execution mode, JSONL serialization. A few built-in scorers (exact match, contains, regex). **Effort: days to a week.**
- **Key question**: Does the generic Scorer trait feel good in practice? Write 5 different scorers and see if the API is natural or fights you.
- **Risk**: Over-generification makes the API verbose for the common case. Mitigated by AI-specific convenience layers.
- **Effort signal**: Small experiment
- **Leverage signal**: Foundational — everything else builds on this

---

### Area 2: OTel Observation Mode

- **What**: Evaluate agents from their OTel traces without executing them. The framework sends requests with W3C `traceparent` headers (or accepts a correlation ID), collects traces from a trace backend by trace ID, extracts structured output from spans, and feeds it to scorers. Zero or minimal agent-side changes.
- **Shape**: Library module / feature gate (`evallib-otel` or cargo feature)
- **Rooted in**: I-03, T-04, H-04, the user's strong conviction about agentevals-dev's approach
- **Domain support**: Only agentevals-dev (112 stars) does OTel-based evaluation, and it's Python-only. OTel GenAI semantic conventions are the emerging standard (Stream 6, Stream 7 Trend 3). Gap 4 in synthesis: "Cross-framework agent evaluation via OTel" is rated HIGH priority. The `traceparent` correlation approach (researched during cross-reference) is simpler and more robust than agentevals-dev's custom session management.
- **Existing solution quality**: agentevals-dev has the right idea but is Python-only, requires either SDK wrapping or env var configuration, uses in-memory storage, and has a tiny community (112 stars). The abstraction is clean for OTel→eval but limited in scope. Root cause: early stage and narrow focus.
- **Core technical problem**: Extracting structured "output" from raw OTel spans. GenAI semantic conventions define span structure, but extracting "what the agent did" from a tree of spans requires domain knowledge (which spans are LLM calls? tool calls? the final answer?). This extraction is the hard part — scoring is the same as any other mode.
- **Smallest useful version**: Send HTTP request with `traceparent` → wait for response → query Jaeger/Tempo API by trace ID → extract LLM response from `gen_ai` spans → feed to scorer. Supports the "blueprint writer" use case. **Effort: 1-2 weeks.**
- **Key question**: Does traceparent propagation actually work end-to-end with common agent frameworks (LangChain, OpenAI Agents SDK)? Needs a prototype to verify.
- **Risk**: OTel GenAI conventions are still evolving — span extraction logic may need updates as conventions change. Mitigated by abstracting extraction behind a trait.
- **Effort signal**: Medium build
- **Leverage signal**: Broad value — enables evaluating any OTel-instrumented agent

---

### Area 3: Statistical Multi-Trial Evaluation

- **What**: Run each sample N times, aggregate results with statistical rigor — Wilson confidence intervals, significance tests, pass@k/pass^k metrics, drift detection (CUSUM). The #1 gap identified in the research synthesis (Gap 1: Critical unmet need).
- **Shape**: Runner feature within the core library
- **Rooted in**: Research Gap 1, Q-04 (features not mentioned), checkpoint discussion
- **Domain support**: Only Agentrial (16 stars) attempts statistical agent evaluation. Every other tool runs single trials and reports simple averages. Stream 5 (#2, #7): non-determinism and lack of statistical rigor are domain-wide challenges. Anthropic's eval guide recommends multiple trials. This is the highest-leverage differentiator available.
- **Existing solution quality**: Agentrial has the right concept (Wilson CI, Fisher exact tests) but 16 stars, Python-only, narrow scope. Root cause: the AI eval community comes from ML/NLP where deterministic test cases are the norm — statistical testing of non-deterministic systems requires different expertise.
- **Core technical problem**: Not algorithmically hard — Wilson CI, Fisher exact tests, and CUSUM are well-understood statistics. The challenge is UX: how to present statistical results clearly (CI ranges, significance levels) without overwhelming users who just want pass/fail.
- **Smallest useful version**: `Run::builder().trials(10).confidence(0.95)` — run each sample 10 times, report mean + CI per scorer, aggregate pass@k. **Effort: a few days on top of the core library.**
- **Key question**: Do users actually need statistical rigor, or do they just want "run it a few times and average"? The research says yes (critical unmet need), but the user should validate with their own agent eval experience.
- **Risk**: Users may find statistical output confusing. Mitigated by simple defaults (just show pass rate + CI) with detailed stats available on request.
- **Effort signal**: Small experiment (given core library exists)
- **Leverage signal**: Broad value — highest-leverage differentiator in the domain

---

### Area 4: CLI Binary with Subprocess Execution Mode

- **What**: A standalone binary that reads sample datasets from JSONL/JSON, executes a target program as a subprocess, captures output, runs scorers, and outputs structured results. Enables evaluation without writing Rust code.
- **Shape**: Binary / CLI tool
- **Rooted in**: I-02, I-07 (binary+JSON for multi-language), T-03
- **Domain support**: Promptfoo (19.1k stars, now OpenAI) proved the CLI+config approach works. The Promptfoo gap: it's TypeScript and now OpenAI-owned — provider-neutral alternatives are needed (Stream 7 Trend 1, Stream 9 Pivoted Projects). Stream 6 (Pattern 5) documents CLI-driven evaluation as a common pattern.
- **Existing solution quality**: Promptfoo has excellent CLI UX but is TypeScript, YAML-config-heavy, prompt-focused (not agent-focused), and now OpenAI-owned. Inspect AI has a CLI but is Python and research-focused.
- **Core technical problem**: Designing a configuration format (YAML? TOML? JSON?) that's flexible enough for diverse use cases without becoming Promptfoo-level YAML sprawl. The subprocess protocol (JSON on stdin/stdout) needs clear specification.
- **Smallest useful version**: `evalctl run --dataset samples.jsonl --command "python my_agent.py" --scorer exact_match` — run a command per sample, compare stdout to reference, report scores. **Effort: days on top of core library.**
- **Key question**: How much configuration complexity is acceptable? Promptfoo's YAML configs become unwieldy — can we do better?
- **Risk**: Configuration format design is notoriously hard. Start minimal, expand based on need.
- **Effort signal**: Small experiment
- **Leverage signal**: Broad value — makes the tool accessible to non-Rust users

---

### Area 5: Self-Instrumenting Framework (OTel for Eval Internals)

- **What**: The eval framework itself emits OTel spans for its operations — scorer execution, LLM judge calls, trace collection, aggregation. Enables cost tracking, performance profiling, and meta-evaluation using the same OTel infrastructure.
- **Shape**: Library feature (cargo feature gate)
- **Rooted in**: User's checkpoint reaction (cost tracking via OTel), Discovery #1 (self-instrumentation = dogfooding)
- **Domain support**: Stream 5 (#3) documents eval cost as a real pain point. Stream 7 identifies meta-evaluation as emerging. No existing tool self-instruments with OTel. This would be a unique technical feature.
- **Existing solution quality**: No existing tool does this. Cost tracking is manual or absent in all tools surveyed.
- **Core technical problem**: Minimal — OTel instrumentation is well-understood. The design question is which operations to instrument and what attributes to include (token counts, model name, latency, cost).
- **Smallest useful version**: Instrument scorer execution with OTel spans. Include timing. Add token count/cost attributes when using LLM-based scorers. **Effort: days.**
- **Key question**: Is the overhead of OTel instrumentation acceptable for a performance-sensitive eval framework? Likely yes if behind a feature gate.
- **Risk**: Minimal. Feature-gated, well-understood technology.
- **Effort signal**: Small experiment
- **Leverage signal**: Niche value initially, broad value as the framework scales

---

### Area 6: Transform Pipeline (Output → Artifact → Score)

- **What**: An optional step between output acquisition and scoring that transforms raw output into a scorable artifact. Examples: render JSON to PNG, compile code, run tests, parse structured output.
- **Shape**: Optional API in core library (function slot)
- **Rooted in**: I-06, user's Excalidraw and GitHub issue use cases
- **Domain support**: No existing tool has a formal transform pipeline. Inspect AI's Solver concept includes processing steps but is integrated into their execution model. The pattern (raw_output → transform → artifact → score) is universal but unformalized.
- **Existing solution quality**: Nothing exists. Users handle transforms in ad-hoc code outside the eval framework.
- **Core technical problem**: Type safety — the transform changes the output type (`O → O'`), which means the scorer must accept `O'` not `O`. In Rust, this requires either generic type chaining or trait objects (`dyn Any`). Need to balance type safety with ergonomics.
- **Smallest useful version**: A `transform` closure on the run builder: `Run::builder().transform(|output| render_png(output))`. The scorer then receives the transformed output. **Effort: hours if the type system cooperates.**
- **Key question**: How often does the user actually need transforms? If it's 2 out of 10 use cases, an optional closure is fine. If it's 8 out of 10, a more structured pipeline API is needed.
- **Risk**: Type system complexity. Mitigated by keeping it simple (optional closure, not a pipeline engine).
- **Effort signal**: Small experiment
- **Leverage signal**: Niche value (only needed for artifact-producing agents)

---

## Validated Hypotheses

### H-02: Existing eval tools are too opinionated about execution
**Validated.** Stream 5 documents lock-in across multiple tools. Stream 9 shows single-provider coupling failed (Approach 5). Stream 3 shows teams use 3-5 tools because no single tool fits all execution contexts. The market has validated demand for decoupled approaches.

### H-04: OTel spans are sufficient for many (but not all) use cases
**Validated with documented limitations.** Stream 4 (Pattern 4) supports trace-based evaluation. The user's own use cases confirm: OTel sufficient for text-output agents (blueprint writer), insufficient for artifact-output agents (Excalidraw, test execution).

---

## Challenged Hypotheses

### H-01: Evaluation is generic (doesn't require AI-specific core)
**Partially challenged.** The fundamental loop is generic, but AI evaluation has patterns (multi-turn conversations, trajectory evaluation, reference-free scoring, non-determinism) that don't reduce trivially to input→output→grade. The hypothesis holds as a design principle (generic core + AI layers), but the generic core must be flexible enough to express AI patterns without contortion. The `reference: Option<T>` insight is critical — many AI evals have no reference.

### H-05: Rust is viable and advantageous
**Partially challenged.** Rust is viable (no competition in Rust, Restate proves multi-language works) but the Python SDK delay is a real adoption risk. Advantageous for performance and binary distribution, but the AI ecosystem's Python dominance means the user is building for a small initial audience.

---

## Unresolved Hypotheses

### H-03: A single library can support all execution modes
**Unresolved.** The decomposition (acquisition + grading layers) is sound in theory. The grading layer is clearly shared. The acquisition layer has genuinely different concerns per mode. No existing tool has tried this. **Resolving requires prototyping** — build the core library, implement inline mode and OTel mode, and see if the abstraction holds.

---

## Technical Opinions — Reality Check

### T-01: AI-agnostic core API
**Supported.** No tool does this. The generic approach is viable if types are flexible (see H-01 analysis). Rust's type system (traits, generics, enums) is well-suited for this design.

### T-02: Rust for core
**Supported.** Wide-open space. No competition. Advantages in performance and binary distribution. Risk in ecosystem adoption is accepted.

### T-03: Binary+JSON for multi-language scorers
**Supported and proven** by agentevals-dev. For library-mode integrations, native bindings (PyO3, WASM) would be needed later — but the user deferred multi-language, so binary+JSON is sufficient for now.

### T-04: OTel not a hard requirement
**Supported.** Feature-gated OTel is the right design. Core library with zero OTel dependency, `evallib-otel` feature adds observation mode and self-instrumentation.

### T-05: Storage deferred
**Supported.** Serializable results (JSONL) are sufficient. Stream 9 validates that premature platform-building fails. Git-based storage (commit result files) is a compelling zero-infrastructure pattern.

### T-06: Scorer catalog not important initially
**Supported with caveat.** 5-10 basic scorers (exact match, contains, regex, JSON schema validation) are needed for the framework to be immediately useful. An LLM-as-judge scorer is the minimum AI-specific scorer to demonstrate the layered architecture.

---

## Assumption Register

| ID | Assumption | Status | Evidence | Action Needed |
|----|-----------|--------|----------|---------------|
| A-01 | Core eval loop separable from execution | Supported | Decomposition into acquisition + grading layers is sound (cross-ref). "Output" type must include traces/trajectories. | Prototype to verify |
| A-02 | Binary+JSON sufficient for multi-language | Partially supported | Proven for custom scorers (agentevals-dev). Insufficient for library-mode in Python/TS — needs native bindings (deferred). | Accept for now, revisit for adoption |
| A-03 | Zero-code-change OTel correlation | Supported (relaxed) | W3C traceparent works for HTTP agents. User accepts correlation ID approach. Non-HTTP agents need env var config. | Prototype traceparent flow |
| A-04 | Rust adoption in Python ecosystem | Risk acknowledged | Restate proves mitigation. User defers Python SDK. Accepted risk — building for self first. | Revisit when targeting adoption |
| A-05 | Generic abstractions support AI eval | Probably holds | Flexible types + optional reference + AI convenience layers should work. Rust type system is well-suited. | Prototype scorer trait with AI use cases |

---

## Conflicts to Resolve

1. **Generic purity vs. pragmatic API** — How far to push AI-agnosticism? The user needs to decide where AI-specific types first appear (separate crate? feature? module?). Recommendation: separate module within the same crate initially, separate crate if the boundary proves stable.

2. **OTel observation architecture** — Start with trace-backend query (simpler, works with existing infra) or built-in OTLP receiver (simpler for users without infra)? The user acknowledged both may be needed. Recommendation: trace-backend query first (the traceparent approach), OTLP receiver later.

3. **Configuration format for CLI mode** — YAML (Promptfoo precedent), TOML (Rust convention), or JSON? Each has tradeoffs. Recommendation: TOML for Rust ecosystem alignment, with JSON as an alternative. Avoid YAML sprawl.

---

## Knowledge Gaps

### Researchable
- Rust OTel library capabilities for trace context propagation
- JSON Schema patterns for eval results
- `evalframe` crate's scorer API design

### Requires Prototyping
- Generic Scorer trait ergonomics in practice (write 5+ scorers)
- Traceparent propagation end-to-end with real agent framework
- JSONL result format for diff/comparison workflows
- Type system impact of optional transform step
- PyO3-friendliness of the core API (even if Python is deferred, avoid painting yourself into a corner)

### Requires User Experience
- How often transforms are needed across the user's diverse projects
- How much scorer code is reused vs. project-specific
- Whether statistical output (CI ranges) is useful or confusing in practice

---

## Open Questions

1. What should the crate be named? Something generic (evallib, evalkit, scored) or AI-leaning (agenteval, aieval)?
2. Should the framework define its own OTel semantic conventions for evaluation results? Or use ad-hoc attributes?
3. How should scorer errors be handled? A scorer that crashes vs. a scorer that returns a low score are different. Should there be a `Score::Error` variant?
4. Should the framework support async scorers? LLM-as-judge requires network calls. Rust's async story is good but adds complexity.
5. What's the story for visual evaluation (the Excalidraw PNG use case)? LLM-based visual scoring? Human review interface? Both are complex.
6. Should the Run output include the raw outputs alongside scores? For debugging and review, having the actual output is essential. But for large datasets, this may be too much data.

---

## Recommended Next Steps

1. **Prototype the core Scorer trait** — Write `Sample<I, R>`, `Scorer` trait, `Score` enum in Rust. Implement exact_match, contains, regex, and a mock LLM-as-judge scorer. Evaluate API ergonomics. **This is the single most important validation step.** If the trait feels good, everything else follows. If it doesn't, redesign before building more.

2. **Prototype traceparent correlation** — Set up a simple OTel-instrumented HTTP agent (e.g., a Python Flask app with OpenAI calls + OTel auto-instrumentation). Send a request with a `traceparent` header from Rust. Verify trace ID propagation in Jaeger. This validates the core observation mode assumption.

3. **Build inline execution mode** — Once the Scorer trait works, build the inline runner (load dataset, call function, score, output JSONL). This produces the smallest useful tool.

4. **Add CLI binary** — Wrap the core in a CLI: `evalctl run --dataset samples.jsonl --command "..." --scorer exact_match`. This makes the tool usable without writing Rust.

5. **Add multi-trial runner** — `--trials 10 --confidence 0.95`. This is the highest-leverage differentiator and is straightforward to implement on top of the core.

6. **Add OTel observation mode** — Query a trace backend by trace ID, extract output from GenAI spans, feed to scorers. This validates the decoupled execution model.

7. **Defer**: Python bindings, multi-language support, platform features, dashboards, CI/CD gating, storage, scorer catalog beyond basics.

The domain is worth building in. The positioning (Rust + generic + multi-mode + statistical rigor) is defensible and unique. Start with step 1 — the Scorer trait prototype — and let the design validate or challenge the vision.

---

## Architecture of Opportunity

```
[Core Scorer Trait] enables → Area 1 (library), Area 3 (multi-trial), Area 4 (CLI), Area 6 (transforms)
[OTel Integration]  enables → Area 2 (observation mode), Area 5 (self-instrumentation)
Independent:                   (none — all areas share the core)
```

### Foundational Components

**Component: Core Scorer Trait + Score Types**
- Standalone value: Yes — usable as a Rust eval library immediately
- Build cost: Days
- Unlock value: Everything else. This is the keystone.

**Component: OTel Integration Layer**
- Standalone value: Moderate — useful for trace-based scoring even without the full observation mode
- Build cost: 1-2 weeks
- Unlock value: Observation mode, self-instrumentation, cost tracking, eval replay

### Recommended Build Order

1. Core Scorer Trait (highest standalone value, unlocks 4 areas)
2. Inline Runner + JSONL output (makes it a usable tool)
3. CLI Binary (makes it accessible to non-Rust users)
4. Multi-Trial Runner (highest-leverage differentiator)
5. OTel Observation Mode (second keystone, unlocks observation-mode areas)
6. Self-Instrumentation (low effort once OTel integration exists)
7. Transform Pipeline (add when needed by specific use case)

---

## Raw Material Preserved

See `00-record.md` for the full braindump, Q&A log, and extracted elements.
See `01-cross-reference.md` for detailed cross-referencing against the research corpus.
See `02-discoveries.md` for connections, opportunities, and generative ideation.
