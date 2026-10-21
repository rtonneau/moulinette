# Ticket 07: interactive-violin-and-spectrum

**Model:** heavy
**Model (Jev):** opus-5.5 (confidence 0.42)
**Effort:** high

**Acceptance Criteria:**
- [ ] The `.lst` Analyze screen's violin (time-window) view uses the Ticket 6 chart module for its in-window display, with the same pan/zoom/hover behavior; the existing per-ROI violin PNG file outputs on Run are unchanged.
- [ ] Ticket 5's `.mpa` spectrum-with-ROI-bands view is upgraded from the static placeholder to the Ticket 6 chart module, with ROI bands remaining visible as shaded regions through pan/zoom.
- [ ] Hover tooltips over the spectrum view show the channel/energy and count at the cursor; hover tooltips over the violin view show the group label and density value at the cursor.
- [ ] A test confirms the violin and spectrum views each render through the shared Ticket 6 module (e.g. asserting both construct the same underlying chart-buffer type) rather than duplicating the pan/zoom/hover logic.

**Files to Touch:**
- `src/chart.rs`
- `ui/main.slint`
- `src/gui.rs`
- `src/gui_state.rs`

**Verification Step:**

Run:
```bash
cargo test --bin moulinette-gui && cargo test roi_statistics
```
Expected: all chart-module and ROI-statistics tests still pass with no regressions.

**Notes:**

Depends on Tickets 5 and 6 being committed. If the violin's polygon-fill rendering or the spectrum's shaded bands don't fit the line-chart-shaped module from Ticket 6 cleanly, extend `src/chart.rs` with a second render function rather than forking a parallel pan/zoom/hover implementation.
