# ADR-0003: Package Format Uses metadata.json Plus curves.parquet

## Status

Accepted

## Decision

The optimized local artifact format is:

```text
<name>.laspkg/
  metadata.json
  curves.parquet
```

This ADR describes the log-asset package format specifically. The broader multi-asset system now uses the same single-asset packaging principle for other asset families, but with type-specific manifests and bulk-data descriptors.

`metadata.json` stores:

- source/provenance
- summary/index information
- LAS sections and issues
- curve descriptors and storage-kind metadata

`curves.parquet` stores:

- the sample/index table
- one column per stored curve

## Why

- easy to inspect and debug
- interoperable with external data tooling
- simpler to evolve than one opaque package file
- lets metadata evolve independently from sample storage details

## Consequences

- package versioning must be explicit
- current package behavior must be documented separately from the future canonical schema target
- mixed numeric/text columns are preserved today even though the longer-term target may prefer a stricter canonical sample table
