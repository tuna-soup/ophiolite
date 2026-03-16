# ADR-0002: Staged Arrow and Parquet Adoption

## Status

Accepted

## Decision

Arrow and Parquet are adopted in stages.

Current stage:

- Parquet is used for package persistence
- `CurveTable` is the app-facing in-memory abstraction
- Arrow/Parquet do not dominate the public API

Deferred stage:

- deeper Arrow-backed runtime semantics
- stricter canonical package schema
- broader Arrow-facing conversion APIs

## Why

- the project is still stabilizing canonical metadata, package guarantees, and Tauri-facing query shapes
- locking Arrow too early into the public contract would make current design churn more expensive
- the package/storage benefit is available now without overcommitting the runtime model

## Consequences

- current Arrow/Parquet use should be described as package/storage plumbing plus an interoperability step
- future work should tighten metadata and sample-table semantics before deepening Arrow runtime integration
