# TraceBoost Desktop Security Hardening 2026

## Summary

This note documents a security hardening pass on the TraceBoost desktop app and adjacent Ophiolite runtime surfaces. The core change is a trust-model correction: the renderer is now treated as an untrusted client for high-risk filesystem and execution flows, rather than as a fully trusted local admin surface.

The goal was not to make a hostile local machine "safe". That is not realistic for a desktop app running under a user account. The goal was to remove broad ambient authority from routine app flows, reduce blast radius if the renderer or a dependency is compromised, and stop several avoidable arbitrary-path and arbitrary-execution behaviors from remaining part of the product default.

## Threat Model

The hardening work assumed the following attacker classes:

- A malicious file, dataset, or vendor project import source.
- A compromised frontend dependency, renderer foothold, or webview exploit.
- Another local process running as the same user.
- An internal or future automation layer that forwards app-facing requests with insufficient validation.

The most important assets were:

- Runtime stores and derived outputs.
- Project roots and project metadata.
- Support bundles, logs, and local path information.
- Python operator environments and vendor bridge execution surfaces.

The most important trust boundaries were:

- Svelte renderer to Tauri backend.
- Backend command layer to local filesystem.
- App flows to Python/operator/vendor execution.
- Persisted workspace state to later privileged actions.

## What Changed

### 1. Session-scoped handles replaced raw path authority

The desktop app now uses backend-issued session handles for runtime stores and project roots, plus one-shot grants for output destinations.

This changes the security property of the Tauri boundary:

- Before: the renderer could present raw paths directly for many high-risk operations.
- After: the backend requires a previously granted handle or a fresh output grant for the protected flows.

This matters because a compromised renderer should not be able to turn arbitrary strings into broad filesystem authority.

### 2. Output writes now require purpose-bound grants

Export and processing output flows now consume one-shot grants tied to a specific purpose such as:

- runtime store output
- gather store output
- SEG-Y export
- Zarr export

The grant is consumed once and rejected if reused or presented for the wrong purpose. This blocks a class of destructive overwrite flows where a renderer-supplied absolute path could previously trigger deletion or replacement of arbitrary local content.

### 3. Managed paths are explicitly scoped

For app-managed store roots, the backend can authorize paths only when they live under approved app-controlled directories. This keeps "normal app state" and "user granted exception" as separate concepts, which is important for reviewing future changes.

### 4. Workspace persistence now fails closed

Workspace registry and session state are now written as signed documents with a checksum over the payload. On load:

- unsupported formats are rejected
- checksum mismatches are rejected
- corrupted state is not silently deleted and recreated

In addition, restart behavior now clears session-scoped path authority:

- active store path is cleared
- project root is cleared
- accepted native-engineering store paths are cleared

This prevents persisted session files from becoming an authority cache that survives restarts.

### 5. Diagnostics are safer by default

Diagnostics were tightened in two ways:

- frontend-supplied log content is sanitized and bounded
- support bundles redact sensitive local paths unless the exporter explicitly opts in

This reduces both accidental disclosure and log-forgery style confusion during support or incident review.

### 6. Python operators were downgraded from product-trusted to developer-only

Installing and executing external Python operators is now blocked unless both of these are true:

- the build enables `unsafe-developer-mode`
- the runtime environment explicitly enables developer mode

This does not sandbox Python execution. It does something more immediate and important for the product surface: it stops unsigned, arbitrary Python package execution from being a normal release-mode behavior.

### 7. Vendor bridge execution was locked down

Caller-selected runtime probe executables and bridge executables are now rejected outside developer mode, and in-product vendor bridge execution is disabled outside that mode as well.

This removes a direct path from app-facing request data to attacker-chosen local process execution in normal product flows.

### 8. Dependency hygiene was tightened

The docs surfaces were upgraded away from the previously identified vulnerable Astro range, and CI now runs dependency audit checks for both Rust and Bun surfaces. This is not a substitute for design hardening, but it closes a known supply-chain gap and makes regressions more visible.

## Security Impact

The most important improvement is not a single bug fix. It is the removal of ambient trust from the desktop boundary.

The app now has stronger answers to these questions:

- Can the renderer directly name arbitrary privileged filesystem targets?
  - Much less than before for the protected flows.
- Can a previously granted output destination be replayed for a different action?
  - No, grants are one-shot and purpose-bound.
- Can session files silently reassert path authority after restart?
  - No, restart clears those path-bearing fields.
- Can arbitrary Python and vendor bridge execution happen in normal product mode?
  - No, those surfaces are now developer-only.

## What This Does Not Solve Yet

This hardening pass does not provide true sandboxing for local execution. That remains a future milestone.

In particular:

- developer mode still permits dangerous execution paths by design
- Python operators are gated, not isolated
- vendor execution is gated, not isolated
- a sufficiently privileged local attacker can still act as the user

The correct interpretation is:

- release-mode defaults are materially safer
- the renderer/backend trust boundary is meaningfully tighter
- several high-risk attack paths are now removed from ordinary operation
- deeper containment is still future work

## Recommended Next Steps

The next security steps should be:

1. Add a full command inventory for the remaining Tauri surface and classify every filesystem-bearing command as:
   - handle-bound
   - safe helper
   - developer-only
   - needs redesign
2. Add regression tests around the new authority model:
   - expired handles rejected
   - raw paths rejected where handles are required
   - output grants consumed once
   - wrong-purpose grants rejected
   - corrupted workspace/session state rejected
3. Move Python operator and vendor execution into a sandboxed worker model instead of relying only on product gating.
4. Preserve the trust-boundary decision in ADR form so later features do not accidentally reintroduce renderer ambient authority.

## Closing View

The practical outcome of this work is that TraceBoost desktop is no longer assuming that "local app" means "fully trusted UI". That assumption is convenient, but it is also how desktop apps accumulate quiet critical security debt.

The hardening pass reduced that debt in concrete ways:

- less arbitrary path authority
- less replayable destructive authority
- less implicit trust in persisted state
- less default trust in third-party executable code

That is the right direction for a local-first engineering product expected to run in real environments instead of ideal ones.
