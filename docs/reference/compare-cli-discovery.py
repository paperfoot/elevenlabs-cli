#!/usr/bin/env python3
"""Compare installed binaries offline; no API calls or credentials are needed.

Usage: python3 compare-cli-discovery.py /path/to/paperfoot /path/to/official
Writes a JSON report to stdout. This is a maintainer tool, not a CLI dependency.
"""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import tempfile
import time


def measure(binary, args, env, cwd):
    started = time.perf_counter_ns()
    result = subprocess.run(
        [str(binary), *args],
        env=env,
        cwd=cwd,
        stdin=subprocess.DEVNULL,
        capture_output=True,
        timeout=30,
        check=False,
    )
    elapsed_ms = (time.perf_counter_ns() - started) / 1_000_000
    row = {
        "args": args,
        "exit_code": result.returncode,
        "stdout_bytes": len(result.stdout),
        "stderr_bytes": len(result.stderr),
    }
    parsed = {}
    for stream in ("stdout", "stderr"):
        try:
            value = json.loads(getattr(result, stream))
            parsed[stream] = value
            row[stream + "_json"] = True
            row[stream + "_compact_json_bytes"] = len(
                (json.dumps(value, ensure_ascii=False, separators=(",", ":")) + "\n").encode()
            )
        except (ValueError, UnicodeDecodeError):
            row[stream + "_json"] = False
    return row, parsed, elapsed_ms


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paperfoot", type=Path)
    parser.add_argument("official", type=Path)
    parser.add_argument("--samples", type=int, default=25)
    args = parser.parse_args()
    if args.samples < 1:
        parser.error("--samples must be positive")
    binaries = {name: getattr(args, name).resolve(strict=True) for name in ("paperfoot", "official")}
    cases = {
        "paperfoot": {
            "version": ["--version"],
            "help": ["--help"],
            "index": ["agent-info"],
            "tts": ["agent-info", "--command", "tts"],
            "music": ["agent-info", "--command", "music compose"],
            "invalid": ["unknown-command"],
            "missing_argument": ["tts"],
        },
        "official": {
            "version": ["--version"],
            "help": ["--help"],
            "index": ["--schema"],
            "tts": ["text-to-speech", "convert", "--schema"],
            "music": ["music", "compose", "--schema"],
            "invalid": ["unknown-command"],
            "missing_argument": ["text-to-speech", "convert", "--text", "hello"],
            "dry_run": ["text-to-speech", "convert", "--voice-id", "audit-placeholder", "--text", "hello", "--dry-run"],
        },
    }
    report = {
        "measured_at_utc": datetime.now(timezone.utc).isoformat(),
        "platform": platform.system(),
        "platform_release": platform.release(),
        "architecture": platform.machine(),
        "timing": "Warm local subprocess wall time; includes startup and discovery, excludes API work.",
        "samples": args.samples,
        "binaries": {},
    }
    with tempfile.TemporaryDirectory(prefix="elevenlabs-audit-", dir="/tmp") as directory:
        # Allowlist only: never inherit an API key, config path, .env, or proxy.
        env = {
            "PATH": os.defpath,
            "HOME": directory,
            "XDG_CONFIG_HOME": directory,
            "ELEVENLABS_CLI_CONFIG": directory + "/config.toml",
            "NO_COLOR": "1",
        }
        for name, binary in binaries.items():
            entry = {
                "bytes": binary.stat().st_size,
                "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                "checks": {},
            }
            for label, command in cases[name].items():
                row, parsed, _ = measure(binary, command, env, directory)
                expected = 3 if label in ("invalid", "missing_argument") else 0
                if row["exit_code"] != expected:
                    raise RuntimeError(f"{name}/{label}: expected exit {expected}, got {row}")
                if label in ("index", "tts", "music", "dry_run") and "stdout" not in parsed:
                    raise RuntimeError(f"{name}/{label}: expected JSON stdout")
                if label == "index":
                    field = "commands" if name == "paperfoot" else "operations"
                    entry["discovery_entries"] = len(parsed["stdout"][field])
                if label == "version":
                    if name == "paperfoot":
                        entry["version"] = parsed["stdout"]["data"]["usage"]
                    else:
                        version = subprocess.run(
                            [str(binary), "--version"], env=env, cwd=directory,
                            stdin=subprocess.DEVNULL, capture_output=True,
                            timeout=30, check=True,
                        )
                        entry["version"] = version.stdout.decode().strip()
                if label == "dry_run" and parsed["stdout"].get("dry_run") is not True:
                    raise RuntimeError("Official dry-run did not identify a preview")
                entry["checks"][label] = row
            report["binaries"][name] = entry
        samples = {name: [] for name in binaries}
        for index in range(args.samples):
            # Alternate order to reduce consistent first/second effects.
            order = list(binaries) if index % 2 == 0 else list(reversed(binaries))
            for name in order:
                row, _, elapsed = measure(binaries[name], cases[name]["tts"], env, directory)
                if row["exit_code"] != 0:
                    raise RuntimeError(f"{name}: timing run failed")
                samples[name].append(elapsed)
        for name, values in samples.items():
            report["binaries"][name]["tts_discovery_median_ms"] = round(statistics.median(values), 3)
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
