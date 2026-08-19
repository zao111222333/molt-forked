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
