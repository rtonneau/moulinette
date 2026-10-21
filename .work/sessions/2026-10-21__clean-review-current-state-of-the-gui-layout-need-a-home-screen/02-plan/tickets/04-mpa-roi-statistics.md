# Ticket 04: mpa-roi-statistics

**Model:** medium
**Model (Jev):** sonnet-5.5 (confidence 0.56)
**Effort:** medium

**Acceptance Criteria:**
- [ ] A new function (e.g. `gui_state::roi_statistics`) takes an `MpaFile` spectrum and a list of `RoiSpec` (reusing the existing struct), and for each ROI returns: gross counts (raw sum over the ROI's channel range), net counts (sum minus a linear background drawn between the ROI's two edge channels, clamped at zero), weighted-mean centroid, and standard-deviation width.
- [ ] Energy-unit ROIs resolve through the spectrum's ADC calibration the same way `list_mode_run` already resolves them for `.lst` (reuse or mirror that resolution logic).
- [ ] Net counts never go negative (clamped at zero) even when the ROI's edge channels have unusually high counts relative to the interior.
- [ ] Unit tests cover: a flat-background synthetic ROI where net ≈ gross; a sloped-background synthetic ROI where net differs from gross by the known triangular-background area; a narrow/noisy ROI where the naive subtraction would go negative, asserting the clamp.

**Files to Touch:**
- `src/gui_state.rs`

**Verification Step:**

Run:
```bash
cargo test roi_statistics
```
Expected: all new ROI-statistics unit tests pass.

**Notes:**

This ticket is library-only; no GUI wiring yet (Ticket 5). See `.work/sessions/2026-10-21__clean-review-current-state-of-the-gui-layout-need-a-home-screen/01-grill/resume.md` for the moment-statistics/linear-background design decision (ADR not required for this specific formula, only for the separate monitor-ADC decision in Ticket 8).
