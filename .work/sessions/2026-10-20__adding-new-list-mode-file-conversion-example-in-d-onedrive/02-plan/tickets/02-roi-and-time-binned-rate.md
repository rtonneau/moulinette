# Ticket 02: roi-and-time-binned-rate

**Model:** medium
**Model (Jev):** sonnet-5.5 (confidence 0.56)
**Effort:** medium

**Acceptance Criteria:**
- [ ] `Roi { adc: u8, start_channel: u32, end_channel: u32, name: String }` with `Roi::from_channels(adc, start, end, name) -> Roi` and `Roi::from_energy(adc_header: &AdcHeader, low_kev: f64, high_kev: f64, name: String) -> Result<Roi, RateError>`.
- [ ] `Roi::from_energy` resolves channel bounds using only the linear calibration terms (`energy = caloff + calfact * channel`); it returns `RateError::MissingCalibration` if `caloff`/`calfact` are absent, and `RateError::UnsupportedCalibration` if `calfact2` or `calfact3` is present and non-zero on the given `AdcHeader`.
- [ ] `time_binned_rate(events: &[Event], adc: u8, roi: Option<&Roi>, start_ms: u64, end_ms: Option<u64>, dt_ms: u64) -> Vec<RateBin>` where `RateBin { time_s: f64, rate: f64, rate_err: f64, counts: u64 }`; `end_ms: None` means "through the last event's `time_ms`".
- [ ] With `roi: None`, every recorded channel value for the given `adc` counts, regardless of value. With `roi: Some(r)`, only channel values within `[r.start_channel, r.end_channel]` (inclusive) count.
- [ ] Each bin's `rate = counts as f64 / (dt_ms as f64 / 1000.0)` and `rate_err = (counts as f64).sqrt() / (dt_ms as f64 / 1000.0)`; `time_s` is the bin's center time in seconds.
- [ ] Unit tests: a hand-built `Vec<Event>` with known timestamps/channel values binned at a known `dt_ms`, asserting the exact expected `Vec<RateBin>` both with and without an ROI restriction; an `Roi::from_energy` test against a known linear calibration asserting the exact resolved channel bounds; an `Roi::from_energy` test against an `AdcHeader` with non-zero `calfact2` asserting `RateError::UnsupportedCalibration`.

**Files to Touch:**
- `src/rate.rs` (new)
- `src/lib.rs` (add `pub mod rate;`)

**Verification Step:**

Run:
```bash
cargo test rate::
```
Expected: `test result: ok.` with all ROI-resolution and time-binning tests passing.

**Notes:**

This is a direct Rust port of `mpa3kit.parser.ListFileParser.time_binned_rate` and the linear term of `ADCInfo.energy`, scoped down per the approved grill design (no sparse-matrix path, no quadratic/cubic calibration — `mpa3kit`'s Python reference supports both but moulinette v1 deliberately does not). Depends on `ListFile`/`Event`/`AdcHeader` from ticket 1 (`src/lst.rs`).
