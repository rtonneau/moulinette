# Ticket 04: mpa-roi-statistics

**Status:** ✅ Done

## Local Test Result

`cargo test roi_statistics` — 5 passed, 0 failed. Re-ran independently, plus `cargo clippy --all-targets -- -D warnings` (clean, no new warnings).

## Review Notes

Implemented by a Sonnet subagent (model hint: medium/medium), reviewed inline. `roi_statistics` computes gross (raw sum), net (trapezoidal linear-background subtraction between the two edge channels, clamped at zero — confirmed by the flat/sloped/narrow-noisy tests, including the sloped case matching the exact trapezoid area), and weighted-mean centroid/std-dev width from the background-subtracted per-channel weights (also clamped at zero per channel). Energy-unit ROIs resolve through `conversion::coefficient` (made `pub(crate)`, reused rather than duplicated) by inverting the EY writer's confirmed one-based `E = i*slope + offset` formula — mirrors `rate::Roi::from_energy`'s role for `.lst` without copying its code, since `.mpa` calibration lives in text metadata rather than `lst::AdcHeader`'s numeric fields. Errors are reported per-ROI with the ROI name, covering missing calibration and out-of-bounds resolved channels. Library-only, as the ticket specifies; no GUI wiring (Ticket 5). No changes made after review.

## Blockers / Challenges

None.

## Commits

- 7f976fb feat: compute .mpa ROI peak statistics (ticket 04)

## Time Spent

9m (ticket-start.js to ticket-complete.js)
