# Ticket 05: gui-list-mode-panel

**Model:** heavy
**Model (Jev):** opus-5.5 (confidence 0.22)
**Effort:** high

**Acceptance Criteria:**
- [ ] `gui_state::dropped_measurement` accepts `.lst` files (case-insensitive) in addition to `.mpa`, still rejecting folders and other extensions with its existing error-message pattern.
- [ ] A new `gui_state` function (e.g. `list_mode_parameters`) validates a list-mode panel's inputs (ROI rows: name/ADC/bounds/unit; bin width; time windows; output directory) the same way `parameters`/`preview` validate the converter panel's inputs today, returning a typed result or a clear `String` error.
- [ ] The GUI window switches to a new list-mode analysis panel when a `.lst` file is dropped (ROI list editor with add/remove rows, bin-width field, time-window editor with add/remove rows, output-directory picker), and switches back to today's converter panel for a dropped `.mpa` file.
- [ ] Running the list-mode panel's analysis calls the same `lst::parse` / `rate::time_binned_rate` / `plot::rate_vs_time` / `plot::violin` functions from tickets 1-4 on a worker thread, with the existing busy-spinner and "concurrent runs prevented" guard reused as-is, and never overwrites existing output files.
- [ ] `README.md` gains a section documenting the `list` CLI command's full flag set and the GUI's list-mode panel, and any format/feature summary table in the README is updated to mention `.lst` support.
- [ ] Unit tests (no Slint dependency, in `gui_state.rs`'s existing style) cover the extended `dropped_measurement` (accepts `.lst`, still rejects non-`.mpa`/`.lst` extensions and folders) and the new list-mode parameter-validation function (valid input, missing ROI, malformed bounds, invalid output directory).

**Files to Touch:**
- `src/gui_state.rs`
- `src/gui.rs`
- `ui/main.slint`
- `ui/components/panel.slint`
- `README.md`

**Verification Step:**

Run:
```bash
cargo test --features gui gui_state::
```
Expected: `test result: ok.` for all `gui_state` tests, including the new `.lst`-related ones.

**Notes:**

This ticket only wires existing, already-tested parsing/computation/rendering logic (tickets 1-4) into the existing Slint window; it introduces no new computational logic. Manual verification — launching `moulinette-gui` and dropping a real `.lst` file end-to-end — is expected before considering this ticket done, since Slint UI behavior isn't exercised by `cargo test`. Depends on all of tickets 1-4.
