> **📦 Archived on 2026-04-23** — superseded by no successor — this work was abandoned. Kept for historical reference.

# Stream 1 — Landscape: Existing Projects & Products

> Research date: 2026-04-02
> Domain: AI Evaluation — tools, frameworks, and platforms for evaluating AI model outputs, agent behavior, correctness, and performance

## Sources Consulted

- Project homepages, documentation, and GitHub repositories (see URLs per project)
- GitHub Topics: `llm-evaluation`, `ai-evaluation`, `eval-framework`, `llm-testing`
- Awesome Lists: `Vvkmnn/awesome-ai-eval`, `onejune2018/Awesome-LLM-Eval`
- Comparison articles: aimultiple.com/llm-eval-tools, confident-ai.com/blog/greatest-llm-evaluation-tools-in-2025, arize.com/llm-evaluation-platforms-top-frameworks
- Individual project READMEs and release pages

---

## Must-Include Projects

---

### Inspect AI (UK AISI)

- **URL**: https://inspect.aisi.org.uk/
- **Repository**: https://github.com/UKGovernmentBEIS/inspect_ai — ~1.9k stars, 449 forks, active development
- **Category**: Research-grade LLM evaluation framework
- **One-line description**: Open-source framework for evaluating LLMs across coding, reasoning, agentic tasks, knowledge, behavior, and multimodal understanding.
- **Architecture**: Python library + CLI + web viewer (Inspect View) + VS Code extension. Evaluations consist of Datasets, Solvers, and Scorers composed via `@task` decorator.
- **Core features**:
  - 100+ pre-built evaluations (via companion `inspect_evals` repo)
  - Sandboxing (Docker, Kubernetes, Modal, Proxmox)
  - Built-in agents, multi-agent primitives, external agent support
  - MCP tool integrations
  - Web-based Inspect View for result visualization
  - Support for 15+ model providers (OpenAI, Anthropic, Google, AWS Bedrock, Azure, etc.)
- **Unique differentiator**: Government-backed, research-grade evaluation framework with the largest pre-built eval collection, designed for AI safety assessment with sandboxed execution environments.
- **Strengths**: Comprehensive eval library, sandboxed execution for agentic tasks, strong multi-model support, well-documented, backed by UK AI Safety Institute. Active community with contributions from Arcadia Impact and Vector Institute.
- **Weaknesses**: Primarily research-oriented — less focus on production monitoring/observability. Python-only. No commercial support or cloud platform.
- **Maturity**: Growing — active development, expanding eval library, regular releases. Relatively new but rapidly maturing.
- **Direction**: Expanding eval collection, improving multi-agent evaluation support, deepening MCP integration.
- **Pricing/License**: Open source (MIT-style license)
- **Notable users**: UK AI Safety Institute, AI safety research community

---

### Braintrust

- **URL**: https://www.braintrust.dev/
- **Repository**: Closed source (SaaS platform)
- **Category**: AI observability and evaluation platform (commercial)
- **One-line description**: Enterprise-grade AI observability platform for building, testing, and monitoring AI applications with evaluation, tracing, and prompt management.
- **Architecture**: SaaS platform with SDKs (Python, TypeScript, Go, Ruby, C#). Built on proprietary Brainstore database optimized for nested AI traces.
- **Core features**:
  - Real-time trace inspection (prompts, responses, tool calls)
  - Side-by-side prompt comparison and experiment tracking
  - Multiple scoring methods: LLM-based, code-based, human review
  - CI/CD regression detection
  - Loop Agent for AI-assisted optimization of prompts and scorers
  - Trace-to-Dataset conversion (production failures → regression tests)
  - Custom Trace Views for task-specific annotation
  - MCP integration for IDE-based workflows
- **Unique differentiator**: Proprietary Brainstore database for high-performance trace operations; Loop Agent for AI-assisted prompt optimization; strong enterprise positioning with SOC 2 Type II / GDPR / HIPAA compliance.
- **Strengths**: Framework-agnostic, excellent enterprise features (SSO, RBAC, hybrid deployment), strong customer roster (Vercel, Notion, Coursera, Dropbox, Replit), generous free tier (1M spans/month).
- **Weaknesses**: Closed-source core platform, vendor lock-in risk, premium pricing at scale ($249/mo Pro tier).
- **Maturity**: Growing — well-funded startup, rapidly expanding feature set, strong enterprise adoption.
- **Direction**: Deepening observability, expanding Loop Agent capabilities, competing for enterprise AI platform market.
- **Pricing/License**: Free tier (1M trace spans/month, unlimited users, 10k eval runs), Pro ($249/month), Enterprise (custom).
- **Notable users**: Vercel, Notion, Coursera, Dropbox, Replit, Graphite, Navan

---

### agentevals (agentevals-dev)

- **URL**: https://github.com/agentevals-dev/agentevals
- **Repository**: 112 stars, 257 commits, Apache 2.0
- **Category**: Agent evaluation from OpenTelemetry traces
- **One-line description**: Framework-agnostic agent evaluation tool that scores AI agent behavior from OpenTelemetry traces without re-executing agents.
- **Architecture**: Local-first platform with OTLP HTTP receiver, embedded React web UI, REST API, MCP server, Docker/Helm support. No database required (in-memory session management).
- **Core features**:
  - Tool trajectory matching (exact, in-order, any-order modes)
  - Custom evaluators in Python, JavaScript, or any language
  - LLM-based judges via OpenAI Evals API
  - Zero-code OpenTelemetry receiver (HTTP/protobuf and HTTP/json)
  - Python SDK with session lifecycle management
  - MCP server integration (port 8080)
  - CLI for CI/CD pipelines
- **Unique differentiator**: Evaluates from recorded OTel traces without re-running expensive LLM calls. Fully local — no cloud dependencies. Framework-agnostic via OpenTelemetry standard.
- **Strengths**: Cost-efficient (no re-execution), local-first philosophy, OpenTelemetry-native, supports diverse frameworks (LangChain, Strands, Google ADK, OpenAI Agents SDK).
- **Weaknesses**: Very early stage (112 stars), small community, limited documentation, in-memory storage limits scalability.
- **Maturity**: Early — active development but small user base.
- **Direction**: Expanding evaluator library, deepening framework integrations.
- **Pricing/License**: Apache 2.0, fully open source
- **Notable users**: Not widely known yet

---

### LangChain AgentEvals

- **URL**: https://github.com/langchain-ai/agentevals
- **Repository**: 534 stars, 36 forks, 204 commits, 11 open issues
- **Category**: Agent trajectory evaluation library
- **One-line description**: Collection of evaluators for assessing agent performance with a focus on agent trajectory — intermediate steps and tool call sequences.
- **Architecture**: Python + TypeScript library. Designed to work with LangSmith for logging but usable standalone.
- **Core features**:
  - Agent trajectory match evaluators (strict, unordered, subset, superset)
  - LLM-as-Judge for trajectory accuracy
  - Graph trajectory evaluators (for LangGraph workflows)
  - Customizable tool argument matching
  - Both sync and async Python support
  - LangSmith integration for pytest/Vitest/Jest
- **Unique differentiator**: Deep integration with LangGraph for graph-based agent trajectory evaluation. Focus specifically on intermediate decision-making steps, not just final outputs.
- **Strengths**: Well-maintained by LangChain team, good TypeScript + Python support, practical evaluators for common agent patterns.
- **Weaknesses**: Tightly coupled to LangChain/LangGraph ecosystem, limited evaluator variety compared to broader frameworks, relatively small project.
- **Maturity**: Growing — part of larger LangChain ecosystem, actively maintained.
- **Direction**: Expanding evaluator types, deeper LangGraph integration.
- **Pricing/License**: Open source (MIT)
- **Notable users**: LangChain/LangGraph users

---

### LangChain OpenEvals

- **URL**: https://github.com/langchain-ai/openevals
- **Repository**: ~1k stars, 93 forks, 331 commits, 7 open issues
- **Category**: General-purpose LLM evaluation library
- **One-line description**: Readymade evaluators for LLM apps — LLM-as-judge, extraction, code analysis, string similarity, and agent trajectory matching.
- **Architecture**: Python + TypeScript library. Provider-agnostic via LangChain's universal chat model initialization.
- **Core features**:
  - LLM-as-Judge with prebuilt prompts (correctness, conciseness, helpfulness, safety, security)
  - Extraction & tool call validation (exact match + LLM-based)
  - Code analysis (Pyright, mypy, TypeScript type-checking)
  - Agent trajectory matching (strict, unordered, subset/superset)
  - String similarity (exact match, Levenshtein, embedding similarity)
  - Multi-modal support (text, images, voice — beta)
  - Sandboxed code execution
  - RAG-specific metrics
- **Unique differentiator**: Broad evaluator coverage in a lightweight library — combines LLM-as-judge, code analysis, and trajectory matching. Provider-agnostic.
- **Strengths**: Wide metric variety, multi-modal support, good documentation, provider-agnostic, LangSmith integration.
- **Weaknesses**: LangChain ecosystem dependency for best experience, newer project, some evaluators still in beta.
- **Maturity**: Growing — actively developed, part of LangChain ecosystem.
- **Direction**: Expanding evaluator library, improving multi-modal support.
- **Pricing/License**: Open source (MIT)
- **Notable users**: LangChain ecosystem users

---

### AWS Agent Evaluation (agenteval)

- **URL**: https://github.com/awslabs/agent-evaluation
- **Repository**: 354 stars, 48 forks, 276 commits, Apache 2.0, v0.4.1 (March 2025)
- **Category**: AWS-native agent testing framework
- **One-line description**: LLM-powered evaluator agent that orchestrates multi-turn conversations with target agents and evaluates responses during the conversation.
- **Architecture**: Python library/CLI. An evaluator LLM agent conducts conversations with the target agent and scores responses in real-time.
- **Core features**:
  - Concurrent multi-turn conversation orchestration
  - Real-time response evaluation during conversations
  - Hook system for integration testing
  - CI/CD pipeline integration
  - Built-in support for Amazon Bedrock, Amazon Q Business, SageMaker
  - Extensible for custom agents
- **Unique differentiator**: Evaluator-as-agent architecture — the evaluator itself is an LLM agent that conducts conversations, rather than just scoring static outputs. Deep AWS service integration.
- **Strengths**: Novel conversational evaluation approach, good AWS ecosystem integration, CI/CD ready, concurrent test execution.
- **Weaknesses**: AWS-centric (limited appeal outside AWS ecosystem), smaller community, evaluator-as-agent approach may have higher LLM costs.
- **Maturity**: Growing — active development with regular releases, but niche audience.
- **Direction**: Deeper AWS service integration, expanding evaluation capabilities.
- **Pricing/License**: Apache 2.0, open source
- **Notable users**: AWS customers using Bedrock/Q Business

---

### OpenAI Agent Evals / Evals Platform

- **URL**: https://developers.openai.com/api/docs/guides/agent-evals
- **Repository**: https://github.com/openai/evals (18.1k stars) — framework + benchmark registry. https://github.com/openai/simple-evals (4.4k stars) — lightweight benchmarks.
- **Category**: Platform-integrated evaluation system + open-source eval frameworks
- **One-line description**: OpenAI's integrated evaluation platform for measuring agent quality with reproducible evaluations, plus open-source eval frameworks and benchmark registry.
- **Architecture**: SaaS platform (Evals API) + open-source Python libraries. Three components: Trace Grading (workflow-level), Datasets (iterative improvement), Advanced Evals (external model comparison).
- **Core features**:
  - Trace grading for workflow-level error identification
  - Dataset management for continuous improvement cycles
  - Evaluation against external models
  - Prompt Optimizer (automatic prompt improvement via datasets)
  - Reproducible evaluation runs
  - Open-source benchmark registry (openai/evals)
  - Acquired Promptfoo for red-teaming/security evaluation (March 2026)
- **Unique differentiator**: Tightly integrated with OpenAI's model API and agent platform. Promptfoo acquisition signals eval+security convergence.
- **Strengths**: Direct integration with OpenAI models, massive open-source eval registry, Promptfoo acquisition for security testing, large community.
- **Weaknesses**: OpenAI-centric (evaluating other providers less natural), platform lock-in, simple-evals repo no longer updated for new models (as of July 2025).
- **Maturity**: Mature — long-established open-source repos, growing platform features.
- **Direction**: Converging evaluation with security/red-teaming (Promptfoo), deepening agent evaluation.
- **Pricing/License**: Open-source repos (MIT). Platform features included with OpenAI API access.
- **Notable users**: OpenAI API customers, research community

---

### Humanloop

- **URL**: https://humanloop.com/home
- **Repository**: Closed source (SaaS platform) — note: Humanloop is "joining Anthropic" (acquisition)
- **Category**: Enterprise LLM evaluation and prompt management platform
- **One-line description**: Evaluation platform for LLMs enabling teams to develop, evaluate, and observe AI systems with automated and human-in-the-loop review.
- **Architecture**: SaaS platform with three pillars: Develop (prompt editor + version control), Evaluate (automated + human assessments), Observe (monitoring + alerting).
- **Core features**:
  - Interactive prompt editor with version control
  - CI/CD integration to prevent regressions
  - AI and code-based automatic evaluations
  - Human review interfaces for subject matter experts
  - Online evaluations on live production data
  - User feedback capture
  - Tracing and logging for RAG systems
  - Alerting and guardrails
- **Unique differentiator**: Strong focus on domain expert involvement — enables non-technical stakeholders (PMs, domain experts) to participate in prompt engineering and evaluation. Being acquired by Anthropic.
- **Strengths**: Excellent collaboration features, human-in-the-loop evaluation, SOC-2 Type 2 / GDPR / HIPAA compliance, strong enterprise customer base (Gusto, Filevine, Dixa).
- **Weaknesses**: Closed-source, future uncertain with Anthropic acquisition, may become Anthropic-centric.
- **Maturity**: Mature — established enterprise platform, strong customer base. Transitioning via Anthropic acquisition.
- **Direction**: Integration with Anthropic's ecosystem.
- **Pricing/License**: Commercial SaaS. VPC deployment options available.
- **Notable users**: Gusto, Filevine, Dixa, FMG, Athena, Twain

---

### DeepEval (by Confident AI)

- **URL**: https://deepeval.com/
- **Repository**: https://github.com/confident-ai/deepeval — 14.4k stars, 1.3k forks, Apache 2.0
- **Category**: Comprehensive LLM evaluation framework
- **One-line description**: "Pytest for LLMs" — open-source evaluation framework with 14+ metrics covering RAG, agents, multi-turn conversations, and multimodal evaluation.
- **Architecture**: Python library with `@observe` decorator for tracing. Two-layer agent model: Reasoning Layer (LLM planning) and Action Layer (tool execution). Companion Confident AI cloud platform for team-scale features.
- **Core features**:
  - Comprehensive metrics: G-Eval, DAG, task completion, tool correctness, plan quality, plan adherence, argument correctness, step efficiency
  - RAG metrics: answer relevancy, faithfulness, contextual recall/precision
  - Multi-turn conversation metrics
  - Multimodal evaluation
  - Synthetic dataset generation
  - Automatic prompt optimization
  - Component-level tracing with `@observe`
  - CI/CD integration
  - Benchmark evaluation (MMLU, HellaSwag, GSM8K, etc.)
  - MCP server support
- **Unique differentiator**: Largest open-source metric library with research-backed evaluators. Separate reasoning and action layer evaluation for agents enables precise debugging. "Pytest for LLMs" developer experience.
- **Strengths**: 14.4k GitHub stars, 400k+ monthly downloads, excellent developer experience, comprehensive metric coverage, active development.
- **Weaknesses**: Confident AI cloud platform needed for team features (dashboards, dataset management, monitoring). Python-only.
- **Maturity**: Growing rapidly — high adoption, frequent updates, strong community.
- **Direction**: Expanding agentic evaluation metrics, deepening multimodal support, growing Confident AI platform.
- **Pricing/License**: DeepEval is Apache 2.0 (fully free). Confident AI cloud platform has free tier + paid plans.
- **Notable users**: Broad developer community, 20M+ evaluations reported

---

### Arize AI / Phoenix

- **URL**: https://arize.com/ai-agents/agent-evaluation/
- **Repository**: https://github.com/Arize-ai/phoenix — 9.1k stars (open-source component)
- **Category**: AI observability platform with evaluation capabilities
- **One-line description**: Enterprise AI observability and evaluation platform with open-source Phoenix library for tracing, evaluation, and production monitoring.
- **Architecture**: SaaS platform + open-source Phoenix library. Cyclical development process: create test cases → break down steps → build evaluators → experiment → monitor → revise.
- **Core features**:
  - Router evaluation (skill/function selection, parameter extraction)
  - Path evaluation (iteration counting, loop detection, trace inspection)
  - Convergence scoring (0-1 score for path efficiency)
  - Pre-built LLM-as-judge templates (tool calling, tool selection, parameter extraction, path convergence, agent planning, agent reflection)
  - RAG evaluation (retrieval relevance, QA correctness, hallucination detection)
  - Code generation evaluation
  - Phoenix OSS for notebooks and CI pipelines
- **Unique differentiator**: Strong ML observability heritage (pre-LLM), stress-tested evaluator templates tuned for 70-90% precision. Dual offering: commercial platform + open-source Phoenix.
- **Strengths**: Enterprise-grade observability, comprehensive agent evaluation templates, open-source Phoenix, strong ML roots.
- **Weaknesses**: Limited evaluation dataset support vs. eval-first competitors, primarily observability-focused with evaluation as add-on.
- **Maturity**: Mature — established company with enterprise customers, long ML observability track record.
- **Direction**: Deepening agent evaluation capabilities, expanding Phoenix OSS.
- **Pricing/License**: Commercial platform (pricing not publicly detailed). Phoenix is open source.
- **Notable users**: Enterprise customers across industries

---

### Microsoft AutoGen AgentEval

- **URL**: https://microsoft.github.io/autogen/0.2/blog/2024/06/21/AgentEval/
- **Repository**: Part of https://github.com/microsoft/autogen
- **Category**: Agent evaluation framework within AutoGen multi-agent platform
- **One-line description**: Framework for multi-dimensional utility assessment of LLM-powered applications using CriticAgent, QuantifierAgent, and VerifierAgent.
- **Architecture**: Three-agent system within AutoGen library. CriticAgent proposes evaluation criteria, QuantifierAgent measures performance, VerifierAgent ensures criteria robustness.
- **Core features**:
  - Automated evaluation criteria generation from task descriptions and examples
  - Multi-dimensional performance assessment
  - Criteria stability verification (redundancy elimination)
  - Discriminative power testing (adversarial examples)
  - Human-in-the-loop validation support
  - JSON-formatted conversation chain analysis
- **Unique differentiator**: Self-generating evaluation criteria — uses LLM agents to propose, measure, and verify evaluation criteria rather than requiring manual metric definition.
- **Strengths**: Novel approach to automated criteria generation, Microsoft backing, applicable to diverse domains.
- **Weaknesses**: Tightly coupled to AutoGen framework, criteria verification still under development, higher LLM costs (multiple agents evaluating).
- **Maturity**: Early/Growing — research-oriented, part of evolving AutoGen framework (now at v0.4+).
- **Direction**: Integration with AutoGen Studio for no-code solutions.
- **Pricing/License**: MIT (part of AutoGen)
- **Notable users**: Microsoft research, AutoGen users

---

## Broader Landscape — Additional Projects

## Category A: General-Purpose LLM Evaluation Frameworks

---

### 1. Promptfoo

- **URL**: https://promptfoo.dev
- **Repository**: https://github.com/promptfoo/promptfoo — 19.1k stars, MIT, last commit 2026-03-24
- **Category**: Developer-first LLM evaluation & red teaming CLI
- **One-line description**: CLI and library for testing prompts, agents, and RAGs with built-in red teaming and vulnerability scanning.
- **Architecture type**: CLI + local web dashboard; Node.js/TypeScript
- **Core features**:
  - Automated evaluation of prompts across models
  - Red teaming / vulnerability scanning for LLM apps
  - Side-by-side model comparison (OpenAI, Anthropic, Azure, Bedrock, Ollama, etc.)
  - CI/CD integration; live reload and caching
  - Cost tracking; runs locally (prompts stay private)
- **Unique differentiator**: Developer-first, local-first approach combining prompt evaluation AND security red teaming in a single tool. Battle-tested at scale (10M+ users in production apps).
- **Maturity**: Growing — very active development (v0.121.3 as of March 2026), strong adoption
- **Pricing/License**: MIT (open source); Enterprise tier available

---

### 2. Ragas

- **URL**: https://ragas.io
- **Repository**: https://github.com/explodinggradients/ragas — 13.2k stars, Apache-2.0, last commit 2026-01-13 (v0.4.3)
- **Category**: RAG-focused evaluation library
- **One-line description**: Toolkit for evaluating and optimizing RAG applications with objective metrics and synthetic test data generation.
- **Architecture type**: Python library
- **Core features**:
  - Objective metrics (LLM-based and traditional) for RAG pipelines
  - Automatic test dataset creation across diverse scenarios
  - Integration with LangChain and observability tools
  - Production data feedback loops
- **Unique differentiator**: Purpose-built for RAG evaluation; strong community around RAG-specific metrics (faithfulness, answer relevancy, context precision/recall). Transparent data collection practices.
- **Maturity**: Growing — widely adopted as the de facto RAG eval library
- **Pricing/License**: Apache-2.0 (open source)

---

### 3. EleutherAI LM Evaluation Harness

- **URL**: https://github.com/EleutherAI/lm-evaluation-harness
- **Repository**: 12k stars, MIT, last commit ~2025-12
- **Category**: Academic benchmark evaluation harness
- **One-line description**: Unified framework for evaluating generative language models on 60+ academic benchmarks with hundreds of subtasks.
- **Architecture type**: Python library/CLI with pluggable backends
- **Core features**:
  - 60+ standard academic benchmarks
  - Support for HuggingFace, vLLM, SGLang, commercial APIs
  - Multi-GPU evaluation (data parallelism, tensor sharding)
  - YAML-based config; Jinja2 prompt templating
  - LoRA, GGUF, GPTQ adapter evaluation
- **Unique differentiator**: Powers the HuggingFace Open LLM Leaderboard; cited in hundreds of research papers. The gold standard for academic model benchmarking.
- **Maturity**: Mature — foundational infrastructure for the open-source LLM community
- **Pricing/License**: MIT

---

### 4. OpenAI Evals

- **URL**: https://github.com/openai/evals
- **Repository**: 18.1k stars, MIT, 689 commits
- **Category**: LLM evaluation framework with benchmark registry
- **One-line description**: Framework for evaluating LLMs with an open-source registry of benchmarks for testing across multiple dimensions.
- **Architecture type**: Python framework with YAML/JSON templates
- **Core features**:
  - Registry of pre-made evaluations
  - Custom eval creation without coding (YAML/JSON)
  - Model-graded evaluation
  - W&B and Snowflake logging integration
  - Completion Function Protocol for chains/agents
- **Unique differentiator**: OpenAI's official eval framework; can now run evals directly in OpenAI Dashboard.
- **Maturity**: Mature but evolving — focus shifting to dashboard integration
- **Pricing/License**: MIT

---

### 5. OpenAI Simple-Evals

- **URL**: https://github.com/openai/simple-evals
- **Repository**: 4.4k stars, MIT
- **Category**: Lightweight benchmark reference implementation
- **One-line description**: Lightweight library for zero-shot, chain-of-thought evaluation of language models across standard benchmarks (MMLU, MATH, GPQA, etc.).
- **Architecture type**: Python scripts with sampler adapters
- **Core features**:
  - MMLU, MATH, GPQA, DROP, MGSM, HumanEval, SimpleQA, BrowseComp, HealthBench
  - Sampling interfaces for OpenAI and Anthropic
  - Comprehensive results table comparing 25+ models
- **Unique differentiator**: Simple, opinionated reference implementation. Zero-shot CoT only.
- **Maturity**: Declining — OpenAI announced in July 2025 it will no longer update for new models (maintaining only HealthBench, BrowseComp, SimpleQA)
- **Pricing/License**: MIT

---

### 6. HuggingFace LightEval

- **URL**: https://github.com/huggingface/lighteval
- **Repository**: 2.4k stars, MIT, last commit 2025-11-24 (v0.13.0)
- **Category**: Multi-backend LLM evaluation toolkit
- **One-line description**: All-in-one toolkit for evaluating LLMs across 1000+ tasks with support for multiple inference backends.
- **Architecture type**: Python library/CLI with pluggable backends
- **Core features**:
  - 1000+ evaluation tasks across domains and languages
  - Multiple backends: inspect-ai, Accelerate, vLLM, SGLang, TGI
  - Sample-by-sample result tracking
  - Custom task/metric creation
  - 200+ language support via Flores200
- **Unique differentiator**: HuggingFace's successor to `evaluate`; powers HF Leaderboards. Flexible backend support means models can be evaluated remotely or locally.
- **Maturity**: Growing — actively replacing HuggingFace's older `evaluate` library
- **Pricing/License**: MIT

---

### 7. HuggingFace Evaluate

- **URL**: https://github.com/huggingface/evaluate
- **Repository**: 2.4k stars, Apache-2.0, last update v0.4.6 (2025-09-18)
- **Category**: ML evaluation metrics library
- **One-line description**: Library of standardized metric implementations for NLP and CV model evaluation.
- **Architecture type**: Python library with community Hub integration
- **Core features**:
  - Dozens of popular metrics (NLP, CV)
  - Type checking, metric cards
  - Community-hosted metrics on HF Hub
  - CLI for creating evaluation modules
- **Unique differentiator**: Part of HuggingFace ecosystem; standardized metric interface.
- **Maturity**: Declining — README directs users to LightEval for LLM evaluation
- **Pricing/License**: Apache-2.0

---

### 8. OpenCompass

- **URL**: https://github.com/open-compass/opencompass
- **Repository**: 6.8k stars, last commit recent
- **Category**: Large-scale model benchmarking platform
- **One-line description**: LLM evaluation platform supporting 100+ models over 70+ datasets with ~400,000 questions and distributed evaluation.
- **Architecture type**: Python framework with distributed processing
- **Core features**:
  - Comprehensive model/dataset support
  - One-command distributed evaluation
  - Zero-shot, few-shot, CoT paradigms
  - vLLM/LMDeploy acceleration
  - CompassHub benchmark discovery; CompassRank leaderboard
- **Unique differentiator**: Massive scale (400k questions); Chinese-origin project with strong multilingual coverage. CascadeEvaluator for cost-efficient evaluation.
- **Maturity**: Growing — active research community, frequent updates
- **Pricing/License**: Open source (license in repo)

---

### 9. HELM (Holistic Evaluation of Language Models)

- **URL**: https://crfm.stanford.edu/helm/latest/
- **Repository**: https://github.com/stanford-crfm/helm — 2.7k stars, Apache-2.0, last commit 2026-03-27 (v0.5.14)
- **Category**: Academic holistic evaluation framework
- **One-line description**: Stanford CRFM framework for holistic, reproducible evaluation of foundation models measuring accuracy, safety, efficiency, and bias.
- **Architecture type**: Python framework with web UI and leaderboards
- **Core features**:
  - Standardized benchmarks (MMLU-Pro, GPQA, IFEval, WildBench)
  - Unified model interface across providers
  - Multi-dimensional metrics (efficiency, bias, toxicity beyond accuracy)
  - Web UI for prompt/response inspection
  - Multiple official leaderboards (Capabilities, Safety, VHELM)
- **Unique differentiator**: Goes beyond accuracy with multi-criteria "holistic" evaluation. Academic rigor with published research backing methodologies.
- **Maturity**: Mature — established academic benchmark with ongoing updates
- **Pricing/License**: Apache-2.0

---

### 10. Microsoft PromptFlow

- **URL**: https://github.com/microsoft/promptflow
- **Repository**: 11.1k stars, MIT, last commit 2025-01-09 (v1.17.1)
- **Category**: End-to-end LLM application development + evaluation
- **One-line description**: Development toolkit for the full LLM app lifecycle from prototyping through evaluation to production deployment.
- **Architecture type**: CLI + VS Code extension + Azure integration; Python
- **Core features**:
  - Executable flow creation (LLMs + prompts + Python + tools)
  - Debug/trace LLM interactions
  - Batch testing and evaluation with larger datasets
  - CI/CD integration for evaluation
  - VS Code extension with visual flow editor
  - Azure AI cloud integration
- **Unique differentiator**: Full lifecycle tool integrating evaluation into development workflow via VS Code. Microsoft/Azure backing.
- **Maturity**: Growing — active Microsoft investment
- **Pricing/License**: MIT (open source); optional Azure cloud features

---

### 11. Microsoft PromptBench

- **URL**: https://github.com/microsoft/promptbench
- **Repository**: 2.8k stars, MIT, archived 2026-03-17
- **Category**: Adversarial prompt evaluation
- **One-line description**: Unified framework for LLM evaluation combining prompt engineering, adversarial robustness testing, and dynamic evaluation.
- **Architecture type**: Python library with modular components
- **Core features**:
  - Multiple prompt engineering methods (CoT, EmotionPrompt, Expert Prompting)
  - Adversarial prompt attack evaluation
  - DyVal: dynamic evaluation to address test data contamination
  - PromptEval: efficient multi-prompt evaluation (5% sampling)
- **Unique differentiator**: Uniquely integrates both offensive (adversarial) and defensive (prompt engineering) evaluation. DyVal addresses contamination.
- **Maturity**: Declining — archived March 2026
- **Pricing/License**: MIT

---

### 12. AlpacaEval

- **URL**: https://github.com/tatsu-lab/alpaca_eval
- **Repository**: 2k stars, Apache-2.0 (code) / CC BY NC 4.0 (data), last commit 2025-01-27
- **Category**: Instruction-following automatic evaluator
- **One-line description**: Fast, cheap automatic evaluator for instruction-following LLMs with 0.98 correlation to human preferences.
- **Architecture type**: Python library with LLM-as-judge pairwise comparison
- **Core features**:
  - Leaderboard of evaluated models
  - Validated against 20K human annotations
  - Toolkit for building custom automatic evaluators
  - Length-controlled win rates to reduce bias
  - Caching, batching, output randomization
- **Unique differentiator**: Achieves 0.98 Spearman correlation with ChatBot Arena while costing <$10 and running in <3 minutes. Length-controlled win rates are novel.
- **Maturity**: Mature — widely referenced in research
- **Pricing/License**: Apache-2.0 (code), CC BY NC 4.0 (data)

---

## Category B: LLM Observability Platforms (with Evaluation Features)

---

### 13. Langfuse

- **URL**: https://langfuse.com
- **Repository**: https://github.com/langfuse/langfuse — 24.3k stars, MIT (core), last commit recent (6,672 commits)
- **Category**: LLM engineering platform (observability + evals)
- **One-line description**: Open-source LLM engineering platform providing observability, metrics, evals, prompt management, and playground.
- **Architecture type**: Full-stack platform (React frontend, ClickHouse DB, Docker/K8s deployment)
- **Core features**:
  - LLM application observability (traces, LLM calls, retrieval, embeddings, agents)
  - Prompt management with version control
  - Evaluations: LLM-as-judge, user feedback, manual labeling, custom pipelines
  - Datasets for testing and benchmarks
  - Prompt playground
  - Typed SDKs (Python, JS/TS)
- **Unique differentiator**: Combines observability with evaluation in one open-source platform. Strong self-hosting story. Growing faster than most competitors.
- **Maturity**: Growing rapidly — one of the fastest-growing LLMOps tools
- **Pricing/License**: MIT (core); enterprise features under separate license

---

### 14. Opik (Comet ML)

- **URL**: https://www.comet.com/site/products/opik/
- **Repository**: https://github.com/comet-ml/opik — 18.6k stars, Apache-2.0
- **Category**: AI observability + evaluation + optimization platform
- **One-line description**: Open-source platform for debugging, evaluating, and monitoring LLM applications with 60+ framework integrations.
- **Architecture type**: Distributed system; cloud-hosted or self-hosted (Docker/K8s)
- **Core features**:
  - Deep tracing of LLM calls and conversations
  - LLM-as-judge metrics and experiment management
  - Production monitoring dashboards (40M+ traces/day capacity)
  - Agent Optimizer for prompt/agent enhancement
  - Guardrails for responsible AI
  - PyTest CI/CD integration
- **Unique differentiator**: Bridges dev-time tracing with production-scale monitoring. Agent Optimizer is a unique feature for automated prompt improvement.
- **Maturity**: Growing — backed by Comet ML, rapid star growth
- **Pricing/License**: Apache-2.0 (open source); Comet cloud offering

---

### 15. Arize Phoenix

- **URL**: https://phoenix.arize.com
- **Repository**: https://github.com/Arize-ai/phoenix — 9.1k stars, open source
- **Category**: AI observability + evaluation platform
- **One-line description**: Open-source AI observability platform with OpenTelemetry tracing, LLM evaluation, dataset management, experiments, and prompt playground.
- **Architecture type**: Vendor-agnostic platform; local, Jupyter, Docker, or cloud deployment
- **Core features**:
  - OpenTelemetry-based tracing
  - LLM evaluation (response + retrieval)
  - Versioned datasets for experiments
  - Prompt playground
  - Prompt management with version control
  - 25+ instrumentation libraries
  - MCP integration for coding agents
- **Unique differentiator**: OpenTelemetry-native; MCP integration for Cursor/Claude Code. Part of the broader Arize commercial platform.
- **Maturity**: Growing — strong OSS community + commercial backing
- **Pricing/License**: Open source (core); Arize commercial platform for enterprise

---

### 16. Helicone

- **URL**: https://helicone.ai
- **Repository**: https://github.com/Helicone/helicone — 5.4k stars, Apache-2.0, last release 2025-08-21
- **Category**: LLM observability + AI gateway
- **One-line description**: Open-source LLM observability platform with AI gateway for monitoring, evaluating, and routing across 100+ models.
- **Architecture type**: Microservices (NextJS, Cloudflare Workers, Express, Supabase, ClickHouse, MinIO)
- **Core features**:
  - AI Gateway accessing 100+ models with intelligent routing
  - Cost and latency tracking
  - Agent tracing and session inspection
  - Prompt versioning
  - SOC 2 / GDPR compliant
- **Unique differentiator**: Combines observability with an operational AI gateway for routing. Self-hosting via Docker/Helm for data sovereignty.
- **Maturity**: Growing — 10k free monthly requests, enterprise tier
- **Pricing/License**: Apache-2.0; free tier (10k/month); paid plans

---

### 17. Laminar (lmnr)

- **URL**: https://www.lmnr.ai
- **Repository**: https://github.com/lmnr-ai/lmnr — 2.7k stars, Apache-2.0, last commit 2026-04-01
- **Category**: AI agent observability platform
- **One-line description**: Open-source observability platform purpose-built for AI agents with OTel-native tracing, evaluation, and monitoring.
- **Architecture type**: Full-stack (TypeScript frontend, Rust backend, Python SDK, Docker Compose)
- **Core features**:
  - OpenTelemetry-native tracing with auto-instrumentation
  - Evaluation SDK and CLI for local/CI testing
  - AI monitoring via natural language event definitions
  - SQL-based data querying + dashboard builder
  - Data annotation UI
- **Unique differentiator**: Rust-based backend for ultra-fast full-text search and real-time trace visualization. Agent-first design.
- **Maturity**: Growing — very recent, active development
- **Pricing/License**: Apache-2.0

---

### 18. Agenta

- **URL**: https://agenta.ai
- **Repository**: https://github.com/Agenta-AI/agenta — 4k stars, MIT, last commit 2025-01-08
- **Category**: LLMOps platform (playground + management + evaluation + observability)
- **One-line description**: All-in-one open-source LLMOps platform combining prompt playground, management, evaluation, and observability.
- **Architecture type**: Full-stack platform (web frontend, API backend, SDK, Docker)
- **Core features**:
  - Prompt management & engineering with version control
  - Flexible testsets with pre-built/custom evaluators
  - LLM observability with OTel support
  - Human feedback integration
  - 50+ LLM model support
- **Unique differentiator**: All-in-one approach emphasizing collaboration between engineers and subject matter experts through visual playground.
- **Maturity**: Growing
- **Pricing/License**: MIT; cloud version available

---

### 19. W&B Weave

- **URL**: https://wandb.ai/site/evaluations/
- **Repository**: https://github.com/wandb/weave — 1.1k stars, Apache-2.0, last commit 2026-04-01
- **Category**: Generative AI development toolkit with evaluation
- **One-line description**: Toolkit for logging, debugging, and building rigorous evaluations for LLM applications from Weights & Biases.
- **Architecture type**: Python-dominant library with trace server
- **Core features**:
  - Log/debug LLM inputs, outputs, execution traces
  - Comparative "apples-to-apples" evaluations
  - Integration with OpenAI, Anthropic, Google AI Studio
  - Dataset versioning and dashboards
- **Unique differentiator**: Tight integration with the broader W&B ecosystem (experiment tracking, model registry). Composable evaluation design.
- **Maturity**: Growing — W&B's strategic bet on GenAI
- **Pricing/License**: Apache-2.0; W&B cloud pricing applies

---

### 20. TruLens

- **URL**: https://www.trulens.org
- **Repository**: https://github.com/truera/trulens — 3.2k stars, MIT, v2.7.1 (2026-03-10)
- **Category**: LLM evaluation + tracking framework
- **One-line description**: Feedback function framework for systematically evaluating and tracking LLM experiments and AI agents.
- **Architecture type**: Python framework with dashboard UI
- **Core features**:
  - Feedback function implementation
  - RAG Triad evaluation framework
  - Honest/Harmless/Helpful evaluation
  - Stack-agnostic instrumentation
  - Comparative analysis UI
- **Unique differentiator**: RAG Triad (answer relevance, context relevance, groundedness) is a widely-referenced evaluation methodology. Combines instrumentation with evaluation.
- **Maturity**: Growing — steady development, good adoption
- **Pricing/License**: MIT

---

### 21. UpTrain

- **URL**: https://uptrain.ai
- **Repository**: https://github.com/uptrain-ai/uptrain — 2.3k stars, open source
- **Category**: LLM evaluation + improvement platform
- **One-line description**: Open-source platform for evaluating and improving GenAI applications with 20+ pre-configured checks and root cause analysis.
- **Architecture type**: Python package with locally-hosted web dashboard
- **Core features**:
  - 20+ pre-configured evaluations (language, code, embeddings)
  - Interactive local dashboard
  - Root cause analysis on failures
  - Multiple evaluator LLM support (OpenAI, Anthropic, Mistral, Azure)
  - Customizable evals with few-shot examples
- **Unique differentiator**: Local-first data security combined with root cause analysis capabilities. Data never leaves your environment.
- **Maturity**: Growing
- **Pricing/License**: Open source; enterprise tier

---

### 22. MLflow (Evaluation Module)

- **URL**: https://mlflow.org
- **Repository**: https://github.com/mlflow/mlflow — 25.1k stars, Apache-2.0
- **Category**: AI/ML engineering platform with LLM evaluation
- **One-line description**: The largest open-source AI engineering platform, with a dedicated evaluation module offering 50+ built-in metrics and LLM judges.
- **Architecture type**: Modular platform; local, cloud, or K8s deployment
- **Core features**:
  - 50+ built-in metrics and LLM judges
  - Quality tracking over time
  - Regression detection pre-production
  - Custom evaluation criteria
  - Integration with broader ML lifecycle (tracking, registry, deployment)
- **Unique differentiator**: Evaluation embedded within the industry-standard ML platform. 60+ framework integrations. Databricks backing.
- **Maturity**: Mature — ubiquitous in ML engineering
- **Pricing/License**: Apache-2.0; Databricks managed offering

---

### 23. RagaAI Catalyst

- **URL**: https://raga.ai
- **Repository**: https://github.com/raga-ai-hub/RagaAI-Catalyst — 16.1k stars
- **Category**: AI observability + evaluation + optimization
- **One-line description**: Comprehensive platform for LLM project management including evaluation, agentic tracing, prompt management, and guardrails.
- **Architecture type**: Python SDK with dashboard; REST API backend
- **Core features**:
  - Project & dataset management
  - Metric-based RAG evaluation
  - Agentic tracing (execution graph viz, timeline views)
  - Prompt management with version control
  - Synthetic data generation
  - Guardrail management
- **Unique differentiator**: Multi-agentic system debugging with execution graph visualization. All-in-one approach.
- **Maturity**: Growing
- **Pricing/License**: SDK open source; platform pricing applies

---

## Category C: AI Safety, Red Teaming & Security Evaluation

---

### 24. NVIDIA Garak

- **URL**: https://github.com/NVIDIA/garak
- **Repository**: 7.4k stars, Apache-2.0, last commit 2024-12-05
- **Category**: LLM vulnerability scanner / red teaming toolkit
- **One-line description**: Generative AI red-teaming toolkit probing for hallucination, data leakage, prompt injection, toxicity, and jailbreaks.
- **Architecture type**: Plugin-based modular system (probes, detectors, generators, evaluators, harnesses)
- **Core features**:
  - Multiple probe types (DAN, encoding injection, glitch tokens, jailbreaks)
  - 10+ LLM platform support
  - JSONL reports and vulnerability tracking
  - Plugin development framework
- **Unique differentiator**: "Nmap for LLMs" — comprehensive probe library covering emerging attack vectors with flexible generator system.
- **Maturity**: Growing — NVIDIA backing
- **Pricing/License**: Apache-2.0

---

### 25. Microsoft PyRIT

- **URL**: https://github.com/microsoft/PyRIT (migrated from Azure/PyRIT)
- **Repository**: MIT, archived 2026-03-27 at Azure org (migrated to Microsoft org)
- **Category**: AI red teaming toolkit
- **One-line description**: Microsoft's Python Risk Identification Toolkit for proactive identification of risks in generative AI systems.
- **Architecture type**: Python framework; modular security-focused tool
- **Core features**:
  - Responsible AI assessment
  - Red team tooling for GenAI
  - Orchestrated LLM attack suites
  - Automated red team workflows
- **Unique differentiator**: Microsoft's official AI red teaming framework. Security professional focus.
- **Maturity**: Growing — migrated to Microsoft org for continued development
- **Pricing/License**: MIT

---

### 26. DeepTeam (by Confident AI)

- **URL**: https://github.com/confident-ai/deepteam
- **Repository**: 1.4k stars, Apache-2.0, v1.0.4 (2025-11-12)
- **Category**: LLM red teaming / penetration testing
- **One-line description**: Open-source LLM red teaming framework with 50+ vulnerabilities and 20+ attack methods supporting OWASP, NIST, and MITRE frameworks.
- **Architecture type**: Python framework built on DeepEval
- **Core features**:
  - 50+ ready-to-use vulnerability types
  - 20+ adversarial attack methods
  - OWASP Top 10, NIST AI RMF, MITRE ATLAS support
  - 7 production-ready guardrails
  - LLM-as-Judge for evaluation
  - No predefined system spec needed
- **Unique differentiator**: Companion to DeepEval from the same team; dynamically generates adversarial attacks without requiring system specs.
- **Maturity**: Early-growing — first stable release Nov 2025
- **Pricing/License**: Apache-2.0; Confident AI platform integration

---

### 27. Agentic Security

- **URL**: https://github.com/msoedov/agentic_security
- **Repository**: 1.8k stars, last commit 2024-04-13
- **Category**: Agent workflow vulnerability scanner
- **One-line description**: Vulnerability scanner for agent workflows and LLMs with multimodal attacks, fuzzing, and RL-based adaptive probing.
- **Architecture type**: Python (FastAPI backend, web UI, CLI)
- **Core features**:
  - Multimodal attacks (text, image, audio)
  - Multi-step jailbreak simulations
  - Comprehensive fuzzing
  - RL-based adaptive attacks
  - CI/CD integration
  - Dynamic dataset generation with mutation
- **Unique differentiator**: Adaptive, RL-based probes that evolve with model defenses. Multimodal attack surface.
- **Maturity**: Early-growing
- **Pricing/License**: Custom license (FFCC19)

---

### 28. BCG X ARTKIT

- **URL**: https://github.com/BCG-X-Official/artkit
- **Repository**: 165 stars, Apache-2.0, last commit 2025-01-07
- **Category**: Automated red teaming & testing framework
- **One-line description**: Python framework for automated prompt-based testing with multi-turn conversation simulation and data lineage tracking.
- **Architecture type**: Async Python framework with pipeline orchestration
- **Core features**:
  - Simple API with async processing
  - Response caching
  - Multi-provider support (OpenAI, Anthropic, Google, AWS, HF, Groq)
  - Multi-turn conversation simulation
  - Data lineage tracking
  - Flow diagram visualizations
- **Unique differentiator**: Enterprise-grade (BCG X origin); emphasizes customization over automation. Human-driven testing accelerated by AI.
- **Maturity**: Early — small star count but enterprise backing
- **Pricing/License**: Apache-2.0

---

### 29. Bloom (Anthropic Safety Research)

- **URL**: https://github.com/safety-research/bloom
- **Repository**: 1.3k stars, MIT, last commit 2025-06-24
- **Category**: Behavioral evaluation suite generator
- **One-line description**: Open-source tool that generates evaluation suites probing LLMs for specific behavioral patterns like sycophancy, self-preservation, and political bias.
- **Architecture type**: Python CLI + library; 4-stage pipeline (Understanding, Ideation, Rollout, Judgment)
- **Core features**:
  - Seed-configurable evaluation suite generation
  - Variation dimensions for behavior stability testing
  - W&B integration for experiments
  - Extended thinking support (Claude, OpenAI)
  - Interactive chat interface
  - Web-based result viewer
- **Unique differentiator**: Generates dynamic evaluations (not fixed benchmarks) for behavioral/safety analysis. Seed-based reproducibility with variation.
- **Maturity**: Early-growing — research-focused
- **Pricing/License**: MIT

---

### 30. Guardrails AI

- **URL**: https://guardrailsai.com
- **Repository**: https://github.com/ShreyaR/guardrails — 6.6k stars, Apache-2.0, last commit 2026-03-16 (v0.9.2)
- **Category**: LLM output validation + guardrails
- **One-line description**: Framework for detecting AI risks and generating structured LLM outputs with a Hub of reusable validators.
- **Architecture type**: Modular Python framework; standalone Flask server or library integration
- **Core features**:
  - Input/output guards for risk detection
  - Guardrails Hub with pre-built validators
  - Structured data generation
  - Function calling support
  - REST API server
  - Guardrails Index benchmark (24 guardrails, 6 categories)
- **Unique differentiator**: Combines guardrails/validation with structured output. First benchmark comparing guardrail performance (Guardrails Index).
- **Maturity**: Growing — active community and Hub ecosystem
- **Pricing/License**: Apache-2.0

---

## Category D: Specialized / Niche Evaluation Tools

---

### 31. AutoRAG

- **URL**: https://github.com/Marker-Inc-Korea/AutoRAG
- **Repository**: 4.7k stars
- **Category**: RAG pipeline auto-optimization
- **One-line description**: AutoML-style tool that automatically evaluates and finds the optimal RAG pipeline for your specific data.
- **Architecture type**: Node-based pipeline architecture with YAML config
- **Core features**:
  - Automated RAG pipeline optimization
  - Three-stage processing (parsing, chunking, QA creation)
  - Multiple retrieval strategies
  - Comprehensive evaluation metrics
  - HuggingFace Space demos
- **Unique differentiator**: AutoML approach to RAG — automatically tries module combinations and picks the best pipeline.
- **Maturity**: Growing
- **Pricing/License**: Open source

---

### 32. lmms-eval

- **URL**: https://github.com/EvolvingLMMs-Lab/lmms-eval
- **Repository**: 4k stars
- **Category**: Multimodal model evaluation
- **One-line description**: Unified evaluation toolkit for large multimodal models across text, image, video, and audio with 100+ tasks.
- **Architecture type**: Modular framework with ChatMessages protocol; YAML task config
- **Core features**:
  - 100+ evaluation tasks across modalities
  - 30+ model implementations
  - Video I/O optimization (3.58x faster with TorchCodec)
  - Statistical rigor (confidence intervals, paired t-tests)
  - Agentic task evaluation
  - 17 languages
- **Unique differentiator**: The only comprehensive multimodal evaluation toolkit. Emphasizes statistical rigor over single-number scores.
- **Maturity**: Growing — fills a unique niche
- **Pricing/License**: Open source

---

### 33. EvalPlus

- **URL**: https://github.com/evalplus/evalplus
- **Repository**: 1.7k stars, Apache-2.0, last commit 2024-10-20
- **Category**: Code generation evaluation
- **One-line description**: Rigorous code evaluation framework extending HumanEval and MBPP with 80x and 35x more tests respectively.
- **Architecture type**: Multi-backend Python system with Docker sandboxing
- **Core features**:
  - HumanEval+ (80x more tests than HumanEval)
  - MBPP+ (35x more tests)
  - EvalPerf for code efficiency measurement
  - Multiple backends (HF, vLLM, APIs)
  - Safe Docker execution
- **Unique differentiator**: Reveals gap between basic and comprehensive code testing — measures code robustness, not just correctness.
- **Maturity**: Mature — NeurIPS 2023, COLM 2024; used by Meta, Qwen, DeepSeek
- **Pricing/License**: Apache-2.0

---

### 34. MTEB (Massive Text Embedding Benchmark)

- **URL**: https://github.com/embeddings-benchmark/mteb
- **Repository**: 3.2k stars, Apache-2.0, last commit 2026-04-02
- **Category**: Embedding model evaluation
- **One-line description**: Comprehensive benchmark for evaluating text embedding models across classification, retrieval, clustering, similarity, and reranking tasks.
- **Architecture type**: Python framework with CLI + API
- **Core features**:
  - Interactive leaderboard
  - Multiple task types
  - Multilingual and multimodal evaluation
  - Sentence Transformers integration
- **Unique differentiator**: The standard benchmark for embedding models. MMTEB expansion covers multilingual evaluation.
- **Maturity**: Mature — standard reference for embedding evaluation
- **Pricing/License**: Apache-2.0

---

### 35. BEIR

- **URL**: https://github.com/beir-cellar/beir
- **Repository**: 2.1k stars, Apache-2.0, last commit 2025-01
- **Category**: Information retrieval benchmark
- **One-line description**: Heterogeneous benchmark for evaluating retrieval models across 15+ diverse IR datasets.
- **Architecture type**: Python framework
- **Core features**:
  - 15+ diverse IR datasets
  - Lexical, dense, sparse, reranking evaluation
  - Standard metrics (NDCG@k, MAP@k, Recall@k, Precision@k, MRR)
  - Custom dataset support
- **Unique differentiator**: Heterogeneous evaluation across diverse retrieval tasks rather than single datasets.
- **Maturity**: Mature — NeurIPS 2021, SIGIR 2024
- **Pricing/License**: Apache-2.0

---

### 36. UQLM (CVS Health)

- **URL**: https://github.com/cvs-health/uqlm
- **Repository**: 1.1k stars, Apache-2.0
- **Category**: Hallucination detection via uncertainty quantification
- **One-line description**: Python library for LLM hallucination detection using state-of-the-art uncertainty quantification techniques.
- **Architecture type**: Modular Python package with LangChain integration
- **Core features**:
  - Black-box scorers (semantic entropy, consistency-based)
  - White-box scorers (token probability-based)
  - LLM-as-judge
  - Ensemble methods
  - Claim-level hallucination detection for long text
  - Response refinement (remove low-confidence claims)
- **Unique differentiator**: Comprehensive UQ methods spanning zero-cost to multi-generation approaches. Claim-level granularity for long-form content.
- **Maturity**: Growing — CVS Health backing
- **Pricing/License**: Apache-2.0

---

### 37. Vectara Open RAG Eval

- **URL**: https://github.com/vectara/open-rag-eval
- **Repository**: 351 stars, Apache-2.0
- **Category**: RAG evaluation without golden answers
- **One-line description**: RAG evaluation toolkit that works without golden answers using UMBRELA and AutoNuggetizer techniques.
- **Architecture type**: Modular Python framework with YAML config
- **Core features**:
  - TREC-RAG evaluation metrics
  - No golden answers required (UMBRELA, AutoNuggetizer)
  - Modular connectors (Vectara, LlamaIndex, LangChain)
  - Detailed per-query scoring
  - Web-based visualization (openevaluation.ai)
  - Query generation from corpus
- **Unique differentiator**: Eliminates need for golden answers — makes RAG evaluation scalable without manual reference data.
- **Maturity**: Early-growing
- **Pricing/License**: Apache-2.0

---

### 38. ChainForge

- **URL**: https://github.com/ianarawjo/ChainForge
- **Repository**: 3k stars, MIT, last commit 2026-04-02
- **Category**: Visual prompt evaluation IDE
- **One-line description**: Visual programming environment for battle-testing prompts across models with combinatorial evaluation and immediate visualization.
- **Architecture type**: Hybrid (ReactFlow frontend + Flask backend); TypeScript dominant
- **Core features**:
  - Query multiple LLMs simultaneously
  - Compare response quality across permutations
  - Built-in evaluation metrics with visualization
  - Synthetic data generation
  - Ground truth evaluation
  - Prompt chaining
- **Unique differentiator**: Visual, combinatorial approach — takes cross products of inputs to generate hundreds of permutations. No-code evaluation.
- **Maturity**: Growing — active development
- **Pricing/License**: MIT

---

### 39. Giskard

- **URL**: https://giskard.ai
- **Repository**: https://github.com/Giskard-AI/giskard — 5.2k stars, Apache-2.0, last commit 2026-03-26
- **Category**: LLM agent testing & evaluation
- **One-line description**: Open-source evaluation and testing library for LLM agents with red teaming, prompt injection detection, and data leakage scanning.
- **Architecture type**: Modular Python library; async-first; v3 rewrite
- **Core features**:
  - Giskard Checks: testing/eval with scenario API, LLM-as-judge
  - Giskard Scan: agent vulnerability scanning (in progress)
  - Giskard RAG: RAG evaluation (planned)
  - Minimal dependencies
  - Works with individual LLMs to multi-step agent pipelines
- **Unique differentiator**: Fresh v3 rewrite emphasizing modularity and async-first design. Wraps anything from single LLMs to complex agents.
- **Maturity**: Growing — active rewrite indicates strong investment
- **Pricing/License**: Apache-2.0

---

### 40. Mocktopus

- **URL**: https://github.com/evalops/mocktopus
- **Repository**: 6 stars, MIT
- **Category**: Deterministic LLM API mocking
- **One-line description**: Drop-in replacement for OpenAI/Anthropic APIs enabling deterministic, zero-cost testing of LLM applications.
- **Architecture type**: Local mock server; YAML-based scenario configuration
- **Core features**:
  - Drop-in API replacement (change base URL only)
  - Deterministic responses
  - Tool/function calling support
  - SSE streaming
  - OpenAI + Anthropic API compatibility
  - Offline operation
- **Unique differentiator**: Specifically addresses the "non-deterministic LLM responses break tests" problem. Scenario-driven approach without code changes.
- **Maturity**: Early — very new
- **Pricing/License**: MIT

---

### 41. Agentrial

- **URL**: https://github.com/alepot55/agentrial
- **Repository**: 16 stars, MIT, last commit 2026-02-05
- **Category**: Statistical agent evaluation
- **One-line description**: "pytest for AI agents" — statistical evaluation framework running agents N times with confidence intervals and regression detection.
- **Architecture type**: Python CLI with modular framework adapters
- **Core features**:
  - Multi-trial statistical testing with Wilson confidence intervals
  - Step-level failure attribution (Fisher exact tests)
  - Real cost tracking (45+ LLM models)
  - Regression detection for CI/CD
  - Agent Reliability Score (0-100)
  - Production drift monitoring (CUSUM/Page-Hinkley)
  - Framework-agnostic (LangGraph, CrewAI, Pydantic AI)
  - Trajectory visualization (flame graphs)
- **Unique differentiator**: Statistical rigor by default — confidence intervals, not anecdotes. Step-level failure diagnosis. Local-first.
- **Maturity**: Early — very new but interesting concept
- **Pricing/License**: MIT

---

### 42. Galileo AI Agent Leaderboard

- **URL**: https://github.com/rungalileo/agent-leaderboard
- **Repository**: 219 stars, MIT, last commit 2025-07-17
- **Category**: Agent benchmarking (enterprise scenarios)
- **One-line description**: Evaluation framework assessing AI agents across real-world business scenarios in 5 industry domains with multi-turn tool-calling tasks.
- **Architecture type**: Benchmark framework (synthetic data generation + simulation engine + metrics)
- **Core features**:
  - 15+ models evaluated
  - 5 industry domains (Banking, Healthcare, Investment, Telecom, Insurance)
  - 100 scenarios per domain with 5-8 interconnected goals per conversation
  - Action Completion (AC) and Tool Selection Quality (TSQ) metrics
  - HuggingFace Spaces leaderboard
- **Unique differentiator**: Enterprise-focused multi-turn evaluation with interdependent tasks, not isolated tool-calling tests.
- **Maturity**: Early-growing
- **Pricing/License**: MIT

---

### 43. IFEval (Google Research)

- **URL**: https://github.com/google-research/google-research/tree/master/instruction_following_eval
- **Category**: Instruction-following evaluation
- **One-line description**: Google Research framework for evaluating LLM compliance with verifiable instructions through constraint-verification prompts.
- **Architecture type**: Python evaluation scripts
- **Core features**:
  - Standardized instruction-following assessment
  - Constraint verification prompts
  - JSONL-based prompt/response evaluation
- **Unique differentiator**: Automated, verifiable instruction compliance testing (no LLM judge needed for most checks).
- **Maturity**: Mature — widely used in leaderboards (included in HELM, Open LLM Leaderboard)
- **Pricing/License**: Apache-2.0 (part of google-research)

---

### 44. Anthropic Model Evals (Dataset)

- **URL**: https://github.com/anthropics/evals
- **Repository**: 363 stars, CC-BY-4.0, last commit 2022-12-12
- **Category**: Behavioral evaluation datasets
- **One-line description**: Model-written evaluation datasets for testing LLM behaviors including persona, sycophancy, advanced AI risk, and gender bias.
- **Architecture type**: Dataset repository (not a framework)
- **Core features**:
  - Persona evaluation datasets
  - Sycophancy detection
  - Advanced AI risk datasets
  - Winogenerated bias tests
- **Unique differentiator**: Model-written evaluation data — demonstrates scalable eval creation. Foundational for AI safety research.
- **Maturity**: Mature (static) — dataset, not actively developed software
- **Pricing/License**: CC-BY-4.0

---

## Category E: Commercial / SaaS Evaluation Platforms

---

### 45. Patronus AI

- **URL**: https://www.patronus.ai
- **Repository**: https://github.com/patronus-ai/patronus-py — 7 stars, SDK only
- **Category**: Enterprise LLM evaluation platform
- **One-line description**: Evaluation platform with multimodal LLM-as-judge, hallucination detection, and industry-specific benchmarks.
- **Architecture type**: Client-server; local SDK + remote Patronus infrastructure
- **Core features**:
  - Tracing & observability
  - Remote evaluators (hallucination detection etc.)
  - Custom evaluators
  - Experiment framework
  - YAML-based configuration
- **Unique differentiator**: Enterprise-grade hosted evaluators combining local execution with remote LLM-based evaluation.
- **Maturity**: Growing — commercial platform with Python SDK
- **Pricing/License**: SDK open source; platform is commercial

---

### 46. Athina AI

- **URL**: https://www.athina.ai
- **Category**: LLM evaluation & monitoring platform (SaaS)
- **One-line description**: SOC-2 compliant LLM evaluation and monitoring platform with 50+ preset evaluations.
- **Architecture type**: Cloud SaaS platform
- **Core features**:
  - 50+ preset evaluations
  - SOC-2 compliance
  - Monitoring & alerting
  - Custom evaluation creation
- **Unique differentiator**: Enterprise compliance focus (SOC-2). Preset evaluations lower barrier to entry.
- **Maturity**: Growing
- **Pricing/License**: Commercial SaaS

---

### 47. Parea AI

- **URL**: https://www.parea.ai
- **Category**: LLM development tools (SaaS)
- **One-line description**: Developer tools for evaluating, testing, and monitoring LLM-powered applications with actionable insights.
- **Architecture type**: Cloud SaaS platform
- **Core features**:
  - Evaluation and testing
  - Monitoring
  - Actionable insights
- **Unique differentiator**: Developer-focused with emphasis on actionable feedback.
- **Maturity**: Growing
- **Pricing/License**: Commercial SaaS

---

### 48. LangSmith (LangChain)

- **URL**: https://smith.langchain.com
- **Category**: LLM development platform with evaluation
- **One-line description**: Hosted tracing plus datasets, batched evals, and regression gating for LangChain apps.
- **Architecture type**: Cloud SaaS platform
- **Core features**:
  - LLM tracing
  - Dataset management
  - Batched evaluations
  - Regression testing gates
  - Integration with LangChain ecosystem
- **Unique differentiator**: Native LangChain integration; first-party tooling for the most popular LLM framework.
- **Maturity**: Mature — tightly coupled with LangChain ecosystem
- **Pricing/License**: Commercial SaaS; free tier available

---

### 49. Deepchecks

- **URL**: https://deepchecks.com
- **Repository**: https://github.com/deepchecks/deepchecks — 4k stars, AGPL-3.0
- **Category**: ML validation (tabular, NLP, CV) with LLM evaluation content
- **One-line description**: Holistic ML validation solution for data and model testing from research to production.
- **Architecture type**: Modular Python package with CI/CD and monitoring
- **Core features**:
  - Built-in checks for tabular, NLP, CV
  - Testing management + CI integration
  - Production monitoring
  - LLM evaluation playbook/content
- **Unique differentiator**: Broader ML validation scope (not just LLMs). Strong content/education around evaluation practices.
- **Maturity**: Mature — but LLM-specific capabilities are secondary
- **Pricing/License**: AGPL-3.0 (core); commercial monitoring

---

## Category F: Pipeline / Orchestration Tools with Eval Capabilities

---

### 50. ZenML

- **URL**: https://zenml.io
- **Repository**: https://github.com/zenml-io/zenml — 5.3k stars, Apache-2.0, last commit 2026-03-19
- **Category**: AI/ML pipeline orchestration with evaluation steps
- **One-line description**: Pipeline framework that bakes evaluation steps and guardrail metrics into LLM workflows across any infrastructure.
- **Architecture type**: Client-server with web dashboard; Python
- **Core features**:
  - Automatic containerization and run tracking
  - Infrastructure abstraction
  - Integration with MLflow, LangGraph, Langfuse
  - Evaluation loops in RAG pipelines
- **Unique differentiator**: Orchestrates the full MLOps lifecycle including evaluation as a pipeline step. Broader than just evaluation.
- **Maturity**: Growing
- **Pricing/License**: Apache-2.0

---

### 51. LlamaIndex (Evaluation Module)

- **URL**: https://docs.llamaindex.ai/en/stable/module_guides/evaluating/
- **Repository**: https://github.com/run-llama/llama_index — 48.2k stars
- **Category**: LLM framework with built-in evaluation module
- **One-line description**: Evaluation modules within LlamaIndex for replaying queries, scoring retrievers, and comparing query engines.
- **Architecture type**: Python library modules within larger framework
- **Core features**:
  - Query replay and evaluation
  - Retriever scoring
  - Query engine comparison
  - Integration with LlamaIndex RAG pipelines
- **Unique differentiator**: Evaluation tightly integrated into the data framework — evaluate in the same environment you build.
- **Maturity**: Mature (as part of LlamaIndex)
- **Pricing/License**: MIT

---

## Category G: Auxiliary / Supporting Tools

---

### 52. ContextCheck

- **URL**: https://github.com/Addepto/contextcheck
- **Repository**: 92 stars, MIT, last commit 2024-11-18
- **Category**: YAML-based LLM/RAG testing for CI
- **One-line description**: Framework for evaluating LLMs and RAG systems using YAML test definitions with CI/CD integration.
- **Architecture type**: Python CLI
- **Core features**:
  - YAML test scenario definition
  - Heuristic + LLM-based + human validation
  - Jinja2 templating for requests
  - CI/CD pipeline integration
- **Unique differentiator**: Fully YAML-configurable; accessible to non-developers for AI test-driven development.
- **Maturity**: Early
- **Pricing/License**: MIT

---

### 53. LLAMATOR

- **URL**: https://github.com/LLAMATOR-Core/llamator
- **Repository**: 204 stars
- **Category**: Red teaming for chatbots
- **One-line description**: Python red-teaming framework for testing chatbots and GenAI systems.
- **Architecture type**: Python framework
- **Maturity**: Early
- **Pricing/License**: Open source

---

### 54. Log10

- **URL**: https://log10.io
- **Repository**: https://github.com/log10-io/log10 — 96 stars, MIT, archived 2025-05-01
- **Category**: LLM logging + evaluation
- **One-line description**: Unified LLM data management for logging, monitoring, and improving LLM applications.
- **Architecture type**: Python client library with managed backend
- **Core features**:
  - One-line instrumentation
  - AutoFeedback for automated evaluation
  - Model comparison/benchmarking
  - RLHF readiness
- **Unique differentiator**: Minimal integration friction ("log10(openai)"). Combined logging + eval.
- **Maturity**: Declining — archived May 2025
- **Pricing/License**: MIT; managed platform

---

### 55. AIConfig (LastMile AI)

- **URL**: https://github.com/lastmile-ai/aiconfig
- **Repository**: 1.1k stars, MIT, last release 2024-03-18
- **Category**: Prompt/model config management
- **One-line description**: Framework for managing AI prompts, models, and parameters as JSON configs separate from application code.
- **Architecture type**: Python/Node.js SDK with VS Code editor
- **Core features**:
  - JSON-serializable prompt configs
  - VS Code editor for prototyping
  - Multi-language SDKs
  - Model-agnostic; supports prompt chaining, RAG
- **Unique differentiator**: Config-as-code approach to prompt management. Enables evaluation by separating AI behavior from code.
- **Maturity**: Declining — last release March 2024
- **Pricing/License**: MIT

---

## Comparison Matrix

### Must-Include Projects

| # | Project | Category | Architecture | Stars | Maturity | OSS License | Key Strength | Key Weakness |
|---|---------|----------|-------------|-------|----------|-------------|-------------|-------------|
| MI-1 | Inspect AI | Research-grade Eval | Library+CLI+Web | 1.9k | Growing | MIT | 100+ pre-built evals, sandboxing | No production monitoring |
| MI-2 | Braintrust | Enterprise Eval Platform | SaaS | N/A | Growing | Closed | Enterprise-grade, Brainstore DB | Vendor lock-in, closed source |
| MI-3 | agentevals (agentevals-dev) | OTel Agent Eval | Library+OTel Receiver | 112 | Early | Apache-2.0 | OTel-native, no re-execution | Tiny community |
| MI-4 | LangChain AgentEvals | Agent Trajectory Eval | Library | 534 | Growing | MIT | LangGraph trajectory focus | LangChain coupling |
| MI-5 | LangChain OpenEvals | General LLM Eval | Library | 1k | Growing | MIT | Broad evaluator variety | LangChain dependency |
| MI-6 | AWS Agent Evaluation | Agent Testing | Library+CLI | 354 | Growing | Apache-2.0 | Evaluator-as-agent approach | AWS-centric |
| MI-7 | OpenAI Agent Evals | Platform+OSS Eval | SaaS+Library | 18.1k | Mature | MIT (OSS parts) | OpenAI integration, Promptfoo | OpenAI lock-in |
| MI-8 | Humanloop | Enterprise Eval+Prompt | SaaS | N/A | Mature | Closed | Human-in-the-loop, domain experts | Anthropic acquisition uncertainty |
| MI-9 | DeepEval | Comprehensive LLM Eval | Library+Cloud | 14.4k | Growing | Apache-2.0 | Largest metric library, "pytest for LLMs" | Cloud needed for team features |
| MI-10 | Arize/Phoenix | Observability+Eval | SaaS+OSS Library | 9.1k | Mature | Partial | ML heritage, agent eval templates | Observability-first |
| MI-11 | AutoGen AgentEval | Multi-agent Eval | Library (in AutoGen) | N/A | Early | MIT | Self-generating eval criteria | AutoGen coupling |

### Broader Landscape

| # | Project | Category | Architecture | Stars | Maturity | OSS License | Key Strength | Key Weakness |
|---|---------|----------|-------------|-------|----------|-------------|-------------|-------------|
| 1 | Promptfoo | Eval+RedTeam CLI | CLI/Dashboard | 19.1k | Growing | MIT | Dev-first, local, combined eval+security | Node.js (not Python-native) |
| 2 | Ragas | RAG Evaluation | Python Library | 13.2k | Growing | Apache-2.0 | De facto RAG eval standard | RAG-specific only |
| 3 | lm-eval-harness | Academic Benchmarks | Python CLI | 12k | Mature | MIT | Powers HF Leaderboard; 60+ benchmarks | Academic focus; not for app eval |
| 4 | OpenAI Evals | LLM Evaluation | Python Framework | 18.1k | Mature | MIT | Official OpenAI; large registry | OpenAI-centric |
| 5 | OpenAI Simple-Evals | Benchmark Reference | Python Scripts | 4.4k | Declining | MIT | Simple, clean reference | No longer updated for new models |
| 6 | HF LightEval | Multi-backend Eval | Python CLI | 2.4k | Growing | MIT | 1000+ tasks; flexible backends | Young; replacing older evaluate |
| 7 | HF Evaluate | Metrics Library | Python Library | 2.4k | Declining | Apache-2.0 | Standardized metrics | Being superseded by LightEval |
| 8 | OpenCompass | Large-scale Benchmarks | Python Framework | 6.8k | Growing | OSS | 400k questions; distributed eval | Chinese-origin; less Western adoption |
| 9 | HELM | Holistic Eval | Python+Web | 2.7k | Mature | Apache-2.0 | Multi-criteria holistic evaluation | Academic; heavy infrastructure |
| 10 | PromptFlow | LLM Lifecycle | CLI+VS Code | 11.1k | Growing | MIT | Full lifecycle; VS Code integration | Microsoft ecosystem dependency |
| 11 | PromptBench | Adversarial Eval | Python Library | 2.8k | Declining | MIT | Combined offensive+defensive eval | Archived March 2026 |
| 12 | AlpacaEval | Instruction-following | LLM-as-judge | 2k | Mature | Apache-2.0 | 0.98 correlation with humans | Instruction-following only |
| 13 | Langfuse | Observability+Eval | Full-stack Platform | 24.3k | Growing | MIT | Fastest-growing LLMOps platform | Eval is secondary to observability |
| 14 | Opik | Observability+Eval | Distributed Platform | 18.6k | Growing | Apache-2.0 | Production scale (40M traces/day) | Backed by Comet ML; lock-in risk |
| 15 | Phoenix | Observability+Eval | Platform | 9.1k | Growing | OSS | OTel-native; MCP integration | Part of Arize commercial ecosystem |
| 16 | Helicone | Observability+Gateway | Microservices | 5.4k | Growing | Apache-2.0 | AI gateway + observability | Eval is minor feature |
| 17 | Laminar | Agent Observability | Full-stack (Rust) | 2.7k | Growing | Apache-2.0 | Rust backend; agent-first | Young; small community |
| 18 | Agenta | LLMOps All-in-one | Full-stack | 4k | Growing | MIT | All-in-one with collaboration | Jack of all trades |
| 19 | W&B Weave | GenAI Toolkit | Python Library | 1.1k | Growing | Apache-2.0 | W&B ecosystem integration | Low standalone adoption |
| 20 | TruLens | Eval+Tracking | Python+Dashboard | 3.2k | Growing | MIT | RAG Triad methodology | Narrower scope |
| 21 | UpTrain | Eval+Improvement | Python+Dashboard | 2.3k | Growing | OSS | Local-first; root cause analysis | Smaller community |
| 22 | MLflow | ML Platform+Eval | Modular Platform | 25.1k | Mature | Apache-2.0 | Industry standard; 50+ metrics | LLM eval is one small module |
| 23 | RagaAI Catalyst | Full-stack AI Ops | SDK+Dashboard | 16.1k | Growing | OSS (SDK) | Agentic tracing; all-in-one | Stars may be inflated |
| 24 | Garak | LLM Red Teaming | Plugin-based | 7.4k | Growing | Apache-2.0 | "Nmap for LLMs"; NVIDIA backing | Security-only focus |
| 25 | PyRIT | AI Red Teaming | Python Framework | N/A | Growing | MIT | Microsoft official red teaming | Migrating between orgs |
| 26 | DeepTeam | LLM Pen Testing | Python Framework | 1.4k | Early | Apache-2.0 | OWASP/NIST/MITRE compliance | Young; depends on DeepEval |
| 27 | Agentic Security | Vuln Scanner | FastAPI+WebUI | 1.8k | Early | Custom | RL-adaptive attacks; multimodal | Custom license; limited activity |
| 28 | ARTKIT | Red Teaming | Async Python | 165 | Early | Apache-2.0 | BCG X enterprise backing | Very small community |
| 29 | Bloom | Behavioral Eval | Python CLI | 1.3k | Early | MIT | Dynamic eval generation for safety | Research-focused; niche |
| 30 | Guardrails AI | Output Validation | Python Framework | 6.6k | Growing | Apache-2.0 | Guardrails Hub ecosystem; benchmark | Not evaluation per se |
| 31 | AutoRAG | RAG Optimization | Pipeline Framework | 4.7k | Growing | OSS | AutoML for RAG pipelines | RAG-specific |
| 32 | lmms-eval | Multimodal Eval | Modular Framework | 4k | Growing | OSS | Only comprehensive multimodal eval | Narrow: multimodal models only |
| 33 | EvalPlus | Code Eval | Multi-backend | 1.7k | Mature | Apache-2.0 | 80x more tests; rigorous | Code generation only |
| 34 | MTEB | Embedding Eval | Python Framework | 3.2k | Mature | Apache-2.0 | Standard embedding benchmark | Embeddings only |
| 35 | BEIR | IR Benchmark | Python Framework | 2.1k | Mature | Apache-2.0 | Heterogeneous retrieval eval | IR-specific |
| 36 | UQLM | Hallucination Detection | Python Library | 1.1k | Growing | Apache-2.0 | Multiple UQ methods; claim-level | Hallucination-specific |
| 37 | Open RAG Eval | RAG Eval (no golden) | Python Framework | 351 | Early | Apache-2.0 | No golden answers needed | Small; Vectara-backed |
| 38 | ChainForge | Visual Prompt Eval | ReactFlow+Flask | 3k | Growing | MIT | Visual combinatorial evaluation | Not for production CI/CD |
| 39 | Giskard | Agent Testing | Async Python | 5.2k | Growing | Apache-2.0 | Fresh v3 rewrite; agent-focused | Mid-rewrite; features in progress |
| 40 | Mocktopus | LLM API Mocking | Mock Server | 6 | Early | MIT | Deterministic LLM testing | Tiny; very new |
| 41 | Agentrial | Statistical Agent Eval | Python CLI | 16 | Early | MIT | Statistical rigor; step-level diagnosis | Very new; small community |
| 42 | Agent Leaderboard | Enterprise Agent Bench | Benchmark | 219 | Early | MIT | Enterprise multi-turn scenarios | Limited to benchmarking |
| 43 | IFEval | Instruction-following | Python Scripts | N/A | Mature | Apache-2.0 | Verifiable instruction compliance | Narrow scope |
| 44 | Anthropic Evals | Behavioral Datasets | Dataset Repo | 363 | Static | CC-BY-4.0 | Safety/behavioral evaluation data | Dataset only; not a tool |
| 45 | Patronus AI | Enterprise Eval | SaaS+SDK | 7 (SDK) | Growing | Commercial | Hosted evaluators; enterprise | Proprietary platform |
| 46 | Athina AI | Enterprise Eval | SaaS | N/A | Growing | Commercial | SOC-2; 50+ presets | Proprietary |
| 47 | Parea AI | Dev Eval Tools | SaaS | N/A | Growing | Commercial | Developer-focused insights | Proprietary |
| 48 | LangSmith | LangChain Eval | SaaS | N/A | Mature | Commercial | Native LangChain integration | LangChain-coupled |
| 49 | Deepchecks | ML Validation | Python Package | 4k | Mature | AGPL-3.0 | Broad ML validation | Not LLM-focused |
| 50 | ZenML | ML Pipelines+Eval | Client-Server | 5.3k | Growing | Apache-2.0 | Full pipeline orchestration | Eval is a feature, not focus |
| 51 | LlamaIndex Eval | RAG Eval Module | Python Library | 48.2k* | Mature | MIT | Integrated with LlamaIndex | Module within larger framework |
| 52 | ContextCheck | YAML-based Testing | Python CLI | 92 | Early | MIT | No-code YAML test definitions | Very small |
| 53 | LLAMATOR | Chatbot Red Teaming | Python Framework | 204 | Early | OSS | Chatbot-specific red teaming | Small community |
| 54 | Log10 | LLM Logging+Eval | Python Client | 96 | Declining | MIT | Minimal integration friction | Archived May 2025 |
| 55 | AIConfig | Prompt Config Mgmt | SDK+Editor | 1.1k | Declining | MIT | Config-as-code for prompts | Stale since March 2024 |

---

## Landscape Summary by Sub-Niche

### Tier 1: High-Adoption General Evaluation (>5k stars, active)
- Promptfoo, Ragas, lm-eval-harness, OpenAI Evals, Langfuse, Opik, MLflow

### Tier 2: Specialized but Significant
- **Academic Benchmarking**: HELM, OpenCompass, LightEval, AlpacaEval
- **RAG-Specific**: Ragas, AutoRAG, Open RAG Eval, LlamaIndex Eval
- **Security/Red Teaming**: Garak, PyRIT, DeepTeam, Agentic Security, ARTKIT
- **Multimodal**: lmms-eval
- **Code**: EvalPlus
- **Embeddings/Retrieval**: MTEB, BEIR
- **Safety/Behavioral**: Bloom, Anthropic Evals

### Tier 3: Observability Platforms with Eval Features
- Langfuse, Opik, Phoenix, Helicone, Laminar, Agenta, W&B Weave, RagaAI Catalyst

### Tier 4: Commercial SaaS
- Patronus AI, Athina AI, Parea AI, LangSmith

### Tier 5: Emerging / Experimental
- Agentrial, Mocktopus, ContextCheck, Agent Leaderboard

### White Spaces Identified
1. **Statistical rigor for agent evaluation** — only Agentrial (16 stars) addresses this
2. **Deterministic testing infrastructure** — Mocktopus (6 stars) is alone
3. **Enterprise multi-turn agent evaluation** — Agent Leaderboard is the only attempt
4. **Unified eval across modalities (text+code+tools+agents)** — no single tool does this well
5. **Evaluation-as-code for CI/CD** — Promptfoo leads but competition is thin
6. **Cross-framework agent evaluation** — most tools are coupled to specific agent frameworks

---

## Key Landscape Observations

1. **Massive fragmentation**: 65+ significant projects across must-include and broader landscape, with no single dominant platform.
2. **Two camps**: Open-source libraries (DeepEval, RAGAS, Inspect, Promptfoo) vs. commercial platforms (Braintrust, LangSmith, Maxim, Galileo).
3. **Convergence trend**: Observability platforms (Langfuse, Arize, Opik) adding evaluation; eval tools (DeepEval, TruLens) adding observability.
4. **Agent evaluation is the frontier**: Multiple projects racing to define agent evaluation methodology — trajectory matching, tool call validation, plan quality, convergence scoring.
5. **Consolidation beginning**: OpenAI acquired Promptfoo (March 2026), Anthropic acquiring Humanloop, Snowflake backing TruLens — Big AI companies consuming eval startups.
6. **Security convergence**: Evaluation and red-teaming/security testing are merging (Promptfoo, Giskard, Garak, DeepTeam).
7. **OpenTelemetry emerging as standard**: Multiple projects (agentevals-dev, TruLens, Langfuse, Phoenix, Laminar) building on OTel for trace-based evaluation.
8. **Python dominance**: Nearly all eval tools are Python-first. TypeScript support exists (Promptfoo, OpenEvals, Langfuse) but is secondary.
