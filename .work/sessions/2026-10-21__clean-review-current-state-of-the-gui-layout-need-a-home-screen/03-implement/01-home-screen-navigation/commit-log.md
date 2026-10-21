# Ticket 01: home-screen-navigation

**Status:** ✅ Done

## Local Test Result

`cargo test --bin moulinette-gui` — 3 passed, 0 failed (the two pre-existing Slint tests plus the new `home_screen_cards_then_convert_after_picking_a_card`). Re-ran independently after the subagent's report; output matches.

## Review Notes

Implemented by a Sonnet subagent (model hint: medium/medium), reviewed inline. Checked each Acceptance Criterion against the diff: home screen renders two Panel-styled cards with accessible labels "Analyze"/"Convert" before any file loads; picking a card sets the new `screen` property and a "← Home" button appears, calling `go_home()` which resets `busy`/`loaded`/`has-results` and all file-specific text the same way a fresh load does; the file open/drop path is unchanged and screen-agnostic; `.mpa` on Convert and `.lst` on Analyze behave exactly as the pre-existing panels (verified via the carried-over `scrolling_moves_drop_target_and_reaches_convert_in_short_window` and `list_mode_panel_switches_and_edits_rows` tests, both still passing); a mismatched combination reports "Not supported yet" with `error: false` via the new `mismatched` computation in both `main.slint` and `gui.rs`, and hides the conversion/analysis panels and footer button entirely rather than showing disabled controls. No changes made after review — the subagent's implementation matched the spec as submitted.

## Blockers / Challenges

None.

## Commits

- 2a8f3e7 feat: add home screen with Analyze/Convert navigation (ticket 01)

## Time Spent

16m (ticket-start.js to ticket-complete.js)
