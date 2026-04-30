# Benchmark Methodology

This document defines how `Ophiolite Charts` performance claims should be measured and discussed.

## Modes

- `smoke`
  - Goal: catch obvious regressions during normal development.
  - Typical use: local wrapper or renderer iteration.
  - Policy: low repetition count, fast turnaround, no public claims.
- `development`
  - Goal: compare two implementation branches with moderate noise tolerance.
  - Typical use: renderer changes, interaction plumbing, viewport math changes.
  - Policy: repeated pointer sweeps, machine metadata captured, raw JSON retained.
- `authoritative`
  - Goal: produce publishable evidence for docs, release notes, or sales support.
  - Typical use: benchmark snapshots taken on a controlled machine/browser setup.
  - Policy: fixed fixtures, fixed browser/runtime, recorded machine profile, repeated runs, raw results retained alongside summary stats.

## Required Metadata

Every benchmark run must record:

- timestamp
- benchmark mode
- git commit or working-tree note
- browser/runtime identity
- operating system
- CPU concurrency and device memory if available
- device pixel ratio
- chart family and fixture name
- renderer path
- repeat count and warmup policy

## Required Fixtures

The default benchmark set should include at least:

- seismic section
- well correlation / well panel
- rock physics crossplot

Each fixture should have:

- a named dataset
- a stable point/trace/sample count
- a documented renderer path
- a fixed viewport or interaction script

## Measurement Rules

- Separate setup measurements from steady-state interaction measurements.
- Use repeated sweeps or interaction loops, not a single `performance.now()` pair, for publishable interaction claims.
- Report at least `mean`, `median`, `p95`, `min`, and `max`.
- Preserve raw timings so variance can be inspected later.
- Do not compare different fixtures, browsers, or machines without stating that difference explicitly.

## Current Harness

The benchmark app in [apps/benchmark-app/src/main.ts](../apps/benchmark-app/src/main.ts) now supports:

- `mode=smoke|development|authoritative`
- repeated pointer-sweep measurements
- captured browser/runtime metadata
- raw JSON results shown alongside the human-readable summary

Example:

```text
/apps/benchmark-app?mode=development
```

The texture-upload microbenchmark in [scripts/capture-texture-upload-benchmark.mjs](../scripts/capture-texture-upload-benchmark.mjs) isolates WebGL2 upload costs for seismic display textures. It should be used before changing the heatmap amplitude texture format.

Example:

```bash
bun run charts:bench:texture-upload
```

The wiggle-geometry microbenchmark in [scripts/capture-wiggle-geometry-benchmark.ts](../scripts/capture-wiggle-geometry-benchmark.ts) compares CPU-expanded wiggle vertices with the instanced wiggle representation used by the WebGL worker path. It also records a warm-cache instanced mode to separate first-draw visible-scale measurement from repeated redraw cost.

Example:

```bash
bun run charts:bench:wiggle-geometry
```

The visual test suite includes a Svelte playground screenshot baseline for seismic wiggle mode on the local WebGL renderer. Use it after renderer changes that affect wiggle parity.

Example:

```bash
bun run test:visual -- svelte-playground-wiggle.spec.ts
```

## Publishing Rule

Do not publish benchmark marketing claims until the result set includes:

- authoritative mode
- named fixture definitions
- machine/browser metadata
- raw results retained in version control or attached release artifacts
- the exact measurement script used
