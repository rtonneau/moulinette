# Ticket 04: violin-plot

**Status:** ✅ Done

## Local Test Result

`cargo test kde:: && cargo test plot::` (target-dir redirected to the session scratch dir): both `test result: ok.`. Also ran the full `cargo test` (41 lib + 12 main-bin + 1 gui + 3 cli + 5 gui_workflow + 0 doc, all pass), `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo build --release --no-default-features --bin moulinette`: all clean/succeed.

## Review Notes

Subagent (Opus/high) added `src/kde.rs` (`gaussian_kde`, Silverman bandwidth, NumPy-style linear-interpolated percentiles), `plot::violin` in `src/plot.rs`, and `--window`/`WindowSpec`/violin wiring in `src/main.rs`. Checked every Acceptance Criterion: `gaussian_kde` signature and bandwidth formula match exactly; verified the cross-check test's intermediate values by hand (mean=2, variance=1, std_dev=1, q1=1.5, q3=2.5, iqr=1, h≈0.539155) — correct, and the resulting densities are plausible (symmetric around the mean, as the second test also confirms structurally). `plot::violin` signature matches, builds one filled polygon per group from `gaussian_kde`, positioned side by side with its label. `--window START_S:END_S` is repeatable, validates `start_s < end_s` and rejects NaN; omitting it produces zero violin files (confirmed by its own integration test) while ticket 3's CSV/PNG output is unchanged; giving it produces one `<roi_name>_violin.png` per ROI, one violin per window in window order (confirmed by the paired integration test). Duplicate-output-destination checking was correctly extended to include the new violin paths.

Specifically verified the subagent reused ticket 3's review-fixed overwrite-safety pattern rather than reintroducing the `try_exists()`-only gap: `plot::violin` claims `output_path` atomically via `OpenOptions::new().write(true).create_new(true).open(...)` before any `plotters` drawing, exactly like `rate_vs_time`. This was flagged explicitly in this ticket's dispatch brief given ticket 3's review finding, and the subagent applied it correctly on the first pass.

No corrections needed this time — `cargo fmt`/`cargo clippy` were already clean in the handoff (subagent fixed a `clippy::neg_cmp_op_on_partial_ord` itself before reporting), and the independent full-suite/fmt/clippy/no-default-features-build re-run in this review confirmed it.

## Blockers / Challenges

None.

## Commits

- ec2e390 feat: add Gaussian KDE violin plots and --window flag to list CLI (ticket 04)

## Time Spent

11m (ticket-start.js to ticket-complete.js)
