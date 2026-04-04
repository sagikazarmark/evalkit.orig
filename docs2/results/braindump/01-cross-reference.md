# Phase 4 — Cross-Reference

Every claim below cites a specific research stream or a web search performed during this phase.

---

## Ideas Cross-Referenced

### I-01: Generic, Domain-Agnostic Eval Library in Rust with Layered Architecture

**Overlap with existing projects:**

No existing project takes the "generic eval core with AI layers on top" approach. Every tool in the landscape (Stream 1) is AI-specific in its core API:
- DeepEval: `LLMTestCase`, `ConversationalTestCase` — AI baked into the type system
- RAGAS: `SingleTurnSample`, `MultiTurnSample` — conversation-native
- Inspect AI: `Dataset → Solver → Scorer` — "Solver" implies an AI agent
- Promptfoo: prompts, providers, assertions — LLM-native vocabulary
- agentevals-dev: OTel GenAI spans, LLM invocations — AI observability-native

The generic framing is genuinely novel in this space. The closest analogy outside AI eval is pytest (generic test framework with domain-specific plugins), but pytest is a test runner, not an evaluation framework.

**Rust specifically:** Web search confirms near-zero Rust eval tooling:
- `adk-eval` (1,057 downloads) — tied to ADK-Rust agent framework
- `evalframe` (14 downloads) — Lua DSL with Rust host, interesting architecture but no adoption
- `tool-eval`, `mcp-check` (10 downloads each) — brand new, April 2026
- No general-purpose Rust eval framework exists. The space is wide open.

**Which user segments would this serve?** (Stream 3)
- Segment 1 (AI/LLM Application Engineer): Yes — if Python/TS access is solved via binary protocol
- Segment 6 (Platform/Infrastructure Engineer): Yes — Rust performance, OTel integration
- New segment not in research: Rust AI developers (small but growing community)
- Segments 2-5 are primarily Python users — binary protocol or FFI must bridge the gap

**Pain points addressed** (Stream 5):
- Fragmented tooling (#6): A flexible library could be connective tissue across tools
- Lock-in concerns (LangSmith, DeepEval/Confident AI): Directly addresses this
- Python-only limitation: Creates a non-Python option (though this is a double-edged sword — see A-04)

**Trajectory** (Stream 7):
- Consolidation creates demand for neutral, independent tools (Promptfoo → OpenAI, Humanloop → Anthropic leave gaps)
- "Evaluation-as-code" trend aligns with library-first approach

**Failure archaeology check** (Stream 9):
- Metrics-only libraries without platforms failed (Approach 1) — but user is aware and explicitly defers platform
- Standalone prompt management failed (Approach 4) — not applicable here
- Key lesson: "Minimal differentiation in a crowded observability space is fatal" (Log10 failure). The differentiation here is Rust + generic + multi-mode — that's substantial.

**Solution quality assessment of closest competitors:**

| Dimension | DeepEval | agentevals-dev | Promptfoo |
|-----------|----------|----------------|-----------|
| Use-case coverage | Comprehensive AI evals | Narrow (OTel agents only) | Broad (eval + red-team) |
| Where it falls short | Python-only, heavy LLM deps, Confident AI lock-in | Python-only, only OTel, tiny community | TypeScript, now OpenAI-owned, prompt-focused |
| Abstraction quality | Good metrics but AI-opinionated core | Clean for OTel→eval but limited | YAML config clean for simple, unwieldy for complex |
| Composability | Metrics composable, framework monolithic | Some (custom evaluators) | Moderate |
| Extensibility | Custom metrics within their framework | Evaluators in any language (binary+JSON) | Custom providers and assertions |
| Opinionatedness | Opinionated about pytest, AI types | Opinionated about OTel | Opinionated about YAML, prompts |
| Root cause of shortcomings | Designed AI-first, Python-only by choice | Scope choice (OTel-only) + early stage | TypeScript choice + OpenAI acquisition |

**Verdict on I-01:** Genuine gap. No Rust eval framework exists. No generic-core eval framework exists. The combination is unique and defensible. Risk: Python ecosystem dominance means multi-language access must be excellent from early on.

---

### I-02: Multiple Execution Modes as First-Class Citizens

**Overlap:**
No existing tool supports all four modes:
- **Library mode**: DeepEval, RAGAS, OpenEvals (Python libraries)
- **Binary/CLI mode**: Promptfoo, Inspect AI, EleutherAI harness
- **OTel observation mode**: agentevals-dev only (Stream 1)
- **Agent harness mode**: Inspect AI (Solver), AWS Agent Eval (evaluator-as-agent)

Most tools pick one or two modes and optimize for those. The "all modes" positioning is unique.

**Architectural feasibility** (Stream 4):
The research documents 7 architectural patterns. The user's vision maps to:
- Library mode → Pattern 1 (Library/Framework)
- Binary mode → Pattern 5 (CLI-Driven) + Pattern 6 (Declarative/Config)
- OTel mode → Pattern 4 (OpenTelemetry-Native)
- Agent harness → Pattern 7 (Research/Sandbox) or Pattern 5 (Evaluator-as-Agent)

These patterns are documented separately because they involve different tradeoffs. The question is whether a single core can serve all four without becoming a "least common denominator" that's mediocre at each.

**Key insight from the user's own correction:** Integrated mode (framework controls execution) and Observation mode (framework receives and grades) are structurally different. This maps cleanly to a two-layer architecture:
1. **Grading layer** (shared): takes input + output + reference → score. Same across all modes.
2. **Acquisition layer** (mode-specific): how the output is obtained — varies per mode.

This decomposition supports the multi-mode vision if the abstraction boundary between layers is clean.

**Verdict on I-02:** Architecturally viable if decomposed into acquisition + grading layers. No existing tool does this. The structural insight (integrated vs. observation) is sound and should be a first-class architectural concept.

---

### I-03: OTel-Native Observation Mode

**What agentevals-dev actually does** (web search during this phase):

agentevals-dev uses two correlation paths:
1. **WebSocket SDK path**: Python SDK creates a session with `session_id`, injects an OTel `SpanProcessor` that tags all spans with the session ID. Requires wrapping agent code in a context manager. **This is NOT zero-code-change.**
2. **OTLP HTTP path**: Agent exports traces to agentevals' OTLP endpoint. Correlation via OTel resource attributes (`agentevals.session_name`, `agentevals.eval_set_id`) set as environment variables. Agent code unchanged, but **environment variables must be set**.

Eval-case matching at evaluation time uses **text matching** on user content (first user message), not timing.

**The correlation problem (A-03) — solutions researched:**

W3C Trace Context injection is the strongest zero-code-change pattern:
1. Eval framework generates a trace ID per sample
2. Sends HTTP request to agent with `traceparent` header
3. Well-instrumented agent (standard OTel SDK) automatically propagates the trace ID through all internal spans — **no agent code changes needed**
4. After response, query trace backend by trace ID to collect all agent spans
5. Map: `sample_id → trace_id → [agent spans]`

This works because trace context propagation is the default behavior of OTel HTTP instrumentation. The agent doesn't need to know about the eval framework.

**Limitations:**
- Agent must use OTel HTTP instrumentation (common but not universal)
- Agent must accept HTTP requests (not all agents are HTTP services)
- For non-HTTP agents (e.g., CLI agents, queue-based agents), trace context injection isn't applicable
- Sequential execution (one sample at a time, collect by time window) is the fallback but is slow and fragile

**OTel GenAI semantic conventions** (Stream 6): No standardized evaluation-specific attributes exist yet. The `gen_ai.evaluation.result` event mentioned in Stream 6 is not established in the current spec. The eval framework would define its own attributes (`eval.run.id`, `eval.sample.id`, etc.).

**Verdict on I-03:** OTel observation mode is viable and powerful. The correlation problem IS solvable for HTTP-based agents via W3C Trace Context — this is genuinely zero-code-change. For non-HTTP agents, environment variable configuration (agentevals-dev approach) or sequential execution are fallbacks. The user's assumption A-03 holds for the common case (HTTP agents with OTel instrumentation).

---

### I-04: Flexible Scoring/Grading System

**Terminology landscape** (Stream 2):

The research is unambiguous: naming the "component that measures" is the **most fragmented term in the domain**:
- Grader (Anthropic)
- Scorer (Inspect AI, Braintrust)
- Evaluator (LangChain, Arize)
- Metric (DeepEval, RAGAS)
- Assertion (Promptfoo)
- Check (Deepchecks)
- Judge (when LLM-based)

The research concludes: "A new entrant could gain clarity advantage by choosing consistent, well-documented terminology."

**Recommendation based on research:** "Scorer" is the most neutral term. "Grader" implies a teacher-student relationship. "Evaluator" is overloaded (refers to both the component AND the person). "Metric" conflates the measurement with the measurer. "Assertion" is too test-framework-specific. **"Scorer" clearly denotes "a thing that produces a score" without implying domain.**

For the generic core, "Scorer" also works because it doesn't carry AI-specific connotations.

**Score types in the wild:**
- Continuous 0-1: DeepEval metrics, RAGAS metrics
- Binary pass/fail: Promptfoo assertions, Anthropic regression evals
- Labels/categories: classification tasks, taxonomy labels
- Numeric metrics: latency, token count, cost (from OTel spans)
- Multi-dimensional: DeepEval runs multiple metrics per test case, each returning one score

**Single vs. multi-score (Q-05):**
The dominant pattern is: **each scorer returns one score, but multiple scorers run per test case.** This is what DeepEval, RAGAS, Inspect AI, and Promptfoo all do. The multi-scorer pattern is simpler to reason about:
- Each scorer is a pure function: input → score
- Composition happens by running multiple scorers
- Results are a flat list of (scorer_name, score) pairs

However, there IS a case for multi-score returns: when computing multiple related scores requires the same expensive computation (e.g., one LLM call that evaluates coherence, relevance, and completeness simultaneously — Anthropic warns against this as "monolithic scoring" but acknowledges the cost pressure).

**Recommendation:** Default to single-score-per-scorer. Allow multi-score as an opt-in capability for performance-critical cases (batch related scores in one computation). The API should support both without making the common case complex.

**Verdict on I-04:** Well-supported by research. "Scorer" is the recommended term. Single-score-per-scorer as default, multi-score as opt-in. The score type system (continuous, binary, label, metric) should be an enum/trait, not hardcoded.

---

### I-05: Result Comparison and Analysis

**Current state** (Streams 3, 5):
- Comparing results across experiments is a documented pain point (Stream 3, Job 3)
- CI/CD threshold calibration is trial-and-error (Stream 3, Job 4)
- Braintrust and LangSmith have experiment comparison dashboards, but these are platform features
- Libraries (DeepEval, RAGAS) have minimal comparison features — they output scores, comparison is external

**Statistical rigor gap** (Stream 5, #7):
- Most eval tools report simple pass rates and averages
- No confidence intervals, significance tests, or proper statistical treatment
- Only Agentrial (16 stars) addresses multi-trial statistics (Wilson confidence intervals, Fisher exact tests)
- This is identified as the #1 gap in the synthesis (Gap 1: Critical unmet need)

**Relevance to user's vision:**
The user described two comparison modes:
1. Historical comparison (trend over time) — no tool does this well without a platform
2. Baseline comparison (main branch vs. PR) — Promptfoo and DeepEval support CI/CD gating but without statistical rigor

Both modes require either stored results (deferred by user) or a way to serialize and compare result files. The user's approach (serializable output, caller handles storage) is compatible — comparison can work on two result files.

**Verdict on I-05:** Research strongly supports this. Statistical rigor in comparison is a critical gap. The user's approach (serializable results, external comparison) is a reasonable starting point. The library should produce structured, serializable result objects that support comparison operations.

---

### I-06: Post-Processing / Transform Pipeline

**Nothing in the research directly addresses this.** No existing eval tool has a formal "transform pipeline" between output acquisition and grading.

The closest patterns:
- Inspect AI's "Solver" concept includes processing steps (tools, transforms) before scoring — but this is integrated into their execution model
- agentevals-dev extracts "invocations" from traces — this IS a transform (raw spans → structured invocations) but it's hardcoded, not user-extensible
- DeepEval's `@observe` decorator captures structured data — but transforms happen in user code, not the framework

The user's use cases (Excalidraw JSON → PNG, test execution → pass/fail) suggest transforms that are:
- Domain-specific (the framework can't know how to render Excalidraw)
- Potentially expensive (rendering, test execution)
- Optional (many use cases don't need them)

**Recommendation:** Model transforms as an optional step between output acquisition and scoring. The API should be: `Output → (optional) Transform → TransformedOutput → Scorer`. The transform is a user-provided function, not a framework feature. This keeps the framework generic while supporting the use case.

**Verdict on I-06:** Not addressed by existing tools. Should be a lightweight, optional API — a function slot between acquisition and scoring, not a pipeline engine.

---

### I-07: Multi-Language Support via Rust Core

**Restate's approach** (web search during this phase):

Restate uses a Rust shared core (`restate-sdk-shared-core`) with language-specific bindings:
- **Rust SDK**: Direct cargo dependency
- **Python SDK**: PyO3 + maturin (native extension)
- **TypeScript SDK**: WASM via wasm-bindgen
- **Go SDK**: WASM via wazero (pure-Go WASM runtime)
- **Java/Kotlin**: Independent implementation (no shared core)

This is NOT FFI (C ABI). They chose **WASM as the portable compilation target** for JS and Go, and **PyO3** for Python (better performance than WASM).

**Implications for eval framework:**
The Restate pattern is proven and well-suited:
1. Rust core library — the eval primitives
2. Python bindings via PyO3 — critical given Python ecosystem dominance (Stream 6, Constraint 4)
3. TypeScript bindings via WASM — important for Node.js AI apps
4. Binary+JSON protocol for custom scorers in any language — proven by agentevals-dev

This is more robust than pure binary protocol for library-mode integrations. Users get native-feeling SDKs in their language, not subprocess-based APIs.

**Python SDK is non-negotiable** (Stream 6, Constraint 4): "Python is the dominant language for AI/ML. Not having a Python SDK is disqualifying for most use cases." The binary protocol alone isn't sufficient — Python users expect `pip install` + `import eval_lib` + decorator/function API.

**Verdict on I-07:** Restate's approach (Rust core + PyO3 for Python + WASM for TS/Go) is the proven path. Binary+JSON protocol is complementary for custom scorers. The user's instinct about Rust + multi-language is sound, and Restate proves it works at scale.

---

## Hypotheses Cross-Referenced

### H-01: Evaluation is generic (doesn't require AI-specific abstractions at core)

**Research evidence:**

Partially supports, with a significant caveat.

The fundamental loop (input → blackbox → output → grade) IS domain-agnostic. But AI evaluation has structural patterns that don't reduce trivially:

1. **Multi-turn conversations**: Input isn't a single value — it's a sequence of messages. Output is also a sequence. The scorer needs to understand turn structure. (Stream 2: the `messages` array format is the lingua franca)

2. **Trajectory evaluation**: The "output" of an agent isn't just the final answer — it's the entire sequence of steps (tool calls, reasoning, intermediate results). Scoring trajectory requires understanding ordered sequences. (Stream 2, Stream 4)

3. **Reference-free evaluation**: Many AI evals have NO reference/expected output. LLM-as-judge evaluates quality without a ground truth. The user's model (input + output + reference → score) doesn't always apply — sometimes it's just (input + output → score). (Stream 4, Decision 2)

4. **Non-determinism**: The same input produces different outputs across runs. This is fundamental to AI evaluation and affects how results are aggregated. (Stream 5, #2)

**Assessment:** The generic core CAN work if:
- "Input" is a generic type (could be a string, a message sequence, or anything)
- "Reference" is optional (not all evals have expected outputs)
- "Output" is a generic type (could be text, a trace, a file, an execution result)
- Scoring function signature is `(input: T, output: U, reference: Option<V>) → Score`

The AI-specific concepts (message arrays, trajectories, LLM-as-judge) would be concrete implementations of these generic types and scorer functions.

**Verdict:** H-01 holds as a design principle. The generic core is viable if types are sufficiently flexible. The risk (A-05) is that generic abstractions require so much type-level flexibility that ergonomics suffer for the common (AI) case. AI-specific convenience layers on top are essential.

---

### H-02: Existing eval tools are too opinionated about execution

**Research strongly supports this.**

Evidence:
- Stream 5 documents lock-in as a pain point across multiple tools (LangSmith pricing lock-in, DeepEval Confident AI coupling, Promptfoo now OpenAI-owned)
- Stream 9 shows single-provider tools failed (Approach 5: "Framework-agnostic tools won")
- Stream 3 documents multi-tool workflows as the norm — teams use 3-5 tools because no single tool fits all modes
- agentevals-dev's OTel approach was called "quite genius" by the user precisely because it decouples evaluation from execution

The structural insight: most eval tools conflate two concerns:
1. How to get the output (execution)
2. How to grade the output (evaluation)

The user wants to separate these. The research validates that users are already doing this ad hoc (cobbling together multiple tools for different execution contexts).

**Verdict:** H-02 is well-supported. Execution coupling is a real problem, and the market has validated demand for decoupled approaches (agentevals-dev's OTel-only approach, the multi-tool reality).

---

### H-03: A single low-level library can support all execution modes

**Untested — no existing tool does this.**

The closest:
- Promptfoo supports CLI + library + CI/CD — but not OTel observation mode
- Inspect AI supports library + CLI + sandbox — but not OTel or lightweight library mode

The decomposition that makes this viable (from I-02 analysis):
1. **Grading layer** (shared): `(input, output, reference?) → Score` — identical across modes
2. **Acquisition layer** (per-mode): how `output` is obtained
   - Library mode: call a function, get return value
   - Binary mode: run a subprocess, capture stdout/response
   - OTel mode: receive traces, extract output from spans
   - Agent harness: manage agent lifecycle, collect output

The grading layer is trivially shared. The acquisition layer has genuinely different concerns per mode. The question is whether the "output" type that flows from acquisition to grading can be sufficiently generic without losing information.

**Verdict:** H-03 is plausible but unproven. The decomposition is sound in theory. The risk is that different acquisition modes produce fundamentally different "output" shapes (text vs. trace vs. file) that make the grading layer's generic interface unwieldy.

---

### H-04: OTel spans are sufficient as evaluation input for many use cases

**Supported with documented limitations.**

From Stream 4 (Pattern 4, OTel-Native):
- Gain: vendor-agnostic, no re-execution cost, standardized
- Give up: "trace data may not capture everything needed for evaluation (e.g., internal model reasoning)"
- Breaks when: "evaluation requires information not captured in traces"

The user's own use cases confirm the limitation:
- Blueprint writer (OTel sufficient — markdown in LLM spans)
- Excalidraw agent (OTel insufficient — need the actual JSON artifact + rendered PNG)
- GitHub issue agent (OTel insufficient — need to run the generated test)

**Verdict:** H-04 holds. OTel is sufficient for evaluation when the relevant output data appears in spans. When the output is an external artifact, OTel alone isn't enough. This is well-understood and the user already accounts for it.

---

### H-05: Rust is a viable and advantageous choice

**Partially supported, with a clear mitigation path.**

The research says Python SDK is non-negotiable (Stream 6, Constraint 4). But:
- Restate proves Rust core + PyO3/WASM bindings works at scale
- ripgrep, tree-sitter, and other Rust CLI tools are widely adopted via binary distribution
- The Rust AI ecosystem is small but growing (adk-eval, evalframe show activity)

The advantage of Rust:
- Performance: eval runs with many samples benefit from fast execution
- Binary distribution: single binary, no dependency management (vs. Python's pip/venv complexity)
- OTel: Rust has solid OTel libraries (opentelemetry-rust)
- Reliability: no runtime errors from dynamic typing — important for eval infrastructure

The risk:
- Python users can't write scorers as Python functions in-process unless PyO3 bindings exist
- Community contributions are harder in Rust than Python
- AI ecosystem tools (LLM client libraries, etc.) are more mature in Python

**Verdict:** H-05 holds if Python access is solved. Restate's approach is the proven path. Rust for the core is a competitive advantage, not a liability, IF the Python story is strong.

---

## Technical Opinions Cross-Referenced

### T-01: Eval API must be AI-agnostic

**Supported as design principle.** No existing tool does this, which is both the opportunity and the risk. See H-01 analysis.

The key question is whether generic types (`T` for input, `U` for output) can express AI patterns ergonomically. The answer is probably yes in Rust (strong type system, traits, generics) — less certain in dynamically typed languages where the generic interface may feel untyped.

### T-02: Rust for core implementation

**Supported.** See H-05 analysis. Wide-open space in Rust (no significant competition).

### T-03: Binary+JSON for multi-language scorers

**Supported and proven** by agentevals-dev. Their custom evaluator pattern (binary that reads JSON from stdin, writes JSON to stdout) works well for external scorers.

**Enhancement:** For library-mode integrations, consider PyO3 (Python) and WASM (TS/Go) in addition to binary protocol. This is the Restate approach and provides native-feeling SDKs.

### T-04: OTel should not be a hard requirement

**Supported.** Stream 6 shows OTel is the standard for observability, but not all eval use cases involve observability. Library-mode evaluation (call a function, grade the output) doesn't need OTel. Making OTel mandatory would alienate users who just want a scoring library.

**Recommendation:** OTel as a first-class integration, not a dependency. The core library has zero OTel dependency. An `otel` feature/module provides the observation mode.

### T-05: Storage is deferred

**Supported by the failure archaeology.** Stream 9 shows that projects that tried to be platforms too early (Log10) failed. Starting with serializable output and letting patterns emerge is prudent.

**Research note:** No standard format for eval results exists (Q-07). JSONL is the most common for results logging. The `gen_ai.evaluation.result` event in OTel is not standardized yet. A serializable result struct in a well-defined schema (JSON Schema?) would be a contribution to the ecosystem.

### T-06: Pre-built scorer catalog not important initially

**Supported with a caveat.** Stream 9 (Approach 1) shows metrics-only libraries fail — but that's about having ONLY metrics with no framework. The user isn't building a metrics-only library; they're building a framework with extensibility. Custom scorers matter more than a catalog for the initial target user (the user themselves).

**Caveat from Stream 5:** "Metric commoditization" (Barrier 4) means core metrics are easy to implement and don't differentiate. The long-term value is in the framework, not the scorers. But for adoption beyond the initial user, having 5-10 common scorers (exact match, contains, regex, LLM-as-judge, embedding similarity) significantly lowers the barrier.

---

## Assumptions Cross-Referenced

### A-01: Core eval loop separable from execution — LOAD-BEARING

**Status: Supported with conditions.**

The decomposition (acquisition layer + grading layer) works if:
1. The "output" flowing from acquisition to grading is a well-defined generic type
2. The grading layer doesn't need to know HOW the output was produced
3. Context from the acquisition process (timing, OTel attributes, metadata) can be passed alongside the output without coupling

Condition 2 may be violated in some cases: trajectory scoring (Stream 2) requires knowing the sequence of steps, which is an artifact of HOW the agent executed. But this can be modeled as "the output IS the trajectory" rather than "the grading layer knows about execution." This is a design choice that preserves the separation.

**Verdict:** Supported. The separation holds if "output" is flexible enough to include traces/trajectories as first-class output types.

### A-02: Binary+JSON sufficient for multi-language — MODERATE RISK

**Status: Partially supported.**

Binary+JSON works for custom scorers (agentevals-dev proves it) and for CLI tool usage. It does NOT work well for library-mode integrations where users want:
- `pip install evallib` → `from evallib import Scorer`
- Decorators, composable functions, IDE autocomplete

For this, native bindings (PyO3 for Python, WASM for TS) are needed. Binary+JSON is complementary, not sufficient alone.

**Verdict:** A-02 needs refinement. Binary+JSON is sufficient for custom scorers and CLI mode. For library-mode in Python/TS, native bindings are needed.

### A-03: Zero-code-change OTel correlation — LOAD-BEARING

**Status: Supported for HTTP-based agents.**

Research during this phase confirms: W3C Trace Context propagation (`traceparent` header) provides zero-code-change correlation for HTTP-based agents:
- Eval framework sets trace ID in `traceparent` header
- Standard OTel HTTP instrumentation automatically propagates the trace ID
- All agent spans inherit the trace ID
- Eval framework queries backend by trace ID

This requires:
- Agent accepts HTTP requests (common for API-based agents)
- Agent uses standard OTel HTTP instrumentation (common when OTel is configured)
- Agent doesn't override trace context propagation (rare)

For non-HTTP agents (CLI tools, queue-based, etc.):
- Environment variable approach (agentevals-dev style) — requires config change, not code change
- Sequential execution with time-window collection — zero-change but slow and fragile

**Verdict:** A-03 holds for the common case (HTTP agents with OTel). For edge cases, minor configuration changes are needed but not code changes.

### A-04: Rust-based tool can gain adoption in Python ecosystem — SIGNIFICANT RISK

**Status: Risk acknowledged, mitigation path exists.**

Stream 6, Constraint 4: "Not having a Python SDK is disqualifying for most use cases."

Mitigation: Restate proves the Rust core + PyO3/WASM pattern works. But this requires **building and maintaining Python/TS SDKs** — significant ongoing effort for a solo developer.

The alternative: if the tool is primarily CLI-based (like Promptfoo), Python SDK is less critical. Users interact via CLI + YAML/JSON config, not Python imports. This may be the pragmatic path for initial adoption.

**Verdict:** A-04 is a real risk. The mitigation (PyO3 bindings) is proven but effort-intensive. Prioritizing CLI + binary protocol for initial release, with Python bindings as a fast follow, may be the right sequencing.

### A-05: Generic abstractions support AI eval well — MODERATE RISK

**Status: Probably holds, but needs careful design.**

AI eval patterns that need accommodation in the generic core:
1. Optional reference (not all evals have ground truth) — model reference as `Option<T>`
2. Multi-turn inputs (message sequences) — model input as generic type that can be a sequence
3. Traces as output (trajectory evaluation) — model output as generic type including structured trace data
4. Non-determinism (multiple trials) — model at the run level, not the scoring level

The risk is ergonomics: `Scorer<Input = Vec<Message>, Output = TraceData, Reference = Option<Vec<Message>>>` is correct but ugly. AI-specific type aliases and convenience functions are essential.

**Verdict:** A-05 holds if the generic core is paired with AI-specific convenience layers. Without those layers, the generic types will feel like fighting the framework.

---

## Questions Answered

### Q-01: How does an eval process look like exactly?

Based on research (Streams 3, 4), the low-level steps are:

**Integrated mode:**
1. Load sample dataset (inputs + optional references)
2. For each sample:
   a. Feed input to the target system (function call, API request, etc.)
   b. Collect output (return value, response, traces)
   c. (Optional) Transform output (render, execute, extract)
   d. Run scorer(s): `(input, output, reference?) → Score`
   e. Record result: `(sample_id, scores, metadata, timing)`
3. Aggregate results across samples (means, pass rates, distributions)
4. (Optional) Compare to baseline or threshold
5. Output structured result (serialize to JSON/JSONL)

**Observation mode:**
1. Load sample dataset (inputs + optional references)
2. For each sample:
   a. Send input to the target system with correlation context (traceparent)
   b. Wait for response
   c. Collect traces from OTel backend by trace ID
   d. Extract structured output from traces (tool calls, messages, etc.)
   e. Run scorer(s) on extracted output
   f. Record result
3. Aggregate and output (same as integrated)

### Q-02: What are the execution modes called?

Based on research terminology (Streams 2, 4), recommended naming:

| Mode | Description | Research parallel |
|------|-------------|-------------------|
| **Inline** | Framework calls target function in-process | Library/Framework pattern (Stream 4) |
| **Subprocess** | Framework runs target as external process | CLI-Driven pattern |
| **Observe** | Framework receives traces from independently-running target | OTel-Native pattern |
| **Harness** | Framework manages target lifecycle (agent runner) | Research/Sandbox pattern |

Alternative: "Direct" instead of "Inline". "Passive" instead of "Observe". But "Observe" best captures the structural difference (the framework is a receiver, not an orchestrator).

### Q-03: Scorers? Graders? Evaluators?

**Recommendation: "Scorer"** (see I-04 analysis above).

Full terminology recommendation for the framework:
- **Sample**: A single test case (input + optional reference). Neutral, used by RAGAS.
- **Dataset**: Collection of samples.
- **Scorer**: Component that produces a score. Generic, used by Inspect AI + Braintrust.
- **Score**: The result of scoring (value + type + metadata).
- **Result**: Complete output of evaluating one sample (all scores + metadata).
- **Run**: One complete evaluation pass over a dataset.
- **Trial**: One execution of a target system on one sample (for multi-trial scenarios).

This avoids AI-specific terms at the core level and aligns with the most neutral terminology in the research.

### Q-04: Features not mentioned?

Yes — several features from the research that the user didn't mention:

1. **Multi-trial execution with statistical aggregation** (Stream 5 #7, Gap 1): Running the same sample N times and aggregating results with confidence intervals. This is the #1 gap in the synthesis. The user's framework could include this as a run-level feature (run each sample K times, aggregate statistically).

2. **Pairwise comparison scoring** (Stream 4): Instead of scoring one output, compare two outputs and judge which is better. Used for model comparison and prompt A/B testing. Structurally different from single-output scoring.

3. **Dataset generation** (Stream 3, Job 2): Synthetic test data generation. Low priority per user's scope, but worth noting as a future layer.

4. **Human-in-the-loop calibration** (Stream 3, workflow): Human labeling to calibrate automated scorers. Important for LLM-as-judge. Requires a review interface — deferred with platform.

5. **Evaluation of evaluators (meta-evaluation)** (Stream 7): How do you know your scorer is good? Correlation with human judgments. Important for trust but advanced.

6. **Cost tracking** (Stream 5 #3): Evaluation itself costs money (LLM calls for scoring). Tracking and optimizing eval cost is a real concern.

### Q-05: Single vs. multi-score?

See I-04 analysis. **Single-score-per-scorer as default, multi-score as opt-in.** The dominant pattern in the research is multiple scorers per sample, each returning one score.

### Q-06: OTel correlation without code changes?

See A-03 analysis. **W3C Trace Context (`traceparent` header) for HTTP agents. Sequential execution or env var configuration as fallbacks.**

### Q-07: Storage format?

No standard exists. JSONL is the most common for streaming results. A well-defined JSON schema for the result struct would be a contribution. Consider:
- One JSONL line per sample result
- Run-level metadata in a separate file or header
- Score values with type information (continuous, binary, label)
- Timing and cost metadata

### Q-08: OTel architecture (ingest point vs. collector vs. trace store query)?

Three viable approaches, each with tradeoffs:

| Approach | Pros | Cons | Who does it |
|----------|------|------|-------------|
| **Built-in OTLP receiver** | Simple setup, all-in-one | Reinventing the wheel, scalability | agentevals-dev |
| **OTel Collector integration** | Standard, scalable, flexible pipeline | More setup, external dependency | Langfuse, Phoenix |
| **Query trace store** | Works with existing infra, no agent config change | Requires a trace backend (Jaeger/Tempo), latency | Novel approach |

**Recommendation:** Support both OTLP receiver (simple case) and trace store query (enterprise case). The OTLP receiver can be a lightweight embedded component for development. For production, querying an existing trace store (Jaeger, Grafana Tempo) via their APIs is more practical.

For the traceparent correlation approach, querying the trace store by trace ID after the response is the cleanest pattern — the eval framework doesn't need to be an OTLP receiver at all. It just queries the backend.

---

## External Dependencies

| Dependency | Likelihood of holding | Impact if it doesn't | User's bet |
|------------|----------------------|---------------------|------------|
| OTel GenAI conventions stabilize | High (strong momentum, many adopters) | OTel observation mode needs frequent updates | Moderate bet — OTel is the right standard |
| Rust OTel libraries remain maintained | High (opentelemetry-rust is CNCF) | OTel integration becomes harder | Low risk |
| PyO3 continues to work well | High (mature, widely used) | Python bindings require alternative approach | Low risk |
| Agent frameworks keep emitting OTel | High (trend is toward OTel adoption) | OTel observation mode has fewer targets | Low risk |
| Python remains dominant for AI | Very high | Rust-only adoption limited to niche | Not a bet — the user accounts for this |

No high-risk external dependencies identified. The technology bets (Rust, OTel, PyO3) are all on stable, well-maintained foundations.

---

## Competitive Positioning

**Who else could build this?**

- **agentevals-dev**: Has the OTel-native approach but is Python-only and OTel-only. Could expand, but they're optimizing for a different target (SaaS platform feel).
- **DeepEval team (Confident AI)**: Has the community and metrics library. Could make it more generic. But they're optimizing for their cloud platform revenue, not a generic library.
- **Langfuse**: Has OTel integration and observability. Could add eval library features. But they're an observability platform, not an eval library.
- **A Rust AI company**: Could build this. But Rust AI companies are few and focused on inference (Candle, Burn), not evaluation.

**Why haven't they?**
- Python dominance means Rust isn't the obvious choice for an eval tool
- Most builders pick an execution model and optimize for it (easier to ship)
- "Generic eval" conflicts with AI-specific marketing and fundraising narratives
- The layered architecture requires more upfront design investment
- The intersection of Rust + AI + eval + systems design is an unusual skill set

**Is this defensible?**
- Rust + generic + multi-mode is defensible because it requires unusual expertise
- The architectural vision (acquisition/grading separation, multi-mode, OTel-native) is non-trivial to replicate
- It is NOT a "anyone with a weekend" project — the design challenge is real
- BUT: if it gains traction, Python-native alternatives could replicate the architecture in Python more quickly. The Rust advantage is performance and binary distribution, not the architecture.

**Verdict:** The positioning is defensible for the medium term. The biggest competitive risk is a Python-native tool replicating the multi-mode architecture with better Python ergonomics.

---

## Complexity & Layering Assessment

### Core Technical Problem
Designing type-level abstractions in Rust that are generic enough to support multiple input/output types, multiple scoring types, and multiple execution modes — while remaining ergonomic for the common case (AI evaluation with text input/output and continuous scoring).

### Known-Hard vs. Unknown-Hard
**Known-hard.** The evaluation loop is well-understood. The multi-mode architecture is a design challenge, not a research problem. OTel integration is documented. The hard part is API design, not algorithmic novelty.

### Smallest Useful Layer
Rust library with:
- `Sample<I, R>` (input + optional reference)
- `Scorer<I, O, R>` trait (function from input + output + reference → Score)
- `Score` enum (Continuous(f64), Binary(bool), Label(String), Metric { name, value })
- `Result` struct (sample + scores + metadata)
- `Run` struct (results + aggregates)
- Inline execution mode only (call a function, score the output)

This is buildable in days. It has standalone value as a typed evaluation primitive library.

### What It Enables
Once this core exists:
- Binary/CLI mode: wrap core in a CLI that reads samples from JSONL, calls a subprocess, scores output
- OTel observation mode: add OTel trace collection, extract output from spans, feed to core scorers
- Agent harness: add a simple agent runner that feeds samples to an LLM, collects output, scores
- Python bindings: PyO3 wrapper around core types and scorers
- Statistical aggregation: multi-trial runner that uses core scoring
- Comparison: diff two Run results

### Full Vision Scope
Large: core library + CLI + OTel mode + agent harness + Python/TS bindings + statistical aggregation + comparison + CI/CD gating + storage + dashboards. This is a multi-year project at full scope.

### Incremental Path
Yes — clear incremental path:
1. Core library (Rust, inline mode) — weeks
2. CLI binary (subprocess mode) — weeks
3. OTel observation mode — weeks
4. Python bindings (PyO3) — weeks
5. Statistical multi-trial — weeks
6. Result comparison + CI/CD gating — weeks
7. Agent harness — weeks
8. Storage, dashboards — months

Each layer has standalone value. No layer requires the next to be useful.

### Complexity Multipliers
- Multi-language bindings maintenance (PyO3, WASM) — ongoing effort
- OTel convention evolution — requires tracking spec changes
- Scorer type flexibility — Rust generics can get complex; trait bounds may surprise
- Serialization format design — JSON Schema for interoperability

### Prior Art Quality
Extensive prior art in Python to learn from. No Rust prior art to build on. The Python tools provide clear patterns for scoring APIs, result formats, and CI/CD integration. The Rust implementation will need to translate these patterns, not invent them.
