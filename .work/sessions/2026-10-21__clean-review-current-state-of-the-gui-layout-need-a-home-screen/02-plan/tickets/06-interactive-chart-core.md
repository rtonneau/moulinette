# Ticket 06: interactive-chart-core

**Model:** heavy
**Model (Jev):** opus-5.5 (confidence 0.62)
**Effort:** high

**Acceptance Criteria:**
- [ ] A new module (e.g. `src/chart.rs`) renders a `plotters` line chart into an in-memory RGBA buffer (via `BitMapBackend::with_buffer`) given an axis range, and exposes it as a `slint::Image` property.
- [ ] Mouse-wheel scroll over the chart area narrows/widens the visible axis range around the cursor position (zoom); click-drag pans the visible range; both trigger a re-render of the buffer.
- [ ] Hovering shows a tooltip (as a Slint `Text` positioned near the cursor) with the nearest data point's exact x/y values, computed from the cursor's pixel position mapped back through the current axis-range-to-pixel transform.
- [ ] The `.lst` Analyze screen's rate-vs-time view uses this module for its in-window display; the existing `rate_vs_time.png`/per-ROI CSV file outputs on Run are unchanged and unaffected by the in-window view's pan/zoom state.
- [ ] A test exercises the pixel-to-data and data-to-pixel mapping functions directly (not through simulated mouse events) with known axis ranges and buffer dimensions, asserting round-trip accuracy within floating-point tolerance.

**Files to Touch:**
- `src/chart.rs` (new)
- `src/lib.rs`
- `ui/main.slint`
- `src/gui.rs`

**Verification Step:**

Run:
```bash
cargo test chart
```
Expected: the pixel/data mapping round-trip test passes.

**Notes:**

This is the riskiest ticket in the plan (see Risks & Mitigation) — keep the first implementation to a single line series (the existing rate-vs-time data) before Ticket 7 generalizes it to multiple series (violin) and a different chart type (spectrum with shaded ROI bands). Reuse the existing `plot::rate_vs_time` chart-building code (axis setup, series styling) as a reference for layout, re-targeting its `ChartBuilder` calls from a file-backed `BitMapBackend` to a buffer-backed one.
