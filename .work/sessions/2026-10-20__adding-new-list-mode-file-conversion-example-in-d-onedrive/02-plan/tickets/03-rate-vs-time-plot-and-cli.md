# Ticket 03: rate-vs-time-plot-and-cli

**Model:** heavy
**Model (Jev):** opus-5.5 (confidence 0.19)
**Effort:** high

**Acceptance Criteria:**
- [ ] `plotters` is added to `[dependencies]` in `Cargo.toml`, unconditionally (not gated behind the `gui` feature).
- [ ] `plot::rate_vs_time(series: &[(String, Vec<RateBin>)], output_path: &Path) -> Result<(), PlotError>` renders a PNG (via `plotters`' `BitMapBackend`) with one labeled line per named series on a single shared y-axis; x-axis labeled "Acquisition Time [s]", y-axis labeled "Acquisition Rate [Yield/s]".
- [ ] `write_rate_csv(bins: &[RateBin], output_path: &Path) -> Result<(), io::Error>` writes a CSV with header `time_s,rate,rate_err,counts` and one row per bin.
- [ ] `moulinette list <input.lst> --roi NAME:ADC:LOW:HIGH[:unit] --dt-ms N --output-dir DIR` is a new subcommand in `src/main.rs`: `--roi` is repeatable (at least one required), `unit` selects `ch` (channel bounds) or `kev` (energy bounds, default), `--dt-ms` defaults to `15000` when omitted, `--output-dir` is required. It parses the input file once, computes one `RateBin` series per `--roi`, writes one CSV per series plus one combined rate-vs-time PNG to `DIR`, and never overwrites an existing output file (reusing `workflow`'s existing refusal pattern).
- [ ] `cargo build --release --no-default-features --bin moulinette` still succeeds.
- [ ] `README.md`'s "dependency-free CLI-only build" claim is corrected to state that `plotters` is now always a dependency of the CLI-only build.
- [ ] Unit tests in `main.rs`'s existing `mod tests` style cover `list`'s argument parsing (valid invocation, missing `--roi`, malformed `--roi` value, duplicate `--output-dir`), mirroring the existing `convert` option tests.
- [ ] An integration test runs `list` against a small synthetic `.lst` fixture (built with `lst::` or a committed minimal fixture built the same way as ticket 1's unit-test fixtures) and asserts the written CSV's exact contents against a hand-computed expectation.

**Files to Touch:**
- `Cargo.toml`
- `src/plot.rs` (new)
- `src/main.rs`
- `README.md`

**Verification Step:**

Run:
```bash
cargo test && cargo build --release --no-default-features --bin moulinette
```
Expected: all tests pass (including the new `list` CLI and `plot::rate_vs_time`/CSV tests); the `--no-default-features` build succeeds.

**Notes:**

Violin-plot output is out of scope here (ticket 4). If `list`'s argument parser needs a placeholder for the violin-related `--window` flag to avoid a breaking change in ticket 4, accepting-and-ignoring it is acceptable, but do not render anything from it yet. Depends on `RateBin`/`time_binned_rate`/`Roi` from ticket 2 and `ListFile`/`lst::parse` from ticket 1.
