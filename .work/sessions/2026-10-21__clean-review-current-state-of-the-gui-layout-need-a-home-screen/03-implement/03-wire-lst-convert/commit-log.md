# Ticket 03: wire-lst-convert

**Status:** ✅ Done

## Local Test Result

`cargo test lst_convert` — 1 passed (`lst_convert_writes_histogram_as_xy`). Re-ran the full `cargo test` suite independently: 53+12+3+3+6 passed, 0 failed, 0 regressions.

## Review Notes

Implemented by a Sonnet subagent (model hint: medium/medium), reviewed inline. On the Convert screen, a loaded `.lst` file is wrapped as `MpaFile { sections: vec![], spectra: histogram_list_file(&file) }` and stored in `state.file`, so the existing converter code path (spectra dropdown, preview, convert, overwrite-refusal, warnings) is reused completely unchanged — confirmed by reading the diff, no new logic duplicated. The Slint-facing `list_mode` flag is now `list_mode && screen != SCREEN_CONVERT`, so `ui/main.slint` needed no changes: a `.lst` file on Convert renders the pre-existing converter panel, not the ROI panel, purely from that one flag. `mismatched` was narrowed to only `.mpa` on Analyze, which still correctly reports "Not supported yet" until Ticket 5. The new integration test converts a synthetic `.lst` fixture's ADC-1 histogram to XY and asserts the written bytes equal `conversion::convert` run directly on the Ticket 2 output, closing the loop end-to-end. No changes made after review.

## Blockers / Challenges

None.

## Commits

- ebd058f feat: wire .lst-histogram conversion into Convert screen (ticket 03)

## Time Spent

4m (ticket-start.js to ticket-complete.js)
