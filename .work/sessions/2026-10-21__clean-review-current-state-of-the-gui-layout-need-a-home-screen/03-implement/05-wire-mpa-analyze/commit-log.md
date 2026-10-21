# Ticket 05: wire-mpa-analyze

**Status:** ✅ Done

## Local Test Result

`cargo test mpa_analyze` — 2 passed, 0 failed. Re-ran independently, plus the full `cargo test` suite (84 passed, 0 failed, including the pinned `list_mode_panel_switches_and_edits_rows`) and `cargo clippy --all-targets -- -D warnings` (clean).

## Review Notes

Implemented by a Sonnet subagent (model hint: medium/medium), reviewed inline. The `.mpa`-on-Analyze panel reuses the shared `rois`/`roi-units`/`add-roi`/`remove-roi` row markup (no time-window section), a new `format_roi_statistics` text rendering of Ticket 4's `RoiStatistics` in the shared Results panel, and a new static `plotters`-rendered spectrum chart (RGB8 buffer via `BitMapBackend::with_buffer`, wrapped as a `slint::Image`) with each resolvable ROI's channel range shaded — an ROI whose bounds can't resolve (e.g. missing calibration) is skipped from the chart rather than failing it, matching `roi_statistics`'s own per-ROI error handling. The footer button/callback now has a third branch (`screen == 2` → "Compute ROI statistics" → `analyze()`), reusing the existing busy-flag/worker-thread/timer pattern unchanged. `mismatched` was removed entirely since every screen/file combination is now supported.

One pre-existing quirk, surfaced by the subagent and confirmed on review: before any file is loaded, the Analyze screen still falls back to showing the Convert-settings panel (format dropdown etc.), because ticket 01's original `mismatched` gate required `loaded` to be true, so pre-load the converter panel showed by default on both screens — ticket 01's own `list_mode_panel_switches_and_edits_rows` test already locked in and asserts this ("converter panel by default" with `screen` set to Analyze). This ticket's Verification Step explicitly requires that test to keep passing unmodified, so the subagent preserved the fallback via `(screen == 1 || (screen == 2 && !loaded))` rather than fixing it. Low severity (no functional harm, just a few confusing seconds of visible Convert-only controls before a file loads) and out of this ticket's scope to fix without touching that pinned test — left as a known follow-up, not a new ticket, since the user hasn't asked for one.

## Blockers / Challenges

None.

## Commits

- 72dc0b3 feat: wire .mpa ROI statistics into Analyze screen (ticket 05)

## Time Spent

21m (ticket-start.js to ticket-complete.js)
