#!/usr/bin/env python3
"""Validate the vendored ElevenLabs OpenAPI schema and optionally detect drift."""

from __future__ import annotations

import argparse
import json
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any


HERE = Path(__file__).resolve().parent
DEFAULT_SPEC = HERE / "openapi.elevenlabs.json"
DEFAULT_UPSTREAM = "https://api.elevenlabs.io/openapi.json"
HTTP_METHODS = {"delete", "get", "head", "options", "patch", "post", "put", "trace"}
REQUEST_MEDIA_TYPES = {"application/json", "multipart/form-data"}
PARAMETER_LOCATIONS = {"header", "path", "query"}
PATH_PARAMETER = re.compile(r"\{([^{}]+)\}")


class ValidationError(Exception):
    """A compact, user-facing schema validation failure."""


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--spec",
        type=Path,
        default=DEFAULT_SPEC,
        help="schema to validate (default: vendored snapshot)",
    )
    parser.add_argument(
        "--live",
        action="store_true",
        help="download the official schema and fail if it differs from --spec",
    )
    parser.add_argument(
        "--url",
        default=DEFAULT_UPSTREAM,
        help=argparse.SUPPRESS,
    )
    return parser.parse_args()


def load_json(path: Path) -> dict[str, Any]:
    try:
        with path.open("r", encoding="utf-8") as handle:
            value = json.load(handle)
    except OSError as exc:
        raise ValidationError(f"cannot read {path}: {exc}") from exc
    except json.JSONDecodeError as exc:
        raise ValidationError(f"invalid JSON in {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise ValidationError(f"schema root in {path} must be an object")
    return value


def download_json(url: str) -> dict[str, Any]:
    request = urllib.request.Request(
        url,
        headers={"Accept": "application/json", "User-Agent": "elevenlabs-cli-schema-check"},
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            payload = response.read()
    except (OSError, urllib.error.URLError) as exc:
        raise ValidationError(f"cannot download official schema: {exc}") from exc
    try:
        value = json.loads(payload)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ValidationError(f"official schema is not valid JSON: {exc}") from exc
    if not isinstance(value, dict):
        raise ValidationError("official schema root must be an object")
    return value


def decode_pointer_token(token: str) -> str:
    return token.replace("~1", "/").replace("~0", "~")


def resolve_ref(spec: dict[str, Any], reference: str) -> Any:
    if not reference.startswith("#/"):
        raise ValidationError(f"unsupported non-local reference: {reference}")
    value: Any = spec
    for raw_token in reference[2:].split("/"):
        token = decode_pointer_token(raw_token)
        if not isinstance(value, dict) or token not in value:
            raise ValidationError(f"unresolved reference: {reference}")
        value = value[token]
    return value


def validate_refs(spec: dict[str, Any]) -> None:
    pending: list[Any] = [spec]
    while pending:
        value = pending.pop()
        if isinstance(value, dict):
            reference = value.get("$ref")
            if reference is not None:
                if not isinstance(reference, str):
                    raise ValidationError("$ref values must be strings")
                resolve_ref(spec, reference)
            pending.extend(value.values())
        elif isinstance(value, list):
            pending.extend(value)


def resolved_object(spec: dict[str, Any], value: Any, context: str) -> dict[str, Any]:
    seen: set[str] = set()
    while isinstance(value, dict) and "$ref" in value:
        reference = value["$ref"]
        if not isinstance(reference, str):
            raise ValidationError(f"{context}: $ref must be a string")
        if reference in seen:
            raise ValidationError(f"{context}: circular reference {reference}")
        seen.add(reference)
        value = resolve_ref(spec, reference)
    if not isinstance(value, dict):
        raise ValidationError(f"{context}: expected an object")
    return value


def operation_alias(operation: dict[str, Any]) -> str:
    operation_id = operation.get("operationId")
    method_name = operation.get("x-fern-sdk-method-name") or operation_id
    groups = operation.get("x-fern-sdk-group-name", [])
    if isinstance(groups, str):
        groups = [groups]
    if not isinstance(groups, list) or any(
        not isinstance(group, str) or not group for group in groups
    ):
        raise ValidationError("x-fern-sdk-group-name must be a string or list of strings")
    if not isinstance(method_name, str) or not method_name:
        raise ValidationError("operation alias requires x-fern-sdk-method-name or operationId")
    return ".".join([*groups, method_name])


def iter_operations(spec: dict[str, Any]):
    paths = spec.get("paths")
    if not isinstance(paths, dict):
        raise ValidationError("schema must contain a paths object")
    for path in sorted(paths):
        path_item = paths[path]
        if not isinstance(path_item, dict):
            raise ValidationError(f"path item {path} must be an object")
        for method in sorted(HTTP_METHODS):
            if method not in path_item:
                continue
            operation = path_item[method]
            if not isinstance(operation, dict):
                raise ValidationError(f"{method.upper()} {path} must be an object")
            yield path, method, path_item, operation


def parameter_map(
    spec: dict[str, Any],
    path: str,
    method: str,
    path_item: dict[str, Any],
    operation: dict[str, Any],
) -> dict[tuple[str, str], dict[str, Any]]:
    parameters: dict[tuple[str, str], dict[str, Any]] = {}
    path_parameters = path_item.get("parameters", [])
    operation_parameters = operation.get("parameters", [])
    if not isinstance(path_parameters, list) or not isinstance(operation_parameters, list):
        raise ValidationError(f"{method.upper()} {path}: parameters must be an array")
    for raw in [*path_parameters, *operation_parameters]:
        parameter = resolved_object(spec, raw, f"{method.upper()} {path} parameter")
        location = parameter.get("in")
        name = parameter.get("name")
        if location not in PARAMETER_LOCATIONS:
            raise ValidationError(
                f"{method.upper()} {path}: unsupported parameter location {location!r}"
            )
        if not isinstance(name, str) or not name:
            raise ValidationError(f"{method.upper()} {path}: parameter name is missing")
        parameters[(location, name)] = parameter
    return parameters


def validate_spec(spec: dict[str, Any], label: str) -> list[dict[str, str]]:
    validate_refs(spec)
    operation_ids: dict[str, str] = {}
    aliases: dict[str, str] = {}
    operations: list[dict[str, str]] = []

    for path, method, path_item, operation in iter_operations(spec):
        identity = f"{method.upper()} {path}"
        if not (path.startswith("/v") or path == "/docs"):
            raise ValidationError(f"{identity}: route must start with /v (or be /docs)")

        operation_id = operation.get("operationId")
        if not isinstance(operation_id, str) or not operation_id:
            raise ValidationError(f"{identity}: operationId is missing")
        if operation_id in operation_ids:
            raise ValidationError(
                f"duplicate operationId {operation_id!r}: "
                f"{operation_ids[operation_id]} and {identity}"
            )
        operation_ids[operation_id] = identity

        alias = operation_alias(operation)
        if alias in aliases:
            raise ValidationError(f"duplicate alias {alias!r}: {aliases[alias]} and {identity}")
        aliases[alias] = identity

        parameters = parameter_map(spec, path, method, path_item, operation)
        declared_path_parameters = {
            name for location, name in parameters if location == "path"
        }
        template_parameters = set(PATH_PARAMETER.findall(path))
        if template_parameters != declared_path_parameters:
            missing = sorted(template_parameters - declared_path_parameters)
            extra = sorted(declared_path_parameters - template_parameters)
            details = []
            if missing:
                details.append(f"undeclared template parameters: {', '.join(missing)}")
            if extra:
                details.append(f"path parameters absent from template: {', '.join(extra)}")
            raise ValidationError(f"{identity}: {'; '.join(details)}")

        request_body = operation.get("requestBody")
        if request_body is not None:
            body = resolved_object(spec, request_body, f"{identity} request body")
            content = body.get("content", {})
            if not isinstance(content, dict):
                raise ValidationError(f"{identity}: request body content must be an object")
            unsupported = sorted(set(content) - REQUEST_MEDIA_TYPES)
            if unsupported:
                raise ValidationError(
                    f"{identity}: unsupported request media: {', '.join(unsupported)}"
                )

        operations.append(
            {"alias": alias, "operationId": operation_id, "method": method, "path": path}
        )

    if not operations:
        raise ValidationError("schema contains no HTTP operations")
    print(
        f"valid {label}: {len(operations)} operations, "
        f"{len(spec['paths'])} paths, {len(aliases)} aliases"
    )
    return operations


def operation_objects(spec: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {
        operation["operationId"]: {
            "method": method,
            "path": path,
            "operation": operation,
        }
        for path, method, _, operation in iter_operations(spec)
    }


def compare_specs(local: dict[str, Any], live: dict[str, Any]) -> None:
    if local == live:
        print("live schema matches vendored snapshot")
        return

    local_operations = operation_objects(local)
    live_operations = operation_objects(live)
    local_ids = set(local_operations)
    live_ids = set(live_operations)
    added = sorted(live_ids - local_ids)
    removed = sorted(local_ids - live_ids)
    changed = sorted(
        operation_id
        for operation_id in local_ids & live_ids
        if local_operations[operation_id] != live_operations[operation_id]
    )

    def compact(values: list[str]) -> str:
        limit = 12
        shown = ", ".join(values[:limit])
        if len(values) > limit:
            shown += f", ... (+{len(values) - limit})"
        return shown or "none"

    print(f"schema drift: added={compact(added)}", file=sys.stderr)
    print(f"schema drift: removed={compact(removed)}", file=sys.stderr)
    print(f"schema drift: changed={compact(changed)}", file=sys.stderr)
    if not (added or removed or changed):
        print("schema drift: non-operation metadata/components changed", file=sys.stderr)
    raise ValidationError(
        "official schema differs from vendored snapshot; run docs/reference/refresh.sh"
    )


def main() -> int:
    args = parse_args()
    try:
        local = load_json(args.spec)
        validate_spec(local, str(args.spec))
        if args.live:
            live = download_json(args.url)
            validate_spec(live, "official live schema")
            compare_specs(local, live)
    except ValidationError as exc:
        print(f"schema check failed: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
