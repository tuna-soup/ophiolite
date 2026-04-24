# ADR-0036: tbvolc v1 Padded-Payload Contract

## Status

Accepted

## Context

`tbvolc` exists as an exact compressed sibling to active `tbvol`.

The intended product shape is already clear:

- `tbvol` remains the hot working store
- `tbvolc` is an exact compressed storage/archive sibling
- processing/runtime code should not be forced into a compressed-first execution model

The remaining ambiguity was narrower than that broader product decision.

`tbvolc` index entries record:

- `offset`
- `length`
- `stored_ci`
- `stored_cx`

That could be read in two different ways:

1. the payload itself may be clipped to the effective logical edge span
2. the payload always expands to the full padded tile shape, and `stored_ci` / `stored_cx` are only validation metadata about the effective edge coverage

The current runtime implementation already follows the second model:

- writers compress full padded tile payloads
- readers decompress full padded tile payloads
- `stored_ci` / `stored_cx` are checked against effective tile coverage, but not used to define a shorter decoded payload shape

Without an explicit architectural decision, `tbvolc` would appear to promise a looser format contract than the runtime actually supports.

## Decision

`tbvolc` v1 is defined as a compressed serialization of padded `tbvol` tiles.

The durable rules are:

- every `tbvolc` amplitude payload in v1 expands to the full padded tile shape implied by `tile_shape`
- `stored_ci` and `stored_cx` record the effective logical edge span for validation and diagnostics only
- `stored_ci` / `stored_cx` do not mean the payload itself is clipped
- `tbvolc` v1 remains exact-lossless only
- `tbvolc` v1 remains an archive/storage sibling format, not a separate compute-native tile contract
- runtime preview/materialization code may continue to treat `tbvol` as the hot compute substrate without supporting a distinct clipped-payload decode path for `tbvolc`

If Ophiolite later wants true clipped edge payloads, that is a format change and requires a new `tbvolc` version rather than a silent reinterpretation of v1 metadata.

## Why

This decision matches the implementation that already exists and keeps the storage contract honest.

The wrong shape for v1 would be:

- index metadata that implies variable logical payload shape
- decoders and writers that still assume full padded payloads
- future callers guessing whether `stored_ci` / `stored_cx` are semantic payload-shape fields or merely checks

That would make the format harder to validate and easier to misuse.

The right shape for v1 is:

- one padded payload contract
- one clear role for edge-span metadata
- one explicit path for future evolution through a format-version bump

This keeps `tbvolc` aligned with the existing `tbvol` tile model and avoids creating a second storage contract accidentally.

## Consequences

### Accepted consequences

- `tbvolc` readers and writers can stay simple in v1 because they operate on full padded tile buffers
- edge-span metadata remains useful for archive validation, diagnostics, and compatibility checks
- `tbvolc` archive compatibility checks can fail closed when the recorded effective spans do not match the geometry-implied edge spans
- the product story stays consistent with the current runtime story: archive/store sibling first, compressed-input execution later and separately if needed

### Explicit non-goals

- no true clipped edge-payload decoding in `tbvolc` v1
- no promise that `stored_ci` / `stored_cx` can be used as variable decoded tile dimensions in v1
- no silent extension of `tbvolc` v1 into a more flexible payload contract without a format bump
- no compression-driven redesign of `tbvol` as the hot compute store

## Implementation Shape

The intended v1 shape is:

```text
tbvol padded tile
  -> exact compression
  -> tbvolc payload
  -> exact decompression
  -> same padded tile shape

stored_ci / stored_cx
  -> validation metadata
  -> diagnostics / compatibility checks
  -> not decoded payload dimensions
```

That means:

- `tile_shape` is the payload-shape contract
- `stored_ci` / `stored_cx` are the effective logical-coverage contract

## Success Criteria

This decision is working when:

- `tbvolc` readers reject index metadata that implies a different effective edge span than the geometry requires
- `tbvolc` readers and writers continue to operate on full padded tile buffers in v1
- docs and code describe the same contract
- future work that wants clipped payloads is forced to make an explicit versioned decision instead of relying on ambiguous v1 behavior

## Follow-on Documents

- `ADR-0034-canonical-processing-identity-debug-and-compatibility-surface.md`
- `processing-lineage-cache-compatibility-policy.md`
- `../storage/TBVOL_EXACT_COMPRESSED_STORAGE_PROPOSAL.md`
