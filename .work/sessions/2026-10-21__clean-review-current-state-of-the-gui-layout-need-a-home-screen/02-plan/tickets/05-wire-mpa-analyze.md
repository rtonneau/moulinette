# Ticket 05: wire-mpa-analyze

**Model:** medium
**Model (Jev):** sonnet-5.5 (confidence 0.36)
**Effort:** medium

**Acceptance Criteria:**
- [ ] On the Analyze screen, loading an `.mpa` file replaces the "Not supported yet" message (Ticket 1) with an ROI editor (reusing the existing row UI: name, ADC, low, high, unit — time-window fields stay hidden, since they're `.lst`-only) and a results table showing each ROI's gross/net counts, centroid and width from Ticket 4.
- [ ] The spectrum itself renders as a static (pre-interactive; Ticket 7 upgrades this) `plotters` chart with each configured ROI's channel range shown as a shaded band.
- [ ] Running the analysis is gated by the same `busy` flag and worker-thread pattern as today's `.lst` analysis and `.mpa` conversion (no concurrent runs).
- [ ] A GUI test (following the existing `list_mode_panel_switches_and_edits_rows` pattern) loads a synthetic `.mpa` fixture on the Analyze screen, adds an ROI row, and asserts the stats table reflects Ticket 4's computed values.

**Files to Touch:**
- `ui/main.slint`
- `src/gui.rs`
- `src/gui_state.rs`
- `tests/gui_workflow.rs`

**Verification Step:**

Run:
```bash
cargo test mpa_analyze
```
Expected: the new GUI test passes, and existing `.lst` Analyze tests (`list_mode_panel_switches_and_edits_rows`) still pass unmodified.

**Notes:**

Depends on Tickets 1 and 4 being committed. The ROI editor UI element is shared markup with the `.lst` panel's ROI rows; only the time-window section and the results table's columns differ.
