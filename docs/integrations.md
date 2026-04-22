# Integrations Backlog

> Trait implementations that connect evalkit to the outside world. Organized by direction of data flow.
>
> Status legend: `[x]` shipped · `[~]` partial · `[ ]` todo.

---

## Acquisitions (`impl Acquisition<I, O>`)

### Direct model providers (built on **anyllm**)

> Exact wrapper shape depends on anyllm's actual provider trait — to be confirmed during ROADMAP Phase 0(a). Names below are illustrative.

- [ ] `AnyLlmAcquisition` — wrap an anyllm provider handle as an `Acquisition`
- [ ] `AnyLlmChatAcquisition` — single-turn chat, string in / string out
- [ ] `AnyLlmStructuredAcquisition<T>` — structured output via JSON schema, deserializes to `T`
- [ ] `AnyLlmToolAcquisition` — tool-calling loop; returns final assistant message + tool trace

Via anyllm this gives first-class OpenAI / Anthropic / Gemini / OpenAI-compatible / Cloudflare support for free. Other providers worth surfacing even though they likely route through anyllm's compat layer:

- [ ] `LiteLLMAcquisition`     — LiteLLM proxy target (huge in polyglot shops)
- [ ] `OllamaAcquisition`      — confirm whether anyllm's OpenAI-compat covers this cleanly; if not, ship native
- [ ] `VllmAcquisition`        — vLLM native endpoint
- [ ] `SglangAcquisition`      — SGLang endpoint

### Transport-level
- [x] `HttpAcquisition`       — extracted into `evalkit-providers`
- [x] `SubprocessAcquisition` — extracted into `evalkit-providers`; protocol documented in `docs/plugin-protocol.md`
- [ ] `WebSocketAcquisition`
- [ ] `GrpcAcquisition`

### Trace-based (acquire output by fetching spans)
- [x] `OtlpReceiver` (ingest side, shipped in `evalkit-otel`)
- [x] `JaegerBackend` (shipped in `evalkit-otel`)
- [ ] `TempoBackend`
- [ ] `DatadogTraceBackend`
- [ ] `OtelCollectorBackend`  — generic OTLP-endpoint puller

### Sandboxed / containerized
- [ ] `DockerAcquisition`     — spin up a container per sample
- [ ] `WasmAcquisition`       — call a WASM module
- [ ] `McpAcquisition`        — invoke an MCP server tool

### Streaming sources (for Phase 2 `Executor`)
- [ ] `KafkaSource`           — consume traces/events from a Kafka topic
- [ ] `NatsSource`            — NATS subject subscriber
- [ ] `FileTailerSource`      — tail a JSONL file
- [ ] `OtlpReceiverSource`    — adapt the existing OTLP receiver as a streaming source

### Replay / fixtures
- [ ] `FixtureAcquisition`    — serve pre-recorded outputs from a JSONL
- [ ] `CachedAcquisition`     — content-addressed cache wrapper (for fast reruns & deterministic CI)
- [ ] `MockAcquisition`       — configurable scripted responses (for library tests)

### Composition
- [ ] `RetryingAcquisition`   — N retries with backoff
- [ ] `TimeoutAcquisition`    — bound latency
- [ ] `RateLimitedAcquisition`
- [ ] `FallbackAcquisition`   — primary + fallback chain
- [ ] `MultiplexAcquisition`  — fan out to N providers, return first / all

---

## Dataset loaders

`Dataset` is already constructable from `Vec<Sample>`. Loaders provide `TryFrom<Source>` or free functions.

### Files
- [x] `read_jsonl`
- [ ] `read_json` — flat array
- [ ] `read_csv`
- [ ] `read_tsv`
- [ ] `read_parquet`
- [ ] `read_yaml`
- [ ] `read_toml`

### Remote
- [ ] `load_from_url`           — fetch + sniff format
- [ ] `load_from_s3`
- [ ] `load_from_gcs`
- [ ] `load_from_http_paginated`

### Ecosystems
- [ ] `load_from_huggingface`   — by repo id + split, local cache
- [ ] `load_from_openai_evals`  — registry YAML/JSONL shape
- [ ] `load_from_inspect_task`  — Inspect AI `Sample` compat
- [ ] `load_from_promptfoo_yaml`
- [ ] `load_from_ragas_dataset`
- [ ] `load_from_sqlite`        — arbitrary query → dataset

### Public benchmark adapters (curated + versioned)
Add lazily as demand appears — most users bring their own data.
- [ ] `mmlu`
- [ ] `gpqa`
- [ ] `humaneval`
- [ ] `swe_bench`
- [ ] `gsm8k`
- [ ] `truthfulqa`
- [ ] `ifeval`
- [ ] `mtbench`

---

## Exporters (eval results → external systems)

### File formats
- [x] `write_jsonl`              (JSONL schema v1 once frozen)
- [ ] `write_parquet`
- [ ] `write_csv`                (flattened)
- [ ] `write_markdown_report`
- [ ] `write_html_report`

### Observability platforms
- [x] `langfuse` exporter     — standalone `evalkit-exporters-langfuse` crate with umbrella-crate compatibility API
- [ ] `phoenix` exporter         — OTel spans + Arize schema
- [ ] `braintrust` exporter      — REST API + experiments
- [ ] `mlflow` exporter          — runs + metrics + artifacts
- [ ] `wandb` exporter
- [ ] `datadog` exporter         — custom metrics + log events
- [ ] `helicone` exporter
- [ ] `opik` exporter            — Comet's OSS eval platform

### OTel-native
- [x] `otel-span` emitter        — interim `evalkit-otel::OtelResultEmitter` for `RunResult` spans in the `evalkit.*` namespace
- [ ] `otel-metrics` emitter     — push aggregate metrics

### Push targets
- [ ] `prometheus_pushgateway` exporter
- [ ] `slack` notifier           — post summary to Slack webhook

> PR comment formatting lives in the **GitHub Action** (ROADMAP Phase 3), not as a general exporter — it consumes the markdown output of `evalkit diff` directly.

---

## Trace backends (for `OtlpReceiver` consumers & trace-based acquisition)

- [x] `OtlpReceiver`  — OTLP/HTTP ingest in `evalkit-otel`
- [ ] `OtlpGrpcReceiver`
- [x] `JaegerBackend`  — fetch spans by id in `evalkit-otel`
- [ ] `TempoBackend`
- [ ] `DatadogTraceBackend`
- [ ] `InMemoryTraceStore` — test fixture

---

## Plugin runtime (subprocess protocol)

Once the plugin spec lands:

- [ ] `PluginAcquisition`       — invoke a subprocess plugin as `Acquisition`
- [ ] `PluginScorer`             — invoke a subprocess plugin as `Scorer`
- [ ] Reference Python shim (`evalkit-plugin` on pypi)
- [ ] Reference TypeScript shim (`@evalkit/plugin` on npm)
- [ ] Conformance test harness  — run a plugin against a golden set of requests

---

## Cost / tokenization helpers

- [ ] `TiktokenTokenizer`        — OpenAI cl100k / o200k
- [ ] `AnthropicTokenizer`
- [ ] `LlamaTokenizer`
- [ ] Cost tables per provider (in-repo JSON, updatable)
- [ ] `CostCalculator`           — combine tokenizer + cost table for pre-call estimation

---

## CI / workflow integrations

- [ ] GitHub Action              — wraps `evalkit run` + PR comment exporter
- [ ] GitLab CI template
- [ ] Pre-commit hook integration
- [ ] VS Code extension          — invoke runs from editor (later)

---

## Cross-cutting requirements

- Every integration lives in its own crate (`evalkit-<name>`) to keep dependency graphs tight.
- Feature-flagged `reqwest` / `hyper` / `tonic` etc. where optional.
- Integration tests use `MockAcquisition` or `InMemoryTraceStore`; no live network unless marked `#[ignore]`.
- Exporters serialize through the frozen `evalkit-schema` types — never their own ad-hoc shape.
