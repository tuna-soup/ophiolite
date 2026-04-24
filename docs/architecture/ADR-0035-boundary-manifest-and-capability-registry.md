# ADR-0035: Boundary Manifest and Capability Registry

## Status

Accepted

## Context

The public SDK direction is now clearer than it was a few months ago, and the
first boundary-enforcement layer is now implemented in source rather than ADR
prose alone.

Before this ADR was implemented:

- `public-sdk-package-matrix.md` explained intended boundaries, but it was not
  a machine-readable source of truth
- `crates/ophiolite-sdk` was a handwritten facade with no repo-level check that
  kept adapter-only compatibility shims out of the public-core promise
- `traceboost/contracts/*` were intentionally compatibility surfaces, but their
  status was not enforced alongside package-class ownership
- `apps/traceboost-demo` had a frontend import guard, but Rust path
  dependencies, desktop command ownership, and shell-level coupling were not
  checked the same way
- capability discovery existed in several local forms:
  - CLI operation catalogs
  - operator catalogs
  - project operator locks
  - import-provider registries
  - chart-family registries

That shape is workable during rapid iteration, but it is weaker than the
examples studied in `ParaView` and `GDAL`.

Those upstreams both separate:

- durable owned vocabulary
- discovery metadata
- activation/loading policy
- first-party application assembly

The mechanisms differ, but the common lesson is the same: the boundary must be
expressed in source in a form that tooling can validate.

## Decision

`ophiolite` now uses two shared architecture primitives:

1. a machine-readable workspace boundary manifest
2. a built-in capability registry model that separates discovery, validation,
   and activation

### Boundary Manifest

The workspace boundary source of truth lives under:

```text
[workspace.metadata.ophiolite.boundaries]
```

The initial schema records:

- boundary schema version
- package classes
- class-level dependency rules
- package-to-class assignments
- exposure/publication intent
- named anchor crates such as the public surface facade and capability registry

The first package classes are:

- `platform_core`
- `platform_support`
- `contract_shared`
- `adapter_compat`
- `control_surface`
- `app_support`
- `app_shell`
- `repo_tooling`
- `internal_compat`

This metadata is not only documentation. It is now the current input for:

- Cargo-metadata boundary validation via `scripts/ophiolite-boundary-check`
- package-class ownership review and dependency-rule enforcement
- desktop command-boundary policy review alongside app-local transport seams

Follow-on uses remain in scope for later work:

- generated or validated `ophiolite-sdk` exports
- generated or validated compatibility-surface allowlists
- generated chart SDK surface catalogs and support-tier views

### Capability Registry

The shared capability vocabulary lives in `crates/ophiolite-capabilities`.

It records:

- capability kind
- capability source
- availability state
- validation state
- activation state
- stability
- load policy
- isolation mode
- host compatibility
- bindings
- contracts
- artifacts
- typed detail payloads per capability kind

The first capability kinds are:

- `operator`
- `import_provider`
- `chart_adapter`
- `workflow_action`

The durable source categories are:

- `built_in`
- `optional_package`
- `app_local`

The critical rule is:

- discovery is allowed to say a capability exists
- validation is allowed to say whether it is compatible
- activation/loading is a separate step and must not be implied by discovery

That rule applies equally to:

- operator packages
- TraceBoost import providers
- future chart/runtime extensions

## Why

This decision addresses three recurring failure modes:

1. the repo can describe a boundary without enforcing it
2. the repo can discover a capability only by partially activating it
3. the app shell can become the accidental owner of platform semantics

The boundary manifest solves the first problem.

The capability registry solves the second.

Together they make the third problem easier to detect, because shell-local code
must either:

- fit into an allowed package class and dependency rule
- or register as an explicit `app_local` capability

That is a better shape than allowing shell behavior to become ambient,
undocumented platform surface area.

The same rule applies to shell command boundaries:

- TraceBoost desktop commands may consume canonical contracts and shared capability records
- the command table itself remains adapter-local
- transport names and shell-specific workflow glue do not become public platform surface area unless they are promoted deliberately into a platform-owned control surface

## Consequences

### Accepted consequences

- package ownership and dependency rules are now architecture data, not only ADR
  prose
- `ophiolite-sdk` remains the public-core facade, and its boundary is now
  partially enforced by the workspace manifest plus the repo checker even though
  export generation remains follow-on work
- compatibility crates remain valid, but their status becomes explicit and
  reviewable
- capability discovery can converge across operator catalogs, import-provider
  registries, and future extension seams
- first-party apps such as TraceBoost are allowed to register `app_local`
  capabilities, but they do not become canonical merely because they exist

### Explicit non-goals

- no full dynamic plugin-loading framework yet
- no immediate removal of `traceboost/contracts/*`
- no immediate auto-generation of desktop handlers/backend dispatch
- no claim that every current registry in the repo is already modeled by the new
  capability crate

## Initial Implementation Shape

```text
workspace.metadata.ophiolite.boundaries
  -> package class rules
  -> package assignments
  -> boundary enforcement in scripts/ophiolite-boundary-check
  -> future codegen hooks

crates/ophiolite-capabilities
  -> shared capability vocabulary
  -> discovery records
  -> validation + activation lifecycle records
  -> typed detail payloads

existing sources
  -> operator package manifests
  -> built-in operator catalogs
  -> import-provider registries
  -> chart adapter registries
  -> chart package manifests and support-tier records
  -> workflow operation catalogs

current adapters
  -> CapabilityRegistry
  -> CapabilityLifecycleRegistry
  -> validated activation paths
  -> app-local command-boundary manifest
  -> generated bridge stubs
```

## Implementation Status

The first implementation wave is complete:

1. the workspace boundary manifest now lives in the root `Cargo.toml`
2. the shared capability vocabulary crate now lives in `crates/ophiolite-capabilities`
3. Cargo/package boundary validation now runs through `scripts/ophiolite-boundary-check`
4. TraceBoost import-provider discovery now flows through the shared capability registry
5. TraceBoost import-provider activation now uses explicit validation and
   activation lifecycle records
6. the TraceBoost desktop command table is now declared and validated from
   `apps/traceboost-demo/desktop-command-boundary.json`
7. the frontend bridge now uses generated desktop bridge stubs instead of
   handwritten command-name strings
8. the chart SDK now validates package/module manifests from `charts/manifests/*`
   and generates a stable module catalog consumed by charts public docs

The first-pass Cargo/package validator now lives in
`scripts/ophiolite-boundary-check` and can be run with:

```text
cargo run -p ophiolite-boundary-check
```

The first-pass TraceBoost desktop command-boundary validator now lives in:

```text
node scripts/validate-traceboost-command-boundary.mjs
```

Its policy source of truth lives in:

```text
apps/traceboost-demo/desktop-command-boundary.json
```

The generated frontend bridge stubs derived from that policy/backend command table now come from:

```text
node apps/traceboost-demo/scripts/generate-desktop-bridge-stubs.mjs
```

The chart-module manifest validator now lives in:

```text
bun scripts/validate-module-manifests.ts
```

Its generated outputs now include:

```text
charts/manifests/generated/module-catalog.json
charts/apps/public-docs/src/lib/generated/manifest-catalog.ts
```

Remaining follow-on work under this ADR is intentionally narrower:

1. adapt built-in operator discovery into the shared capability registry
2. decide whether chart/runtime extension seams should reuse the same lifecycle
   model directly or through thin adapters
3. generate or validate selected public-surface exports from the boundary
   manifest once the publishable core hardens further
4. decide how far chart renderer/backend lifecycle and telemetry should converge
   with the shared capability vocabulary versus remaining chart-local contracts

## First Hardening Target

The first extension seam to harden is the TraceBoost import-provider/session
boundary, because it already separates:

- provider discovery
- session initialization
- validation state
- activation side effects

That makes it the cleanest proving ground for the capability registry before the
same pattern is applied to operator packages and later chart/runtime extensions.

That hardening is now live in the first app-local seam:

- discovery comes from shared capability records
- validation records whether a discovered capability is activation-ready
- session start activates the concrete provider implementation
- failed activation returns the capability to an explicit non-active state

## Related Documents

- `ADR-0032-processing-authority-and-thin-client-migration.md`
- `ADR-0033-public-sdk-core-and-adapter-boundaries.md`
- `public-sdk-package-matrix.md`
- `traceboost-import-provider-registry-sketch.md`
- `ADR-0030-unified-operator-catalog-and-seismic-first-class-registry.md`
