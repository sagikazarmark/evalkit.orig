# Stream 8 — Community & Adoption Signals

## Sources

- GitHub repository pages for all major projects (star counts, fork counts, contributor data)
- https://www.langchain.com/state-of-agent-engineering (LangChain State of AI Agents 2026)
- https://openai.com/index/openai-to-acquire-promptfoo/
- https://awesomeagents.ai/tools/best-llm-eval-tools-2026/
- https://www.confident-ai.com/blog/greatest-llm-evaluation-tools-in-2025
- https://maven.com/parlance-labs/evals (AI Evals Course)
- https://maven.com/aiproducthub/genai-evals-certification (AI Evals for PMs)
- https://techcrunch.com/2026/03/09/openai-acquires-promptfoo-to-secure-its-ai-agents/

---

## Adoption Metrics

### Tier 1: Mass Adoption (>10k stars)
| Project | Stars | Trend | Evidence |
|---------|-------|-------|----------|
| Langfuse | 24.3k | Growing rapidly | Fastest-growing LLMOps platform. YC W23. 6,672 commits. |
| Promptfoo | 19.1k | Growing → Acquired | 350k+ developers, 130k monthly active. Used by 25%+ Fortune 500. Acquired by OpenAI March 2026 ($86M valuation). |
| OpenAI Evals | 18.1k | Stable/Declining | Pioneer framework but being superseded by platform-integrated evals. |
| Opik (Comet) | 18.6k | Growing rapidly | 40M+ traces/day capacity. Apache 2.0 fully open-source. Backed by Comet ML. |
| DeepEval | 14.4k | Growing rapidly | 400k+ monthly downloads, 20M+ evaluations. Apache 2.0. Confident AI cloud platform. |
| RAGAS | 13.2k | Growing (slowing) | De facto RAG evaluation standard. Apache 2.0. v0.4.3 (Jan 2026). |
| EleutherAI Harness | 12k | Stable | Powers HF Open LLM Leaderboard. Cited in hundreds of papers. Used by NVIDIA, Cohere, BigScience. |

### Tier 2: Strong Adoption (3k-10k stars)
| Project | Stars | Trend | Evidence |
|---------|-------|-------|----------|
| Arize Phoenix | 9.1k | Growing | Open-source arm of Arize commercial platform. OTel-native. |
| NVIDIA Garak | 7.4k | Growing | NVIDIA-backed LLM red-teaming toolkit. Apache 2.0. |
| Guardrails AI | 6.6k | Growing | 6.6k stars. Guardrails Hub ecosystem. Apache 2.0. |
| OpenCompass | 6.8k | Growing | Chinese-origin large-scale benchmarking. 400k questions. |
| Giskard | 5.2k | Growing | v3 rewrite for agent testing. Apache 2.0. |
| Helicone | 5.4k | Growing | AI gateway + observability. Apache 2.0. |
| ZenML | 5.3k | Growing | Pipeline orchestration with eval steps. Apache 2.0. |
| OpenAI simple-evals | 4.4k | Declining | No longer updated for new models (July 2025). Reference implementations only. |
| AutoRAG | 4.7k | Growing | AutoML for RAG pipelines. |
| Agenta | 4k | Growing | All-in-one LLMOps platform. MIT. |
| lmms-eval | 4k | Growing | Only comprehensive multimodal eval toolkit. |

### Tier 3: Niche Adoption (1k-3k stars)
| Project | Stars | Trend | Evidence |
|---------|-------|-------|----------|
| TruLens | 3.2k | Stable | 116 releases. MIT. Snowflake backing. |
| MTEB | 3.2k | Stable | Standard embedding benchmark. Apache 2.0. |
| ChainForge | 3k | Growing | Visual prompt evaluation IDE. MIT. |
| Microsoft PromptBench | 2.8k | Declining | Archived March 2026. |
| Laminar | 2.7k | Growing | Rust backend, agent-first. Apache 2.0. |
| HELM | 2.7k | Stable | Stanford CRFM. Academic standard. Apache 2.0. |
| HF LightEval | 2.4k | Growing | Replacing HF Evaluate. MIT. |
| UpTrain | 2.3k | Stable | Local-first evaluation. |
| BEIR | 2.1k | Stable | IR benchmark. Apache 2.0. |
| AlpacaEval | 2k | Stable | Instruction-following evaluator. Apache 2.0. |
| EvalPlus | 1.7k | Stable | Code evaluation. Apache 2.0. |
| Agentic Security | 1.8k | Early | Agent workflow vulnerability scanner. |
| DeepTeam | 1.4k | Growing | LLM red-teaming by Confident AI. Apache 2.0. |
| Bloom | 1.3k | Early | Behavioral evaluation by Anthropic safety research. MIT. |
| LangChain OpenEvals | 1k | Growing | Readymade evaluators. MIT. |

### Must-Include Projects
| Project | Stars | Trend | Evidence |
|---------|-------|-------|----------|
| Inspect AI | 1.9k | Growing | UK AISI. Active community. Expanding eval library. |
| LangChain AgentEvals | 534 | Growing | Part of LangChain ecosystem. Active development. |
| AWS Agent Evaluation | 354 | Growing | AWS-native. 5 releases, Apache 2.0. |
| agentevals-dev | 112 | Early | OTel-native, local-first. Very small community. |

---

## Community Health

### Strong Community Health
- **Langfuse**: High contributor diversity (6,672 commits), responsive issue management, active roadmap discussions. MIT license encourages contributions.
- **DeepEval**: Active maintainer team (MAINTAINERS.md), 400k+ monthly downloads indicate strong daily use. GitHub discussions active.
- **Promptfoo**: 255 contributors before acquisition. Active blog and documentation. Unclear post-acquisition community dynamics.
- **EleutherAI Harness**: Strong academic community. Hundreds of research citations. High contributor diversity across organizations.

### Moderate Community Health
- **Opik**: High star count (18.6k) but newer project. Comet ML provides organizational backing. 60+ integration contributions.
- **RAGAS**: Good community but showing signs of plateauing. Core team concentrated. Latest release was Jan 2026.
- **Arize Phoenix**: Active development, strong commercial backing, but community contributions are secondary to company-led development.
- **Inspect AI**: Government-backed project with institutional contributors (UK AISI, Arcadia Impact, Vector Institute). Not a typical open-source community.

### Concerning Community Health
- **TruLens**: Steady but small community (3.2k stars, 257 forks). Snowflake acquisition may narrow focus. Bus factor risk.
- **agentevals-dev**: Very small (112 stars). Single developer risk. Limited documentation. Too early to assess sustainability.
- **OpenAI Evals**: High stars but declining active development. Being superseded by platform features. Community contributions may slow.

---

## Business Models & Funding

### Venture-Funded Startups
- **Braintrust**: Raised $80M in February 2026 at $800M valuation. Revenue model: freemium SaaS (Free → $249/mo Pro → Enterprise).
- **Confident AI (DeepEval)**: Funded startup. Revenue: Confident AI cloud platform ($19.99-49.99/user/month). Open-source DeepEval drives adoption.
- **Promptfoo**: Raised $23M total, $86M valuation at acquisition. Revenue: Enterprise tier (pre-acquisition). Now OpenAI subsidiary.
- **Arize AI**: Well-funded AI observability company. Revenue: Commercial platform + Phoenix OSS for community.
- **Galileo AI**: Funded. Revenue: Free tier + enterprise plans. Luna-2 evaluation models.
- **Maxim AI**: Funded. Revenue: Usage-based SaaS.
- **FutureAGI**: Funded. Revenue: Commercial platform + open-source component.

### Corporate-Backed Projects
- **LangSmith**: LangChain Inc (funded). Revenue: Usage-based SaaS ($2.50-5/1k traces).
- **Opik**: Comet ML (funded). Revenue: Comet.com cloud. Opik is fully open-source.
- **Humanloop**: Funded, now being acquired by Anthropic. Revenue: Commercial SaaS.
- **Inspect AI**: UK Government (AISI). No revenue model. Public good.
- **TruLens**: Truera (acquired by Snowflake). Revenue: Snowflake platform integration.

### Open Source / Community
- **EleutherAI Harness**: Non-profit (EleutherAI). Funded by research grants.
- **RAGAS**: Open-source community. Some commercial consulting.
- **HELM**: Stanford CRFM. Academic funding.
- **Langfuse**: YC W23 startup. Revenue: Cloud managed service. Core open-source (MIT).

### Unsustainable / At Risk
- **PromptBench**: Archived (March 2026). Microsoft research project, not commercial.
- **OpenAI simple-evals**: No longer updated. Reference implementations only.
- **Log10**: Archived (May 2025). Failed to sustain business.
- **AIConfig**: Stale since March 2024. LastMile AI appears to have pivoted.

---

## Key People & Organizations

### Influential Voices
- **Hamel Husain** — Leading educator on AI evals. Teaches "AI Evals For Engineers & PMs" course on Maven. Wrote influential blog posts on eval methodology.
- **Shreya Shankar** — Co-instructor of evals course. UC Berkeley researcher on LLM evaluation.
- **Harrison Chase** — LangChain founder. Shapes eval discourse through LangSmith, AgentEvals, OpenEvals, and State of AI Agents reports.
- **Confident AI team** — Driving DeepEval adoption and eval best practices. 20M+ evaluations. Blog is a major resource.
- **Anthropic Engineering** — "Demystifying Evals for AI Agents" is widely cited as the most comprehensive agent eval guide.

### Organizations Investing Heavily
- **OpenAI**: Acquired Promptfoo. Platform-integrated evals. Major commitment.
- **Anthropic**: Acquiring Humanloop. Publishing eval methodology. Bloom safety research.
- **LangChain**: LangSmith, AgentEvals, OpenEvals, State of AI Agents reports. Most comprehensive eval ecosystem.
- **Comet ML**: Opik (18.6k stars). Significant open-source investment.
- **Arize AI**: Phoenix OSS + commercial platform. Long ML observability heritage.
- **UK Government (AISI)**: Inspect AI + inspect_evals. 100+ pre-built evaluations.
- **NVIDIA**: Garak (7.4k stars), NeMo Guardrails. AI safety tooling.

---

## Migration Patterns

### Direction 1: From Homebrew Scripts to Frameworks
- **From**: Custom Python scripts for evaluation
- **To**: DeepEval, Promptfoo, RAGAS, or other frameworks
- **Why**: Reproducibility, pre-built metrics, CI/CD integration, community-validated approaches
- **Evidence**: Growth of framework adoption (DeepEval 400k+/month downloads)

### Direction 2: From Framework-Only to Platform
- **From**: DeepEval/RAGAS (library only) for development
- **To**: Adding Langfuse/Braintrust/LangSmith for production monitoring + collaboration
- **Why**: Need observability, dashboards, team collaboration that libraries don't provide
- **Evidence**: Langfuse/LangSmith growth, teams commonly use library + platform together

### Direction 3: From LangSmith to Open-Source Alternatives
- **From**: LangSmith (LangChain's commercial platform)
- **To**: Langfuse, Opik, or Arize Phoenix
- **Why**: Cost (LangSmith pricing scales), data sovereignty (self-hosting), reduced vendor lock-in
- **Evidence**: Langfuse's rapid growth (24.3k stars), multiple "Langfuse alternatives" comparison articles

### Direction 4: From General Eval to Agent-Specific Eval
- **From**: Using general LLM evaluation metrics on agent outputs
- **To**: Agent-specific evaluation (trajectory matching, tool call validation, plan quality)
- **Why**: General metrics don't capture agent behavior quality. Agent-specific tools provide better signal.
- **Evidence**: Growth of AgentEvals, DeepEval agent metrics, TruLens Agent GPA

### Direction 5: From Separate Eval + Safety to Unified
- **From**: Quality evaluation (DeepEval/RAGAS) and security testing (Garak/Promptfoo) as separate workflows
- **To**: Unified quality + safety evaluation
- **Why**: Same infrastructure, same data, complementary signals. OpenAI's Promptfoo acquisition validates this.
- **Evidence**: DeepTeam (companion to DeepEval), Giskard v3, OpenAI+Promptfoo
