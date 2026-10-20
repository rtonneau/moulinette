# Ticket 05: gui-list-mode-panel

**Status:** ✅ Done

## Local Test Result

`cargo test --features gui gui_state::` (target-dir redirected to the session scratch dir): `test result: ok. 9 passed; 0 failed`, exactly matching the handoff. Also independently re-ran: full `cargo test` (50 lib + 12 main-bin + 2 gui + 3 cli + 5 gui_workflow, all pass — the gui.rs binary's test count going from 1→2 confirms the new Slint headless test ran), `cargo fmt --all -- --check` (clean), `cargo clippy --all-targets -- -D warnings` (clean), and critically `cargo build --release --bins` (the default-features build that compiles `ui/main.slint` via `build.rs` — this is the only way a Slint syntax error would surface, since `cargo test` alone doesn't catch it): succeeded in 1m46s with no errors.

## Review Notes

Subagent (Opus/high) added `gui_state::dropped_measurement` `.lst` support, `is_list_mode`, `list_mode_parameters`/`RoiFields`/`RoiSpec`/`TimeWindow`/`ListParameters`, `list_mode_outputs`, `load_list`, `list_inspection`, `list_mode_run` in `gui_state.rs`; a new `RoiRow`/`TimeWindowRow` Slint struct, dynamic `for`-loop ROI/time-window row editors, panel-switching `if root.list-mode: ... else: ...` blocks, and a relabelled footer button in `ui/main.slint`; and the Rust-side wiring (panel switch on load, `add_roi`/`remove_roi`/`add_time_window`/`remove_time_window`/`analyze` callbacks reusing the existing worker-thread+timer+busy-flag pattern) plus a new headless Slint test in `gui.rs`. Checked every Acceptance Criterion against the code: `.lst` accepted case-insensitively alongside `.mpa`, folders/other extensions still rejected ✓; `list_mode_parameters` validates ROI rows (name/ADC 1-8/bounds/unit, uniqueness, filename-safety), bin width, time windows and output directory, returning a typed result or `String` error, no Slint dependency ✓; the window switches panels on `.lst` vs `.mpa` (confirmed both by reading `ui/main.slint`'s `if root.list-mode` blocks and by the new Slint test asserting "Add ROI"/"Final channel" presence flips correctly) ✓; `list_mode_run` calls the same `rate::time_binned_rate`/`plot::write_rate_csv`/`plot::rate_vs_time`/`plot::violin` from tickets 1-4, runs on a worker thread via the pre-existing channel+timer pattern, shares the single `busy` flag with conversion (verified by reading `on_analyze`'s guard and the fact only one `timer`/`busy` pair exists), and never overwrites existing output (confirmed by `list_mode_run_writes_outputs_and_never_overwrites`'s second-run assertions, and by `plot::write_rate_csv`/`rate_vs_time`/`violin` all using `create_new` internally per tickets 3-4's review fixes — `list_mode_run`'s own pre-check loop is a non-atomic `try_exists`, same as `workflow::render`'s equivalent pre-check, with the real protection coming from each writer's atomic claim, exactly matching the established pattern) ✓; README gained the full `list` flag table and a GUI panel section, plus an input-type summary table ✓; unit tests cover `dropped_measurement` (accepts `.lst`, still rejects other extensions/folders) and `list_mode_parameters` (valid input, missing ROI, malformed bounds, bad bin width/windows, invalid output directory) ✓.

Noted design choices, not defects: (1) `src/main.rs`'s `run_list` (CLI) and `gui_state::list_mode_run` (GUI) independently chain the same `rate`/`plot` functions rather than one calling the other — a real but minor duplication, forced by `main.rs` being a binary the library can't depend on and being outside this ticket's Files to Touch; worth a follow-up refactor ticket if it drifts, not blocking here. (2) The GUI's ROI/output validation is stricter than the CLI's (e.g. rejects ADC 0/9+, low>high, duplicate/unsafe names) — reasonable for an interactive form giving immediate feedback, and not a regression since the CLI's own tests still pass unchanged.

Visual verification of the running GUI (launching `moulinette-gui` and dropping a real `.lst` file) was not performed — no display/screenshot tooling is available in this session. This is a known limitation, explicitly surfaced by the implementing subagent and confirmed here; the build succeeding, the headless Slint test exercising panel-switch/add/remove/footer-click behavior, and the full `gui_state` test suite are the verification actually available, and it should be flagged to the user.

## Blockers / Challenges

None blocking. Visual GUI verification could not be performed in this environment (see Review Notes) — recommend the user launch `moulinette-gui` by hand and drop a real `.lst` file before relying on this feature.

## Commits

- 8e72847 feat: add GUI list-mode analysis panel for .lst files (ticket 05)

## Time Spent

22m (ticket-start.js to ticket-complete.js)
