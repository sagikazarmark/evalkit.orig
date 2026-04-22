# CLI Config

This document describes the TOML config format consumed by `evalkit-cli run` today.

## Command

```bash
evalkit run --dataset samples.jsonl --config eval.toml --output results.jsonl
```

## Top-Level Shape

```toml
[acquisition]
url = "https://example.test/infer"
# or
# command = ["python3", "model.py"]

[run]
trials = 1
concurrency = 4
sample_timeout_secs = 30

[[scorer]]
type = "exact_match"

[threshold]
exact_match = 0.9
```

## `[acquisition]`

Exactly one of `url` or `command` must be set.

Fields:
- `url: string` - HTTP endpoint for acquisition
- `command: string | string[]` - subprocess acquisition command
- `input_field: string` - request JSON key, default `"input"`
- `output_field: string` - response JSON key, default `"output"`
- `timeout_secs: integer` - acquisition timeout in seconds, default `30`

Rules:
- Setting both `url` and `command` is an error.
- Setting neither `url` nor `command` is an error.
- `command = []` is an error.

## `[run]`

Fields:
- `trials: integer` - per-sample trial count, default `1`
- `concurrency: integer` - max in-flight sample executions, default `4`
- `sample_timeout_secs: integer` - optional timeout applied to the entire sample acquisition

Values lower than `1` are clamped to `1` by the builder path used today.

## `[[scorer]]`

At least one scorer entry is required.

Common fields:
- `type: string` - scorer kind
- `name: string` - optional override for the emitted score name

Supported scorer types today:

### `exact_match`

```toml
[[scorer]]
type = "exact_match"
```

### `contains`

```toml
[[scorer]]
type = "contains"
```

### `regex`

```toml
[[scorer]]
type = "regex"
pattern = "#\\d+"
```

Required fields:
- `pattern: string`

### `json_schema`

```toml
[[scorer]]
type = "json_schema"
schema = { type = "object", required = ["answer"] }
```

Required fields:
- `schema: JSON value`

Unknown scorer types are rejected.

## `[threshold]`

Maps scorer names to minimum acceptable numeric values.

```toml
[threshold]
exact_match = 0.8
latency = 100.0
```

Threshold evaluation uses:
- `pass_rate` for binary scorers
- `mean` for numeric scorers
- `mean` for metric scorers
- no threshold value for label scorers

## Dataset Format

The dataset file is JSONL. Each non-empty line must be an object of the form:

```json
{"id":"sample-1","input":"What is 2 + 2?","reference":"4"}
```

Fields:
- `id: string` - optional explicit sample id
- `input: string` - required
- `reference: string` - optional reference output

If `id` is omitted, `evalkit` generates a deterministic sample id.

Empty dataset files are rejected.
