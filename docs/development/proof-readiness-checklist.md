# Proof-Readiness Checklist

Use this checklist before calling a capability public, product-facing, or
evidence-backed. Keep entries short and link to the design note, issue, recipe,
report, benchmark, or test that proves each answer.

## Capability

- Name:
- Owner:
- Status:
- Scope:
- Proof artifact links:

## Checklist

| Area | Ready answer | Evidence link |
|---|---|---|
| Ownership | The capability is placed under Ophiolite, Ophiolite Charts, TraceBoost, or deferred ecosystem scope. | |
| Contracts or DTOs | Canonical contracts are Rust-owned where meaning is reusable; app-local DTOs stay app-local. | |
| Runtime owner | Runtime, adapter, chart, or app behavior has one clear owner and no parallel implementation. | |
| Control surfaces | CLI, Python, app, or chart controls are thin over the owning behavior and return the same evidence. | |
| Fixtures and manifests | Synthetic fixtures, curated public-data manifests, or opt-in real-data manifests cover the proof path. | |
| Warnings and blockers | Risky inputs produce explicit warnings or blockers, and the expected cases are tested or documented. | |
| Recipe integration | Workflow-level behavior has a typed recipe step or a documented reason it does not need one. | |
| Report integration | Workflow-level behavior writes durable JSON report evidence or links to a lower-level report artifact. | |
| Chart or view evidence | Visual behavior has a chart/view path, screenshot or regression coverage, and interaction evidence where needed. | |
| Benchmark mode | Performance-sensitive claims have benchmark mode, fixture, environment, repetitions, and raw result links. | |
| Documentation | User, developer, or architecture docs explain the capability without overstating unsupported claims. | |
| Validation | Automated tests, validation commands, or manual proof steps are listed with pass/fail expectations. | |

## Decision

- Proof-ready:
- Remaining blockers:
- Accepted gaps:
- Follow-up owner:
