> **📦 Archived on 2026-04-23** — superseded by no successor — this work was abandoned. Kept for historical reference.

# Stream 6 — Ecosystem & Integration Surface

## Sources

- https://opentelemetry.io/blog/2025/ai-agent-observability/
- https://opentelemetry.io/docs/specs/semconv/gen-ai/
- https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-events/
- https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-metrics/
- https://ai-sdk.dev/docs/ai-sdk-core/telemetry
- https://www.datadoghq.com/blog/llm-otel-semantic-convention/
- https://agenta.ai/blog/the-ai-engineer-s-guide-to-llm-observability-with-opentelemetry
- Project documentation from Stream 1 landscape analysis

---

## Standards & Protocols

### OpenTelemetry (OTel) GenAI Semantic Conventions
- **What it governs**: Standardized schema for tracking prompts, model responses, token usage, tool/agent calls, and provider metadata across any GenAI system
- **Adoption level**: High and growing — adopted by Langfuse, Arize Phoenix, agentevals-dev, TruLens, Laminar, Datadog, LangWatch, Agenta. OTel GenAI SemConv v1.37+
- **Key specifications**:
  - `gen_ai.evaluation.result` event — captures evaluation results, parented to the GenAI operation span
  - Agent spans for GenAI agent operations
  - Model spans for GenAI model operations
  - Events for inputs and outputs
  - Metrics for token usage, latency, costs
- **Link**: https://opentelemetry.io/docs/specs/semconv/gen-ai/
- **Note**: This is THE emerging standard for AI observability interoperability. Any new entrant must support OTLP.

### OTLP (OpenTelemetry Protocol)
- **What it governs**: Wire protocol for transmitting telemetry data (traces, metrics, logs)
- **Adoption level**: Universal in observability ecosystem — gRPC and HTTP variants
- **Link**: https://opentelemetry.io/docs/specs/otlp/

### NIST AI TEVV (Testing, Evaluation, Verification and Validation)
- **What it governs**: Emerging federal standard for AI system testing and evaluation. Zero Drafts project launched July 2025 defining key terms, lifecycle stages, and guiding principles.
- **Adoption level**: Early — still in draft stage, but will influence compliance requirements for government/regulated industries
- **Link**: https://www.nist.gov/artificial-intelligence/ai-standards

### OWASP Top 10 for LLM Applications
- **What it governs**: Security vulnerabilities specific to LLM applications (prompt injection, data leakage, etc.)
- **Adoption level**: Widely referenced by security evaluation tools (Promptfoo, DeepTeam, Giskard)
- **Link**: https://owasp.org/www-project-top-10-for-large-language-model-applications/

### MITRE ATLAS (Adversarial Threat Landscape for AI Systems)
- **What it governs**: Knowledge base of adversary tactics and techniques against AI systems
- **Adoption level**: Referenced by red-teaming tools (DeepTeam, enterprise security teams)

### NIST AI RMF (AI Risk Management Framework)
- **What it governs**: Framework for managing risks of AI systems throughout their lifecycle
- **Adoption level**: Growing — adopted by enterprises for AI governance, referenced by eval tools

---

## Data Formats

### JSONL (JSON Lines)
- **What it's used for**: Evaluation datasets, benchmark data, results logging
- **Who uses it**: OpenAI Evals (benchmark registry), EleutherAI harness (task results), most eval frameworks
- **Status**: De facto standard for streaming/line-oriented AI data

### YAML
- **What it's used for**: Evaluation configuration, test case definitions, task specifications
- **Who uses it**: Promptfoo (test configs), EleutherAI harness (task configs), ContextCheck, Patronus AI
- **Status**: Dominant for configuration; some use JSON interchangeably

### OpenAI Chat Completion Format (Messages Array)
- **What it's used for**: Representing conversations and agent trajectories
- **Who uses it**: LangChain AgentEvals, OpenEvals, most eval tools that evaluate chat-based outputs
- **Status**: De facto standard for representing LLM conversations. `{"role": "user/assistant/system/tool", "content": "..."}`

### LangChain BaseMessage Format
- **What it's used for**: Alternative conversation representation within LangChain ecosystem
- **Who uses it**: LangChain AgentEvals, LangSmith
- **Status**: LangChain-specific; convertible to/from OpenAI format

### Jaeger JSON Trace Format
- **What it's used for**: OpenTelemetry trace data serialization
- **Who uses it**: agentevals-dev (supports Jaeger JSON and OTLP)
- **Status**: One of several OTel-compatible formats

---

## Adjacent Tools & Services

### LLM Providers (API)
- **Role**: The models being evaluated — also serve as LLM-as-judge evaluators
- **How it connects**: Every eval tool integrates with model providers. OpenAI, Anthropic, Google, AWS Bedrock, Azure, Mistral, etc.
- **Key integration pattern**: Model provider SDKs or unified interfaces (LiteLLM, LangChain model init)

### Agent Frameworks
- **Role**: Build the AI agents and applications being evaluated
- **How it connects**: Agent output → eval framework input. Some tight integration (LangGraph ↔ AgentEvals), some loose (any framework → OTel → eval)
- **Key players**: LangGraph, CrewAI, AutoGen, OpenAI Agents SDK, Google ADK, Strands, Pydantic AI
- **Integration pattern**: Framework-specific adapters or generic trace/output capture

### Vector Databases
- **Role**: Store and retrieve embeddings for RAG systems being evaluated
- **How it connects**: RAG evaluation tools evaluate retrieval quality from vector DB results
- **Key players**: Pinecone, Weaviate, Qdrant, Chroma, Milvus, pgvector
- **Integration pattern**: Eval tool evaluates retrieval results; doesn't directly integrate with vector DB

### CI/CD Platforms
- **Role**: Run evaluation suites as part of deployment pipeline
- **How it connects**: GitHub Actions, GitLab CI, Jenkins trigger eval tools via CLI
- **Key players**: GitHub Actions (dominant), GitLab CI, Jenkins, CircleCI
- **Integration pattern**: Eval tool CLI invoked in CI job, results reported as PR comments or checks

### Observability Platforms (Traditional)
- **Role**: Infrastructure monitoring that AI observability extends
- **How it connects**: AI trace data can flow to traditional observability platforms via OTel
- **Key players**: Datadog (has native LLM observability), New Relic, Grafana
- **Integration pattern**: OTel collector → traditional observability platform

### Prompt Management Tools
- **Role**: Version control and management of prompts being evaluated
- **How it connects**: Prompt changes trigger evaluations; prompt versions tracked alongside eval results
- **Key players**: Langfuse (built-in), LangSmith (built-in), Braintrust (built-in), Humanloop (built-in)
- **Integration pattern**: Prompt versioning integrated within eval platforms, or separate tools (Agenta, Portkey)

### IDE Extensions
- **Role**: In-editor evaluation and debugging
- **How it connects**: VS Code extensions for eval authoring, debugging, result viewing
- **Key players**: Inspect AI VS Code extension, Braintrust MCP integration, Promptfoo MCP
- **Integration pattern**: VS Code extension or MCP server for IDE integration

### MCP (Model Context Protocol)
- **Role**: Emerging protocol for connecting AI models with tools and data sources
- **How it connects**: Eval tools expose MCP servers for IDE integration; eval tools evaluate MCP-based agent interactions
- **Key players**: Braintrust (MCP for IDE), agentevals-dev (MCP server), Opik (MCP), Arize Phoenix (MCP)
- **Integration pattern**: MCP server exposing eval/observability data to coding agents

---

## Integration Patterns

### Pattern 1: SDK Instrumentation
- **Description**: Import SDK, wrap LLM calls with tracing decorators/wrappers, data flows automatically to eval platform
- **Who implements it**: Langfuse (Python/JS SDKs), Braintrust (multi-language SDKs), LangSmith (Python/TS/Go/Java SDKs), DeepEval (`@observe` decorator)
- **Standard**: Proprietary SDK APIs, but converging toward OTel-compatible instrumentation

### Pattern 2: OpenTelemetry Auto-Instrumentation
- **Description**: Use OTel auto-instrumentation libraries for LLM providers. Traces flow via OTLP to any OTel-compatible backend.
- **Who implements it**: Langfuse, Arize Phoenix, Laminar, Datadog, agentevals-dev
- **Standard**: OTel GenAI Semantic Conventions

### Pattern 3: Proxy/Gateway Instrumentation
- **Description**: Route LLM API calls through a proxy that captures requests/responses for evaluation
- **Who implements it**: Helicone (AI gateway), Portkey (AI gateway)
- **Standard**: HTTP proxy interception

### Pattern 4: pytest/Test Framework Integration
- **Description**: Evaluations run as test cases within pytest (Python) or Vitest/Jest (TypeScript)
- **Who implements it**: DeepEval (pytest plugin), OpenEvals (pytest/Vitest), Opik (pytest), Promptfoo (custom test runner)
- **Standard**: pytest plugin API, Vitest/Jest APIs

### Pattern 5: CLI-Driven Evaluation
- **Description**: Run evaluations from command line with configuration files, output results to stdout/files/dashboards
- **Who implements it**: Promptfoo, EleutherAI harness, Inspect AI, agentevals-dev
- **Standard**: No standard CLI conventions

---

## Compatibility Constraints

### Constraint 1: OpenTelemetry Support
- **Why required**: OTel GenAI Semantic Conventions are becoming the standard for AI observability. Tools that don't support OTLP will be cut off from the growing OTel ecosystem.
- **What breaks if you ignore it**: Can't interoperate with Datadog, Grafana, existing observability infrastructure. Can't receive traces from OTel-instrumented applications.

### Constraint 2: OpenAI Chat Completion Format
- **Why required**: The `messages` array format (role + content) is the lingua franca for representing LLM conversations.
- **What breaks if you ignore it**: Can't process outputs from most LLM applications. Need custom format converters.

### Constraint 3: Multi-Model Provider Support
- **Why required**: Teams use multiple models (OpenAI, Anthropic, Google, open-source). Eval tools must support diverse providers for both targets and judges.
- **What breaks if you ignore it**: Teams locked into a single provider can't use your tool. Most eval tools support 10+ providers.

### Constraint 4: Python SDK
- **Why required**: Python is the dominant language for AI/ML. Not having a Python SDK is disqualifying for most use cases.
- **What breaks if you ignore it**: Lose 80%+ of potential users. TypeScript support is important but secondary.

### Constraint 5: CI/CD Integration
- **Why required**: Evaluation must run automatically in deployment pipelines. CLI or API access is minimum requirement.
- **What breaks if you ignore it**: Teams can't automate quality gates. Eval becomes manual and gets skipped.

### Constraint 6: LLM-as-Judge Support
- **Why required**: It's the dominant evaluation methodology. Users expect to be able to use any LLM as a judge.
- **What breaks if you ignore it**: Limited to deterministic evaluation only, which is insufficient for subjective/open-ended evaluation.
