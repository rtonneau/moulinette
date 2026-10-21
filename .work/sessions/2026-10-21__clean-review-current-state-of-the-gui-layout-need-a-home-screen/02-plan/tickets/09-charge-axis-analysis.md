# Ticket 09: charge-axis-analysis

**Model:** heavy
**Model (Jev):** opus-5.5 (confidence 0.4)
**Effort:** high

**Acceptance Criteria:**
- [ ] The `.lst` Analyze screen gains an opt-in checkbox ("Enable charge-axis chart") that, when checked, shows the monitor-ADC dropdown and optional counts-per-µC field from Ticket 8's `ChargeCalibration`.
- [ ] When enabled, a new chart (rendered through the Ticket 6 module) plots each ROI's existing time-binned rate against the running monitor-ADC count (or µC, if calibrated) accumulated up to that time bin, instead of elapsed time.
- [ ] The existing rate-vs-time chart, CSV and PNG outputs are unaffected by enabling the charge-axis chart; it is purely an additional in-window view, not a new file output.
- [ ] A unit test computes the charge-axis series for a synthetic `.lst` fixture with a known monitor ADC and asserts each bin's x-value equals the monitor ADC's cumulative count up to that bin's end time.

**Files to Touch:**
- `src/rate.rs` (or `src/gui_state.rs`, wherever the series transform best fits alongside `time_binned_rate`)
- `ui/main.slint`
- `src/gui.rs`

**Verification Step:**

Run:
```bash
cargo test charge_axis
```
Expected: the new charge-axis series test passes, and existing `rate`/`list_mode_run` tests still pass unmodified.

**Notes:**

Depends on Tickets 6 and 8 being committed. This is the last ticket in the plan; after it, every Success Metric in the grill's resume should be satisfied.
