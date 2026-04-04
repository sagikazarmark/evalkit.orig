# AI Evaluation Research — Conversation Log

**Date**: 2026-04-02
**Domain**: AI Evals — performance evals, correctness evals, agent evals
**Model**: Claude Opus 4.6 (1M context)

---

## Task

The user requested a comprehensive research investigation into the AI evaluation domain using a detailed 9-stream research protocol (`research-v2.md`). The protocol specified:

- 9 sequential research streams, each producing its own document
- A final synthesis document
- Must-include projects: Inspect AI (AISI), Braintrust, agentevals-dev/agentevals, LangChain AgentEvals, LangChain OpenEvals, AWS Agent Evaluation, OpenAI Agent Evals, Humanloop, DeepEval, Arize, Microsoft AutoGen AgentEval

---

## Execution Summary

### Phase 1: Setup & Planning
- Created `research/` directory
- Set up 10 tasks for tracking progress (9 streams + synthesis)
- Attempted to dispatch 5 parallel research agents for Stream 1 — agents failed because they lacked WebSearch/WebFetch permissions
- Pivoted to doing all web research directly from the main conversation

### Phase 2: Stream 1 — Landscape (01-landscape.md)
**Approach**: Extensive web fetching and searching across all must-include projects plus broader landscape discovery.

**Must-include projects researched via direct WebFetch**:
- Inspect AI (https://inspect.aisi.org.uk/) — 1.9k stars, UK AISI, research-grade eval framework
- Braintrust (https://www.braintrust.dev/) — SaaS platform, $800M valuation, $80M raised Feb 2026
- agentevals-dev (https://github.com/agentevals-dev/agentevals) — 112 stars, OTel-native agent eval
- LangChain AgentEvals (https://github.com/langchain-ai/agentevals) — 534 stars, trajectory evaluation
- LangChain OpenEvals (https://github.com/langchain-ai/openevals) — 1k stars, readymade evaluators
- AWS Agent Evaluation (https://github.com/awslabs/agent-evaluation) — 354 stars, evaluator-as-agent
- OpenAI Agent Evals (https://developers.openai.com/api/docs/guides/agent-evals) — platform-integrated evals
- Humanloop (https://humanloop.com/) — joining Anthropic, human-in-the-loop evaluation
- DeepEval (https://github.com/confident-ai/deepeval) — 14.4k stars, "pytest for LLMs"
- Arize/Phoenix (https://arize.com/) — 9.1k stars (Phoenix), ML observability heritage
- Microsoft AutoGen AgentEval — CriticAgent/QuantifierAgent/VerifierAgent pattern

**Additional projects discovered** (via background agent + direct research):
- Promptfoo (19.1k stars, acquired by OpenAI March 2026 for $86M)
- RAGAS (13.2k stars, de facto RAG evaluation standard)
- Langfuse (24.3k stars, fastest-growing LLMOps platform)
- Opik (18.6k stars, Comet ML, fully open-source)
- EleutherAI lm-evaluation-harness (12k stars, powers HF Leaderboard)
- TruLens (3.2k stars, Snowflake-backed, Agent GPA framework)
- LangSmith (LangChain commercial platform)
- Giskard (5.2k stars, v3 rewrite for agent testing)
- Deepchecks (no-code evaluator builder)
- Maxim AI (agent simulation at scale)
- Galileo AI (Luna-2 specialized evaluation models)
- FutureAGI (multi-modal + voice AI eval)
- Plus 30+ more projects from the background agent's landscape scan

**Key data sources**:
- GitHub repo pages (stars, forks, commits, licenses)
- Project homepages and documentation
- Comparison articles: aimultiple.com, confident-ai.com, arize.com
- Awesome lists: awesome-ai-eval, Awesome-LLM-Eval

**Result**: 65+ projects documented with comparison matrix. One background agent (landscape discovery) successfully wrote a comprehensive broader landscape document covering 55 additional projects.

### Phase 3: Stream 2 — Domain Language (02-domain-language.md)
**Approach**: Web searches for terminology, standards, naming conventions. Fetched Anthropic's eval guide and Microsoft's metric list.

**Key searches**:
- AI evaluation terminology glossary
- LLM evaluation terminology "ground truth" "golden answer"
- AI agent evaluation terminology "trajectory" "tool call" "trace" "span"
- NIST AI evaluation standards ISO AI testing
- "LLM as judge" "model graded" terminology
- RAG evaluation metrics terminology differences

**Key findings**:
- Highest terminology fragmentation: the component that evaluates (grader vs scorer vs evaluator vs judge vs metric)
- Expected output naming: ground truth vs golden answer vs reference vs target vs expected
- Trace vs trajectory vs transcript used with different emphasis
- Faithfulness (RAGAS) vs groundedness (TruLens) for same concept
- OTel semantic conventions becoming standard for observability terms

### Phase 4: Stream 3 — User Segments & Workflows (03-user-workflows.md)
**Approach**: Fetched LangChain readiness checklist, Pragmatic Engineer evals article, searched for workflow tutorials and user personas.

**Key sources**:
- LangChain agent evaluation readiness checklist (6-phase, 37-item checklist)
- Pragmatic Engineer "AI Evals for Devs" (NurtureBoss case study, error analysis methodology)
- LangChain State of AI Agents 2026 (57% agents in production, 52% have evals, 89% have observability)

**Key findings**:
- 6 user segments identified (AI engineer, ML engineer, safety researcher, PM/domain expert, security engineer, platform engineer)
- 8 jobs-to-be-done mapped with friction points and automation gaps
- Critical insight: 89% observability vs 52% eval adoption = massive gap
- Teams use 3-5 different tools across evaluation lifecycle
- Error analysis (the most important step) has the worst tooling

### Phase 5: Stream 4 — Architecture & Technical Patterns (04-architecture.md)
**Approach**: Synthesized architectural patterns from all project research. Searched for design decisions, anti-patterns, lessons learned.

**Key findings**:
- 7 architectural patterns identified (library, SaaS platform, observability-first, OTel-native, evaluator-as-agent, declarative/config-driven, research/sandbox)
- 8 recurring technical decisions mapped with what most projects choose
- 8 anti-patterns documented (vibe checking, over-reliance on LLM-as-judge, benchmark score as quality proxy, single-trial evaluation, grading path not outcome, same model family as judge, confusing guardrails with evaluation, monolithic scoring)

### Phase 6: Stream 5 — Pain Points & Challenges (05-pain-points.md)
**Approach**: Searched for GitHub issues, complaints, limitations for major projects. Searched for domain-wide challenges.

**Key searches**:
- DeepEval issues problems GitHub
- Langfuse evaluation limitations
- "LLM evaluation" "hard problems" challenges unsolved

**Key findings**:
- Project-specific pain points for DeepEval, Langfuse, Promptfoo, LangSmith, Braintrust, Inspect AI, RAGAS, agentevals-dev
- 8 domain-wide challenges (LLM-as-judge reliability, non-determinism, cost, benchmark saturation, open-ended evaluation, fragmented tooling, lack of statistical rigor, dataset creation burden)
- 6 unmet needs identified
- 6 workaround patterns documented

### Phase 7: Stream 6 — Ecosystem & Integration Surface (06-ecosystem.md)
**Approach**: Deep research on OpenTelemetry GenAI semantic conventions, standards, protocols, data formats, adjacent tools.

**Key searches**:
- AI evaluation standards protocols OpenTelemetry
- OTel GenAI semantic conventions documentation

**Key findings**:
- OTel GenAI Semantic Conventions v1.37+ is THE emerging standard
- `gen_ai.evaluation.result` event exists in OTel spec
- 6 standards/protocols mapped (OTel, OTLP, NIST TEVV, OWASP Top 10 for LLMs, MITRE ATLAS, NIST AI RMF)
- 5 data formats (JSONL, YAML, OpenAI chat format, LangChain BaseMessage, Jaeger JSON)
- 8 adjacent tool categories (LLM providers, agent frameworks, vector DBs, CI/CD, observability, prompt management, IDE extensions, MCP)
- 5 integration patterns and 6 compatibility constraints

### Phase 8: Stream 7 — Trajectory & Emerging Solutions (07-trajectory.md)
**Approach**: Searched for trends, predictions, emerging projects, AI/LLM disruption in evaluation.

**Key searches**:
- AI evaluation trends 2025 2026
- LLM evaluation AI disruption automated meta-evaluation
- LangChain state of AI agents 2026 report

**Key findings**:
- 6 active trends (consolidation, eval+security convergence, OTel standardization, agent eval methodology crystallizing, OSS platforms rise, maturity models)
- 6 emerging projects/approaches (Agentrial, Mocktopus, Galileo Luna-2, agentevals-dev, Bloom, Agent-as-a-Judge research)
- 4 paradigm shifts (benchmark→application eval, output→trajectory eval, testing→continuous eval, single→multi-provider)
- 7 predictions for the domain's future
- Stanford: "The era of AI evangelism is giving way to an era of AI evaluation"

### Phase 9: Stream 8 — Community & Adoption Signals (08-community.md)
**Approach**: Compiled GitHub metrics, searched for funding, business models, migration patterns.

**Key searches**:
- LLM eval tool funding investment business model
- Braintrust AI evaluation pricing

**Key findings**:
- Adoption metrics for 30+ projects organized by tier
- Braintrust: $80M raised Feb 2026 at $800M valuation
- Promptfoo: $23M raised, acquired by OpenAI at $86M valuation
- Confident AI: $19.99-49.99/user/month for cloud platform
- 5 migration patterns (homebrew→frameworks, library→platform, LangSmith→OSS alternatives, general→agent-specific, separate→unified eval+safety)
- Key influencers: Hamel Husain, Shreya Shankar, Harrison Chase, Anthropic Engineering

### Phase 10: Stream 9 — Failure Archaeology (09-failure-archaeology.md)
**Approach**: Searched for failed/abandoned eval projects, archived GitHub repos, pivots.

**Key searches**:
- AI eval startup failed abandoned archived GitHub
- Specific GitHub repos identified as archived/stale

**Key findings**:
- 6 dead/abandoned projects (PromptBench archived, simple-evals frozen, Log10 archived, AIConfig stale, HF Evaluate deprecated, PyRIT migrated)
- 4 pivoted projects (Promptfoo→OpenAI, Humanloop→Anthropic, TruLens→Snowflake, Giskard v2→v3)
- 5 structural barriers (platform effect, acquisition gravity, "eval as feature" problem, metric commoditization, OSS sustainability)
- 5 attempted approaches that didn't work (metrics-only library, traditional NLP metrics, static benchmarks as quality proxy, standalone prompt management, single-model evaluation)

### Phase 11: Synthesis (synthesis.md)
**Approach**: Read all 9 stream documents and distilled into actionable synthesis.

**Key outputs**:
1. **Domain Summary** — 5 paragraphs covering the domain state
2. **Landscape Map** — ASCII visualization of market segments and white space
3. **Consensus & Controversy** — 8 areas of agreement, 6 areas of disagreement
4. **Gap Analysis** — 7 gaps with severity, evidence, and opportunity
5. **Prioritization Matrix** — All 7 gaps ranked by severity × feasibility × competitive window × segment reach
6. **Risk Assessment** — 4 risk categories with 14 specific risks and mitigations
7. **Positioning Analysis** — Crowded positions, open positions, wedge opportunities, positioning traps
8. **Go/No-Go Signals** — 9 go signals, 5 no-go signals
9. **Strategic Signals** — 6 day-one requirements, 5 highest-leverage differentiators, 6 things NOT to do
10. **Open Questions** — 10 questions requiring user interviews or prototyping

**Verdict**: GO — with focused positioning. Highest-priority gaps: statistical agent evaluation, error analysis → eval pipeline, unified quality + safety evaluation.

---

## Web Searches Performed

Total web searches: ~30
Total web fetches: ~20

### WebFetch URLs:
1. https://inspect.aisi.org.uk/
2. https://www.braintrust.dev/
3. https://github.com/agentevals-dev/agentevals
4. https://github.com/langchain-ai/agentevals
5. https://github.com/langchain-ai/openevals
6. https://github.com/awslabs/agent-evaluation
7. https://humanloop.com/home
8. https://deepeval.com/guides/guides-ai-agent-evaluation
9. https://developers.openai.com/api/docs/guides/agent-evals
10. https://arize.com/ai-agents/agent-evaluation/
11. https://microsoft.github.io/autogen/0.2/blog/2024/06/21/AgentEval/
12. https://github.com/promptfoo/promptfoo
13. https://github.com/confident-ai/deepeval
14. https://github.com/explodinggradients/ragas
15. https://github.com/langfuse/langfuse
16. https://github.com/truera/trulens
17. https://github.com/comet-ml/opik
18. https://github.com/EleutherAI/lm-evaluation-harness
19. https://www.confident-ai.com/blog/greatest-llm-evaluation-tools-in-2025
20. https://aimultiple.com/llm-eval-tools
21. https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents
22. https://learn.microsoft.com/en-us/ai/playbook/technology-guidance/generative-ai/working-with-llms/evaluation/list-of-eval-metrics
23. https://blog.langchain.com/agent-evaluation-readiness-checklist/
24. https://newsletter.pragmaticengineer.com/p/evals

### WebSearch Queries (sample):
- "AI evaluation framework comparison landscape 2025 2026 LLM eval tools"
- "promptfoo LLM evaluation tool features GitHub"
- "RAGAS evaluation framework features GitHub 2025"
- "langfuse evaluation observability features GitHub 2025"
- "trulens evaluation framework features GitHub 2025"
- "Maxim AI evaluation platform features pricing 2025"
- "Opik Comet evaluation platform features GitHub 2025"
- "giskard AI testing evaluation features GitHub 2025"
- "deepchecks LLM evaluation features GitHub 2025"
- "Inspect AI GitHub stars contributors UKGovernmentBEIS"
- "LangSmith evaluation platform features pricing"
- "OpenAI simple-evals GitHub evals repository"
- "EleutherAI lm-evaluation-harness GitHub features"
- "FutureAGI evaluation platform features"
- "Braintrust AI evaluation pricing plans"
- "DeepEval pricing license open source Confident AI"
- "Galileo AI evaluation platform features pricing"
- "AI evaluation terminology glossary"
- "NIST AI evaluation standards ISO AI testing terminology"
- "LLM as judge model graded terminology"
- "RAG evaluation metrics terminology naming differences"
- "AI evaluation workflow how to evaluate LLM tutorial"
- "agent evaluation workflow tutorial automation pipeline"
- "LLM evaluation glue code manual steps pain points"
- "LangChain state of AI agents 2026 report"
- "LLM evaluation architecture patterns design decisions"
- "LLM evaluation anti-patterns lessons learned"
- "LLM evaluation problems complaints Reddit Hacker News"
- "AI evaluation standards protocols OpenTelemetry"
- "AI evaluation trends 2025 2026 emerging future"
- "AI evaluation shutdown discontinued deprecated failed"
- "DeepEval issues problems GitHub"
- "Langfuse evaluation limitations issues GitHub"
- "LLM evaluation hard problems challenges unsolved"
- "LLM eval tool comparison funding investment business model"
- "AI eval startup failed abandoned archived GitHub"
- "LLM evaluation AI disruption automated meta-evaluation"

---

## Background Agents Dispatched

5 background agents were launched for Stream 1 landscape research. 4 out of 5 failed due to lacking WebSearch/WebFetch permissions. 1 agent (landscape discovery) succeeded and produced a comprehensive document covering 55 projects beyond the must-include list, which was merged into the final 01-landscape.md.

---

## Files Produced

```
research/
├── 01-landscape.md          (~1200 lines, 65+ projects)
├── 02-domain-language.md     (~400 lines, 30+ concepts)
├── 03-user-workflows.md      (~300 lines, 6 segments, 8 jobs)
├── 04-architecture.md        (~250 lines, 7 patterns, 8 decisions, 8 anti-patterns)
├── 05-pain-points.md         (~250 lines, 20+ pain points, 6 unmet needs)
├── 06-ecosystem.md           (~200 lines, 6 standards, 5 patterns, 6 constraints)
├── 07-trajectory.md          (~250 lines, 6 trends, 6 emerging, 7 predictions)
├── 08-community.md           (~250 lines, 30+ projects metrics, 5 migrations)
├── 09-failure-archaeology.md (~200 lines, 6 failed, 4 pivots, 5 barriers)
├── synthesis.md              (~400 lines, full analysis with go/no-go)
└── conversation-log.md       (this file)
```

---

## Key Takeaways

1. **The AI eval space is massive and fragmented** — 65+ projects, no dominant player
2. **Agent evaluation is the frontier** — methodology crystallizing, tooling nascent
3. **Statistical rigor is the biggest gap** — near-zero competition (Agentrial, 16 stars)
4. **OTel is the right foundation** — emerging standard for framework-agnostic evaluation
5. **Consolidation is accelerating** — OpenAI+Promptfoo, Anthropic+Humanloop creating vacuums
6. **Quality is the #1 deployment barrier** — 32% cite it, massive market pull
7. **89% observability vs 52% eval adoption** — huge gap to close
8. **Verdict: GO with focused positioning** — statistical agent eval, error analysis pipeline, or unified quality+safety
