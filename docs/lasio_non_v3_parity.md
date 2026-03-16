# lasio Non-v3 Parity Matrix

This project targets semantic parity with `lasio` for read and in-memory model behavior on LAS 1.2, 2.0, and 2.1 fixtures. It does not attempt Python API syntax parity, network-backed examples, pandas dataframe behavior, or LAS 3 support.

## Implemented

| lasio area | lithos coverage |
| --- | --- |
| `test_read_header_line.py` | `tests/header_line_parity.rs` covers time values, colon disambiguation, dotted mnemonics, units beginning with `.`, units with spaces, and no-period lines |
| `test_api.py` | `tests/model_parity.rs`, `tests/read_parity.rs`, and `tests/input_parity.rs` cover key lookup, indexed access, mutation, replacement, stacking, file/string/reader inputs, and examples helpers |
| `test_stack_curves.py` | `tests/model_parity.rs` covers prefix stacking, explicit ordering, natural sorting, and missing-curve errors |
| `test_null_policy.py` | `tests/read_parity.rs` covers strict/none/aggressive behavior; parser supports custom null rules and read substitutions |
| `test_open_file.py` | `tests/input_parity.rs` covers `PathBuf`, file readers, and inline LAS strings |
| `test_examples.py` | `tests/read_parity.rs` plus `src/examples.rs` cover local example discovery/opening |
| `test_encoding.py` | `tests/encoding_parity.rs` covers UTF-8, UTF-8 BOM, UTF-16 autodetect/explicit, and Latin encoding fixtures |
| `test_enhancements.py` | `tests/enhancements_parity.rs` covers mnemonic case modes, depth-unit detection/conversion, non-standard sections, leading-zero identifiers, and tab-delimited data |

## Deferred

| lasio area | reason |
| --- | --- |
| Writer/export tests (`test_write.py`) | This crate still focuses on read/model parity and bundle generation, not structure-preserving LAS writing |
| Pandas dataframe assertions in `test_enhancements.py` | No dataframe layer exists in Rust; equivalent indexed tabular APIs would be a separate feature |
| JSON/writer-specific serialization parity | Current serde serialization covers the Rust model, not lasio’s writer/export semantics |

## Out of Scope

| lasio area | reason |
| --- | --- |
| LAS 3 tests | Explicitly rejected in the parser for now |
| URL/GitHub example loading | Local-only fixtures are used in this repo |

## Current Verification

Run:

```powershell
cargo test
```

The current suite exercises:

- header-line parsing parity
- wrapped and unwrapped reads
- duplicate and missing mnemonics
- null policy handling
- input-source handling
- encodings
- stack/query behavior
- mnemonic case behavior
- depth-unit inference/conversion
- non-standard section preservation
