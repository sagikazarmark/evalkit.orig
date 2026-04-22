from __future__ import annotations

import json
import sys
from dataclasses import dataclass
from typing import Any, Callable

PLUGIN_PROTOCOL_VERSION = "1"


@dataclass(frozen=True)
class PluginSpec:
    kind: str
    name: str
    version: str
    capabilities: tuple[str, ...]


@dataclass(frozen=True)
class PluginError(Exception):
    code: str
    message: str
    details: Any = None


def acquisition_plugin(
    name: str,
    *,
    version: str = "0.1.0",
    capabilities: tuple[str, ...] | list[str] = (),
) -> Callable[[Callable[[str], str]], Callable[[str], str]]:
    return _decorate_plugin("acquisition", name, version, capabilities)


def scorer_plugin(
    name: str,
    *,
    version: str = "0.1.0",
    capabilities: tuple[str, ...] | list[str] = (),
) -> Callable[[Callable[..., dict[str, Any]]], Callable[..., dict[str, Any]]]:
    return _decorate_plugin("scorer", name, version, capabilities)


def _decorate_plugin(
    kind: str,
    name: str,
    version: str,
    capabilities: tuple[str, ...] | list[str],
) -> Callable[[Callable[..., Any]], Callable[..., Any]]:
    spec = PluginSpec(
        kind=kind,
        name=name,
        version=version,
        capabilities=tuple(capabilities),
    )

    def decorator(func: Callable[..., Any]) -> Callable[..., Any]:
        setattr(func, "__evalkit_plugin_spec__", spec)
        return func

    return decorator


def run_plugin(plugin: Callable[..., Any]) -> None:
    spec = getattr(plugin, "__evalkit_plugin_spec__", None)
    if spec is None:
        raise TypeError("plugin must be decorated with acquisition_plugin or scorer_plugin")

    request = _read_request()
    _write_json(
        {
            "kind": spec.kind,
            "name": spec.name,
            "version": spec.version,
            "schema_version": PLUGIN_PROTOCOL_VERSION,
            "capabilities": list(spec.capabilities),
        }
    )

    try:
        if spec.kind == "acquisition":
            response = {"output": plugin(request["input"])}
        elif spec.kind == "scorer":
            response = {
                "score": plugin(
                    request["input"],
                    request["output"],
                    request.get("reference"),
                    request.get("run_id"),
                    request.get("sample_id"),
                    request["trial_index"],
                    request.get("metadata", {}),
                )
            }
        else:
            raise RuntimeError(f"unsupported plugin kind: {spec.kind}")
    except PluginError as err:
        response = {
            "error": {
                "code": err.code,
                "message": err.message,
                "details": err.details if err.details is not None else {},
            }
        }

    _write_json(response)


def _read_request() -> dict[str, Any]:
    line = sys.stdin.readline()
    if not line:
        raise RuntimeError("expected one JSON request line on stdin")

    payload = json.loads(line)
    if not isinstance(payload, dict):
        raise TypeError("plugin request must be a JSON object")
    return payload


def _write_json(payload: dict[str, Any]) -> None:
    sys.stdout.write(json.dumps(payload, separators=(",", ":")))
    sys.stdout.write("\n")
    sys.stdout.flush()
