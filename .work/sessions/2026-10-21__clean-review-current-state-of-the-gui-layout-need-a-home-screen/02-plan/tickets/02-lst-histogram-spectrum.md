# Ticket 02: lst-histogram-spectrum

**Model:** medium
**Model (Jev):** sonnet-5.5 (confidence 0.54)
**Effort:** medium

**Acceptance Criteria:**
- [ ] A new function (e.g. `gui_state::histogram_list_file` or a new `lst`/`conversion`-adjacent module) takes a parsed `ListFile` and produces one channel-count spectrum per `AdcHeader`, counting each event's channel value (clamped to `[0, n_channels)`) into its ADC's histogram.
- [ ] The produced spectrum shape is accepted directly by the existing `conversion::convert`/`workflow::render` path (same struct or a thin adapter), with no change to the converter's output format logic.
- [ ] Events with no value for a given ADC (`adc_values[i] == None`) are excluded from that ADC's histogram, not counted as channel 0.
- [ ] Unit tests cover: an ADC with a known small set of events producing the exact expected per-channel counts; an event whose channel equals `n_channels - 1` (upper boundary included); an ADC with zero events producing an all-zero histogram of the declared length.

**Files to Touch:**
- `src/gui_state.rs` (or a new `src/histogram.rs` added to `src/lib.rs`'s module list)
- `src/lst.rs` (only if `ListFile`/`AdcHeader` need a new accessor)

**Verification Step:**

Run:
```bash
cargo test histogram
```
Expected: all new histogram unit tests pass.

**Notes:**

Reuses the synthetic `.lst` fixture pattern already in `gui_state.rs`'s `#[cfg(test)]` module (`synthetic_lst`) as a starting point for the new tests. This ticket is library-only; no GUI wiring yet (Ticket 3).
