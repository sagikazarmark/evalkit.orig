# Integrations Backlog

> Trait implementations that connect evalkit to the outside world. Organized by direction of data flow.
>
> Status legend: `[x]` shipped · `[~]` partial · `[ ]` todo.

---

## Output Sources (`impl OutputSource<I, O>`)

### Direct model providers (built on **anyllm**)

> Exact wrapper shape depends on anyllm's actual provider trait — to be confirmed during ROADMAP Phase 0(a). Names below are illustrative.

- [ ] `AnyLlmSource` — wrap an anyllm provider handle as an `OutputSource`
- [ ] `AnyLlmChatSource` — single-turn chat, string in / string out
- [ ] `AnyLlmStructuredSource<T>` — structured output via JSON schema, deserializes to `T`
- [ ] `AnyLlmToolSource` — tool-calling loop; returns final assistant message + tool trace

Via anyllm this gives first-class OpenAI / Anthropic / Gemini / OpenAI-compatible / Cloudflare support for free. Other providers worth surfacing even though they likely route through anyllm's compat layer:

- [ ] `LiteLLMSource`     — LiteLLM proxy target (huge in polyglot shops)
- [ ] `OllamaSource`      — confirm whether anyllm's OpenAI-compat covers this cleanly; if not, ship native
- [ ] `VllmSource`        — vLLM native endpoint
- [ ] `SglangSource`      — SGLang endpoint

### Transport-level
- [x] `HttpSource`       — extracted into `evalkit-providers`
- [x] `SubprocessSource` — extracted into `evalkit-providers`; protocol documented in `docs/plugin-protocol.md`
- [ ] `WebSocketSource`
- [ ] `GrpcSource`

### Trace-based (produce output by fetching spans)
- [x] `OtlpReceiver` (ingest side, shipped in `evalkit-otel`)
- [x] `JaegerBackend` (shipped in `evalkit-otel`)
- [ ] `TempoBackend`
- [ ] `DatadogTraceBackend`
- [ ] `OtelCollectorBackend`  — generic OTLP-endpoint puller

### Sandboxed / containerized
- [ ] `DockerSource`     — spin up a container per sample
- [ ] `WasmSource`       — call a WASM module
- [ ] `McpSource`        — invoke an MCP server tool

### Streaming sources (for Phase 2 `Executor`)
- [ ] `KafkaSource`           — consume traces/events from a Kafka topic
- [ ] `NatsSource`            — NATS subject subscriber
- [x] `FileTailerSource`      — `JsonlFileTailSource` tails appended JSONL `Sample` rows from disk
- [x] `OtlpReceiverSource`    — adapt the existing OTLP receiver as a pull-based executor source over grouped sample spans

### Replay / fixtures
- [ ] `FixtureSource`    — serve pre-recorded outputs from a JSONL
- [ ] `CachedSource`     — content-addressed cache wrapper (for fast reruns & deterministic CI)
- [ ] `MockSource`       — configurable scripted responses (for library tests)

### Composition
- [ ] `RetryingSource`   — N retries with backoff
- [ ] `TimeoutSource`    — bound latency
- [ ] `RateLimitedSource`
- [ ] `FallbackSource`   — primary + fallback chain
- [ ] `MultiplexSource`  — fan out to N providers, return first / all

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
- [x] `otel-span` emitter        — interim `evalkit-otel::OtelResultEmitter` plus `OtelResultSink` for executor-integrated `RunResult` span emission in the `evalkit.*` namespace
- [ ] `otel-metrics` emitter     — push aggregate metrics

### Push targets
- [ ] `prometheus_pushgateway` exporter
- [ ] `slack` notifier           — post summary to Slack webhook

> PR comment formatting lives in the **GitHub Action** (ROADMAP Phase 3), not as a general exporter — it consumes the markdown output of `evalkit diff` directly.

---

## Trace backends (for `OtlpReceiver` consumers & trace-based sources)

- [x] `OtlpReceiver`  — OTLP/HTTP ingest in `evalkit-otel`
- [ ] `OtlpGrpcReceiver`
- [x] `JaegerBackend`  — fetch spans by id in `evalkit-otel`
- [ ] `TempoBackend`
- [ ] `DatadogTraceBackend`
- [ ] `InMemoryTraceStore` — test fixture

---

## Plugin runtime (subprocess protocol)

Once the plugin spec lands:

- [x] `PluginSource`       — invoke a subprocess plugin as `OutputSource` via `SubprocessSource`
- [x] `PluginScorer`             — invoke a subprocess plugin as `Scorer` via `SubprocessScorer`
- [ ] Reference Python shim (`evalkit-plugin` on pypi)
- [x] Reference TypeScript shim (`@evalkit/plugin` source under `typescript/evalkit_plugin/`; typechecked with Bun via `devenv shell`)
- [ ] Conformance test harness  — fixture-driven golden harness beyond the current source/scorer checks

---

## Cost / tokenization helpers

- [ ] `TiktokenTokenizer`        — OpenAI cl100k / o200k
- [ ] `AnthropicTokenizer`
- [ ] `LlamaTokenizer`
- [ ] Cost tables per provider (in-repo JSON, updatable)
- [ ] `CostCalculator`           — combine tokenizer + cost table for pre-call estimation

---

## CI / workflow integrations

- [x] GitHub Action              — source lives under `.github/actions/evalkit-pr-comment/`; local live-PR verification still pending here
- [ ] GitLab CI template
- [ ] Pre-commit hook integration
- [ ] VS Code extension          — invoke runs from editor (later)

---

## Cross-cutting requirements

- Every integration lives in its own crate (`evalkit-<name>`) to keep dependency graphs tight.
- Feature-flagged `reqwest` / `hyper` / `tonic` etc. where optional.
- Integration tests use `MockSource` or `InMemoryTraceStore`; no live network unless marked `#[ignore]`.
- Exporters serialize through the frozen `evalkit-schema` types — never their own ad-hoc shape.
