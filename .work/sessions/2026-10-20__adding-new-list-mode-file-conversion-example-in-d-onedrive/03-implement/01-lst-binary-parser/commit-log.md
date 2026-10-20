# Ticket 01: lst-binary-parser

**Status:** ✅ Done

## Local Test Result

`cargo test lst::` (target-dir redirected to the session scratch dir): `test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 13 filtered out`.
Also ran `cargo test` (full suite) and `cargo fmt --all -- --check` / `cargo clippy --all-targets -- -D warnings`: all clean after the review fix below.

## Review Notes

Subagent (Opus/high) implemented `src/lst.rs`, `src/lib.rs` (`pub mod lst;`), and `docs/lst_format.md`. Checked each Acceptance Criterion against the code and the 11 hand-built test fixtures (header-only, timer word, timer+timerreduce accumulation, sync marker, event with 1/3 ADCs, event with RTC, event with dummy flag, truncated event, unrecognized word, missing `[LISTDATA]`) — all covered, all pass. Verified the timer-word/sync-marker/event discrimination order and the RTC/dummy/ADC-mask bit layout against `mpa3kit`'s Python reference by hand; matches, with the three documented, reasonable divergences (1-based `adc_index` matching `mpa.rs`'s convention, an `Event` always emitted even for a zero-bit mask, and strict errors instead of lenient skip-and-count) all called out in `docs/lst_format.md`.

Found after the subagent's handoff: `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings` (both part of this project's documented standard checks, though not explicitly named in this ticket's Verification Step) were not clean — a few long lines needed rustfmt's wrapping, and `clippy::collapsible_if` fired on the nested `[ADCn]`-section-open check. Fixed inline: ran `cargo fmt --all`, then collapsed the nested `if` into one condition with a `let` chain (`if cond && let Some(idx) = ... { ... }`, valid on this project's Rust 2024 edition). Re-ran fmt/clippy/full test suite clean after the fix.

## Blockers / Challenges

None.

## Commits

- 03d11d5 feat: parse MPA-3 list-mode (.lst) files (ticket 01)

## Time Spent

10m (ticket-start.js to ticket-complete.js)
