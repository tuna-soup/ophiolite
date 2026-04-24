# Recipes

These recipes sit between `Start Here` and the per-family examples.

Use them when you already understand the basic public boundary and need guidance for a specific integration pattern.

## Available recipes

- [embedding-launch-charts.md](./embedding-launch-charts.md)
  Default launch-path embedding guidance using `@ophiolite/charts`.
- [ophiolite-adapters.md](./ophiolite-adapters.md)
  How to keep Ophiolite DTO adaptation explicit instead of leaking transport concerns into wrappers.
- [traceboost-reference-consumer.md](./traceboost-reference-consumer.md)
  How `traceboost-demo` should consume the chart SDK and what that says about ownership boundaries.

## Relationship to examples

Use recipes for cross-cutting guidance.

Use `docs/examples/*.md` for family-specific simple and production examples.

The intended order is:

1. [../start-here.md](../start-here.md)
2. a relevant recipe from this folder
3. a family `simple` example
4. the matching `production` example

Use [../../manifests/generated/module-catalog.json](../../manifests/generated/module-catalog.json) when you need the current checked package and entrypoint map rather than prose guidance.
