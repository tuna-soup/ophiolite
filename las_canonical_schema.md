# Minimal Canonical LAS Schema

## Status

This is a target-state schema note for the later canonical-model and deeper Arrow/Parquet phase.

It is **not** a description of full current conformance.

Current implementation already captures:

- a canonical domain object (`LasFile`)
- an app-facing in-memory table abstraction (`CurveTable`)
- package persistence via `metadata.json + curves.parquet`
- storage-agnostic public semantics

Current implementation does **not** yet fully capture:

- a stricter typed canonical metadata model (`VersionInfo`, `WellInfo`, `IndexInfo`, `CurveInfo`)
- canonical renaming of the index column to `index`
- a numeric-only canonical sample table contract
- Arrow as the dominant in-memory runtime representation

## Overview

The intended long-term architecture is:

```text
LAS file
  -> parser/importer
  -> canonical model
  -> Arrow-backed sample table
  -> package:
       curves.parquet
       metadata.json
```

The system separates:

1. canonical domain model
2. sample-table representation
3. package/storage format

## Canonical Domain Model

The SDK should ultimately expose domain objects instead of storage types.

```rust
LasDocument {
    version: VersionInfo,
    well: WellInfo,
    index: IndexInfo,
    curves: Vec<CurveInfo>,
    parameters: Vec<ParameterInfo>,
    other: Option<String>,
    samples: CurveTable,
}
```

Desired public semantics:

```text
las.version()
las.well()
las.index_info()
las.curve_infos()
las.data()
las.curve("GR")
las.slice_by_index(min, max)
las.write_package(path)
LasDocument::read_package(path)
```

## Canonical Sample Table Rules

Target rules:

1. one row per sample
2. one sample-axis column plus one column per curve
3. stable canonical handling of the index axis
4. LAS null sentinels become nulls in the canonical table
5. units, descriptions, provenance, and original mnemonics stay in metadata

Illustrative shape:

```text
index   GR    RHOB   NPHI
1000.0  85.0  2.35   0.21
1000.5  null  2.34   0.22
1001.0  83.0  null   0.20
```

## Package Layout

Target package layout:

```text
well_123.laspkg/
  curves.parquet
  metadata.json
```

`curves.parquet` should hold the canonical sample matrix.

`metadata.json` should hold:

- schema/package version
- version metadata
- well metadata
- index metadata
- curve descriptors
- parameters
- other text
- provenance and diagnostics

## Roadmap Placement

Before claiming full canonical-schema alignment, `lithos` should first stabilize:

1. canonical metadata types
2. package schema/version guarantees
3. Tauri/backend DTO/query shapes
4. nullability/index/curve descriptor rules

Only after those are stable should the project tighten the runtime and package behavior toward this full schema.
