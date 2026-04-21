#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import math
import re
import statistics
import sys
from pathlib import Path
from typing import Any


SECTION_TILE_MESSAGES = {
    "active_fetch": "Loaded section tile for the active viewport.",
    "cache_hit": "Viewport request satisfied from section tile cache.",
    "prefetch_fetch": "Prefetched adjacent section tile.",
    "prefetch_cache_hit": "Adjacent section tile already present in cache.",
    "prefetch_error": "Adjacent section tile prefetch failed.",
    "fetch_fallback": "Section tile fetch fell back to the current section payload.",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Summarize TraceBoost section-tiling diagnostics from a desktop session log."
    )
    parser.add_argument("--log", required=True, help="Path to a TraceBoost desktop session log")
    parser.add_argument(
        "--format",
        choices=("text", "json"),
        default="text",
        help="Output format"
    )
    parser.add_argument(
        "--max-active-fetch-ms",
        type=float,
        default=None,
        help="Fail if the median active viewport fetch elapsedMs exceeds this value"
    )
    parser.add_argument(
        "--max-fallbacks",
        type=int,
        default=None,
        help="Fail if the number of viewport fetch fallbacks exceeds this value"
    )
    parser.add_argument(
        "--min-cache-hit-events",
        type=int,
        default=None,
        help="Fail if the number of cache-hit events is below this value"
    )
    return parser.parse_args()


def parse_fields(line: str) -> dict[str, Any] | None:
    match = re.search(r"fields=(\{.*\})$", line)
    if not match:
        return None
    try:
        return json.loads(match.group(1))
    except json.JSONDecodeError:
        return None


def parse_duration_ms(line: str) -> float | None:
    match = re.search(r"duration_ms=(\d+(?:\.\d+)?)", line)
    if not match:
        return None
    return float(match.group(1))


def safe_stats(values: list[float]) -> dict[str, float | int] | None:
    if not values:
        return None
    ordered = sorted(values)
    p90_index = min(len(ordered) - 1, round((len(ordered) - 1) * 0.9))
    return {
        "n": len(ordered),
        "min": ordered[0],
        "median": statistics.median(ordered),
        "p90": ordered[p90_index],
        "max": ordered[-1],
        "mean": statistics.mean(ordered),
    }


def round_stats(stats: dict[str, float | int] | None) -> dict[str, float | int] | None:
    if stats is None:
        return None
    rounded: dict[str, float | int] = {}
    for key, value in stats.items():
        if isinstance(value, float):
            rounded[key] = round(value, 3)
        else:
            rounded[key] = value
    return rounded


def build_report(log_path: Path) -> dict[str, Any]:
    lines = log_path.read_text().splitlines()

    active_fetch_fields: list[dict[str, Any]] = []
    prefetch_fields: list[dict[str, Any]] = []
    backend_durations: list[float] = []
    cache_hit_events = 0
    prefetch_cache_hit_events = 0
    prefetch_error_events = 0
    fallback_events = 0

    for line in lines:
        if SECTION_TILE_MESSAGES["active_fetch"] in line:
            fields = parse_fields(line)
            if fields:
                active_fetch_fields.append(fields)
        elif SECTION_TILE_MESSAGES["cache_hit"] in line:
            cache_hit_events += 1
        elif SECTION_TILE_MESSAGES["prefetch_fetch"] in line:
            fields = parse_fields(line)
            if fields:
                prefetch_fields.append(fields)
        elif SECTION_TILE_MESSAGES["prefetch_cache_hit"] in line:
            prefetch_cache_hit_events += 1
        elif SECTION_TILE_MESSAGES["prefetch_error"] in line:
            prefetch_error_events += 1
        elif SECTION_TILE_MESSAGES["fetch_fallback"] in line:
            fallback_events += 1
        elif "Section tile view loaded (binary)" in line:
            duration = parse_duration_ms(line)
            if duration is not None:
                backend_durations.append(duration)

    active_fetch_elapsed = [
        float(fields["elapsedMs"])
        for fields in active_fetch_fields
        if isinstance(fields.get("elapsedMs"), (int, float))
    ]
    active_fetch_payloads = [
        float(fields["payloadBytes"])
        for fields in active_fetch_fields
        if isinstance(fields.get("payloadBytes"), (int, float))
    ]
    prefetch_elapsed = [
        float(fields["elapsedMs"])
        for fields in prefetch_fields
        if isinstance(fields.get("elapsedMs"), (int, float))
    ]

    last_active_fetch = active_fetch_fields[-1] if active_fetch_fields else None

    return {
        "log_path": str(log_path),
        "event_counts": {
            "active_fetch": len(active_fetch_fields),
            "cache_hit": cache_hit_events,
            "prefetch_fetch": len(prefetch_fields),
            "prefetch_cache_hit": prefetch_cache_hit_events,
            "prefetch_error": prefetch_error_events,
            "fetch_fallback": fallback_events,
        },
        "active_fetch_elapsed_ms": round_stats(safe_stats(active_fetch_elapsed)),
        "backend_tile_duration_ms": round_stats(safe_stats(backend_durations)),
        "prefetch_elapsed_ms": round_stats(safe_stats(prefetch_elapsed)),
        "active_fetch_payload_bytes": round_stats(safe_stats(active_fetch_payloads)),
        "sample_last_active_fetch": {
            key: last_active_fetch.get(key)
            for key in (
                "axis",
                "index",
                "traceRange",
                "sampleRange",
                "viewportTraceRange",
                "viewportSampleRange",
                "lod",
                "elapsedMs",
                "payloadBytes",
                "cacheHits",
                "fetches",
                "prefetchRequests",
            )
        }
        if last_active_fetch
        else None,
    }


def evaluate_thresholds(report: dict[str, Any], args: argparse.Namespace) -> list[str]:
    failures: list[str] = []

    active_stats = report.get("active_fetch_elapsed_ms")
    if args.max_active_fetch_ms is not None:
        if not active_stats or active_stats.get("median") is None:
            failures.append("no active viewport fetch samples were found")
        elif float(active_stats["median"]) > args.max_active_fetch_ms:
            failures.append(
                f"median active fetch {active_stats['median']} ms exceeds {args.max_active_fetch_ms} ms"
            )

    fallback_count = int(report["event_counts"]["fetch_fallback"])
    if args.max_fallbacks is not None and fallback_count > args.max_fallbacks:
        failures.append(
            f"fetch fallback count {fallback_count} exceeds {args.max_fallbacks}"
        )

    cache_hit_count = int(report["event_counts"]["cache_hit"])
    if args.min_cache_hit_events is not None and cache_hit_count < args.min_cache_hit_events:
        failures.append(
            f"cache-hit event count {cache_hit_count} is below {args.min_cache_hit_events}"
        )

    return failures


def print_text_report(report: dict[str, Any], failures: list[str]) -> None:
    print(f"log: {report['log_path']}")
    print("event counts:")
    for key, value in report["event_counts"].items():
        print(f"  {key}: {value}")

    def print_stats(label: str, stats: dict[str, Any] | None) -> None:
        if not stats:
            print(f"{label}: none")
            return
        print(
            f"{label}: n={stats['n']} min={stats['min']} median={stats['median']} "
            f"p90={stats['p90']} max={stats['max']} mean={stats['mean']}"
        )

    print_stats("active fetch elapsed ms", report.get("active_fetch_elapsed_ms"))
    print_stats("backend tile duration ms", report.get("backend_tile_duration_ms"))
    print_stats("prefetch elapsed ms", report.get("prefetch_elapsed_ms"))
    print_stats("active fetch payload bytes", report.get("active_fetch_payload_bytes"))

    sample = report.get("sample_last_active_fetch")
    if sample:
        print("sample last active fetch:")
        for key, value in sample.items():
            print(f"  {key}: {value}")

    if failures:
        print("threshold failures:")
        for failure in failures:
            print(f"  - {failure}")
    else:
        print("threshold failures: none")


def main() -> int:
    args = parse_args()
    log_path = Path(args.log).expanduser().resolve()
    if not log_path.is_file():
        print(f"error: log file not found: {log_path}", file=sys.stderr)
        return 2

    report = build_report(log_path)
    failures = evaluate_thresholds(report, args)

    if args.format == "json":
        print(json.dumps({"report": report, "threshold_failures": failures}, indent=2))
    else:
        print_text_report(report, failures)

    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
