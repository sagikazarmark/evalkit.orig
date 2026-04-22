# Subprocess Plugin Protocol

This document describes the versioned subprocess plugin protocol implemented by the acquisition and scorer runtimes today.

Current status:
- Acquisition plugins support a versioned handshake.
- Scorer plugins support a versioned handshake.
- Structured plugin error payloads are preserved for both kinds.
- Legacy no-handshake subprocess acquisition plugins remain accepted for compatibility.
- Scorer plugins use the canonical v1 request/response envelope only.

## Scope

The CLI can execute an external command as the acquisition step for a run and as a scorer step for a run. `evalkit-providers` exposes typed protocol structs plus acquisition/scorer conformance checks.

Configured in TOML as:

```toml
[acquisition]
command = ["python3", "model.py"]
input_field = "input"
output_field = "output"
timeout_secs = 30
```

## Transport

- The plugin is launched once per sample.
- `stdin` is piped to the child process.
- `stdout` is piped from the child process.
- `stderr` is ignored by the CLI.
- For v1 handshake-capable acquisition plugins, the plugin writes two stdout lines after receiving the request:
- line 1: handshake
- line 2: response

## Handshake

Handshake line:

```json
{
  "kind": "acquisition",
  "name": "demo-plugin",
  "version": "0.1.0",
  "schema_version": "1",
  "capabilities": ["structured-errors"]
}
```

Fields:
- `kind`: `"acquisition"` or `"scorer"`
- `name`: stable plugin name
- `version`: plugin implementation version
- `schema_version`: plugin protocol version, currently `"1"`
- `capabilities`: optional string list

## Acquisition Request Format

Canonical v1 request:

```json
{"input":"<input text>"}
```

Legacy compatibility:
- Older subprocess plugins may still receive a request keyed by the configured `input_field`.
- That legacy mode is kept only for compatibility during the protocol transition.

Example with default field names:

```json
{"input":"What is 2 + 2?"}
```

After the JSON line is written, the CLI closes `stdin` so the child sees EOF.

## Acquisition Response Format

Success response:

```json
{"output":"<output text>"}
```

Structured error response:

```json
{
  "error": {
    "code": "bad_input",
    "message": "input failed validation",
    "details": {"field":"input"}
  }
}
```

Legacy compatibility:
- Older subprocess plugins may still emit a single response object using the configured `output_field` and no handshake line.

Example with default field names:

```json
{"output":"4"}
```

## Semantics

- A handshake-capable acquisition plugin must emit a valid handshake before its response line.
- Handshake `kind` must be `"acquisition"`.
- Handshake `schema_version` must be `"1"`.
- Successful plugin responses must include `output` and must not include `error`.
- Failed plugin responses must include `error` and must not include `output`.
- Empty stdout is treated as an acquisition failure.
- Invalid JSON is treated as an acquisition failure.
- A timeout is enforced by the CLI using `timeout_secs`.

## Scorer Plugin Shape

The protocol reserves `kind: "scorer"` for scorer plugins.

Canonical v1 request shape:

```json
{
  "input": "<input text>",
  "output": "<candidate output>",
  "reference": "<optional reference>",
  "run_id": "<optional run id>",
  "sample_id": "<optional sample id>",
  "trial_index": 0,
  "metadata": {"...":"..."}
}
```

Canonical v1 response shape:

```json
{"score": {"type":"binary","value":true}}
```

or:

```json
{
  "error": {
    "code": "invalid_output",
    "message": "candidate output was not valid JSON",
    "details": {}
  }
}
```

Semantics:
- Scorer plugins must emit a handshake before the score response line.
- Handshake `kind` must be `"scorer"`.
- Handshake `schema_version` must be `"1"`.
- Successful scorer responses must include `score` and must not include `error`.
- Failed scorer responses must include `error` and must not include `score`.
- Empty stdout is treated as a scorer failure.
- Invalid JSON is treated as a scorer failure.

## Exit Status

The CLI waits for the child process to exit, but a non-zero exit status does not invalidate an otherwise well-formed response. If the child emitted valid JSON on the first stdout line, that response is accepted.

## Error Mapping

The CLI maps subprocess failures into `AcquisitionError` for acquisition plugins:

- spawn / IO / JSON parse / missing field -> `AcquisitionError::ExecutionFailed`
- plugin handshake / protocol violations -> `AcquisitionError::ExecutionFailed`
- structured plugin error payloads -> `AcquisitionError::ExecutionFailed` with the plugin payload preserved in the boxed source error
- timeout -> `AcquisitionError::Timeout`

For scorer plugins:
- spawn / IO / JSON parse / missing field -> `ScorerError::ProviderError`
- plugin handshake / protocol violations -> `ScorerError::ProviderError`
- structured plugin error payloads -> `ScorerError::ProviderError` with the plugin payload preserved in the boxed source error
- timeout -> `ScorerError::Timeout`

## Command Encoding

The `command` field may be provided as either:

```toml
command = "python3 model.py"
```

or:

```toml
command = ["python3", "model.py"]
```

The array form is preferred when arguments contain spaces.

## Conformance

`evalkit-providers` exposes acquisition/scorer conformance checks that validate:
- the handshake line
- plugin kind
- protocol schema version
- response envelope shape

This document is the source of truth for the protocol implemented by the acquisition path today.
