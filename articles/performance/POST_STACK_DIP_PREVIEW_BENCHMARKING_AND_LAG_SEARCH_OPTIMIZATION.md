# Post-Stack Dip Preview Benchmarking And Lag-Search Optimization

## Purpose

This note records:

- what the focused `Dip` preview benchmark on the large F3 smoke `tbvol` actually showed
- why the existing neighborhood prefix cache is not the main performance lever for `Dip`
- what comparable public implementations suggest about longer-term dip optimization directions
- what low-risk kernel optimization was applied to the current Ophiolite lag-search implementation
- what the rerun benchmark showed after that change

This is an engineering note, not a product claim.

## Evidence level

The benchmark evidence in this note is `development`, not `authoritative`.

Reasons:

- the benchmark family is an ignored test harness in `apps/traceboost-demo/src-tauri`
- the machine was interactive and visibly busy during measurement
- browser, WebView, Codex, Node, and Bun processes were active during the rerun

The reruns are still useful because they are like-for-like comparisons on:

- the same code path
- the same dataset
- the same benchmark entrypoint
- the same benchmark binary
- the same section indices
- the same `3`-iteration median reporting shape

## Benchmark family and entrypoint

Benchmark family:

- ignored desktop preview-session harness
- `apps/traceboost-demo/src-tauri/src/preview_session_bench.rs`

Focused entrypoint:

```powershell
target\debug\deps\traceboost_desktop_lib-854fac91ccf07999.exe `
  --ignored --nocapture `
  benchmark_desktop_preview_session_dip_profile_large_f3
```

Run conditions:

- `RUST_TEST_THREADS=1`
- same compiled test binary before and after the optimization rerun
- no other heavy benchmark job running in parallel

## Dataset

The benchmark used:

- `H:\traceboost-bench\f3_dataset-smoke.tbvol`

This is the large F3 smoke store already used by the preview-session benchmark harness.

The relevant section sizes for interpretation are:

- inline preview section: `951` traces
- xline preview section: `651` traces

That matters because a large part of the inline/xline timing difference is simply more traces per section, not a pathological axis-specific bug.

## Benchmark question

The focused question was:

> does cached trace-local prefix reuse materially improve `Dip` preview time, or is the `Dip` kernel itself the dominant cost?

The benchmark isolates two variants:

- `dip_balanced_no_prefix`
- `dip_balanced_bandpass_prefix`

and measures:

- `desktop_stateless_preview`
- `desktop_session_repeat`
- `desktop_session_prefix_edit`

## Baseline finding before the kernel change

Before the lag-search optimization, the focused dip benchmark showed:

- `Dip` with no prefix and `Dip` with a cached `Bandpass` prefix had very similar runtimes
- repeated session preview was only marginally faster than stateless preview
- the prefix cache was functioning, but the total wall-clock was still dominated by the `Dip` operator itself

Representative medians before the optimization:

| Case | Median ms |
| --- | ---: |
| inline `dip_balanced_no_prefix` stateless | `16901.747` |
| inline `dip_balanced_bandpass_prefix` stateless | `16868.975` |
| xline `dip_balanced_no_prefix` stateless | `11421.959` |
| xline `dip_balanced_bandpass_prefix` stateless | `11634.679` |

The practical conclusion was:

- the prefix cache is correct and worth keeping
- but `Dip` is not prefix-bound
- so more prefix-cache work is unlikely to produce major `Dip` wins

## What the current Ophiolite `Dip` implementation does

The current `Dip` preview path in:

- `crates/ophiolite-seismic-runtime/src/post_stack_neighborhood.rs`

computes a dip estimate by:

1. taking a center trace and sample
2. visiting neighboring traces in the requested neighborhood
3. scanning a range of vertical lags for each neighbor
4. selecting the lag with the highest local normalized cross-correlation
5. fitting inline and xline dip components from those lag observations

The important point is that this is a brute-force local lag search with repeated short-window cross-correlation.

That is semantically clear, but it is expensive because the hot loop multiplies:

- samples
- neighbors
- lag candidates
- cross-correlation window work

## What public/reference implementations suggest

The local public references do not point toward “more aggressive prefix caching” as the main optimization strategy.

They point toward different dip-estimation algorithms.

### OpendTect

OpendTect exposes multiple dip families that are not the same as this brute-force lag-search kernel.

Examples in the local clone:

- `C:\Users\crooijmanss\dev\OpendTect\include\Algo\dippca.h`
  - dip/azimuth from a PCA-based method
- `C:\Users\crooijmanss\dev\OpendTect\plugins\AttribExp\expvardip.h`
  - minimum-variance dip probing/scanning

That is a useful signal:

- OpendTect’s open dip surfaces are algorithmically richer than simple local lag scanning
- if Ophiolite later wants a larger speed/quality jump, the likely branch is a new dip-estimation family, not more micro-optimizing the current prefix path

### Madagascar

Madagascar also points away from brute-force lag scanning for slope/dip estimation.

Examples in the local clone:

- `C:\Users\crooijmanss\dev\src\user\chen\fbpwd.c`
  - omnidirectional plane-wave-destruction dip estimation
- `C:\Users\crooijmanss\dev\src\user\chen\fbdip.c`
  - dip estimation by linear phase filter bank

This reinforces the same conclusion:

- production/open research code often uses predictive, filter-bank, or tensor-like formulations
- these methods attack dip estimation more directly than repeated local lag scans

## Low-risk optimization that was applied

The immediate goal was not to change dip semantics.

So the optimization kept the current algorithm and only removed repeated work from the hot loop.

Changes applied in:

- `crates/ophiolite-seismic-runtime/src/post_stack_neighborhood.rs`

### Optimization details

For each center sample, the code now:

- computes the center gate bounds once
- slices the center window once
- computes the center-window L2 norm once
- reuses that center window and norm across all lag candidates for all neighbor comparisons at that sample

The lag-search helper was also reshaped so that each lag candidate only needs to:

- derive the neighbor window offset
- slice the neighbor window
- compute the neighbor-side norm and dot product

instead of repeatedly:

- recomputing center gate bounds
- re-slicing the center window
- recomputing center-window energy

This is a semantics-preserving optimization.

The dip definition did not change.

## Rerun results after the lag-search optimization

Focused rerun medians:

| Case | Before ms | After ms | Delta |
| --- | ---: | ---: | ---: |
| inline no-prefix stateless | `16901.747` | `15718.055` | `-7.00%` |
| inline no-prefix session repeat | `16893.526` | `15994.060` | `-5.32%` |
| xline no-prefix stateless | `11421.959` | `10877.739` | `-4.76%` |
| xline no-prefix session repeat | `11552.375` | `10621.973` | `-8.05%` |
| inline prefixed stateless | `16868.975` | `16130.033` | `-4.38%` |
| inline prefixed session repeat | `16782.164` | `16338.550` | `-2.64%` |
| xline prefixed stateless | `11634.679` | `11630.608` | `-0.03%` |
| xline prefixed session repeat | `11558.452` | `10957.014` | `-5.20%` |

The exact rerun output also still shows the same high-level behavior:

- inline is slower than xline because inline sections contain more traces on this dataset
- prefix caching still exists, but it is not the dominant lever for `Dip`

## Interpretation

The rerun supports four conclusions.

### 1. The low-risk kernel cleanup was worth doing

It produced a real gain, roughly:

- `4%` to `8%` on most measured paths

That is enough to keep.

### 2. The main `Dip` cost is still the dip kernel itself

Even after the optimization:

- `Dip` preview is still in the `10.6s` to `16.3s` range on the focused F3 smoke runs
- those runtimes are far larger than any plausible savings from a single cached `Bandpass` prefix

So the main cost center remains:

- local lag scanning
- per-neighbor cross-correlation work
- per-sample dip fitting

### 3. Prefix caching is correct but secondary for `Dip`

The prefix cache still matters for mixed pipelines and for cheaper neighborhood operators.

It is just not the main `Dip` lever.

For `Dip`, the runtime is dominated by the terminal operator.

### 4. Bigger future gains likely require an algorithm branch

The public references suggest that meaningful next-step speedups may require moving beyond this specific lag-search formulation.

Candidates for a later branch include:

- PCA / tensor-style dip estimation
- minimum-variance dip estimation
- plane-wave-destruction slope estimation
- filter-bank-based dip estimation

Those are design choices, not incremental micro-optimizations.

## Recommended next steps

### Short-term

- keep the current lag-search optimization
- do not spend more time on prefix-cache tuning for `Dip`
- if more performance work is needed inside the same semantics, profile:
  - lag loop cost
  - neighbor loop cost
  - section/matrix preview duplication

### Medium-term

Prototype one algorithmic alternative in a separate operator or experimental branch:

- `DipPca`
- `DipMinVariance`
- `LocalSlopePwd`

That should be benchmarked separately rather than silently replacing the current `Dip`.

### Benchmarking discipline

If these numbers are going to drive a default or a product claim, rerun them under `authoritative` conditions:

- machine as idle as practical
- no browser or local-agent contention
- fixed benchmark binary
- same F3 smoke store
- single benchmark workload at a time

## Files changed in this round

- `crates/ophiolite-seismic-runtime/src/post_stack_neighborhood.rs`
  - lag-search inner-loop optimization
- `apps/traceboost-demo/src-tauri/src/preview_session_bench.rs`
  - focused `Dip` profile benchmark entrypoint
- `traceboost/runtime/src/lib.rs`
  - adaptive recommendation re-export needed by the benchmark build path
- `traceboost/app/traceboost-app/src/lib.rs`
  - small compatibility fix in imports needed to rebuild the benchmark path cleanly
