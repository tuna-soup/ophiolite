
# Lithos Crate Architecture and Project Structure

This document describes the recommended crate/module structure for the Lithos project.
It explains the responsibilities of each layer and how they interact.

Lithos should separate LAS parsing, canonical modeling, runtime curve access,
and package persistence into distinct crates to keep the system maintainable
as the project grows.

---

# Core Architectural Principle

Lithos should separate the lifecycle of LAS data into four layers:

1. Parse raw LAS text
2. Represent it as canonical domain objects
3. Provide runtime in-memory access to curve data
4. Persist/load optimized package artifacts

These map to the following crates:

- `lithos-core`
- `lithos-parser`
- `lithos-table`
- `lithos-package`
- `lithos-cli`

---

# Overview of Responsibilities

| Crate | Responsibility |
|------|----------------|
| core | Canonical LAS domain model |
| parser | Import raw LAS text into the canonical model |
| table | Runtime in-memory curve access and manipulation |
| package | Optimized storage (`metadata.json + curves.parquet`) |
| cli | Command-line tooling |

---

# lithos-core

`core` contains the canonical LAS domain model and shared types.

This crate defines **what LAS means in Lithos**.

Examples of structures:

- `LasFile`
- `VersionInfo`
- `WellInfo`
- `CurveInfo`
- `ParameterInfo`
- `IndexInfo`
- `SectionItem`
- error types
- validation result types

Example:

```rust
pub struct LasFile {
    pub version: VersionInfo,
    pub well: WellInfo,
    pub curves: Vec<CurveInfo>,
    pub parameters: Vec<ParameterInfo>,
    pub other: Option<String>,
    pub samples: CurveTable,
}
```

Responsibilities:

- canonical LAS semantics
- shared domain types
- validation rules
- error types

Things **not** allowed here:

- raw LAS parsing
- Arrow/Parquet storage logic
- CLI commands

---

# lithos-parser

`parser` is responsible for reading LAS files and converting them into `LasFile`.

Responsibilities:

- raw text parsing
- wrapped/unwrapped handling
- encoding detection
- section parsing
- tolerant parsing behavior
- duplicate mnemonic resolution
- parse diagnostics

Typical components:

```
lexer.rs
sections.rs
ascii_data.rs
importer.rs
diagnostics.rs
```

Output:

```
LAS text → LasFile
```

Things **not** allowed here:

- Parquet writing
- package schema logic
- CLI formatting

---

# lithos-table

`table` contains the runtime sampled data layer.

This crate powers the application-facing table abstraction (`CurveTable`).

Responsibilities:

- column access by mnemonic
- index slicing
- stacked curve access
- mutation helpers
- window queries
- statistics helpers (optional)

Example API:

```rust
table.curve("GR")
table.index()
table.slice_by_index(min, max)
```

Future evolution:

- Arrow-backed storage
- ML-friendly access
- vectorized operations

Things **not** allowed here:

- LAS grammar parsing
- package serialization
- CLI logic

---

# lithos-package

`package` defines the optimized storage format used by Lithos.

Package format:

```
well_123.laspkg/
  metadata.json
  curves.parquet
```

Responsibilities:

- package schema definition
- metadata JSON serialization
- Parquet read/write
- package loading
- schema versioning
- validation

Example workflow:

```
LasFile → package writer → metadata.json + curves.parquet
```

or

```
package → package loader → LasFile + CurveTable
```

This crate contains the logic that bridges domain objects and storage formats.

---

# lithos-cli

The CLI crate provides command-line utilities built on top of the SDK.

The CLI should **not contain business logic**.
It should orchestrate functionality from other crates.

Example commands:

```
cargo run -- import <input.las> <package_dir>
cargo run -- inspect-file <input.las>
cargo run -- summary <package_dir>
cargo run -- list-curves <package_dir>
```

Command responsibilities:

| Command | Description |
|-------|-------------|
| import | Convert LAS → package |
| inspect-file | Print LAS metadata |
| summary | Print package metadata |
| list-curves | List curves in a package |

Future commands may include:

- validation
- conversion utilities
- batch import
- schema inspection

---

# Dependency Direction

Dependencies should flow downward toward the domain layer.

```
parser  ──► core
table   ──► core
package ──► core
package ──► table
cli     ──► parser / package / core / table
```

Key rule:

**core should not depend on other crates.**

---

# Suggested Workspace Layout

```
crates/
  lithos-core
  lithos-parser
  lithos-table
  lithos-package
  lithos-cli

docs/
  architecture/
  ADRs/

examples/
testdata/
```

Optional future crates:

```
lithos-python
lithos-ops
```

---

# Data Lifecycle in Lithos

Typical workflow:

```
LAS file
   ↓
parser
   ↓
LasFile (canonical model)
   ↓
CurveTable (runtime access)
   ↓
package writer
   ↓
metadata.json + curves.parquet
```

Package loading path:

```
metadata.json + curves.parquet
   ↓
package loader
   ↓
LasFile + CurveTable
```

---

# Why This Structure Matters

Benefits:

- clear separation of concerns
- easier testing
- simpler dependency graph
- safer evolution of storage formats
- stable public API

This architecture allows Lithos to evolve storage implementations
without breaking the domain model.

---

# Key Design Philosophy

Lithos follows several guiding principles:

- Domain-first APIs rather than storage-format APIs
- Storage formats are implementation details
- Simple, inspectable artifacts
- Clear separation of parsing, runtime access, and persistence
- Strong Rust ergonomics

---

# Summary

The Lithos architecture divides responsibilities across five crates:

| Crate | Purpose |
|------|---------|
| core | canonical LAS semantics |
| parser | LAS file ingestion |
| table | runtime curve access |
| package | optimized persistence |
| cli | tooling interface |

This layered structure ensures Lithos remains maintainable as the
project grows into a full subsurface data SDK.
