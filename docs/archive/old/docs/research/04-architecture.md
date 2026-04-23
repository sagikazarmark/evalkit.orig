> **📦 Archived on 2026-04-23** — superseded by no successor — this work was abandoned. Kept for historical reference.

# Stream 4 — Architecture & Technical Patterns

## Sources

- https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents
- https://learn.microsoft.com/en-us/ai/playbook/technology-guidance/generative-ai/working-with-llms/evaluation/list-of-eval-metrics
- https://arxiv.org/html/2507.21504v1 (Evaluation and Benchmarking of LLM Agents Survey)
- https://arxiv.org/html/2508.02994v1 (Agent-as-a-Judge)
- https://www.goodeyelabs.com/insights/llm-evaluation-2025-review
- https://stackshala.medium.com/the-six-hardest-problems-in-llm-output-evaluation-79058a2433e0
- https://newsletter.pragmaticengineer.com/p/evals
- https://blog.langchain.com/agent-evaluation-readiness-checklist/
- Project documentation from Stream 1 landscape analysis

---

## Architectural Patterns

### Pattern 1: Library/Framework Pattern
- **Who uses it**: DeepEval, RAGAS, OpenEvals, AgentEvals, TruLens, EleutherAI harness, Inspect AI
- **How it works**: Python (or TypeScript) library imported into user's codebase. Evaluation logic runs in-process or via CLI. Users write eval scripts that compose datasets, metrics, and models. Results logged locally or to optional cloud service.
- **Tradeoffs**:
  - **Gain**: Maximum flexibility, no vendor dependency, easy CI/CD integration, composable metrics
  - **Give up**: No built-in observability, collaboration features, or production monitoring. Team-scale features require adding another tool.
- **When it breaks**: When teams need collaboration (multiple people reviewing results), production monitoring, or when non-technical stakeholders need to participate. Also breaks at scale without proper infrastructure for result storage and comparison.

### Pattern 2: Platform/SaaS Pattern
- **Who uses it**: Braintrust, LangSmith, Humanloop, Maxim AI, Galileo, Confident AI
- **How it works**: Managed cloud service with SDKs for instrumentation. Data flows from user's application → platform API → storage → dashboard. Evaluation runs on platform infrastructure or client-side with results uploaded.
- **Tradeoffs**:
  - **Gain**: Collaboration, dashboards, experiment comparison, production monitoring, managed infrastructure, RBAC/compliance
  - **Give up**: Vendor lock-in, data leaves your environment, ongoing costs that scale with usage, less flexibility for custom evaluation logic
- **When it breaks**: When data residency requirements prevent cloud upload, when costs scale faster than value, when custom evaluation logic doesn't fit platform abstractions.

### Pattern 3: Observability-First with Eval Bolted On
- **Who uses it**: Langfuse, Arize Phoenix, Opik, Helicone, Laminar
- **How it works**: Primary focus is tracing/observability — capturing all LLM interactions as traces with spans. Evaluation added as a layer on top: LLM-as-judge evaluators run on collected traces, either in real-time or batch.
- **Tradeoffs**:
  - **Gain**: Rich production data, comprehensive tracing, natural feedback loop from production to evaluation
  - **Give up**: Evaluation is secondary — fewer pre-built metrics, less sophisticated eval-specific features
- **When it breaks**: When teams need deep evaluation capabilities (complex agent trajectory analysis, synthetic data generation, benchmark suites) — observability tools typically can't match dedicated eval frameworks.

### Pattern 4: OpenTelemetry-Native Pattern
- **Who uses it**: agentevals-dev, TruLens (v2.7+), Langfuse, Phoenix, Laminar
- **How it works**: Instrumentation uses OpenTelemetry SDK. Traces are sent via OTLP (HTTP/gRPC) to receivers. Evaluation performed on collected OTel traces without re-executing the AI system.
- **Tradeoffs**:
  - **Gain**: Vendor-agnostic observability, evaluate from recorded traces (no re-execution cost), standardized semantic conventions for GenAI, interoperability across tools
  - **Give up**: OTel overhead in instrumentation, trace data may not capture everything needed for evaluation (e.g., internal model reasoning), OTel GenAI semantic conventions still evolving
- **When it breaks**: When evaluation requires information not captured in traces (e.g., model confidence scores, internal chain-of-thought), or when OTel instrumentation overhead is unacceptable.

### Pattern 5: Evaluator-as-Agent Pattern
- **Who uses it**: AWS Agent Evaluation, AutoGen AgentEval, Deepchecks (Agent-as-a-Judge)
- **How it works**: The evaluator itself is an LLM agent that conducts conversations with the target agent, generates evaluation criteria, and scores responses. Multiple specialized agents (Critic, Quantifier, Verifier in AutoGen) collaborate on evaluation.
- **Tradeoffs**:
  - **Gain**: Flexible evaluation that adapts to the target system, can discover failure modes not pre-defined, natural for multi-turn agent evaluation
  - **Give up**: High LLM costs (evaluator makes many LLM calls), non-deterministic evaluation results, harder to debug evaluator behavior, "who evaluates the evaluator?" problem
- **When it breaks**: When cost is a concern, when reproducibility is critical, when evaluation results need to be deterministic for CI/CD gates.

### Pattern 6: Declarative/Config-Driven Pattern
- **Who uses it**: Promptfoo, EleutherAI harness, ContextCheck
- **How it works**: Evaluations defined in YAML/JSON configuration files. No or minimal code required. CLI tool reads config, executes evaluations, outputs results. Configuration specifies prompts, providers, test cases, and assertions.
- **Tradeoffs**:
  - **Gain**: Low barrier to entry, non-developers can contribute, easy to version control alongside prompts, CI/CD friendly
  - **Give up**: Limited expressiveness for complex evaluation logic, harder to implement custom metrics, config files can become unwieldy for large test suites
- **When it breaks**: When evaluation logic needs complex conditional branching, when custom preprocessing or postprocessing is required, when test cases need to be generated dynamically.

### Pattern 7: Research/Sandbox Pattern
- **Who uses it**: Inspect AI, HELM, EleutherAI harness
- **How it works**: Evaluations run in isolated environments (Docker, Kubernetes, VM). Agents are given access to sandboxed tools (bash, python, web browser). Evaluation measures what the agent does in the sandbox environment.
- **Tradeoffs**:
  - **Gain**: Safe evaluation of powerful agents, reproducible environments, can evaluate computer-use and code-execution agents
  - **Give up**: Complex infrastructure setup, slower execution, harder to integrate with production systems, overhead of containerization
- **When it breaks**: When evaluating lightweight LLM applications that don't need sandboxing — overhead is disproportionate. Also breaks when sandbox doesn't accurately replicate production environment.

---

## Technical Decisions That Recur

### Decision 1: LLM-as-Judge vs. Deterministic Evaluation
- **Options**: Use LLM to judge outputs (flexible, handles nuance) vs. Code-based assertions (deterministic, cheap, fast)
- **What most projects choose**: Both. Layered approach: deterministic checks first (format, schema, keyword presence), then LLM-as-judge for subjective quality. DeepEval, Promptfoo, Arize, and LangSmith all support both.
- **Why**: Neither alone is sufficient. Deterministic checks catch obvious failures cheaply; LLM judges handle nuance but are expensive and non-deterministic.

### Decision 2: Reference-Based vs. Reference-Free Evaluation
- **Options**: Compare against ground truth / golden answers vs. Evaluate quality without references
- **What most projects choose**: Support both, with reference-free becoming more common for agent evaluation where "correct paths" aren't predefined.
- **Why**: Reference-based evaluation is more reliable but requires expensive dataset creation. Agent evaluation often has no single correct answer.

### Decision 3: Which LLM to Use as Judge
- **Options**: Same model family as target, different model family, specialized small evaluation model (Galileo Luna-2)
- **What most projects choose**: Different model family recommended (avoid "grading your own test" bias). GPT-4/GPT-4o most commonly used as judges.
- **Why**: Research shows models systematically favor outputs from their own family. Using a different family reduces bias.

### Decision 4: Single Score vs. Multi-Dimensional Evaluation
- **Options**: Single composite score vs. Multiple specialized metrics
- **What most projects choose**: Multi-dimensional. Anthropic recommends specialized graders per dimension. DeepEval has separate metrics for plan quality, tool correctness, step efficiency, etc.
- **Why**: Single scores hide failure modes. A high overall score can mask critical failures in specific dimensions.

### Decision 5: Online vs. Offline Evaluation
- **Options**: Evaluate during development (offline, on curated datasets) vs. Evaluate in production (online, on live traffic)
- **What most projects choose**: Both, layered. Offline first (lower barrier), online added later. LangChain survey: 52% have offline evals, 44.8% have online evals.
- **Why**: Offline evals catch regressions before deployment; online evals catch issues with real-world data distribution.

### Decision 6: Python vs. Multi-Language Support
- **Options**: Python-only vs. Python + TypeScript/JavaScript
- **What most projects choose**: Python primary, TypeScript secondary. DeepEval, RAGAS, Inspect AI, TruLens = Python only. OpenEvals, AgentEvals, Promptfoo, Langfuse = multi-language.
- **Why**: Python dominates ML/AI ecosystem. But TypeScript is increasingly important for AI applications in web/Node.js environments.

### Decision 7: Self-Hosted vs. Cloud-Only
- **Options**: Cloud-only SaaS vs. Self-hosted option vs. Local-only
- **What most projects choose**: Trend toward offering both. Langfuse, Opik = self-hostable open source. Braintrust, LangSmith = cloud with self-hosted options. Promptfoo, DeepEval = local-first with optional cloud.
- **Why**: Data residency, cost control, and compliance requirements drive self-hosting demand. But managed services reduce operational burden.

### Decision 8: Trace-Based vs. Re-Execution Evaluation
- **Options**: Evaluate from recorded traces (no re-execution) vs. Re-run the AI system for each evaluation
- **What most projects choose**: Most frameworks re-execute (DeepEval, RAGAS). Trace-based evaluation emerging (agentevals-dev, TruLens, Langfuse online evals).
- **Why**: Re-execution gives full control but is expensive. Trace-based is cheaper but limited to what was captured. Agent evaluation increasingly favors trace-based approaches due to cost.

---

## Anti-Patterns & Cautionary Tales

### Anti-Pattern 1: "Vibe Checking"
- Modify prompt, try a few inputs, deploy if it "looks good." No systematic evaluation.
- **Why it fails**: Non-deterministic outputs mean a few good examples don't guarantee quality. Regressions go undetected.
- **Source**: Pragmatic Engineer newsletter, Anthropic eval guide

### Anti-Pattern 2: Over-Reliance on LLM-as-Judge
- Using LLM judges for everything, including cases where deterministic checks would suffice.
- **Why it fails**: Expensive, slow, non-deterministic, introduces bias. LLM judges systematically favor their own model family's outputs.
- **Source**: 2025 Year in Review for LLM Evaluation (Goodeye Labs)

### Anti-Pattern 3: Benchmark Score as Quality Proxy
- Assuming high scores on MMLU/HumanEval/GPQA translate to good production performance.
- **Why it fails**: Benchmark saturation, memorization, and dataset contamination mean scores don't predict real-world quality. Models scoring 90%+ on MMLU fail on novel production queries.
- **Source**: Sebastian Raschka "State of LLMs 2025"

### Anti-Pattern 4: Single-Trial Evaluation
- Running one trial and declaring the change improved (or regressed) quality.
- **Why it fails**: Agent behavior is highly variable. A single trial doesn't capture the distribution of outcomes. Need multiple trials with statistical aggregation.
- **Source**: Anthropic eval guide, LangChain readiness checklist

### Anti-Pattern 5: Grading the Path, Not the Outcome
- Evaluating whether the agent took the exact same steps as a reference trajectory.
- **Why it fails**: Many valid paths exist to the same goal. Strict trajectory matching penalizes creative/efficient solutions.
- **Source**: LangChain readiness checklist ("grade the outcome, not the exact path")

### Anti-Pattern 6: Using Same Model Family as Judge
- Evaluating GPT-4 outputs using GPT-4 as judge, or Claude outputs using Claude as judge.
- **Why it fails**: Self-enhancement bias — models rate their own family's outputs higher. Creates blind spots for shared failure modes.
- **Source**: Amazon Prime Video pattern, research on LLM judge biases

### Anti-Pattern 7: Confusing Guardrails with Evaluation
- Treating runtime safety checks (guardrails) as a substitute for systematic evaluation.
- **Why it fails**: Guardrails are enforcement (block/filter). Evaluation is measurement (understand quality). Both are needed but serve different purposes.
- **Source**: Anthropic eval guide

### Anti-Pattern 8: Monolithic Scoring
- Using a single prompt to have an LLM judge evaluate multiple dimensions simultaneously.
- **Why it fails**: LLMs applying many criteria in one pass tend to prioritize some while ignoring others. Decompose into specialized graders per dimension.
- **Source**: DeepEval multi-metric approach, Anthropic "specialized graders" recommendation
