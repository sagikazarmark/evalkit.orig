> **📦 Archived on 2026-04-23** — superseded by [Stream 2 — Domain Language](../../../docs/research/02-domain-language.md). Kept for historical reference.

# Stream 2 — Domain Language

## Sources

- https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents
- https://learn.microsoft.com/en-us/ai/playbook/technology-guidance/generative-ai/working-with-llms/evaluation/list-of-eval-metrics
- https://www.confident-ai.com/blog/llm-evaluation-metrics-everything-you-need-for-llm-evaluation
- https://magazine.sebastianraschka.com/p/llm-evaluation-4-approaches
- https://www.evidentlyai.com/llm-guide/llm-evaluation
- https://www.evidentlyai.com/llm-guide/llm-as-a-judge
- https://www.evidentlyai.com/llm-guide/llm-evaluation-metrics
- https://www.guild.ai/glossary/ai-evaluation-evals
- https://docs.ragas.io/en/stable/concepts/metrics/available_metrics/
- https://www.confident-ai.com/blog/rag-evaluation-metrics-answer-relevancy-faithfulness-and-more
- https://www.braintrust.dev/articles/what-is-rag-evaluation
- https://langfuse.com/docs/evaluation/evaluation-methods/llm-as-a-judge
- https://en.wikipedia.org/wiki/LLM-as-a-Judge
- https://www.nist.gov/artificial-intelligence/ai-standards
- https://arize.com/docs/ax/evaluate/evaluators/trace-and-session-evals/trace-level-evaluations/agent-trajectory-evaluations
- https://deepeval.com/guides/guides-ai-agent-evaluation
- https://cloud.google.com/blog/topics/developers-practitioners/a-methodical-approach-to-agent-evaluation

### Authority Sources
- **Specification/Standard**: NIST AI TEVV Zero Drafts (2025), ISO/IEC 42001
- **Authoritative guide**: Anthropic "Demystifying Evals for AI Agents" (2025)
- **Authoritative guide**: Microsoft AI Playbook — Evaluation Metrics
- **Project documentation**: RAGAS metrics documentation, DeepEval documentation, Inspect AI documentation
- **Community conventions**: LangChain/LangSmith, Arize Phoenix, TruLens, Braintrust

---

## Glossary

### Foundational Concepts

#### Evaluation (Eval)
- **Canonical term(s)**: Evaluation, Eval (abbreviation) — source: Anthropic, universally used
- **Alternative terms**: Assessment, Test, Check
- **Who uses what**:
  - All projects: "eval" / "evaluation"
  - NIST: "Testing, Evaluation, Verification and Validation (TEVV)" — treats evaluation as one component of a broader framework
  - DeepEval: "unit test" (positions evals as analogous to software testing)
  - Anthropic: "a test for an AI system: give an AI an input, then apply grading logic to its output to measure success"
- **Settled or contested**: Settled. "Eval" is universal shorthand. However, the *scope* of what "eval" means varies — from a single test case to an entire testing pipeline.
- **Notes**: The distinction between "eval" (specific test) and "evaluation" (the process/discipline) is implicit, rarely formalized.

---

#### Benchmark
- **Canonical term(s)**: Benchmark — source: academic community, HuggingFace, EleutherAI
- **Alternative terms**: Standardized test, Leaderboard eval
- **Who uses what**:
  - EleutherAI lm-evaluation-harness: "benchmark" (60+ standardized benchmarks)
  - HELM (Stanford): "benchmark" (holistic evaluation)
  - OpenAI simple-evals: "benchmark" (MMLU, MATH, GPQA, etc.)
  - DeepEval: "benchmark" (MMLU, HellaSwag, GSM8K)
  - Promptfoo: rarely uses "benchmark" — prefers "test" and "assertion"
- **Settled or contested**: Settled in academic context. In application/product evaluation, "benchmark" is less common — teams prefer "eval" or "test suite."
- **Notes**: Benchmarks are standardized datasets + metrics used for comparing models. Evals are broader and can be custom. Benchmark ⊂ Eval.

---

#### Metric / Score
- **Canonical term(s)**: Metric, Score — source: universal
- **Alternative terms**: Measure, Grade, Rating, Assessment
- **Who uses what**:
  - TruLens: recently renamed from "Feedback" to "Metric" as unified API
  - DeepEval: "Metric" (PlanQualityMetric, TaskCompletionMetric, etc.)
  - RAGAS: "Metric" (Faithfulness, Answer Relevancy, etc.)
  - Inspect AI: "Scorer" (the component that produces metrics)
  - Arize: "Evaluator" (produces scores via templates)
  - OpenEvals: "Evaluator" (function that returns a score)
- **Settled or contested**: "Metric" is the most common term for what is being measured. The *component* that produces a metric has varied naming (see Grader/Scorer/Evaluator below).

---

### Evaluation Architecture Concepts

#### Grader / Scorer / Evaluator / Judge
- **Canonical term(s)**: No single canonical term — this is one of the most fractured terms in the domain
- **Alternative terms**: Grader, Scorer, Evaluator, Judge, Assessor, Checker, Critic
- **Who uses what**:
  - Anthropic: **"Grader"** — "logic scoring some aspect of agent performance"
  - Inspect AI: **"Scorer"** — component that evaluates solver outputs
  - DeepEval: **"Metric"** — the metric object itself does the evaluation
  - RAGAS: **"Metric"** — same pattern as DeepEval
  - OpenEvals: **"Evaluator"** — function that evaluates and returns scores
  - LangChain AgentEvals: **"Evaluator"** — trajectory evaluator functions
  - Arize/Phoenix: **"Evaluator"** — pre-built LLM-as-judge templates
  - Braintrust: **"Scorer"** — scoring functions applied to traces
  - Promptfoo: **"Assertion"** — 70+ assertion types
  - OpenAI Evals: **"Eval"** — the eval itself contains grading logic
  - Deepchecks: **"Check"** — ML validation terminology
  - TruLens: **"Feedback Function"** (legacy) → **"Metric"** (new API)
- **Settled or contested**: **Highly contested.** This is the single most fragmented term in the domain. Every project uses a different word for essentially the same concept.
- **Notes**: The lack of consensus here creates real confusion when switching between tools. "Grader" (Anthropic), "Scorer" (Inspect AI, Braintrust), and "Evaluator" (LangChain, Arize) are the three most common variants.

---

#### Task / Test Case / Problem
- **Canonical term(s)**: No single canonical term
- **Alternative terms**: Task, Test Case, Problem, Scenario, Example, Sample
- **Who uses what**:
  - Inspect AI: **"Task"** (decorated with `@task`)
  - Anthropic: **"Task"** — a single test with defined inputs and success criteria
  - DeepEval: **"Test Case"** (LLMTestCase class) — mirrors pytest naming
  - Promptfoo: **"Test"** — defined in YAML configuration
  - EleutherAI harness: **"Task"** — YAML-configured evaluation task
  - OpenAI Evals: **"Eval"** — the eval and the test case are conflated
  - RAGAS: **"Sample"** — a single evaluation data point
  - Giskard: **"Scenario"** — scenario-based test generation
  - AWS Agent Eval: **"Test Plan"** — collection of test conversations
- **Settled or contested**: Contested. "Task" is most common in research; "Test Case" in developer-oriented tools.

---

#### Dataset / Test Set
- **Canonical term(s)**: Dataset — source: universal in ML
- **Alternative terms**: Test Set, Test Suite, Evaluation Suite, Benchmark Dataset, Golden Dataset
- **Who uses what**:
  - Inspect AI: **"Dataset"** — labeled samples with inputs and targets
  - DeepEval: **"Dataset"** — collection of test cases
  - RAGAS: **"Test Set"** — generated via knowledge graph
  - Braintrust: **"Dataset"** — curated from production traces or manual creation
  - LangSmith: **"Dataset"** — test sets for offline evaluation
  - Promptfoo: **"Test Suite"** — YAML-defined collection of tests
  - Anthropic: **"Evaluation Suite"** — collection of tasks
- **Settled or contested**: Mostly settled. "Dataset" is standard, though the distinction between "dataset" (data) and "test suite" (data + assertions) varies.

---

#### Ground Truth / Golden Answer / Reference / Target
- **Canonical term(s)**: Ground Truth — source: ML tradition
- **Alternative terms**: Golden Answer, Golden Reference, Reference Output, Target, Expected Output, Label
- **Who uses what**:
  - Microsoft: **"Ground Truth"** — the expected outcome
  - Inspect AI: **"Target"** — target values in dataset samples
  - DeepEval: **"Expected Output"** — in test cases
  - RAGAS: **"Ground Truth"** / **"Reference"** — for recall metrics
  - OpenEvals: **"Reference Output"** — in evaluator calls
  - Braintrust: **"Expected"** — in dataset entries
  - Promptfoo: **"Expected"** — in YAML test definitions
  - AWS: **"Ground Truth"** — curation and review practices documented
- **Settled or contested**: Semantically settled (everyone agrees on the concept), but naming is fractured. "Ground truth," "golden answer," "reference," "target," and "expected output" are used interchangeably.

---

### Observability Concepts

#### Trace / Trajectory / Transcript
- **Canonical term(s)**: Trace (observability context), Trajectory (evaluation context) — both used
- **Alternative terms**: Transcript, Execution Log, Run
- **Who uses what**:
  - Anthropic: Uses all three interchangeably — "the complete record of a trial, including outputs, tool calls, reasoning, intermediate results"
  - OpenTelemetry/Langfuse/Phoenix: **"Trace"** — observability-native term
  - LangChain AgentEvals: **"Trajectory"** — sequence of agent steps
  - DeepEval: **"Trace"** — captured via `@observe` decorator
  - Arize: **"Trace"** (observability) / **"Trajectory"** (for agent evaluation)
  - agentevals-dev: **"Trace"** — OTel traces specifically
  - TruLens: **"Trace"** — OpenTelemetry spans
  - Braintrust: **"Trace"** — nested spans of AI interactions
- **Settled or contested**: Partially settled. "Trace" dominates in observability contexts (influenced by OpenTelemetry). "Trajectory" is preferred in agent evaluation research. Some projects (Anthropic) treat them as synonyms.
- **Notes**: The distinction matters: a "trace" is an observability record; a "trajectory" implies a sequence of decisions/actions evaluated for quality.

---

#### Span
- **Canonical term(s)**: Span — source: OpenTelemetry standard
- **Alternative terms**: Step, Event, Operation
- **Who uses what**:
  - OpenTelemetry / Langfuse / Phoenix / agentevals-dev / TruLens: **"Span"** — individual operation within a trace
  - DeepEval: **"Span"** — typed as `type="llm"`, `type="tool"`, or `type="agent"`
  - Braintrust: **"Span"** — nested within traces
  - Inspect AI: **"Step"** — individual solver step within evaluation
  - Anthropic: lists span types as "LLM calls, tool invocations, retrieval operations or reasoning steps"
- **Settled or contested**: Settled in OTel-aligned tools. Less used in research-focused tools that prefer "step."

---

#### Tool Call / Function Call
- **Canonical term(s)**: Tool Call (dominant), Function Call (legacy OpenAI term)
- **Alternative terms**: Action, Tool Use, Tool Invocation
- **Who uses what**:
  - OpenAI: Originally "function call" → now **"tool call"** (API change)
  - Anthropic: **"Tool Use"**
  - LangChain/AgentEvals: **"Tool Call"**
  - DeepEval: **"Tool Call"** (ToolCorrectnessMetric, ArgumentCorrectnessMetric)
  - Inspect AI: **"Tool"** — built-in tools (bash, python, web_search, etc.)
  - Arize: **"Tool Call"** — evaluated via tool calling templates
- **Settled or contested**: Mostly settled around "tool call." OpenAI's migration from "function call" to "tool call" was significant. Anthropic uses "tool use" but the concept is identical.

---

### LLM-as-Judge Concepts

#### LLM-as-a-Judge
- **Canonical term(s)**: LLM-as-a-Judge — source: academic literature, now Wikipedia-notable
- **Alternative terms**: LLM Judge, Model-graded, LLM-based evaluation, AI judge, LLM evaluator
- **Who uses what**:
  - Most projects: **"LLM-as-a-Judge"** or **"LLM-as-Judge"**
  - OpenAI Evals: **"Model-graded"** (legacy term from original evals framework)
  - Langfuse: **"LLM-as-a-Judge"** — documented evaluation method
  - Arize: **"LLM-as-a-Judge"** — pre-built judge templates
  - DeepEval: **"LLM-as-a-Judge"** — underlying mechanism for most metrics
  - Inspect AI: **"Model grading"** — scorer type
- **Settled or contested**: Largely settled. "LLM-as-a-Judge" has become the standard term, even earning a Wikipedia article. "Model-graded" persists in older OpenAI-influenced tools.

---

#### G-Eval
- **Canonical term(s)**: G-Eval — source: academic paper (Liu et al., 2023)
- **Alternative terms**: None standard
- **Who uses what**:
  - DeepEval: **"GEval"** — custom LLM-as-judge metric with chain-of-thought
  - Academic literature: **"G-Eval"** — framework for LLM evaluation using GPT-4
- **Settled or contested**: Settled as an academic term. DeepEval has popularized it as a specific metric class.

---

### RAG Evaluation Concepts

#### Faithfulness / Groundedness
- **Canonical term(s)**: Faithfulness (RAGAS origin), Groundedness (TruLens origin)
- **Alternative terms**: Factual consistency, Factual correctness
- **Who uses what**:
  - RAGAS: **"Faithfulness"** — factual consistency of answer against context
  - TruLens: **"Groundedness"** — part of RAG Triad
  - DeepEval: **"Faithfulness"** — following RAGAS convention
  - Braintrust: Uses both **"faithfulness"** and **"groundedness"**
  - Arize: **"Hallucination detection"** — inverse framing
  - Microsoft: **"Factual consistency"** — more formal term
- **Settled or contested**: **Contested.** Faithfulness and groundedness measure essentially the same thing (are claims supported by context?) but come from different projects. Faithfulness is slightly more common due to RAGAS influence.
- **Notes**: Subtle difference: "faithfulness" = overall answer truthfulness relative to sources; "groundedness" = individual claim verification against documents.

---

#### Answer Relevancy / Response Relevancy
- **Canonical term(s)**: Answer Relevancy — source: RAGAS
- **Alternative terms**: Response Relevancy, Answer Quality
- **Who uses what**:
  - RAGAS: **"Answer Relevancy"** / **"Response Relevancy"**
  - DeepEval: **"Answer Relevancy"**
  - TruLens: Part of **"RAG Triad"** (answer relevance)
  - Arize: **"QA Correctness"**
- **Settled or contested**: Mostly settled around "Answer Relevancy" due to RAGAS influence.

---

#### Context Precision / Context Recall
- **Canonical term(s)**: Context Precision, Context Recall — source: RAGAS
- **Alternative terms**: Retrieval Precision, Retrieval Recall, Context Relevancy
- **Who uses what**:
  - RAGAS: **"Context Precision"**, **"Context Recall"**, **"Context Relevancy"**
  - DeepEval: **"Contextual Precision"**, **"Contextual Recall"**, **"Contextual Relevancy"**
  - TruLens: **"Context Relevance"** (part of RAG Triad)
  - Arize: **"Retrieval Relevance"**
- **Settled or contested**: Partially contested. RAGAS set the vocabulary, but other tools use slight variations ("Contextual" vs "Context", "Relevancy" vs "Relevance").

---

### Agent Evaluation Concepts

#### Agent Trajectory Evaluation
- **Canonical term(s)**: Trajectory evaluation — source: LangChain, Anthropic
- **Alternative terms**: Path evaluation, Step evaluation, Tool sequence evaluation
- **Who uses what**:
  - LangChain AgentEvals: **"Trajectory evaluation"** — strict/unordered/subset matching
  - Arize: **"Path evaluation"** — iteration counting, loop detection
  - DeepEval: **"Step efficiency"**, **"Tool correctness"** — component-level
  - agentevals-dev: **"Tool trajectory matching"**
  - TruLens: **"Agent GPA"** — Goal, Plan, Actions framework
  - Anthropic: Trajectory is part of the transcript/trace
- **Settled or contested**: The *concept* is settled (evaluating the steps an agent takes), but naming varies significantly.

---

#### Convergence / Path Efficiency
- **Canonical term(s)**: No single canonical term
- **Alternative terms**: Convergence score, Step efficiency, Path optimality
- **Who uses what**:
  - Arize: **"Convergence Scoring"** — 0-1 numeric score for path efficiency
  - DeepEval: **"StepEfficiencyMetric"**
  - Agentrial: **"Agent Reliability Score"** — 0-100 composite
- **Settled or contested**: Contested. This is a newer concept and each project names it differently.

---

#### Plan Quality / Plan Adherence
- **Canonical term(s)**: No standard yet
- **Who uses what**:
  - DeepEval: **"PlanQualityMetric"**, **"PlanAdherenceMetric"**
  - TruLens: Part of **"Agent GPA"** — Plan component
  - Arize: **"Agent Planning"** — evaluator template
- **Settled or contested**: Emerging concept. DeepEval is the most explicit.

---

### Evaluation Workflow Concepts

#### Online vs Offline Evaluation
- **Canonical term(s)**: Online evaluation, Offline evaluation — source: LangSmith, Braintrust
- **Alternative terms**: Production evaluation vs Development evaluation, Real-time vs Batch evaluation
- **Who uses what**:
  - LangSmith: **"Online evaluation"** (production traffic) vs **"Offline evaluation"** (curated datasets)
  - Braintrust: Similar distinction — production monitoring vs experiment runs
  - Humanloop: **"Online evaluations"** on live data vs automated CI/CD evals
  - Langfuse: Evaluation on historical traces vs pre-deployment
- **Settled or contested**: Mostly settled terminology, widely understood.

---

#### Regression Eval vs Capability Eval
- **Canonical term(s)**: Regression eval, Capability eval — source: Anthropic
- **Alternative terms**: Regression test, Quality gate
- **Who uses what**:
  - Anthropic: **"Regression evals"** (maintain ~100% pass rate) vs **"Capability evals"** (lower initial pass rates, measure improvement)
  - Promptfoo: **"Regression testing"** — CI/CD integration for preventing regressions
  - LangSmith: Regression detection in offline evaluation
- **Settled or contested**: The concept is understood but not all projects use these exact terms.

---

#### pass@k / pass^k
- **Canonical term(s)**: pass@k — source: HumanEval paper (Chen et al., 2021)
- **Alternative terms**: None standard
- **Who uses what**:
  - Anthropic: **"pass@k"** (at least one correct in k attempts) vs **"pass^k"** (all k succeed)
  - EvalPlus: **"pass@k"** — standard code evaluation metric
  - Academic community: **"pass@k"** — widely adopted
- **Settled or contested**: Settled. pass@k is universally understood in code evaluation. pass^k (Anthropic's variant) is less widespread.

---

#### Trial
- **Canonical term(s)**: Trial — source: Anthropic
- **Alternative terms**: Run, Attempt, Execution
- **Who uses what**:
  - Anthropic: **"Trial"** — one attempt at completing a task
  - Agentrial: **"Trial"** — multi-trial statistical testing
  - Most other tools: **"Run"** — less formal
- **Settled or contested**: "Trial" is more formal/precise; "run" is more common in practice.

---

#### Evaluation Harness
- **Canonical term(s)**: Evaluation Harness — source: EleutherAI
- **Alternative terms**: Eval framework, Eval platform, Eval runner
- **Who uses what**:
  - EleutherAI: **"Evaluation Harness"** (lm-evaluation-harness)
  - Anthropic: **"Evaluation Harness"** — infrastructure running evals end-to-end
  - Most commercial tools: **"Platform"** (Braintrust, LangSmith, Arize)
  - Open-source tools: **"Framework"** (DeepEval, RAGAS, Inspect AI)
- **Settled or contested**: "Harness" is used in academic/research contexts. "Framework" and "Platform" dominate in practice.

---

### Safety & Security Concepts

#### Red Teaming
- **Canonical term(s)**: Red teaming — source: security/military tradition, adopted by AI safety
- **Alternative terms**: Adversarial testing, Penetration testing, Vulnerability scanning
- **Who uses what**:
  - Promptfoo: **"Red teaming"** / **"Vulnerability scanning"**
  - Giskard: **"Vulnerability scanning"** / **"Red teaming"**
  - NVIDIA Garak: **"Red teaming"** — "Nmap for LLMs"
  - Microsoft PyRIT: **"Red teaming"** — "Risk Identification Toolkit"
  - DeepTeam: **"Red teaming"** / **"Penetration testing"**
- **Settled or contested**: "Red teaming" is settled for the overall practice. Sub-activities (vulnerability scanning, adversarial testing) have their own terms.

---

#### Guardrails
- **Canonical term(s)**: Guardrails — source: Guardrails AI, industry adoption
- **Alternative terms**: Safety filters, Content filters, Output validation, Shields
- **Who uses what**:
  - Guardrails AI: **"Guards"** / **"Guardrails"**
  - Opik: **"Guardrails"**
  - Galileo: **"Guardrails"** — low-latency production guardrails
  - Humanloop: **"Guardrails"**
  - Meta/Llama: **"Llama Guard"** / **"Shields"**
- **Settled or contested**: "Guardrails" is becoming standard for runtime safety checks. Distinct from evaluation (guardrails are enforcement; evals are measurement).

---

#### Hallucination
- **Canonical term(s)**: Hallucination — widely adopted
- **Alternative terms**: Confabulation, Fabrication, Unfaithful generation
- **Who uses what**:
  - Virtually all projects: **"Hallucination"**
  - Some researchers prefer: **"Confabulation"** (more technically accurate)
  - UQLM: **"Hallucination detection"** via uncertainty quantification
  - Arize: **"Hallucination detection"** — inverse of faithfulness
- **Settled or contested**: "Hallucination" is universally used despite some academic pushback on the metaphor's accuracy.

---

### Inspect AI-Specific Terminology

#### Solver
- **Canonical term(s)**: Solver — unique to Inspect AI
- **Notes**: Inspect AI uses "Solver" for the component that processes inputs — ranging from simple model calls to complex agent scaffolds. No other project uses this term. Most analogous to an "agent" or "chain" in other frameworks.

#### Dataset → Solver → Scorer Pipeline
- **Notes**: Inspect AI's three-component architecture (Dataset, Solver, Scorer) is unique. Other projects typically have two components: inputs/datasets and evaluators/graders.

---

## Naming Patterns

### Common Prefixes/Suffixes
- **Metric suffix**: DeepEval appends "Metric" to all metrics (PlanQualityMetric, TaskCompletionMetric)
- **Eval prefix/suffix**: "Eval" used as both prefix (EvalPlus) and suffix (AgentEval, AlpacaEval, DeepEval)
- **Score suffix**: BERTScore, MoverScore, BLEU Score, ROUGE Score

### Abbreviation Norms
- **LLM**: Large Language Model (universal)
- **RAG**: Retrieval-Augmented Generation (universal)
- **OTel**: OpenTelemetry (observability community)
- **TEVV**: Testing, Evaluation, Verification and Validation (NIST)
- **RLHF**: Reinforcement Learning from Human Feedback (training, but referenced in eval contexts)
- **CoT**: Chain-of-Thought (prompting technique used in evaluation)
- **SLM**: Small Language Model (e.g., Galileo's Luna-2 for evaluation)

### Compound Term Patterns
- **X-as-a-Judge**: LLM-as-a-Judge, Agent-as-a-Judge (Deepchecks)
- **X for LLMs**: "pytest for LLMs" (DeepEval), "Nmap for LLMs" (Garak)
- **X Triad**: RAG Triad (TruLens — answer relevance, context relevance, groundedness)
- **Agent X**: AgentEval, AgentEvals, Agent GPA, Agent Reliability Score

### Terms Borrowed from Adjacent Domains
- **From software testing**: "unit test," "regression test," "CI/CD," "assertion" — accurately borrowed
- **From observability/monitoring**: "trace," "span," "observability," "dashboard" — accurately borrowed from OpenTelemetry
- **From information retrieval**: "precision," "recall," "F1," "NDCG" — accurately borrowed
- **From NLP**: "BLEU," "ROUGE," "BERTScore" — accurately borrowed but increasingly less relevant for LLM evaluation
- **From security**: "red teaming," "penetration testing," "vulnerability scanning" — accurately borrowed
- **From ML ops**: "experiment," "artifact," "model registry" — accurately borrowed
- **Questionable borrowing**: "hallucination" — borrowed from psychology/psychiatry; technically imprecise for LLMs but universally adopted

---

## Summary of Terminology Fragmentation

| Concept | Most Common Terms | Fragmentation Level |
|---------|-------------------|-------------------|
| The test | eval, evaluation | Low |
| Standardized test set | benchmark | Low |
| What is measured | metric, score | Low |
| Component that measures | grader, scorer, evaluator, judge | **Very High** |
| Single test item | task, test case, problem, sample | **High** |
| Expected answer | ground truth, golden, reference, target, expected | **High** |
| Execution record | trace, trajectory, transcript | **Medium** |
| Individual operation | span, step | Low-Medium |
| LLM-based scoring | LLM-as-a-Judge, model-graded | Low |
| Answer accuracy vs context | faithfulness, groundedness | **Medium** |
| Testing for safety | red teaming, vulnerability scanning | Low |
| Runtime safety | guardrails | Low |

The highest fragmentation exists in the naming of evaluation components (grader/scorer/evaluator/judge) and expected outputs (ground truth/golden/reference/target). A new entrant could gain clarity advantage by choosing consistent, well-documented terminology.
