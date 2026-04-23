> **📦 Archived on 2026-04-23** — superseded by no successor — this work was abandoned. Kept for historical reference.

# Stream 3 — User Segments & Workflows

## Sources

- https://newsletter.pragmaticengineer.com/p/evals
- https://blog.langchain.com/agent-evaluation-readiness-checklist/
- https://www.langchain.com/state-of-agent-engineering
- https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents
- https://www.evidentlyai.com/llm-guide/llm-evaluation
- https://arize.com/blog/how-to-add-llm-evaluations-to-ci-cd-pipelines/
- https://www.promptfoo.dev/docs/integrations/ci-cd/
- https://deepeval.com/docs/evaluation-unit-testing-in-ci-cd
- https://www.braintrust.dev/articles/best-ai-evals-tools-cicd-2025
- https://langfuse.com/docs/evaluation/overview
- https://maven.com/parlance-labs/evals
- https://cloud.google.com/blog/topics/developers-practitioners/a-methodical-approach-to-agent-evaluation
- https://orq.ai/blog/agent-evaluation
- https://www.turingcollege.com/blog/evaluating-ai-agents-practical-guide
- https://www.datadoghq.com/blog/llm-evaluation-framework-best-practices/

---

## User Segments

### 1. AI/LLM Application Engineer
- **Segment**: Software engineer building LLM-powered applications (chatbots, RAG systems, AI assistants, agents)
- **Goals**: Ship reliable AI features, prevent regressions, ensure output quality, reduce production incidents
- **Context**: Teams of 2-10 engineers, using LangChain/LlamaIndex/custom frameworks, deploying to production with CI/CD. May not have ML background — software engineering background adapting to AI.
- **Tools they use**: DeepEval, Promptfoo, OpenEvals, Langfuse, Braintrust, LangSmith. IDE extensions, pytest/vitest.
- **How they evaluate tools**: Developer experience (DX) first — easy setup, pytest-like interface, good docs. Then: metric quality, CI/CD integration, cost. Prefer open-source with optional cloud.

### 2. ML/AI Engineer at Enterprise
- **Segment**: Machine learning engineer at a mid-to-large company managing AI systems at scale
- **Goals**: Systematic quality assurance, model comparison, experiment tracking, compliance reporting, production monitoring
- **Context**: Teams of 5-20+, formal ML ops practices, regulatory requirements (finance, healthcare), multiple models in production. Strong ML background.
- **Tools they use**: MLflow, LangSmith, Arize, Braintrust, Langfuse, W&B Weave. Internal custom tooling.
- **How they evaluate tools**: Enterprise features first — SSO, RBAC, compliance (SOC 2, HIPAA), data residency, self-hosting options. Then: integration breadth, scalability, experiment management.

### 3. AI Safety / Alignment Researcher
- **Segment**: Researcher at an AI lab, government institute, or academic institution evaluating model capabilities and risks
- **Goals**: Understand model capabilities, identify dangerous behaviors, benchmark against standards, publish reproducible results
- **Context**: Small teams (1-5), deep technical expertise, need reproducibility, often work across many models. Academic rigor matters more than DX.
- **Tools they use**: Inspect AI, EleutherAI lm-evaluation-harness, HELM, OpenAI Evals, custom scripts. Garak for red-teaming.
- **How they evaluate tools**: Reproducibility, benchmark coverage, sandboxing for agentic tasks, multi-model support. Academic citations and community adoption matter.

### 4. Product Manager / Domain Expert
- **Segment**: Non-technical or semi-technical stakeholder responsible for AI product quality
- **Goals**: Ensure AI meets business requirements, review edge cases, approve prompt changes, set quality standards
- **Context**: Works alongside engineering team, needs to review AI outputs without writing code. May have domain expertise (legal, medical, finance) but not ML expertise.
- **Tools they use**: Humanloop, Braintrust (custom trace views), Agenta, Deepchecks (no-code evaluator builder). Annotation interfaces.
- **How they evaluate tools**: No-code/low-code interfaces, intuitive dashboards, ability to provide feedback, collaboration features with engineering team.

### 5. AI Security / Red Team Engineer
- **Segment**: Security professional focused on AI system vulnerabilities, prompt injection, jailbreaks, data leakage
- **Goals**: Identify vulnerabilities before deployment, test guardrails, comply with security standards (OWASP Top 10 for LLMs)
- **Context**: Part of security team or dedicated AI security role. May come from traditional security background.
- **Tools they use**: Promptfoo (red-teaming), Garak, Giskard, PyRIT, DeepTeam, Agentic Security.
- **How they evaluate tools**: Coverage of attack vectors, compliance framework support (OWASP, NIST, MITRE ATLAS), automation capabilities, CI/CD integration.

### 6. Platform/Infrastructure Engineer
- **Segment**: Engineer responsible for AI platform infrastructure — observability, deployment, cost management
- **Goals**: Monitor production AI systems, track costs and latency, ensure SLAs, provide eval infrastructure for other teams
- **Context**: DevOps/SRE background adapting to AI. Manages infrastructure for multiple AI applications across the org.
- **Tools they use**: Langfuse, Arize Phoenix, Helicone, Opik, Datadog (AI monitoring). OpenTelemetry instrumentation.
- **How they evaluate tools**: Scalability (traces/day capacity), self-hosting capability, OpenTelemetry compatibility, multi-tenant support, cost.

---

## Jobs-to-be-Done

### Job 1: Understand What's Going Wrong (Error Analysis)
- **Job**: Systematically identify and categorize failure modes in an AI system before trying to fix them
- **Who**: AI Engineers, ML Engineers, Product Managers
- **Current workflow**:
  1. Collect production traces or test outputs (Langfuse, Braintrust, LangSmith, or custom viewer)
  2. Manually review 20-100+ diverse interactions (often in a spreadsheet or custom UI)
  3. Annotate failures with descriptive observations ("agent missed re-engagement opportunity")
  4. Group annotations into 5-10 failure themes (sometimes with LLM assistance)
  5. Prioritize failure modes by severity and frequency
  6. Design targeted evaluations for each failure mode
- **Friction points**: 
  - Manual review is time-consuming — teams often skip it and jump to "vibe checking"
  - Generic tools don't show enough context (tool calls, retrieved documents, reasoning traces) on one screen
  - NurtureBoss example: built custom data viewer because off-the-shelf tools were insufficient
  - No standard methodology — each team reinvents the error analysis process
- **Automation gaps**: Automated failure clustering, LLM-assisted annotation at scale, standardized error taxonomies
- **Tool boundaries**: Switch between observability tool (traces) → spreadsheet (annotation) → code (designing evals). Each transition loses context.

### Job 2: Build and Manage Evaluation Datasets
- **Job**: Create, curate, and maintain high-quality test datasets that represent real-world scenarios
- **Who**: AI Engineers, ML Engineers, Domain Experts
- **Current workflow**:
  1. Start with manual creation of "golden" examples (20-50 hand-crafted test cases)
  2. Optionally generate synthetic examples using LLMs (RAGAS knowledge-graph approach, or custom generation)
  3. Include positive and negative test cases, edge cases, adversarial inputs
  4. Store in datasets (LangSmith datasets, Braintrust datasets, or CSV/JSON files)
  5. Version and maintain over time — add production failures as new test cases
  6. Match dataset structure to evaluation type (single-turn, multi-turn, trajectory)
- **Friction points**:
  - Creating good test cases requires deep domain expertise
  - Synthetic generation can produce unrealistic scenarios
  - Datasets go stale as the product evolves
  - No good feedback loop from production failures → test cases (manual process)
  - Different tools use different dataset formats — portability is poor
- **Automation gaps**: Automated production-failure-to-test-case pipeline, dataset quality assessment, drift detection on datasets
- **Tool boundaries**: Switch between production logs → dataset creation tool → evaluation runner. Braintrust's "Trace-to-Dataset" feature addresses this gap.

### Job 3: Run Evaluations During Development
- **Job**: Test prompt changes, model swaps, or code changes against quality benchmarks before shipping
- **Who**: AI Engineers, ML Engineers
- **Current workflow**:
  1. Make a change (new prompt, different model, updated retrieval logic)
  2. Run evaluations locally (DeepEval pytest, Promptfoo CLI, or custom scripts)
  3. Review results — compare metrics to baseline
  4. Iterate on changes based on eval results
  5. Pass evaluation thresholds → merge code
- **Friction points**:
  - Running evals is slow and expensive (LLM calls cost money, full eval suites take 10-30 minutes)
  - Non-determinism — same input can produce different results across runs
  - Need to run multiple trials and aggregate (few tools support this natively)
  - Hard to compare results across experiments without a platform
  - Setting up eval infrastructure from scratch is significant effort
- **Automation gaps**: Smart eval selection (run only affected evals), cost-efficient evaluation strategies, statistical significance testing
- **Tool boundaries**: IDE → CLI → eval platform dashboard. Results often viewed in different UIs.

### Job 4: Integrate Evaluations into CI/CD
- **Job**: Automatically run evaluations on every code change and block deployments that fail quality gates
- **Who**: AI Engineers, Platform Engineers
- **Current workflow**:
  1. Configure eval suite as GitHub Action / CI job (Promptfoo, DeepEval, custom scripts)
  2. Define pass/fail thresholds for each metric
  3. Run subset of evals on every PR (fast evals), full suite on merge to main
  4. Block merge if evals fail — similar to test suite for traditional software
  5. Report results as PR comments or dashboard links
- **Friction points**:
  - LLM API costs for running evals in CI (every PR triggers LLM calls)
  - Flaky evals due to non-determinism cause false failures
  - Slow eval suites (10-30+ minutes) slow down CI pipeline
  - Managing API keys and secrets for eval LLMs in CI
  - Threshold calibration is trial-and-error — too strict = false failures, too loose = missed regressions
- **Automation gaps**: Incremental eval execution (only run affected evals), deterministic mock-based evaluation for fast CI, adaptive thresholds
- **Tool boundaries**: CI system (GitHub Actions) → eval tool (Promptfoo/DeepEval) → results reporting (LangSmith/custom). Configuration scattered across YAML files.

### Job 5: Monitor AI Quality in Production
- **Job**: Continuously evaluate live AI system quality and alert on degradation
- **Who**: ML Engineers, Platform Engineers, Product Managers
- **Current workflow**:
  1. Instrument production system with tracing (Langfuse, Arize, Braintrust, LangSmith)
  2. Sample production traffic (1-5%) for automated evaluation
  3. Run LLM-as-Judge or heuristic checks on sampled traces
  4. Set up alerts on metric degradation (Slack, PagerDuty)
  5. Capture user feedback (thumbs up/down, explicit feedback)
  6. Feed production insights back into development datasets
- **Friction points**:
  - Sampling strategy is ad-hoc — no standard for what percentage to evaluate
  - Online evaluation adds latency and cost
  - Correlating eval metrics with actual user satisfaction is hard
  - Alert fatigue if thresholds are poorly calibrated
  - Different tools for tracing vs evaluation vs alerting
- **Automation gaps**: Intelligent sampling (evaluate traces that look anomalous), automatic correlation of eval scores with user outcomes, automated dataset enrichment
- **Tool boundaries**: Tracing system → evaluation engine → alerting system → feedback collection → dataset management. Typically 3-5 different tools.

### Job 6: Benchmark and Compare Models
- **Job**: Evaluate multiple LLM models against each other to select the best one for a use case
- **Who**: AI Engineers, ML Engineers, Researchers
- **Current workflow**:
  1. Define evaluation criteria relevant to use case
  2. Run same test suite against multiple models (Promptfoo side-by-side, EleutherAI harness, HELM)
  3. Compare metrics, cost, latency across models
  4. Consider model-specific tradeoffs (quality vs speed vs cost)
  5. Select model for deployment
- **Friction points**:
  - Setting up evaluation for each model provider requires different API integrations
  - Cost of running comprehensive evaluations across many models
  - Keeping up with new model releases
  - Fair comparison is hard — prompt optimization for one model may disadvantage others
- **Automation gaps**: Automated model comparison across releases, prompt adaptation per model, cost-normalized quality scoring
- **Tool boundaries**: Switch between model providers' APIs → eval framework → comparison dashboard.

### Job 7: Evaluate Agent Behavior (Multi-Step)
- **Job**: Test and evaluate AI agents that perform multi-step tasks with tool use, planning, and environmental interaction
- **Who**: AI Engineers building agents, AI Safety Researchers
- **Current workflow**:
  1. Define agent tasks with success criteria (LangChain readiness checklist: manual review of 20-50 traces first)
  2. Set up evaluation environment (sandboxed execution via Docker/K8s — Inspect AI approach)
  3. Run agent on test tasks, record full trajectory
  4. Evaluate trajectory (tool call correctness, step efficiency, plan quality)
  5. Evaluate outcome (did the task get completed correctly?)
  6. Run multiple trials to account for non-determinism
  7. Aggregate results with statistical rigor
- **Friction points**:
  - Agent evaluation is fundamentally harder than single-turn eval — many valid paths to same goal
  - Setting up sandboxed environments is complex
  - "Grade the outcome, not the exact path" — but defining success criteria for open-ended tasks is hard
  - Non-determinism is more pronounced in agentic workflows
  - Limited tooling for trajectory evaluation — most tools focus on final output
  - Cost of running agent evals is very high (each trial involves many LLM calls)
- **Automation gaps**: Automated sandbox provisioning, trajectory normalization (comparing different valid paths), cost-efficient multi-trial evaluation, reference-free agent evaluation
- **Tool boundaries**: Agent framework (LangGraph/AutoGen/CrewAI) → evaluation framework (AgentEvals/DeepEval/Inspect) → analysis platform. Major seams between these.

### Job 8: Red-Team AI Systems for Safety
- **Job**: Proactively discover vulnerabilities, biases, and unsafe behaviors before deployment
- **Who**: AI Security Engineers, AI Safety Researchers
- **Current workflow**:
  1. Define threat model and attack surface
  2. Select red-teaming probes (prompt injection, jailbreak, data leakage, bias)
  3. Run automated scanning (Promptfoo, Garak, Giskard, DeepTeam)
  4. Review discovered vulnerabilities
  5. Implement mitigations (guardrails, content filters, prompt hardening)
  6. Re-test to verify mitigations work
  7. Ongoing monitoring for novel attack vectors
- **Friction points**:
  - New attack vectors emerge faster than tools can keep up
  - False positive rate can be high — requires expert review
  - Testing multi-turn agent interactions for safety is harder than single-turn
  - No unified framework for vulnerability classification
  - Red-teaming and quality evaluation are separate tools and workflows despite needing the same infrastructure
- **Automation gaps**: Adaptive red-teaming (evolving attacks based on defenses), continuous security monitoring in production, unified eval+safety platform
- **Tool boundaries**: Red-teaming tool (Garak/Promptfoo) → vulnerability tracker → guardrail system → monitoring. Completely separate from quality evaluation pipeline.

---

## Workflow Patterns

### Common Workflows Across Segments
1. **The Eval Flywheel**: Analyze → Measure → Improve → Automate → Repeat. Every segment follows this pattern, differing only in sophistication.
2. **The Three-Layer Evaluation Stack**: Deterministic checks (format, schema) → Heuristic/code-based scoring → LLM-as-Judge for nuance. Most teams layer these.
3. **Manual-First to Automated**: Teams start with manual review, build custom evaluators from insights, then automate. The LangChain readiness checklist codifies this.

### Multi-Tool Workflows
Users routinely cobble together multiple tools:
- **Tracing + Evaluation**: Langfuse/Arize for tracing → DeepEval/custom for scoring → LangSmith for experiment tracking
- **Development + Production**: Promptfoo in CI → Braintrust for experiments → Langfuse for production monitoring
- **Quality + Safety**: DeepEval for quality metrics → Promptfoo/Garak for red-teaming → Guardrails AI for runtime
- **Research**: Inspect AI for agentic benchmarks → EleutherAI harness for model benchmarks → HELM for holistic evaluation

### Glue Work Patterns
- **Custom data viewers**: Teams build custom UIs to view traces with enough context for error analysis (e.g., NurtureBoss example)
- **Dataset format conversion**: Converting between tool-specific dataset formats (JSON, CSV, YAML, LangSmith datasets)
- **Result aggregation scripts**: Custom scripts to aggregate results from multiple eval tools into unified dashboards
- **Threshold calibration notebooks**: Jupyter notebooks for calibrating LLM-as-Judge against human labels
- **Production-to-dev pipelines**: Custom scripts to extract production failures and format them as test cases

### Workflows No Single Tool Handles End-to-End
1. **Error analysis → eval design → evaluation → monitoring → feedback loop**: The complete eval lifecycle spans 3-5 tools
2. **Agent development with safety**: Quality evaluation + red-teaming + guardrail testing done in completely separate tools
3. **Cross-model evaluation with cost optimization**: Comparing models while accounting for cost/latency/quality tradeoffs
4. **Human-in-the-loop evaluation pipeline**: Human labeling → LLM-as-Judge calibration → automated evaluation → production monitoring
5. **Multi-modal evaluation**: Text + code + tool calls + images evaluated together — no single tool handles all modalities

### Maturity Levels (from LangChain/Braintrust ecosystem)
- **Level 0**: Manual testing ("vibe checking" — modify prompt, try a few inputs, ship)
- **Level 1**: Deterministic checks with dozens of eval cases
- **Level 2**: LLM-as-Judge with 200+ test cases, CI/CD integration
- **Level 3**: Multi-criteria evaluation with production monitoring, specialized graders
- **Level 4**: Continuous eval on production traffic, automated red-teaming, feedback loops

### Key Statistics (LangChain State of AI Agents 2026)
- 57% of organizations have agents in production
- 89% have implemented observability (tracing)
- 52% have implemented evaluations (evals significantly lag observability)
- 32% cite quality as the #1 barrier to agent deployment
- 44.8% run online (production) evals
- Quality (32%), latency (20%), and security (24.9% for enterprises) are the top blockers
