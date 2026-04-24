# Ophiolite Charts Module Manifests

This directory holds the small manifest layer for the charts workspace.

The goal is pragmatic:

- make package support tiers explicit
- keep public vs preview vs internal surfaces machine-readable
- document which packages are safe to teach to embedders
- give `traceboost-demo` and future checks one stable vocabulary to consume

This is not a VTK-sized module system. It is a thin catalog for the chart packages that matter to the public SDK story.

## Files

- `schemas/ophiolite-charts-module.schema.json`
  JSON schema for package manifests.
- `packages/*/ophiolite.module.json`
  One manifest per chart package in this first slice.

## Support Tiers

- `public-launch`
  Default documented surface for embedders.
- `public-adapter`
  Explicit integration helper, documented after the default launch path.
- `preview`
  Opt-in surface with narrower guarantees.
- `internal`
  Not part of the external compatibility promise.

## Validation

Run:

```bash
bun run validate:manifests
```

The validator checks:

- manifest shape
- package name alignment with `package.json`
- exported subpath alignment
- support-tier and dependency-role vocabulary
- basic public-boundary consistency rules

On success it also emits:

- `manifests/generated/module-catalog.json`
  Normalized manifest catalog for docs and future tooling.
- `apps/public-docs/src/lib/generated/manifest-catalog.ts`
  Typed mirror consumed by the public docs app.
