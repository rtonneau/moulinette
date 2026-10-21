# Ticket 02: lst-histogram-spectrum

**Status:** ✅ Done

## Local Test Result

`cargo test histogram` — 3 passed, 0 failed (`histogram_list_file_counts_known_events_and_excludes_missing_values`, `histogram_list_file_includes_upper_boundary_channel`, `histogram_list_file_zero_events_is_all_zero_of_declared_length`). Re-ran independently after the subagent's report; output matches.

## Review Notes

Implemented by a Sonnet subagent (model hint: medium/medium), reviewed inline. `histogram_list_file` builds one `mpa::Spectrum` per `AdcHeader`, histogramming each event's channel value for that ADC and skipping `None` values rather than counting them as channel 0 — confirmed by the exclusion test. Values are clamped into `[0, n_channels)` per the Acceptance Criteria's literal wording (clamp, not drop), and the upper-boundary channel (`n_channels - 1`) is included, confirmed by its own test. The result is a genuine `mpa::Spectrum` (channel counts as `Record`s, `CALOFF`/`CALFACT` carried into `metadata` when present), so it flows into `conversion::convert`/`workflow::render` unchanged — no adapter was needed, which is a better fit than the ticket's "thin adapter" fallback. `src/lst.rs` was left untouched since no new accessor was needed. No changes made after review.

## Blockers / Challenges

None.

## Commits

- 0f03895 feat: histogram .lst events into per-ADC spectra (ticket 02)

## Time Spent

5m (ticket-start.js to ticket-complete.js)
