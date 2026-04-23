> **📦 Archived on 2026-04-23** — superseded by [Stream 9 — Failure Archaeology](../../../docs/research/09-failure-archaeology.md). Kept for historical reference.

# Stream 9 — Failure Archaeology

## Sources

- GitHub repository analysis (archived/stale projects from Stream 1 landscape)
- https://openai.com/index/openai-to-acquire-promptfoo/ (acquisition as exit signal)
- https://techstartups.com/2025/12/09/top-ai-startups-that-shut-down-in-2025-what-founders-can-learn/
- https://www.gartner.com/en/newsroom/press-releases/2024-07-29-gartner-predicts-30-percent-of-generative-ai-projects-will-be-abandoned-after-proof-of-concept-by-end-of-2025
- https://is4.ai/blog/our-blog-1/ai-companies-failed-spectacularly-2026-248
- Project documentation, GitHub activity trails, and comparison articles

---

## Dead & Abandoned Projects

### Microsoft PromptBench
- **Project**: PromptBench
- **URL**: https://github.com/microsoft/promptbench
- **What it was**: Unified framework for LLM evaluation combining prompt engineering, adversarial robustness testing, and dynamic evaluation (DyVal for data contamination).
- **Timeline**: Active 2023-2025, archived March 17, 2026
- **Peak traction**: 2.8k GitHub stars
- **What killed it**: Archived by Microsoft. Likely superseded by Microsoft's investment in PromptFlow and Azure AI Foundry evaluation capabilities. Research project that didn't transition to product.
- **Evidence**: GitHub archive notice (March 2026)
- **Lessons**: Microsoft research projects often don't survive productization. DyVal (dynamic evaluation addressing contamination) was genuinely innovative but had no path to product.

### OpenAI simple-evals
- **Project**: simple-evals
- **URL**: https://github.com/openai/simple-evals
- **What it was**: Lightweight benchmark evaluation library for transparent model reporting.
- **Timeline**: Active 2024-2025, frozen July 2025
- **Peak traction**: 4.4k GitHub stars
- **What killed it**: OpenAI shifted to platform-integrated evals (Evals API) and acquired Promptfoo. simple-evals was always a reference implementation, not a product.
- **Evidence**: OpenAI announcement that simple-evals "will no longer be updated for new models" (July 2025)
- **Lessons**: Reference implementations serve a purpose but can't compete with integrated platforms. The move to platform-integrated evaluation was inevitable.

### Log10
- **Project**: Log10
- **URL**: https://github.com/log10-io/log10
- **What it was**: Unified LLM data management platform for logging, monitoring, and improving LLM applications. "One-line instrumentation" approach.
- **Timeline**: Active 2023-2024, archived May 2025
- **Peak traction**: 96 GitHub stars
- **What killed it**: Lack of adoption. In a crowded market with Langfuse (24.3k stars) and similar tools, Log10 couldn't differentiate or gain traction.
- **Evidence**: GitHub archive (May 2025), no recent activity
- **Lessons**: Minimal differentiation in a crowded observability space is fatal. "Simple logging" wasn't enough value when competitors offered logging + evaluation + prompt management.

### AIConfig (LastMile AI)
- **Project**: AIConfig
- **URL**: https://github.com/lastmile-ai/aiconfig
- **What it was**: Framework for managing AI prompts, models, and parameters as JSON configs separate from application code. VS Code editor for prototyping.
- **Timeline**: Active 2023-2024, stale since March 2024
- **Peak traction**: 1.1k GitHub stars
- **What killed it**: LastMile AI appears to have pivoted away from prompt configuration management. Config-as-code for prompts didn't gain traction as a standalone product.
- **Evidence**: Last release March 2024, no recent commits
- **Lessons**: Prompt management as a standalone product is hard — it gets absorbed into larger platforms (Langfuse, LangSmith, Braintrust all include prompt management).

### HuggingFace Evaluate
- **Project**: evaluate
- **URL**: https://github.com/huggingface/evaluate
- **What it was**: Standardized metric implementations for NLP and CV model evaluation. Community Hub for sharing metrics.
- **Timeline**: Active 2022-2024, effectively deprecated
- **Peak traction**: 2.4k GitHub stars
- **What killed it**: Superseded by HuggingFace's own LightEval, which is more modern and supports LLM-specific evaluation patterns.
- **Evidence**: README directs users to LightEval for LLM evaluation
- **Lessons**: Traditional NLP metrics (BLEU, ROUGE) became less relevant for LLM evaluation. Internal competition from a more modern approach killed the older library.

### Microsoft PyRIT (Azure org)
- **Project**: PyRIT (original Azure org)
- **URL**: https://github.com/Azure/PyRIT
- **What it was**: Python Risk Identification Toolkit for AI red teaming.
- **Timeline**: Active at Azure org 2024-2025, archived March 2026
- **Peak traction**: Significant Microsoft investment
- **What killed it**: Not dead — migrated to microsoft/PyRIT org. But the archive of the Azure org version represents organizational churn that could confuse users.
- **Evidence**: Archive notice with migration note
- **Lessons**: Organizational restructuring (Azure → Microsoft org) creates confusion and broken links. Open-source projects at large companies are vulnerable to internal politics.

---

## Pivoted Projects

### Promptfoo → OpenAI
- **Project**: Promptfoo
- **Original vision**: Independent, open-source LLM evaluation and red-teaming CLI for any provider
- **What it became**: OpenAI subsidiary focused on securing AI agents. Will "continue to serve users and customers" but within OpenAI's ecosystem.
- **Why it pivoted**: Acquired by OpenAI for $86M+ (March 2026). OpenAI needed evaluation and red-teaming capabilities integrated into their platform.
- **Implications**: While open-source is promised to continue, perceived neutrality may erode over time. OpenAI-first bias in development priorities is likely.

### Humanloop → Anthropic
- **Project**: Humanloop
- **Original vision**: Independent enterprise evaluation and prompt management platform for any LLM provider
- **What it became**: "Joining Anthropic" — integration into Anthropic's ecosystem
- **Why it pivoted**: Anthropic acquisition. Humanloop's human-in-the-loop evaluation and enterprise features complement Anthropic's model capabilities.
- **Implications**: May become Anthropic-centric. Enterprise customers on other providers may need alternatives.

### TruLens → Snowflake
- **Project**: TruLens
- **Original vision**: Independent LLM evaluation and tracking framework
- **What it became**: Snowflake-integrated evaluation platform
- **Why it pivoted**: Truera (TruLens' parent) was backed/acquired by Snowflake. Focus shifting to Snowflake-native evaluation workflows.
- **Implications**: Snowflake integration is a strength for Snowflake users but may limit appeal for non-Snowflake organizations.

### Giskard v2 → v3
- **Project**: Giskard
- **Original vision**: ML validation and testing framework (including traditional ML, not just LLMs)
- **What it became**: Fresh v3 rewrite focused specifically on LLM agent testing with async-first, modular architecture
- **Why it pivoted**: LLM/agent evaluation market is much larger than traditional ML testing. v3 drops heavy dependencies for agent-specific capabilities.
- **Implications**: v3 rewrite risks fragmenting community between v2 and v3. But focuses on higher-value market.

---

## Structural Barriers

### Barrier 1: Platform Effect / Feature Creep Death
- **Description**: Evaluation tools face pressure to become full platforms (add observability, prompt management, deployment). But platform-building requires 10x more engineering. Tools that stay narrow lose to platforms; tools that expand lose focus.
- **Which failed projects hit this**: Log10 (too narrow — just logging), AIConfig (too narrow — just config management)
- **Is this still present?**: Yes. This tension defines the market. Langfuse and Opik succeeded by building full platforms. Small eval libraries survive only as components of larger ecosystems (OpenEvals within LangChain, DeepEval with Confident AI cloud).

### Barrier 2: Acquisition Gravity
- **Description**: As Big AI companies (OpenAI, Anthropic, Google, Microsoft) build evaluation capabilities, standalone eval tools either get acquired or squeezed. The acqui-hire path is often the best exit.
- **Which projects hit this**: Promptfoo (acquired by OpenAI), Humanloop (acquired by Anthropic)
- **Is this still present?**: Extremely present and accelerating. Standalone evaluation companies face existential questions about independence.

### Barrier 3: The "Eval as Feature" Problem
- **Description**: Evaluation is often perceived as a feature of a larger platform, not a standalone product. LLM providers (OpenAI, Anthropic), agent frameworks (LangChain), and observability tools (Langfuse, Datadog) all add eval capabilities.
- **Which projects hit this**: Most standalone eval tools face this pressure
- **Is this still present?**: Yes. The question for any new entrant: will eval be a feature or a product? DeepEval's success suggests "product" is viable with strong enough DX and metric depth.

### Barrier 4: Metric Commoditization
- **Description**: Core evaluation metrics (faithfulness, relevancy, correctness) are well-understood and easy to implement. Differentiation through metrics alone is hard. Any project can implement LLM-as-judge with basic prompts.
- **Which projects hit this**: Many — it's hard to differentiate on metrics alone
- **Is this still present?**: Yes. Differentiation must come from infrastructure, DX, platform features, or domain specialization — not just metrics.

### Barrier 5: The Open-Source Sustainability Challenge
- **Description**: Open-source eval tools need a business model. Pure open-source can't sustain development. Cloud platforms (Confident AI, Langfuse Cloud, Comet Cloud) are the common monetization approach, but conversion rates are low.
- **Which projects hit this**: Log10 (archived — couldn't sustain), many small tools at risk
- **Is this still present?**: Yes. The dual open-source/cloud model (DeepEval/Confident AI, Langfuse MIT/Cloud, Opik/Comet) is the winning pattern but requires significant investment.

---

## Attempted Approaches That Didn't Work

### Approach 1: Metrics-Only Library Without Platform
- **What was tried**: Building a pure metric library with no platform, dashboards, or data management
- **By whom**: Various early eval libraries, RAGAS (partially — only a library)
- **Why it didn't work**: Teams need more than metrics — they need experiment tracking, dataset management, production monitoring, collaboration. Metrics-only libraries get adopted but need to be paired with a platform, reducing their standalone value.

### Approach 2: Traditional ML Metrics Applied to LLMs
- **What was tried**: Applying traditional NLP metrics (BLEU, ROUGE, BERTScore) to LLM evaluation
- **By whom**: HuggingFace Evaluate, early evaluation approaches
- **Why it didn't work**: LLMs generate subjective, creative, context-dependent outputs. N-gram overlap and embedding similarity metrics have poor correlation with human judgment for LLM outputs. LLM-as-judge is more reliable for most use cases.

### Approach 3: Static Benchmark as Quality Proxy
- **What was tried**: Using standardized benchmark scores (MMLU, HumanEval) to predict production AI application quality
- **By whom**: Industry-wide practice (2022-2024)
- **Why it didn't work**: Benchmark saturation, data contamination, and poor correlation with real-world performance. Models scoring 90%+ on MMLU can still fail on production queries.

### Approach 4: Standalone Prompt Management
- **What was tried**: Building a dedicated tool just for prompt versioning, A/B testing, and management
- **By whom**: AIConfig (LastMile AI), various prompt management tools
- **Why it didn't work**: Prompt management alone isn't enough value for a standalone product. It gets absorbed into larger platforms (Langfuse, LangSmith, Braintrust all include it).

### Approach 5: Single-Model Evaluation Platform
- **What was tried**: Building evaluation tools tightly coupled to a single model provider
- **By whom**: Early OpenAI Evals (OpenAI-centric), provider-specific eval tools
- **Why it didn't work**: Teams use multiple models. Provider lock-in in evaluation is unacceptable when the underlying models change frequently. Framework-agnostic tools (DeepEval, Promptfoo) won.
