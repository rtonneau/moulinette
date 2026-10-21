# Ticket 06: interactive-chart-core

**Status:** ✅ Done

## Local Test Result

`cargo test chart` — 8 passed, 0 failed (pixel/data/element/buffer mapping round-trips, zoom-keeps-cursor-fixed, pan, hover-nearest, polyline clipping, and a home-range-matches-PNG check against `plot::rate_vs_time`'s own pixel geometry). Re-ran independently, plus the full `cargo test` suite (92 passed, 0 failed, no regressions) and `cargo clippy --all-targets -- -D warnings` (one error found and fixed on review — see below — then clean).

## Review Notes

Implemented by an Opus subagent (model hint: heavy/high); the dispatch hit a session-wide Opus rate limit mid-task ("wiring in `src/gui.rs`") but its work on disk was already complete and self-consistent, so I verified and finished the ticket directly rather than resuming the subagent.

Checked against the Acceptance Criteria: `src/chart.rs` (new, 689 lines) renders a `plotters` line chart into an in-memory RGB8 buffer via `BitMapBackend::with_buffer`, exposed as `slint::Image` through `RgbImage`/`to_slint_image`; `LineChart` owns a `ViewRange`/`PlotArea` and the data series, with `zoom`/`pan`/`reset` mutating the view and `render` re-drawing the buffer. `rate_vs_time` in `src/plot.rs` was left untouched — file-backed PNG/CSV output is unchanged — and `chart::LineChart::rate_vs_time` is a separate, buffer-backed constructor built from the same `rate_series` data, with a test (`rate_chart_home_range_matches_png_and_render_sets_area`) pinning its home view to the same axis range `plot::rate_vs_time` uses. `gui_state::list_mode_run` was factored into `rate_series(file, params) -> Result<Vec<(String, Vec<RateBin>)>, String>` (used by both `list_mode_run` and the new GUI chart wiring) plus `list_mode_write`, without changing `list_mode_run`'s signature or breaking its existing test. In `ui/main.slint`, a `TouchArea` (`rate-chart-area`) reports resize/scroll/pointer events through five new callbacks (`rate-chart-resized/zoom/pan/hover/reset`) to `src/gui.rs`, which holds the live `chart::LineChart` in `State::rate_chart`, re-renders on each event, and shows a `Text` tooltip positioned near the cursor from `nearest()`'s exact data-space x/y. Mapping between the `TouchArea`'s logical pixels and the chart's buffer pixels goes through `chart::element_to_buffer`/`buffer_to_element`, tested directly (`element_buffer_scaling_round_trips`) alongside `pixel_to_data`/`data_to_pixel` (`pixel_data_mapping_round_trips_with_known_ranges`, `chart_mapping_matches_plotters_own_pixels`).

One issue found on review and fixed: `clip_segment`'s return type (`Option<((f64, f64), (f64, f64), bool, bool)>`) tripped `clippy::type_complexity` under this project's `-D warnings` gate. Replaced it with a small private `ClippedSegment { start, end, start_is_original, end_is_original }` struct and updated its one call site in `visible_runs`; no behavior change, confirmed by the unchanged `chart_clips_lines_to_the_view` test still passing. No other changes made after review.

## Blockers / Challenges

The dispatched subagent hit an Opus session-wide rate limit partway through (reported mid-"wiring in `src/gui.rs`"); its prior work was already complete on disk and compiled cleanly, so the ticket was finished by independent review rather than by resuming the subagent.

## Commits

- 85fdc5b feat: interactive rate-vs-time chart with zoom/pan/hover (ticket 06)

## Time Spent

5h 12m (ticket-start.js to ticket-complete.js)
