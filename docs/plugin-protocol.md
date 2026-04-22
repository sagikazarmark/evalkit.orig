# Subprocess Plugin Protocol

This document describes the subprocess acquisition protocol implemented today by `evalkit-cli`.

It is intentionally narrow:
- It supports the `Acquisition` role only.
- It does not yet implement a versioned handshake.
- It does not yet support scorer plugins.

Those are planned roadmap items. This document freezes the behavior that already exists so it can be migrated cleanly into the future formal plugin spec.

## Scope

The CLI can execute an external command as the acquisition step for a run.

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

## Request Format

The CLI writes exactly one JSON object line to the child process:

```json
{"<input_field>": "<input text>"}
```

Example with default field names:

```json
{"input":"What is 2 + 2?"}
```

After the JSON line is written, the CLI closes `stdin` so the child sees EOF.

## Response Format

The CLI reads the first line from `stdout` and expects exactly one JSON object containing the configured output field:

```json
{"<output_field>": "<output text>"}
```

Example with default field names:

```json
{"output":"4"}
```

## Semantics

- The configured `input_field` controls the request key.
- The configured `output_field` controls the response key.
- The output value must be a JSON string.
- Empty stdout is treated as an acquisition failure.
- Invalid JSON is treated as an acquisition failure.
- Missing `output_field` is treated as an acquisition failure.
- A timeout is enforced by the CLI using `timeout_secs`.

## Exit Status

The CLI waits for the child process to exit, but a non-zero exit status does not invalidate an otherwise well-formed response. If the child emitted valid JSON on the first stdout line, that response is accepted.

## Error Mapping

The CLI maps subprocess failures into `AcquisitionError`:

- spawn / IO / JSON parse / missing field -> `AcquisitionError::ExecutionFailed`
- timeout -> `AcquisitionError::Timeout`

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

## Future Extension

The Phase 1 roadmap target extends this current protocol with:
- a versioned handshake
- explicit plugin kind metadata
- scorer plugins in addition to acquisition plugins
- capability negotiation
- preserved structured plugin error payloads

Until that lands, this document is the source of truth for the protocol implemented by `evalkit-cli` today.
