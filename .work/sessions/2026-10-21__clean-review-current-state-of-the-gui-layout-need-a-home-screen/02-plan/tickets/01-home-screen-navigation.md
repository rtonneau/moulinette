# Ticket 01: home-screen-navigation

**Model:** medium
**Model (Jev):** sonnet-5.5 (confidence 0.49)
**Effort:** medium

**Acceptance Criteria:**
- [ ] Launching the GUI shows a home screen with two large cards, "Analyze" and "Convert", styled with the existing `Panel` component.
- [ ] Picking a card switches to that screen; a small "Home" link is visible from inside either screen and returns to the home screen, clearing the loaded file and resetting `busy`/`loaded`/`has-results` state the same way loading a new file does today.
- [ ] The file open/drop mechanism works identically on both screens and accepts either `.mpa` or `.lst`.
- [ ] A loaded `.mpa` file on the Convert screen behaves exactly as today's converter panel; a loaded `.lst` file on the Analyze screen behaves exactly as today's list-mode panel.
- [ ] A loaded `.lst` file on the Convert screen, and a loaded `.mpa` file on the Analyze screen, show the status text "Not supported yet" (`error: false`) instead of any conversion/analysis controls, until Tickets 3 and 5 land.

**Files to Touch:**
- `ui/main.slint`
- `ui/components/panel.slint` (only if a new shared card style is extracted)
- `src/gui.rs`
- `src/gui_state.rs`
- `tests/gui_workflow.rs`

**Verification Step:**

Run:
```bash
cargo test --bin moulinette-gui
```
Expected: all tests pass, including a new test asserting the home screen's two cards are present before any file loads, and that picking "Convert" then loading a `.mpa` file reaches the existing Convert button, matching the pre-existing `scrolling_moves_drop_target_and_reaches_convert_in_short_window` assertions.

**Notes:**

Keep the existing `list-mode: bool` property's role (which panel variant renders) separate from the new top-level `screen` property (home/convert/analyze); a loaded file's extension still decides panel content, the new property only decides whether the converter/list-mode panel is reachable at all. Don't implement Tickets 3/5's actual cross-format behavior here — the "Not supported yet" message is this ticket's whole answer for the unimplemented combinations.
