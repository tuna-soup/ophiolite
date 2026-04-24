# TraceBoost As A Reference Consumer

`traceboost-demo` is the flagship first-party consumer of `Ophiolite Charts`.

It should behave like a demanding external customer of the chart SDK, not like a shortcut around the public boundary.

## Ownership split

### Ophiolite Charts owns

- chart-native rendering
- viewport behavior
- wrapper APIs
- chart-family model boundaries
- adapter entrypoints for Ophiolite-shaped data

### TraceBoost owns

- workflow state
- project/session state
- transport and backend calls
- product-specific diagnostics
- app-level orchestration and persistence

## Why this matters

If TraceBoost reaches through the public packages and starts depending on lower-level internals directly, it weakens the same SDK boundary that external consumers would rely on.

If TraceBoost consumes the public package surfaces honestly, it becomes a strong reference consumer that hardens the SDK in realistic workflows.

## Practical consumption order

The intended consumption model is:

```text
TraceBoost app shell
  -> @ophiolite/charts
  -> @ophiolite/charts/adapters/ophiolite when needed
  -> preview surfaces only by explicit opt-in
```

Not:

```text
TraceBoost app shell
  -> charts-core / renderer / domain internals as the default integration path
```

## Why this is useful beyond TraceBoost

This pattern keeps the SDK honest:

- the public story is exercised by a real product-shaped app
- the charts repo can learn from real workflows without turning every app concern into a chart concern
- package manifests and support tiers become meaningful rather than decorative
