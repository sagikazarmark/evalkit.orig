# Stream 7 — Trajectory & Emerging Solutions

## Sources

- https://www.langchain.com/state-of-agent-engineering (LangChain State of AI Agents 2026)
- https://www.goodeyelabs.com/insights/llm-evaluation-2025-review
- https://hai.stanford.edu/news/stanford-ai-experts-predict-what-will-happen-in-2026
- https://www.ibm.com/think/news/ai-tech-trends-predictions-2026
- https://www.technologyreview.com/2026/01/05/1130662/whats-next-for-ai-in-2026/
- https://arxiv.org/html/2508.02994v1 (Agent-as-a-Judge)
- https://opentelemetry.io/blog/2025/ai-agent-observability/
- https://openai.com/index/openai-to-acquire-promptfoo/
- https://www.promptfoo.dev/blog/promptfoo-joining-openai/
- https://humanloop.com/home (Anthropic acquisition)
- https://awesomeagents.ai/tools/best-llm-eval-tools-2026/

---

## Active Trends

### 1. Consolidation: Big AI Companies Acquiring Eval Startups
- **What's happening**: Major AI companies are buying evaluation tools to build integrated platforms. OpenAI acquired Promptfoo (March 2026, 23 employees, valued at $86M). Anthropic is acquiring Humanloop. Snowflake backs TruLens.
- **Evidence**: OpenAI announcement (March 2026), Humanloop "joining Anthropic" announcement, TruLens Snowflake integration
- **Who's driving it**: OpenAI, Anthropic, Snowflake, Databricks (via MLflow)

### 2. Evaluation + Security Convergence
- **What's happening**: Quality evaluation and red-teaming/security testing are merging into unified tools. The insight: if you're already running LLM-as-judge evaluations, adding security probes uses the same infrastructure.
- **Evidence**: OpenAI's Promptfoo acquisition (eval+security in one), DeepTeam by Confident AI (companion to DeepEval), Giskard v3 combining eval+vulnerability scanning
- **Who's driving it**: OpenAI (Promptfoo), Confident AI (DeepEval+DeepTeam), Giskard, enterprise compliance requirements

### 3. OpenTelemetry as the Universal AI Observability Standard
- **What's happening**: OTel GenAI Semantic Conventions are becoming the standard for AI instrumentation. Tools that support OTLP can interoperate with the entire observability ecosystem.
- **Evidence**: Langfuse, Arize Phoenix, Laminar, Datadog, agentevals-dev all adopting OTel. OTel blog on AI agent observability (2025).
- **Who's driving it**: OpenTelemetry community, observability vendors (Datadog, New Relic), AI-native tools (Langfuse, Phoenix)

### 4. Agent Evaluation Methodology Crystallizing
- **What's happening**: After years of ad-hoc approaches, consensus is forming around how to evaluate agents: multi-trial execution, trajectory-level evaluation, outcome-focused grading, specialized graders per dimension.
- **Evidence**: Anthropic's comprehensive eval guide (2025), LangChain readiness checklist, TruLens Agent GPA framework, DeepEval agent metrics
- **Who's driving it**: Anthropic, LangChain, Confident AI, Arize

### 5. The Rise of Open-Source Eval Platforms (Not Just Libraries)
- **What's happening**: Open-source projects are evolving from simple libraries to full platforms with UIs, dashboards, experiment tracking, and deployment. Self-hosted alternatives to commercial platforms are viable.
- **Evidence**: Langfuse (24.3k stars), Opik (18.6k stars) — both full platforms with self-hosting. Inspect AI adding Inspect View web UI.
- **Who's driving it**: Langfuse, Opik (Comet), Arize Phoenix, community demand for data sovereignty

### 6. Evaluation Maturity Models Emerging
- **What's happening**: The industry is codifying evaluation maturity — from "vibe checking" (Level 0) to continuous production evaluation with automated red-teaming (Level 4). This creates a roadmap for teams to follow.
- **Evidence**: Braintrust maturity model (Levels 0-4), LangChain readiness checklist (6 phases), Anthropic eval guide stages
- **Who's driving it**: Braintrust, LangChain, Anthropic

---

## Emerging Projects & Approaches

### Agentrial — Statistical Agent Evaluation
- **What's new**: pytest-like framework that runs agents N times with Wilson confidence intervals, step-level failure attribution via Fisher exact tests, and CUSUM/Page-Hinkley drift detection. Addresses the statistical rigor gap.
- **How mature**: Very early — 16 stars on GitHub
- **URL**: https://github.com/alepot55/agentrial

### Mocktopus — Deterministic LLM API Mocking
- **What's new**: Drop-in replacement for OpenAI/Anthropic APIs enabling deterministic, zero-cost testing. Addresses the non-determinism problem in CI/CD.
- **How mature**: Very early — 6 stars on GitHub
- **URL**: https://github.com/evalops/mocktopus

### Galileo Luna-2 — Specialized Evaluation Models
- **What's new**: Small language models specifically trained for evaluation tasks, claiming 97% cost reduction vs. using large models as judges while maintaining evaluation quality.
- **How mature**: In production at Galileo. Enterprise customers (HP, Twilio, Reddit).
- **URL**: https://galileo.ai/

### agentevals-dev — OTel-Native Agent Evaluation
- **What's new**: Evaluates agent behavior from recorded OpenTelemetry traces without re-executing agents. Framework-agnostic via OTel standard.
- **How mature**: Early — 112 stars, active development
- **URL**: https://github.com/agentevals-dev/agentevals

### Bloom — Dynamic Behavioral Evaluation
- **What's new**: Generates evaluation suites dynamically for probing specific LLM behaviors (sycophancy, self-preservation, political bias). Seed-based reproducibility with variation dimensions.
- **How mature**: Early — 1.3k stars, Anthropic safety research origin
- **URL**: https://github.com/safety-research/bloom

### Agent-as-a-Judge (Research)
- **What's new**: Research framework where sophisticated LLM judges operate as agents — reading documents, running code, browsing web — to evaluate other agents. Meta-evaluation of whether agent-judges can replace humans.
- **How mature**: Research paper stage (2025). Deepchecks implements a version.
- **URL**: https://arxiv.org/html/2508.02994v1

---

## Paradigm Shifts

### Shift 1: From Benchmark-Driven to Application-Driven Evaluation
- **From**: Evaluating models against standardized academic benchmarks (MMLU, HumanEval)
- **To**: Evaluating applications against custom, domain-specific test suites built from production data
- **Why**: Benchmark saturation, data contamination, poor correlation with production quality
- **How far along**: Well underway. Production teams have largely abandoned relying solely on benchmarks. But academic community still benchmark-centric.

### Shift 2: From Output Evaluation to Trajectory Evaluation
- **From**: Evaluating only the final output of an AI system
- **To**: Evaluating the entire trajectory — reasoning steps, tool calls, intermediate decisions
- **Why**: Agent-based AI systems require understanding HOW they arrived at an answer, not just WHAT they produced
- **How far along**: Early-middle. LangChain AgentEvals, DeepEval, Arize have trajectory metrics. But tooling is still immature.

### Shift 3: From Evaluation as Testing to Evaluation as Continuous Practice
- **From**: Running evals before deployment (like QA testing)
- **To**: Continuous evaluation throughout the lifecycle — development, CI/CD, production, feedback loop
- **Why**: AI systems degrade in production due to data drift, model updates, changing user behavior
- **How far along**: Middle. 89% have observability, 52% have evals, 44.8% have online evals (LangChain survey).

### Shift 4: From Single-Provider to Multi-Provider Evaluation
- **From**: Evaluating within one AI provider's ecosystem
- **To**: Framework-agnostic evaluation across providers and frameworks
- **Why**: Teams use multiple models and frameworks; can't afford provider lock-in for evaluation
- **How far along**: Well underway in tools (most support 10+ providers). OpenTelemetry accelerates this.

---

## AI/LLM Impact

### How AI is Currently Applied in This Domain
1. **LLM-as-a-Judge**: The dominant evaluation paradigm — using LLMs to evaluate other LLMs. Present in nearly every eval tool.
2. **Synthetic Test Data Generation**: RAGAS knowledge-graph approach, DeepEval synthetic generation, LLM-based dataset creation.
3. **Automated Error Analysis**: LLM-assisted annotation and pattern discovery in failure traces.
4. **Agent-as-Evaluator**: AWS Agent Evaluation, AutoGen AgentEval — LLM agents conducting evaluations autonomously.
5. **Specialized Evaluation Models**: Galileo Luna-2 — SLMs trained specifically for evaluation tasks, reducing cost by 97%.

### Opportunities for AI-Native Approaches
1. **Adaptive Evaluation**: LLMs that automatically generate new evaluation criteria based on observed failure patterns, without manual specification.
2. **Cross-Model Meta-Evaluation**: Using one model to evaluate the evaluation quality of another — meta-evaluation at scale.
3. **Natural Language Evaluation Specification**: Define evaluation criteria in plain English, have AI generate the corresponding evaluation logic.
4. **Intelligent Production Sampling**: AI-driven selection of which production traces to evaluate, rather than random sampling.
5. **Automated Dataset Curation**: AI that continuously improves evaluation datasets by identifying gaps, removing redundancies, and adding edge cases.

### Risks and Limitations
1. **Circular Validation**: Using AI to evaluate AI creates circular reasoning — models have blind spots that may be shared.
2. **Evaluation Cost Spiral**: As AI evaluators get more sophisticated, evaluation costs approach or exceed the cost of the system being evaluated.
3. **Over-Automation**: Risk of teams trusting AI evaluation without human calibration, leading to false confidence.
4. **Benchmark Gaming**: As models are optimized to pass evaluations, they may learn to game evaluation prompts rather than genuinely improve.

---

## Bets & Predictions

Based on evidence gathered:

1. **OpenTelemetry GenAI Semantic Conventions will become mandatory** — Within 12-18 months, any AI eval/observability tool without OTel support will be at a significant disadvantage. The OTel ecosystem effect is too strong.

2. **Consolidation will accelerate** — Expect 3-5 more acquisitions of eval tools by AI companies and cloud providers in 2026-2027. The standalone eval tool market will shrink.

3. **Agent evaluation will standardize** — The Anthropic/LangChain approach (multi-trial, trajectory-level, outcome-focused, specialized graders) will become the de facto standard within 12 months.

4. **Specialized evaluation models will commoditize** — Galileo's Luna-2 approach will be replicated. Small, cheap, purpose-built evaluation models will replace expensive GPT-4/Claude judges for common metrics.

5. **The quality+safety+observability trinity will unify** — Currently three separate tool categories (eval, red-teaming, observability) will converge into integrated platforms. Early signs: OpenAI+Promptfoo, Opik's guardrails, Giskard v3.

6. **Evaluation-as-Code will become standard** — Declarative evaluation definitions (like Promptfoo YAML or DeepEval pytest) will become as standard as unit testing. CI/CD integration will be table stakes.

7. **The "eval gap" (89% observability vs. 52% evals) will close** — As tooling improves and evaluation becomes easier, the adoption gap will narrow significantly. Observability-first tools adding eval features will drive this.
