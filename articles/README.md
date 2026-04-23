# TraceBoost Articles

This folder collects long-form engineering notes and benchmark writeups that were previously kept in the repository root.

## Categories

### `architecture/`

- `GEOSPATIAL_CRS_ARCHITECTURE_AND_DESIGN.md`
- `CONTRACT_ARCHITECTURE_AND_MIGRATION.md`
- `TRACEBOOST_DESKTOP_SECURITY_HARDENING_2026.md`

### `benchmarking/`

- `INTERACTIVE_SECTION_BROWSING_HARNESS_PLAN.md`
- `PREVIEW_INCREMENTAL_EXECUTION_BENCHMARK_PLAN.md`
- `results/2026-04-22-f3-trace-local-agc-benchmark.json`
- `results/2026-04-22-f3-trace-local-analytic-benchmark.json`
- `results/2026-04-22-f3-trace-local-batch-agc-benchmark.json`
- `results/2026-04-22-f3-trace-local-batch-analytic-benchmark.json`
- `results/2026-04-22-f3-trace-local-batch-agc-auto-mode-benchmark.json`
- `results/2026-04-22-f3-trace-local-batch-agc-conservative-mode-benchmark.json`
- `results/2026-04-22-f3-trace-local-batch-agc-throughput-mode-benchmark.json`
- `results/2026-04-22-f3-trace-local-batch-analytic-auto-mode-benchmark.json`
- `results/2026-04-22-f3-trace-local-batch-analytic-conservative-mode-benchmark.json`
- `results/2026-04-22-f3-trace-local-batch-analytic-throughput-mode-benchmark.json`
- `results/2026-04-22-f3-trace-local-agc-64mib-authoritative-workers8-benchmark.json`
- `results/2026-04-22-f3-trace-local-agc-adaptive-authoritative-workers8-benchmark.json`
- `results/2026-04-22-f3-trace-local-analytic-64mib-authoritative-workers8-benchmark.json`
- `results/2026-04-22-f3-trace-local-analytic-adaptive-authoritative-workers8-benchmark.json`
- `results/2026-04-22-f3-trace-local-batch-agc-auto-64mib-authoritative-workers8-benchmark.json`
- `results/2026-04-22-f3-trace-local-batch-agc-auto-adaptive-authoritative-workers8-benchmark.json`
- `results/2026-04-22-f3-trace-local-batch-analytic-auto-64mib-authoritative-workers8-benchmark.json`
- `results/2026-04-22-f3-trace-local-batch-analytic-auto-adaptive-authoritative-workers8-benchmark.json`
- `results/2026-04-22-f3-trace-local-agc-classification-authoritative-workers8-benchmark.json`
- `results/2026-04-22-f3-trace-local-analytic-classification-authoritative-workers8-benchmark.json`
- `results/2026-04-22-f3-post-stack-neighborhood-preview-similarity-small-authoritative-workers8-benchmark.json`
- `results/2026-04-22-f3-post-stack-neighborhood-preview-similarity-small-prefix-authoritative-workers8-benchmark.json`
- `results/2026-04-22-f3-post-stack-neighborhood-preview-similarity-medium-authoritative-workers8-benchmark.json`
- `results/2026-04-22-f3-post-stack-neighborhood-preview-similarity-large-authoritative-workers8-benchmark.json`
- `results/2026-04-22-f3-post-stack-neighborhood-processing-similarity-small-authoritative-workers8-benchmark.json`
- `results/2026-04-22-f3-post-stack-neighborhood-processing-similarity-small-prefix-authoritative-workers8-benchmark.json`
- `results/2026-04-22-f3-post-stack-neighborhood-processing-similarity-medium-authoritative-workers8-benchmark.json`
- `results/2026-04-22-f3-post-stack-neighborhood-processing-similarity-large-authoritative-workers8-benchmark.json`

The interactive section browsing harness note now matches the current repo state:

- runtime benches already exist for CI-friendly performance checks
- desktop session logs can be summarized with `scripts/validation/traceboost_section_tiling_report.py`
- TraceBoost desktop now has an internal section-browsing benchmark command surface, plus a temporary desktop hook, for repeatable app-path scenarios

### `performance/`

- `PROCESSING_CACHE_ARCHITECTURE_AND_BENCHMARKING.md`
- `SECTION_TILING_AND_INTERACTIVE_SECTION_BROWSING_OPTIMIZATIONS.md`
- `TRACE_LOCAL_EXECUTION_SERVICE_AND_PARTITIONED_BATCH_BENCHMARKING.md`
- `TRACEBOOST_PERFORMANCE_PROFILING_AND_OPTIMIZATIONS.md`

### `storage/`

- `SEISMIC_VOLUME_STORAGE_AND_BENCHMARKING.md`
- `SEISMIC_VOLUME_STORAGE_AND_BENCHMARKING_II.md`
- `TBVOL_EXACT_COMPRESSED_STORAGE_PROPOSAL.md`

## Notes

- The root `README.md` remains in the repository root as the entrypoint for contributors.
- The storage articles are intentionally split:
  - Part I explains why `tbvol` replaced the earlier runtime-store candidates for active compute.
  - Part II documents the later exact-lossless compression study and what it implies for `tbvol` as a processing and storage substrate.
  - The proposal article turns those findings into a concrete product shape for an optional exact compressed storage tier.
