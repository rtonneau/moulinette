# Ticket 03: wire-lst-convert

**Model:** medium
**Model (Jev):** sonnet-5.5 (confidence 0.55)
**Effort:** medium

**Acceptance Criteria:**
- [ ] On the Convert screen, loading a `.lst` file replaces the "Not supported yet" message (Ticket 1) with the existing converter panel, offering all current formats (XY, EY, 1CO, PIX, PIG, MON, TOF, 8CO) over the Ticket 2 histogram instead of an `MpaFile`.
- [ ] Per-ADC spectrum selection in the existing "Spectra to convert" dropdown lists the `.lst` file's ADCs (by `adc_index` and channel count), mirroring today's `.mpa` spectrum list.
- [ ] Existing output filename/overwrite-refusal/warnings behavior is unchanged for `.lst`-sourced conversions.
- [ ] An integration test loads a synthetic `.lst` fixture, selects an ADC, converts to XY format, and asserts the written file's contents match the histogram computed directly via Ticket 2's function.

**Files to Touch:**
- `src/gui.rs`
- `src/gui_state.rs`
- `tests/gui_workflow.rs`

**Verification Step:**

Run:
```bash
cargo test lst_convert
```
Expected: the new integration test passes, and existing `.mpa` conversion tests still pass unmodified.

**Notes:**

Depends on Tickets 1 and 2 being committed. The converter panel's UI itself does not change shape; only what backs it (an `MpaFile` spectrum vs. a Ticket 2 histogram) differs based on the loaded file's extension.
