# CLI Config Spec

This document defines the TOML config consumed by `evalkit run` and reused by `evalkit watch`.

## Invocation

```bash
evalkit run --dataset dataset.jsonl --config eval.toml
evalkit watch --dataset dataset.jsonl --config eval.toml
```

The dataset file is separate from the TOML config.

## Top-Level Shape

```toml
[acquisition]
# ... required

[run]
# ... optional

[[scorer]]
# ... at least one scorer is required

[threshold]
# ... optional
```

## Dataset File

The dataset is JSONL, one sample per line.

Supported fields:

```json
{"id":"sample-1","input":"What is 2 + 2?","reference":"4"}
```

Fields:
- `input`: required string
- `reference`: optional string
- `id`: optional string

Empty lines are ignored.

## `[acquisition]`

Exactly one acquisition mode must be configured.

### HTTP acquisition

```toml
[acquisition]
url = "https://example.com/generate"
input_field = "input"
output_field = "output"
timeout_secs = 30
```

Fields:
- `url`: required for HTTP mode
- `input_field`: optional, default `"input"`
- `output_field`: optional, default `"output"`
- `timeout_secs`: optional, default `30`

### Subprocess acquisition plugin

```toml
[acquisition]
command = ["python3", "plugin.py"]
timeout_secs = 30
```

Fields:
- `command`: required for subprocess mode
- `timeout_secs`: optional, default `30`

Notes:
- `command` may be either a string or an array of strings.
- Array form is preferred when arguments contain spaces.
- Subprocess plugins always use the canonical protocol fields `input` and `output`.
- For subprocess plugins, custom `input_field` and `output_field` values are rejected.

Invalid combinations:
- setting both `url` and `command`
- setting neither `url` nor `command`
- empty `command`

## `[run]`

This table is optional.

```toml
[run]
trials = 3
concurrency = 4
sample_timeout_secs = 10
```

Fields:
- `trials`: optional integer, default `1`
- `concurrency`: optional integer, default `4`
- `sample_timeout_secs`: optional integer

## `[[scorer]]`

At least one scorer entry is required.

Common fields:
- `type`: required string
- `name`: optional string override
- `timeout_secs`: optional integer, used by `plugin` scorers

### `exact_match`

```toml
[[scorer]]
type = "exact_match"
name = "exact"
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
pattern = "^hello"
```

Required fields:
- `pattern`: regex string

### `json_schema`

```toml
[[scorer]]
type = "json_schema"
schema = { type = "object", required = ["answer"] }
```

Required fields:
- `schema`: JSON value representing the schema

### `plugin`

```toml
[[scorer]]
type = "plugin"
name = "external_score"
command = ["python3", "score.py"]
timeout_secs = 5
```

Required fields:
- `command`: string or array command

Notes:
- plugin scorers use the subprocess scorer protocol described in `docs/plugin-protocol.md`
- empty plugin commands are rejected

## `[threshold]`

This table is optional and is evaluated after the run completes.

```toml
[threshold]
exact_match = 0.95
latency = 0.10
```

Behavior:
- binary scorers use pass rate
- numeric scorers use mean
- metric scorers use mean
- label scorers are skipped with a warning because they do not produce a primary numeric value

If any configured threshold is not met, `evalkit run` exits with status code `1`.

## Full Example

```toml
[acquisition]
command = ["python3", "model.py"]
timeout_secs = 30

[run]
trials = 2
concurrency = 4
sample_timeout_secs = 10

[[scorer]]
type = "exact_match"

[[scorer]]
type = "regex"
name = "looks_like_number"
pattern = "^[0-9]+$"

[threshold]
exact_match = 0.90
looks_like_number = 1.0
```

## Non-Config Commands

`evalkit diff` does not use this TOML file. It compares two previously written JSONL run results directly.
