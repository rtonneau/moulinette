# Ticket 02: roi-and-time-binned-rate

**Status:** ✅ Done

## Local Test Result

`cargo test rate::` (target-dir redirected to the session scratch dir): `test result: ok. 7 passed; 0 failed`.
Also ran the full `cargo test`, `cargo fmt --all -- --check`, and `cargo clippy --all-targets -- -D warnings`: all clean after the review fixes below.

## Review Notes

Subagent (Sonnet/medium) implemented `src/rate.rs` and `src/lib.rs` (`pub mod rate;`). Checked each Acceptance Criterion: `Roi`/`from_channels`/`from_energy` signatures match exactly; `from_energy` inverts the linear calibration and returns `MissingCalibration`/`UnsupportedCalibration` per the ticket's exact "present and non-zero" wording for `calfact2`/`calfact3`; `time_binned_rate` signature and `RateBin` fields match; `roi: None` counts every value, `roi: Some` restricts to `[start_channel, end_channel]` inclusive; the rate/rate_err formulas match. Verified by hand that `adc` is deliberately 1-based (matching `AdcHeader::adc_index`'s convention from ticket 1), mapped to the 0-based `Event::adc_values` bit position via `adc - 1` — a reasonable, clearly documented design choice consistent with ticket 1. All 7 required/extra test cases (binning without ROI, with ROI, `end_ms: None` "through last event", known-calibration resolution, missing calibration, non-zero `calfact2`, zero `calfact2`/`calfact3` allowed) checked against hand-computed expectations and pass.

Found after the subagent's handoff: `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings` were not clean — a few lines needed rustfmt's wrapping, and `clippy::manual_div_ceil` fired on `(duration + dt_ms - 1) / dt_ms`. Fixed inline: ran `cargo fmt --all`, then replaced the manual ceiling-division with `duration.div_ceil(dt_ms)`. Re-ran fmt/clippy/full test suite clean after the fix.

## Blockers / Challenges

None.

## Commits

- 1dc972c feat: resolve ROIs and compute time-binned acquisition rate (ticket 02)

## Time Spent

5m (ticket-start.js to ticket-complete.js)
