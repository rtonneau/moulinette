# Ticket 03: rate-vs-time-plot-and-cli

**Status:** ✅ Done

## Local Test Result

`cargo test` (full suite): `test result: ok.` across all 6 test binaries (35 lib + 7 main-bin + 1 gui + 3 cli + 5 gui_workflow + 0 doc), 0 failed. `cargo build --release --no-default-features --bin moulinette`: succeeds. `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings`: both clean, after the review fix below.

## Review Notes

Subagent (Opus/high) added `plotters` unconditionally to `Cargo.toml` (bitmap_backend/bitmap_encoder/line_series/ttf features), `src/plot.rs` (`rate_vs_time`, `write_rate_csv`), the `list` CLI subcommand in `src/main.rs`, and the README dependency-claim fix. Checked every Acceptance Criterion against the code: `plotters` is unconditional (not gated behind `gui`) ✓; `rate_vs_time`/`write_rate_csv` signatures and CSV header match exactly ✓; `list`'s `--roi NAME:ADC:LOW:HIGH[:unit]` (repeatable, ≥1 required, `ch`/`kev` unit, default `kev`), `--dt-ms` (default 15000), `--output-dir` (required) all match ✓; no-default-features build succeeds ✓; README correction accurately reflects ADR 0001 ✓; `main.rs`'s argument-parsing tests and the CSV-exact-contents integration test (hand-verified the synthetic fixture's byte layout and the expected rate/counts by hand against `rate.rs`'s binning logic — correct) are present and pass ✓.

Found and fixed a real correctness gap during review: `rate_vs_time`'s overwrite guard was `output_path.try_exists()` followed, much later, by `plotters-bitmap`'s `present()` — which saves via `image`'s `File::create`-equivalent semantics (confirmed by reading `plotters-bitmap-0.3.7/src/bitmap.rs`), i.e. it always truncates/overwrites regardless of the earlier check. This violates this project's own documented invariant ("existing output files are never overwritten, including files created between validation and writing") and the ticket's own AC wording ("reusing `workflow`'s existing refusal pattern" — `workflow::write` gets its real protection from `OpenOptions::create_new`, not its earlier `try_exists` pre-check). `write_rate_csv` already did this correctly via `create_new`; `rate_vs_time` did not. Fixed in `src/plot.rs`: atomically claim `output_path` with `OpenOptions::new().write(true).create_new(true).open(...)` before any drawing happens, then let `present()` overwrite the empty placeholder we now own — closing the race the same way `workflow::write` does. Re-ran the full test suite, fmt, clippy, and the no-default-features build clean after the fix.

Also noted for awareness (not a defect, just a real cost of the `ttf` plotters feature needed for axis-label text rendering): it pulls in `font-kit`/`dwrote`/`image`/`png` and friends. Confirmed via the no-default-features build that this adds no new native-toolchain requirement (dwrote is a pure Windows API binding), but it is a meaningfully larger dependency tree than the "a plotting crate" framing in the original Assumptions section anticipated. Worth keeping in mind for ticket 4 (violin plot reuses the same `plot.rs`/`plotters` setup, so no further cost there).

## Blockers / Challenges

None.

## Commits

- ef6ab90 feat: add plotters dependency, rate-vs-time plot/CSV, and moulinette list CLI (ticket 03)

## Time Spent

21m (ticket-start.js to ticket-complete.js)
