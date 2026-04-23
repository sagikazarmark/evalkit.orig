> **📦 Archived on 2026-04-23** — superseded by [Stream 5 — Pain Points & Challenges](../../../docs/research/05-pain-points.md). Kept for historical reference.

# Stream 5 — Pain Points & Challenges

## Sources

- https://www.goodeyelabs.com/insights/llm-evaluation-2025-review
- https://stackshala.medium.com/the-six-hardest-problems-in-llm-output-evaluation-79058a2433e0
- https://newsletter.pragmaticengineer.com/p/evals
- https://blog.langchain.com/agent-evaluation-readiness-checklist/
- https://www.langchain.com/state-of-agent-engineering
- https://www.honeyhive.ai/post/avoiding-common-pitfalls-in-llm-evaluation
- https://zenml.io/blog/deepeval-alternatives
- https://github.com/confident-ai/deepeval/issues
- https://github.com/langfuse/langfuse/issues
- https://github.com/orgs/langfuse/discussions/5007
- https://arxiv.org/html/2508.02994v1
- https://pmc.ncbi.nlm.nih.gov/articles/PMC12149859/

---

## Project-Specific Pain Points

### DeepEval
- **Heavy LLM-as-judge dependency** — Nearly all metrics require LLM inference, making evaluation expensive and slow. Each test case triggers another LLM call, compounding costs for large test suites. — Source: ZenML blog "DeepEval Alternatives" — Severity: blocker (at scale)
- **Python-only** — No native TypeScript/JavaScript SDK. Teams building Node.js AI applications must bridge languages or use a different tool. — Source: comparison articles — Severity: annoyance
- **Large context handling issues** — Evaluation with large retrieval contexts (33k+ tokens) produces fragmented, low-quality evaluation reasoning. — Source: GitHub Discussion #1763 — Severity: blocker (for RAG systems with large contexts)
- **Tight Confident AI cloud coupling for team features** — Dashboards, dataset management, production monitoring, and collaboration all require the proprietary Confident AI platform. — Source: architecture analysis — Severity: annoyance
- **Python version compatibility** — Version pinning issues prevent using DeepEval with newer Python versions. — Source: GitHub Issue #1435 — Severity: annoyance

### Langfuse
- **Evaluation context selection** — LLM-as-Judge evaluator only picks the last span of a specific type, potentially evaluating the wrong context. Can leak answers into evaluation. — Source: GitHub Issue #8151 — Severity: blocker
- **Filter limitations** — Only AND operations supported between filter conditions. No OR filtering or observation-type-based filtering in evaluator setup. — Source: GitHub discussions — Severity: annoyance
- **Evaluation is secondary** — Fewer pre-built metrics and evaluation features compared to eval-focused tools. Non-LLM evaluators (classifiers, regex) still on roadmap. — Source: 2025 Roadmap Discussion #5007 — Severity: real pain
- **Variable mapping errors** — Evaluation runs can fail silently when variable mapping doesn't find trace properties. — Source: GitHub Discussion #10093 — Severity: annoyance

### Promptfoo
- **Not Python-native** — TypeScript/Node.js tool in a Python-dominated ecosystem. Python AI engineers must learn YAML config or use the JS API. — Source: comparison articles — Severity: annoyance
- **Prompt-level focus** — Primarily designed for prompt testing rather than full agent/pipeline evaluation. Less suited for complex multi-step agent evaluation. — Source: architecture analysis — Severity: real pain (for agent teams)
- **OpenAI acquisition uncertainty** — March 2026 acquisition by OpenAI raises concerns about vendor neutrality. May gradually favor OpenAI models or ecosystem. — Source: community discussion — Severity: annoyance (potential future concern)

### LangSmith
- **Pricing at scale** — $2.50-5/1k traces adds up quickly for production systems. Teams with high-volume applications face significant costs. — Source: pricing page, community feedback — Severity: real pain
- **Perceived LangChain lock-in** — Despite claiming framework-agnosticism, best experience is with LangChain/LangGraph. Non-LangChain users feel like second-class citizens. — Source: comparison articles — Severity: real pain
- **Closed source** — Cannot self-host core platform without enterprise agreement. — Source: architecture analysis — Severity: blocker (for some orgs)

### Braintrust
- **Closed-source core** — Platform code is proprietary. Can't inspect, modify, or self-host without enterprise deal. — Source: architecture analysis — Severity: blocker (for some orgs)
- **Pricing complexity** — Multiple tiers, usage-based pricing on spans/scores/storage, 14-30 day data retention limits on lower tiers. — Source: pricing page — Severity: annoyance

### Inspect AI
- **No production monitoring** — Designed for research evaluation, not production observability. No trace collection, alerting, or monitoring features. — Source: architecture analysis — Severity: real pain (for production teams)
- **Python-only** — No TypeScript support, limiting adoption for web-based AI applications. — Source: project documentation — Severity: annoyance
- **Steep learning curve for non-researchers** — Solver/Scorer/Dataset abstraction is unfamiliar to application developers coming from pytest/testing backgrounds. — Source: community — Severity: annoyance

### RAGAS
- **RAG-specific tunnel vision** — Deep RAG evaluation but limited general-purpose or agent evaluation capabilities. Teams need additional tools for non-RAG use cases. — Source: comparison articles — Severity: real pain
- **No platform features** — No experiment tracking, artifact storage, lifecycle management, or collaboration features. Just a metric library. — Source: Confident AI comparison — Severity: annoyance

### agentevals (agentevals-dev)
- **Tiny community** — 112 stars, limited documentation, few examples. Risk of abandonment. — Source: GitHub — Severity: blocker
- **In-memory storage** — No persistence by default. Evaluation sessions lost when process exits. — Source: architecture analysis — Severity: real pain

---

## Domain-Wide Challenges

### 1. The LLM-as-Judge Reliability Problem
- **Description**: LLM judges are the backbone of most evaluation frameworks, but they have systematic biases: self-enhancement bias (favoring own model family), verbosity bias (longer = better), position bias, and limited mathematical/logical reasoning.
- **Who is affected**: Everyone using automated evaluation
- **Workarounds**: Use different model family for judging, calibrate against human judgments, use multiple judges and aggregate, combine with deterministic checks
- **Source**: Goodeye Labs 2025 Review, Frontiers research paper

### 2. Non-Determinism in Agent Evaluation
- **Description**: Agent behavior is highly variable — the same agent can take different paths, use different tools, and produce different outcomes on the same task. Single-trial evaluation is statistically meaningless.
- **Who is affected**: Anyone evaluating AI agents
- **Workarounds**: Run multiple trials (10-50+), use statistical aggregation (Wilson confidence intervals), use pass@k/pass^k metrics
- **Source**: Anthropic eval guide, Agentrial project

### 3. Cost of Comprehensive Evaluation
- **Description**: Running evaluation suites is expensive — each test case may require multiple LLM calls (target + judge + multiple metrics). Full agent evaluation suites can cost hundreds of dollars per run.
- **Who is affected**: All teams, especially startups and smaller organizations
- **Workarounds**: Tiered evaluation (cheap deterministic first, expensive LLM-judge selectively), cached evaluations, smaller judge models (Galileo Luna-2), evaluate from traces instead of re-executing
- **Source**: ZenML DeepEval alternatives, community feedback

### 4. Benchmark Saturation and Contamination
- **Description**: Popular benchmarks (MMLU, HumanEval) have reached saturation — models score near-human performance. Data contamination (training on benchmark data) inflates scores further. High benchmark scores don't predict production quality.
- **Who is affected**: Anyone using benchmarks for model selection
- **Workarounds**: Use newer, harder benchmarks (GPQA, SWE-bench), create custom benchmarks from production data, supplement with application-specific evals
- **Source**: Sebastian Raschka State of LLMs 2025, Goodeye Labs review

### 5. Evaluation of Open-Ended Tasks
- **Description**: Many AI tasks (creative writing, research, complex reasoning) have no single correct answer. Reference-based evaluation fails. LLM judges struggle with truly subjective quality.
- **Who is affected**: Teams building creative AI tools, research agents, conversation agents
- **Workarounds**: Reference-free LLM-as-judge with detailed rubrics, pairwise comparison, human evaluation for calibration, multi-dimensional scoring
- **Source**: Six Hardest Problems article, research papers

### 6. Fragmented Tooling / Multi-Tool Tax
- **Description**: No single tool handles the full evaluation lifecycle (error analysis → dataset creation → evaluation → CI/CD → production monitoring → feedback loop). Teams use 3-5 different tools, requiring integration work.
- **Who is affected**: All teams at scale
- **Workarounds**: Choose a platform that covers most needs (Braintrust, LangSmith), build custom integrations, accept some tool overlap
- **Source**: Stream 3 workflow analysis

### 7. Lack of Statistical Rigor
- **Description**: Most eval tools report simple pass rates and averages. No confidence intervals, significance tests, or proper statistical treatment of non-deterministic results. Only Agentrial (16 stars) addresses this.
- **Who is affected**: Teams needing reliable CI/CD quality gates, production monitoring alerts
- **Workarounds**: Custom statistical analysis scripts, multiple trial runs, manual confidence interval calculation
- **Source**: Agentrial project, Anthropic eval guide

### 8. Dataset Creation and Maintenance Burden
- **Description**: Creating high-quality evaluation datasets requires deep domain expertise, is time-consuming, and datasets go stale as products evolve. Synthetic generation can produce unrealistic scenarios.
- **Who is affected**: All teams
- **Workarounds**: Start small (20-50 hand-crafted examples), use production failures as test cases, synthetic data generation (RAGAS, DeepEval), trace-to-dataset pipelines (Braintrust)
- **Source**: LangChain readiness checklist, Pragmatic Engineer

---

## Unmet Needs

### 1. Deterministic Testing Infrastructure for LLM Applications
- **Description**: A way to run unit tests against LLM applications with deterministic, reproducible results — similar to mocking in traditional software testing. Only Mocktopus (6 stars) attempts this.
- **Evidence**: Mocktopus project (6 stars), widespread complaints about flaky eval-based CI/CD

### 2. Cross-Framework Agent Evaluation
- **Description**: Evaluate agents built with any framework (LangGraph, CrewAI, AutoGen, OpenAI Agents SDK, custom) using a single evaluation tool, without framework-specific adapters.
- **Evidence**: Most eval tools are coupled to specific frameworks. agentevals-dev attempts this via OpenTelemetry but is nascent.

### 3. Unified Quality + Safety Evaluation
- **Description**: A single tool/pipeline that handles both quality evaluation (is the output good?) and safety evaluation (is the output safe?) rather than requiring separate tools.
- **Evidence**: Quality tools (DeepEval, RAGAS) and safety tools (Garak, Promptfoo red-teaming) are completely separate. OpenAI's Promptfoo acquisition hints at convergence.

### 4. Non-Technical Stakeholder Evaluation Interface
- **Description**: An accessible interface for domain experts, PMs, and non-technical stakeholders to create evaluation criteria, review results, and provide feedback without writing code.
- **Evidence**: Humanloop and Deepchecks (no-code evaluator builder) partially address this. Most tools are developer-only.

### 5. Real-Time Adaptive Evaluation in Production
- **Description**: Production monitoring that automatically identifies anomalous traces, evaluates them, and surfaces issues without manual configuration of what to evaluate.
- **Evidence**: Current production evals require manual setup of sampling rates, evaluation criteria, and alert thresholds. No tool does intelligent adaptive evaluation.

### 6. Multi-Modal Evaluation at Application Level
- **Description**: Evaluate AI applications that combine text, code, tool calls, images, and audio in a single coherent evaluation framework.
- **Evidence**: lmms-eval handles multi-modal model benchmarks. No tool handles multi-modal application evaluation well.

---

## Workaround Patterns

### 1. Custom Data Viewers
- **What people do**: Build custom UIs (React/Streamlit) to view conversation traces with sufficient context (tool calls, retrieved docs, reasoning) on a single screen
- **What it compensates for**: Generic observability tools don't show enough context for error analysis

### 2. Spreadsheet Annotation
- **What people do**: Export traces to Google Sheets/Excel, manually annotate failure modes, categorize issues
- **What it compensates for**: No standard methodology or tooling for systematic error analysis

### 3. Dataset Format Converters
- **What people do**: Write scripts to convert between tool-specific dataset formats (DeepEval JSON, Promptfoo YAML, LangSmith datasets, CSV)
- **What it compensates for**: No standard dataset format across eval tools

### 4. Threshold Calibration Notebooks
- **What people do**: Build Jupyter notebooks to calibrate LLM-as-judge scores against human labels, adjust thresholds for CI/CD gates
- **What it compensates for**: No built-in calibration workflow in most eval tools

### 5. Multi-Trial Wrapper Scripts
- **What people do**: Write wrapper scripts to run evaluations N times and aggregate results with confidence intervals
- **What it compensates for**: Most eval tools run single trials without statistical analysis

### 6. Production-to-Dataset Pipelines
- **What people do**: Build custom ETL scripts to extract failed production traces and format them as test cases
- **What it compensates for**: No automated feedback loop from production failures to evaluation datasets (except Braintrust's Trace-to-Dataset feature)
