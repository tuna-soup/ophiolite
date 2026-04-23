# Processing Authority Matrix

This document records the current and target authority model for Ophiolite processing migration work.

It exists to prevent a repeat of "shared in principle, app-local in practice".

The rule is simple:

- every concern should have one canonical owner
- every compatibility path should be marked temporary
- every frontend-only copy of a backend concept should either become derived or be removed

## Legend

- `Canonical`: the intended source of truth
- `Derived`: safe presentation or transport projection
- `Compatibility`: temporary migration path
- `Delete`: should be removed after cutover

## Matrix

| Concern | Current canonical owner | Current duplicate / competing owners | Target canonical owner | Current status | Migration note |
| --- | --- | --- | --- | --- | --- |
| Operator catalog vocabulary | `crates/ophiolite-operators` | none of consequence | `crates/ophiolite-operators` | Canonical | Keep as vocabulary-only crate |
| Structured compute operator definitions | `ophiolite-compute` and project wrapping | none of consequence | `ophiolite-compute` and project wrapping | Canonical | Keep family-owned |
| Seismic operator definitions, docs, parameter docs, availability rules | `crates/ophiolite-seismic/src/contracts/operator_catalog.rs` | frontend fallback operator definitions in `apps/traceboost-demo/src/lib/processing-model.svelte.ts` | `crates/ophiolite-seismic` | Mixed | Finish backend catalog parity, then delete frontend fallback ownership |
| Project-level operator discovery | `OphioliteProject::list_operator_catalog(...)` | frontend filtering/presentation logic | `OphioliteProject` plus family-owned availability logic | Canonical | Frontend may still filter for view concerns only |
| Shared processing pipeline contracts | shared Rust contracts plus generated TS | frontend helper aliases and local interface mirrors | shared Rust contracts plus generated TS | Mixed | Keep helpers derived, remove local mirrored DTO ownership |
| Processing authoring rules: create, duplicate, remove, normalize, preset compatibility, checkpoint legality | frontend `ProcessingModel` | workspace generic upsert path stores blobs without semantic ownership | backend processing authoring boundary | Mixed | Move rules into app-local Rust authoring module first |
| Workspace persistence of session pipelines | `WorkspaceState` storage | frontend builds and mutates canonical pipeline blobs directly | backend authoring boundary calling `WorkspaceState` | Mixed | Storage stays in workspace layer, semantic ownership moves out of frontend |
| Processing preset persistence and normalization | `apps/traceboost-demo/src-tauri/src/processing.rs` | frontend also manipulates preset-linked pipeline state | backend authoring boundary plus preset storage | Mixed | Backend should own preset-to-pipeline normalization rules |
| Output-path/signature derivation | frontend `processing-model.svelte.ts` plus backend defaults | multiple helper functions | backend authoring boundary | Mixed | Frontend should request resolved output naming, not define it |
| Execution-plan semantics | `crates/ophiolite-seismic-runtime` | none intended | `crates/ophiolite-seismic-runtime` | Canonical | Keep family-specific linear planning |
| Job and batch orchestration | `crates/ophiolite-seismic-execution` | frontend polling and state display | `crates/ophiolite-seismic-execution` | Canonical | Frontend may observe status, not redefine orchestration |
| Execution policy resolution | `crates/ophiolite-seismic-execution` | frontend user selections | `crates/ophiolite-seismic-execution` | Canonical | UI supplies intent, service resolves policy |
| Preview and run command transport | Tauri/backend commands | frontend bridge helpers | backend commands with generated TS contracts | Canonical + Derived | Bridge should remain transport-only |
| TS contract distribution | root `contracts/ts/ophiolite-contracts` and TraceBoost package exports | two active export paths | root `contracts/ts/ophiolite-contracts` | Mixed | TraceBoost package becomes compatibility distribution only |
| Frontend `DatasetOperator*` DTO interfaces | local TS definitions in `bridge.ts` | canonical Rust/TS exports | generated shared TS contracts | Compatibility | Remove local mirrors after frontend switch |
| Frontend processing UI state: selection, open panels, stale flags, keyboard shortcuts | frontend | none | frontend | Canonical | Keep in frontend |

## Immediate Hotspots

These are the highest-priority ownership violations to address first.

### 1. Frontend fallback operator definitions

Current file:

- `apps/traceboost-demo/src/lib/processing-model.svelte.ts`

Problem:

- labels, aliases, docs, search terms, and creation defaults are still partially frontend-owned

Target:

- backend catalog owns operator meaning
- frontend consumes catalog entries and only adds display formatting where necessary

### 2. Frontend-owned processing authoring semantics

Current files:

- `apps/traceboost-demo/src/lib/processing-model.svelte.ts`
- `apps/traceboost-demo/src/lib/viewer-model.svelte.ts`

Problem:

- the frontend still owns canonical session-pipeline lifecycle and mutation rules

Target:

- backend authoring module owns canonical rules
- frontend becomes a thin command client

### 3. Generic workspace save path as semantic owner by accident

Current files:

- `apps/traceboost-demo/src-tauri/src/workspace.rs`
- `apps/traceboost-demo/src/lib/viewer-model.svelte.ts`

Problem:

- processing session pipelines are persisted through a broad generic workspace update path
- that path is the storage boundary, but it should not be the semantic owner of processing authoring

Target:

- processing authoring boundary validates and normalizes first
- workspace layer persists already-canonical state

### 4. Two active TS contract distribution paths

Current files:

- `scripts/contracts-export/src/main.rs`
- `scripts/traceboost-contracts-export/src/main.rs`

Problem:

- both root and TraceBoost-oriented packages look active

Target:

- root package canonical
- TraceBoost-oriented package compatibility only during migration

## Migration Rules

Every migration PR should follow these rules:

1. add the new canonical path first
2. switch one consumer at a time
3. add parity tests before deleting old paths
4. mark compatibility paths explicitly in code and docs
5. delete only after one successful release-train window

## Exit Condition

This matrix is obsolete when every row marked `Mixed` or `Compatibility` has been reduced to one canonical owner plus optional derived presentation helpers.
