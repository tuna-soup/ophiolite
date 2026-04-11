---
name: operator
description: Add or modify a shared seismic operator family across Ophiolite and TraceBoost, including scope taxonomy, contracts, runtime kernels, app plumbing, frontend exposure when applicable, and validation. Use for trace-local operators, prestack gather-native operators, or adjacent analysis APIs that must stay separate from materializing operators.
---

# Operator

Implement operators in the shared stack, not in the UI.

## Ownership

- Put canonical seismic contracts in `C:\Users\crooijmanss\dev\ophiolite\crates\ophiolite-seismic\src\contracts\`.
- Put operator-family contracts in `C:\Users\crooijmanss\dev\ophiolite\crates\ophiolite-seismic\src\contracts\processing.rs`.
- Put authored-model contracts in `C:\Users\crooijmanss\dev\ophiolite\crates\ophiolite-seismic\src\contracts\models.rs`.
- Put request-response and action contracts in `C:\Users\crooijmanss\dev\ophiolite\crates\ophiolite-seismic\src\contracts\operations.rs`.
- Put resolved display payloads in `C:\Users\crooijmanss\dev\ophiolite\crates\ophiolite-seismic\src\contracts\views.rs`.
- Put stable domain descriptors in `C:\Users\crooijmanss\dev\ophiolite\crates\ophiolite-seismic\src\contracts\domain.rs`.
- Keep trace-local validation and kernels in `C:\Users\crooijmanss\dev\ophiolite\crates\ophiolite-seismic-runtime\src\compute.rs`.
- Treat trace-local pipelines as canonical step lists. `TraceLocalProcessingPipeline` now owns `steps: Vec<TraceLocalProcessingStep>`, and checkpoint/output-save semantics live on `step.checkpoint`, not in request-time or UI-only sideband index lists.
- Keep post-stack geometry-changing derivation families such as subvolume crop as dedicated contracts and runtime entry points; do not hide them inside `TraceLocalProcessingOperation`.
- Put gather-native or section-matrix runtime paths in dedicated Ophiolite runtime modules instead of forcing them into the trace-local executor.
- For prestack gather-native work, prefer dedicated store/runtime APIs such as `ingest_prestack_offset_segy`, `open_prestack_store`, `preview_gather_processing_view`, `materialize_gather_processing_store`, and analysis helpers like `velocity_scan` with optional autopick output instead of stretching the post-stack `tbvol` APIs.
- Treat `C:\Users\crooijmanss\dev\TraceBoost\app\traceboost-app\src\lib.rs` and `C:\Users\crooijmanss\dev\TraceBoost\app\traceboost-frontend\src-tauri\src\lib.rs` as app-shell plumbing.
- Treat TraceBoost contract crates as compatibility surfaces over the Ophiolite taxonomy:
  - `seis-contracts-core::{domain, processing, models, operations, views}`
  - `seis-contracts-views::{section, gather}`
  - `seis-contracts-interop::{datasets, import_ops, processing_ops, workspace, resolve}`
- Expose operators in the current TraceBoost frontend only for live post-stack families the app already owns, currently trace-local processing plus terminal subvolume crop, from `C:\Users\crooijmanss\dev\TraceBoost\app\traceboost-frontend\src\lib\processing-model.svelte.ts` and `C:\Users\crooijmanss\dev\TraceBoost\app\traceboost-frontend\src\lib\components\PipelineOperatorEditor.svelte`.
- Keep packed binary transport shapes out of canonical operator contracts. The current section transport adapters live in `C:\Users\crooijmanss\dev\TraceBoost\app\traceboost-frontend\src\lib\transport\packed-sections.ts` and are re-exported by `bridge.ts` for compatibility.
- If the feature is prestack gather-native and there is no owning app yet, stop at Ophiolite contracts/runtime/tests plus shared contracts export; do not invent TraceBoost UI for it.
- Touch GeoViz only when the feature needs a reusable chart or reusable plot interaction primitive. Do not put operator math there.

## Decision Rule

- Determine scope first in `C:\Users\crooijmanss\dev\ophiolite\crates\ophiolite-seismic\src\contracts\`.
- Use explicit operator families, not one flat operator surface.
- The current live `TraceLocal` family is:
  - `amplitude_scalar`
  - `trace_rms_normalize`
  - `agc_rms`
  - `phase_rotation`
  - `lowpass_filter`
  - `highpass_filter`
  - `bandpass_filter`
  - `volume_arithmetic`
- The current live geometry-changing post-stack family is:
  - `SubvolumeProcessingPipeline` with terminal `crop`
- If the feature transforms traces into traces and remains per-trace, keep it in the shared trace-local family.
- If the feature needs a second compatible post-stack volume but still returns trace-to-trace output on unchanged geometry, keep it trace-local and validate the external input explicitly.
- If the feature changes output geometry, extents, or volume shape, make it a dedicated post-stack derivation family instead of stretching trace-local contracts.
- If the operator needs gather context, make it a gather-native family and keep the API gather-centric.
- If the operator needs section context, make it a section-matrix family.
- If the feature changes output type, prefer a separate analysis request/response contract instead of forcing it into a materializing operator pipeline.
- Current live prestack analysis example: offset-gather `VelocityScanRequest` / `VelocityScanResponse` producing a `SemblancePanel` and optional `VelocityFunctionEstimate`.
- If the operator is inverse-wavelet or deconvolution-like, keep it in a separate inverse-wavelet family with its own assumptions and validation.
- If the operator is trace-local, prefer `ProcessingLayoutCompatibility::AnyTraceMatrix` unless the math truly depends on a narrower layout.
- For phase-one prestack gather-native materializing operators, prefer offset-gather semantics first and validate accordingly.
- Keep product gating in TraceBoost UI, not in Ophiolite contracts.

## Runtime Strategy

- For simple per-trace amplitude operators, use direct loops in `compute.rs`.
- For trace-local moving-window gain operators such as `agc_rms`, reuse per-worker scratch buffers in `TraceComputeState`; do not allocate per trace.
- For spectral trace-local operators, reuse the existing `rayon` trace-parallel path plus shared spectral workspaces in `compute.rs`.
- Reuse per-worker FFT plans and buffers with `try_for_each_init`; do not allocate FFT buffers per trace.
- For geometry-changing post-stack derivations, derive output metadata first, then map output tiles back to source coordinates and apply any optional trace-local prefix before writing.
- When trace-local checkpoints are involved, emit visible intermediate stores from the canonical step flags and keep cache/reuse identity based on the operation prefix, not the checkpoint booleans themselves.
- For gather-native prestack operators, build a separate gather executor and preview/materialize path; do not route gathers through section preview helpers.
- Keep phase-one prestack storage explicit. Post-stack `tbvol` and prestack `tbgath` are different runtime paths.
- Keep prestack analysis explicit too. Velocity scans, semblance panels, picked velocity functions, spectra, and similar outputs are analysis APIs over `tbgath`, not fake operators.
- Preserve prestack layout and gather-axis metadata through gather-native materialization unless the operator family explicitly defines a domain change.

## Cross-Repo Workflow

1. Add or update the operator family contracts in Ophiolite.
2. Keep family boundaries explicit in ids, scope labels, compatibility, and request/response types.
3. Add runtime validation in the executor module that owns that family.
4. Implement the kernel in the owning runtime module.
5. Add runtime tests for validation, scope/compatibility, preview/materialize parity, and numerical behavior.
6. Regenerate shared TypeScript contracts with `cargo run -p contracts-export` from `C:\Users\crooijmanss\dev\TraceBoost`.
7. Update TraceBoost app-shell slugging or exhaustive Rust matches only if the current shell exposes the feature.
8. Update TraceBoost frontend catalogs/editors only for the live post-stack families the app already owns, and keep geometry-changing steps explicit and terminal when the family requires it.
9. If no current UI owns the feature, stop at contracts/runtime/app-shell plumbing plus validation.

## Validation

Run the minimum pass:

```bash
cd C:\Users\crooijmanss\dev\ophiolite && cargo check -p ophiolite-seismic
cd C:\Users\crooijmanss\dev\TraceBoost && cargo run -p contracts-export
cd C:\Users\crooijmanss\dev\ophiolite && cargo test -p ophiolite-seismic-runtime
cd C:\Users\crooijmanss\dev\TraceBoost && cargo test -p traceboost-app
cd C:\Users\crooijmanss\dev\TraceBoost && cargo check -p traceboost-desktop
cd C:\Users\crooijmanss\dev\TraceBoost\app\traceboost-frontend && bun run typecheck
```

If the change is Ophiolite-only because the operator family has no current app owner, stop after the relevant Ophiolite tests and contracts export.

## Pitfalls

- Do not implement operator math in TraceBoost frontend.
- Do not put product-specific UI state into Ophiolite contracts.
- Do not reintroduce sideband checkpoint arrays in requests, presets, or workspace session objects when the canonical trace-local pipeline already carries step-level checkpoint flags.
- Do not treat every seismic transform as a trace-local operator.
- Do not cram geometry-changing crop/subset behavior into `TraceLocalProcessingOperation`.
- Do not model a geometry-changing family as arbitrary reorderable steps if the runtime semantics are terminal-only; keep that restriction explicit in contracts and UI.
- Do not encode analysis outputs like spectra or wavelets as `ProcessingOperation` if they do not return trace data.
- Do not force prestack processing through section-native request types. Use gather-native requests and views.
- Do not wire a future prestack family into the current TraceBoost frontend just to prove the backend exists.
- Do not forget generated TypeScript contracts.
