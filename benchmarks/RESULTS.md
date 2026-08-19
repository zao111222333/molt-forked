# Molt 0.5.0 microbenchmark results

Measured on 2026-08-19 on `aarch64-apple-darwin` (macOS 15.0.1) with
`rustc 1.95.0`. Both revisions were built with `--release`; one warm-up run was
discarded, then each benchmark was run seven times and the median was retained.
Each run uses the benchmark harness's 1,000 inner iterations.

The baseline is the unmodified 0.4.5 `HEAD`; the candidate is Molt 0.5.0. Lower
is better. Times are nanoseconds per evaluated script.

| Benchmark | 0.4.5 | 0.5.0 | Change |
| --- | ---: | ---: | ---: |
| Empty return | 164 | 37 | -77.4% |
| Return one argument | 155 | 40 | -74.2% |
| Return two arguments | 158 | 45 | -71.5% |
| Identity command | 163 | 47 | -71.2% |
| Variable increment | 311 | 175 | -43.7% |
| Variable update | 228 | 81 | -64.5% |
| Procedure call | 951 | 599 | -37.0% |
| Arithmetic expression | 822 | 460 | -44.0% |
| List serialization | 1,793 | 658 | -63.3% |
| Dictionary serialization | 3,629 | 1,135 | -68.7% |
| List join | 1,101 | 173 | -84.3% |
| Subcommand dispatch | 248 | 96 | -61.3% |

No measured target regressed; all candidate medians improved by at least 37%.

## Molt 0.6.0 slim runtime

Measured on the same host and toolchain on 2026-08-19. The exact 0.5.0 `HEAD` and the 0.6.0
candidate were built with `--release`; both were warmed before measurement, run in alternating
order seven times, and compared by median. These runs use 10,000 inner iterations to reduce
timer quantization at the current sub-100 ns command costs. The slim candidate was built with
`--no-default-features`; the full column uses the same checkout with `--features full`. Lower is
better, and the change column compares the default embedding profile with 0.5.0.

| Benchmark | 0.5.0 | 0.6.0 slim | 0.6.0 `full` | Slim change |
| --- | ---: | ---: | ---: | ---: |
| Empty return | 38 | 37 | 36 | -2.6% |
| Return one argument | 40 | 40 | 40 | 0.0% |
| Return two arguments | 45 | 43 | 43 | -4.4% |
| Identity command | 47 | 42 | 42 | -10.6% |
| Variable increment | 170 | 132 | 135 | -22.4% |
| Variable update | 81 | 72 | 71 | -11.1% |
| Procedure call | 650 | 633 | 588 | -2.6% |
| Arithmetic expression | 505 | 510 | 500 | +1.0% |
| List serialization | 661 | 682 | 648 | +3.2% |
| Dictionary serialization | 1,212 | 1,245 | 1,193 | +2.7% |
| List join | 189 | 197 | 176 | +4.2% |
| Subcommand dispatch | 97 | 101 | 107 | +4.1% |

No target shows a sustained regression above the 5% release threshold. Syntax highlighting is
measured separately from execution; the execution path does not collect syntax-analysis tokens.

### Release artifact sizes

Raw release artifacts from the same checkout, before stripping or `wasm-opt`:

| Artifact | Slim | `full` | Full overhead |
| --- | ---: | ---: | ---: |
| Native `moltsh` (aarch64 macOS) | 1,510,992 B | 1,763,280 B | +16.7% |
| WASM demo (`wasm32-unknown-unknown`) | 1,951,438 B | 2,129,560 B | +9.1% |

The demo's `full` feature selects `StandardLibrary::Full`; its default build selects
`StandardLibrary::Slim`. These figures include the Yew demo application and are not the size of
the core interpreter in isolation.
