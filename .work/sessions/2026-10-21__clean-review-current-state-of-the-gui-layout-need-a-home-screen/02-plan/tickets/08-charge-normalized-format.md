# Ticket 08: charge-normalized-format

**Model:** heavy
**Model (Jev):** opus-5.5 (confidence 0.35)
**Effort:** high

**Acceptance Criteria:**
- [ ] A new data structure (e.g. `gui_state::ChargeCalibration { monitor_adc: u8, counts_per_uc: Option<f64> }`) and a new `Format::ChargeNormalized` (or equivalent) conversion format divides each channel's counts by the chosen monitor ADC/spectrum's total counts over the run.
- [ ] When `counts_per_uc` is `None`, the output is the raw monitor-count ratio (dimensionless); when set, each value is additionally scaled to counts-per-µC.
- [ ] If the monitor ADC/spectrum recorded zero total counts, the function returns a structured error ("monitor ADC recorded no counts") instead of dividing by zero.
- [ ] The Convert screen gains a collapsible "Charge normalization" section (hidden unless "Charge-normalized (µC)" is the selected output format) with a monitor-ADC dropdown (populated from the loaded file's ADCs/spectra, `.mpa` directly or `.lst` via the Ticket 2/3 histogram) and an optional counts-per-µC text field.
- [ ] Unit tests cover: a known spectrum/monitor pair producing the expected ratio with `counts_per_uc: None`; the same pair with a calibration value producing the expected µC-scaled output; a monitor with zero total counts producing the structured error.

**Files to Touch:**
- `src/conversion.rs`
- `src/gui_state.rs`
- `ui/main.slint`
- `src/gui.rs`

**Verification Step:**

Run:
```bash
cargo test charge_normalized
```
Expected: all new charge-normalization unit tests pass.

**Notes:**

Depends on Tickets 1 and 3 (the `.lst`-via-histogram path this format also applies to). See ADR `.work/adr/0002-monitor-adc-charge-proxy.md` for why monitor-ADC counts stand in for charge and why the calibration is always user-supplied.
