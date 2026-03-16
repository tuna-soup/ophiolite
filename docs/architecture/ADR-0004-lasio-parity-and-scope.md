# ADR-0004: lasio Parity Scope

## Status

Accepted

## Decision

`lithos` targets semantic parity with `lasio` for non-v3 LAS read/model behavior.

In scope:

- parser behavior for common LAS 1.2/2.0/2.1 cases
- header-line handling
- encodings
- null policies
- mnemonic handling
- read/model/query helpers
- local fixture-based examples

Out of scope:

- LAS 3
- live URL/GitHub fetching
- pandas/dataframe behavior
- writer/export parity as a first-class target

## Why

- the goal is a strong Rust LAS SDK and glue layer, not a Python clone
- non-v3 parity captures the high-value read/model behaviors without overextending the first implementation

## Consequences

- parity docs and tests should remain explicit about what “parity” means
- new scope expansions should be captured in future ADRs rather than silently broadening the target
