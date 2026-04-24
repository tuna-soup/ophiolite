# Ophiolite Adapters

Use this recipe when your application already has Ophiolite contract DTOs and you want to feed them into the public chart wrappers without collapsing the public boundary.

## The rule

Keep Ophiolite-specific decoding at the adapter boundary.

Do this:

```ts
import { adaptOphioliteSectionViewToSeismicSectionData } from "@ophiolite/charts/adapters/ophiolite";
```

Then pass the resulting neutral chart model into `@ophiolite/charts`.

## Why this boundary matters

Without an explicit adapter boundary:

- wrapper props drift toward transport-layer naming
- public examples start teaching Ophiolite internals
- non-Ophiolite consumers see the SDK as an internal frontend package instead of a reusable chart product

With an explicit adapter boundary:

- the public wrapper story stays readable
- Ophiolite integration remains supported
- TraceBoost can consume the same public packages an external consumer would use

## Consumption order

1. Start with `@ophiolite/charts`.
2. Add `@ophiolite/charts/adapters/ophiolite` only when the app input is already an Ophiolite DTO.
3. Keep `@ophiolite/contracts` and transport logic out of the default public examples.

## What this recipe does not mean

It does not mean Ophiolite is unimportant.

It means the Ophiolite-specific step should stay visible and bounded so the chart SDK remains legible on its own.
