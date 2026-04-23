# TraceBoost Contract Style Guide

This guide defines the wire-shape policy for generated Rust -> TypeScript contracts used by TraceBoost.

## Core Rule

Do not mix these meanings accidentally:

- field missing
- field present with `null`
- field present with an empty collection like `[]` or `{}`

Only patch-like inputs are allowed to use omission as a semantic signal.

## Payload Classes

### Full-shape request/response

Use this for:

- normal create requests
- replace requests
- read responses
- job status / registry / view-model responses
- persisted documents unless a format is intentionally sparse

Rules:

- prefer stable field presence on the wire
- optional scalar/object values serialize as `null`
- collections serialize as `[]` or `{}` when empty
- do not use `skip_serializing_if` just to save bytes

### Sparse patch/filter request

Use this for:

- patch/update payloads
- filter/search payloads
- inputs where omission means “leave unchanged” or “unspecified”

Rules:

- omission is allowed and meaningful
- `null` only means “clear” when the domain explicitly supports that operation
- generated TS should model this explicitly with optional fields

### True tri-state field

Use this only when all three states mean different things:

- missing
- `null`
- concrete value

Rules:

- keep these rare
- document the three meanings inline in code
- prefer an explicit representation such as `Option<Option<T>>`

## Rust Annotation Policy

For migrated presence-stable DTOs:

- `Option<T>` fields should normally use `#[serde(default)]` and no `skip_serializing_if`
- collection fields should normally use `#[serde(default)]` and no `skip_serializing_if`
- do not use `#[ts(optional_fields)]` as a blanket escape hatch

For sparse DTOs:

- omission behavior must be intentional and reviewable
- field-level optionality is preferred over struct-wide optionality

## Generated TypeScript Policy

- generated TS is the backend wire contract source of truth
- handwritten bridge types may temporarily buffer migration churn
- migrated presence-stable DTOs should generate required properties with `| null` where appropriate
- sparse DTOs should generate optional properties

## Migration Order

Current migration strategy:

1. upgrade generator/tooling first
2. migrate actively consumed TraceBoost response/read DTOs first
3. add JSON-level tests for `null` and `[]`
4. add scoped audit enforcement
5. expand family-by-family

## Scoped Audit

Run the first-wave audit from the repo root:

```powershell
python .\scripts\validation\audit_contract_presence_policy.py
```

That script is intentionally scoped to the migrated structs. It should be expanded as more families are cleaned up.
