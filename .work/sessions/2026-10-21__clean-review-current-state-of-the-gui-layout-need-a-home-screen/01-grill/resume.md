# Session: Clean/review current state of the GUI layout. Need a home screen to decide whether Analyze or conversion will be performed. both .lst files and .mpa files should be convertible and analysable. Analysis part should be able to display a dynamic graph

**Date:** 2026-10-21T09:55:41.319Z
**Status:** Grill phase complete

## Problem Statement

The GUI currently decides the whole workflow from the dropped/opened file's extension: an `.mpa` file always lands in the converter panel (spectrum format conversion only), an `.lst` file always lands in the list-mode analysis panel (rate-vs-time/violin only, PNG/CSV output only — no in-window plot). There is no screen that lets the user choose what they want to do; `.mpa` cannot be analyzed and `.lst` cannot be converted; and analysis results are never shown inside the window, only written to disk. We need a home screen that separates "what to do" (Analyze or Convert) from "what file", make both actions available for both file types, and show analysis results as live interactive charts instead of file-only PNGs.

## Context & Constraints

- **Current behavior:** `ui/main.slint` + `src/gui.rs` auto-switch between two mutually exclusive panels based on `gui_state::is_list_mode` (file extension). The converter panel drives `conversion::convert`/`workflow::render` over `MpaFile` spectra (XY/EY/1CO/PIX/PIG/MON/TOF/8CO). The list-mode panel drives `rate::time_binned_rate` over `ListFile` events per ROI, writing per-ROI CSV + a shared `rate_vs_time.png` + optional per-ROI violin PNGs via `plot.rs` (`plotters` `BitMapBackend`). No in-window plot exists anywhere today (documented GUI limitation).
- **Pain point:** `.mpa` histograms can't be analyzed (no ROI stats); `.lst` events can't be converted to a spectrum format; nothing you compute in Analyze is visible without opening the generated PNG file; there's no landing/home screen, so the two workflows are not actual user choices, just format dispatch.
- **Dependencies:** Must keep existing conversion formats and their legacy filename/CRLF/overwrite-refusal behavior unchanged; must keep the existing `.lst` rate-vs-time/violin CSV+PNG outputs unchanged; worker-thread + single busy-flag concurrency model (load/convert/analyze never overlap) must still hold; existing Slint test patterns (`i-slint-backend-testing`, `ElementHandle`) continue to apply to new panels.
- **Tech stack:** Rust 2024, Slint 1.18.1 (`backend-winit`, `renderer-software`), `plotters` 0.3 (`bitmap_backend`), `rfd` for native dialogs, `winsafe` for Windows drop-position handling. No existing interactive/charting widget in the stack — the new live charts need a custom approach (e.g. rasterizing via `plotters` into an in-memory buffer shown through a Slint `Image`, with pointer-event handling in Rust recomputing the axis range for pan/zoom and nearest-point lookup for hover tooltips), to be worked out at planning/implementation time.

## Success Metrics

- Launching the GUI shows a home screen with two action cards, Analyze and Convert; a Home affordance from inside either workflow returns to it without losing the loaded file's path.
- Opening or dropping either an `.mpa` or an `.lst` file works under both Convert and Analyze.
- Convert on an `.lst` file histograms its events per ADC into a channel spectrum and offers the same format list as `.mpa` conversion (XY/EY/1CO/PIX/PIG/MON/TOF/8CO), plus a new Charge-normalized (µC) format available to both file types via a chosen monitor ADC and optional counts-per-µC calibration.
- Analyze on an `.mpa` file computes, per ROI, a weighted-mean centroid, standard-deviation width, gross counts and linear-background-subtracted net counts, displayed as a table plus the spectrum chart with ROI bands highlighted.
- Analyze on an `.lst` file keeps today's rate-vs-time and violin outputs, plus an opt-in charge-axis chart (ROI rate against running monitor-ADC counts instead of elapsed time).
- Every Analyze visualization (spectrum+ROI, rate-vs-time, violin, charge-axis) renders as a live in-window chart with pan/zoom/hover tooltips, updating as settings/ROIs change; Run still writes the same CSV/PNG files as before, unchanged.

## Architecture & Approach

Restructure `ui/main.slint` around a top-level `home` / `convert` / `analyze` view state (new `screen` property) instead of the current `list-mode` boolean driving one of two fixed panels. The home screen is two `Panel`-styled cards; selecting one sets `screen` and keeps the existing Open/drop input mechanism, now screen-aware rather than extension-aware for which top-level view it feeds. `gui_state.rs` gains: (1) a histogram-from-events path turning a `ListFile`'s per-ADC event channel values into the same shape `conversion::convert` already consumes, so the existing format writers apply unchanged; (2) ROI statistics (centroid/width/gross/net) over an `MpaFile` spectrum's counts, reusing the existing ROI row validation (`RoiFields`/`RoiSpec`) extended with the moment-stats math; (3) a monitor-ADC + optional counts-per-µC calibration structure shared by the new Charge-normalized conversion format and the new charge-axis analysis series. For in-window charts, keep `plotters` as the renderer but target an in-memory RGBA buffer surfaced through a Slint `Image` (updated on an interaction/redraw tick), with Rust-side pointer-event handling translating drag/wheel into axis-range changes and cursor position into a nearest-data-point lookup for hover tooltips — the same `plotters` chart-building code used for `rate_vs_time`/`violin` today is reused for pixel layout, just re-targeted to a buffer instead of a file, with the file-writing PNG/CSV path left intact for Run.

## Assumptions & Trade-offs

- Monitor-ADC counts are an approximation for integrated beam charge (no file format carries a literal charge field) — recorded as ADR 0002; only holds while the monitor channel's response stays linear with beam current over the run. The counts-per-µC calibration is always user-supplied (manual entry), since no header field provides it.
- ROI peak statistics use plain moment statistics (weighted mean/std-dev) and a linear background between ROI edges, not a Gaussian/peak fit — matches the project's existing math style (e.g. KDE-based violins) and keeps scope bounded; revisit if overlapping peaks turn out to need deconvolution.
- The Charge-normalized format is treated as available to both `.mpa` (picking among its own loaded spectra as the monitor) and `.lst`-histogrammed conversion, generalizing symmetrically rather than being `.lst`-only, since both now expose multiple ADCs/spectra the same way.
- Interactive charts are raster-redraw-on-interaction (not a native vector/GPU chart widget) — acceptable given the current software-renderer Slint backend and no existing charting dependency; revisit only if redraw latency on pan/zoom proves noticeable.
- "Clean/review current state of the GUI layout" is treated as context-gathering for this restructuring (folded into the home-screen/panel rework), not a separate standalone cleanup deliverable.

## Open Questions

None outstanding — the design was approved as summarized. Implementation-level specifics of the pan/zoom/hover interaction model (exact gesture mapping, redraw throttling) are left to planning/ticket breakdown rather than decided here.

## Notes

- Legacy MON format (one-column-counts filename convention) is unrelated to the new "Monitor ADC" charge-proxy concept; kept as two distinct glossary terms to avoid confusion (see `.work/GLOSSARY.md`).
- ADR 0002 (`.work/adr/0002-monitor-adc-charge-proxy.md`) records why monitor-ADC counts stand in for charge, given `docs/conversion_algorithm.md` previously stated no beam/charge normalization exists in this codebase.
- `mattpocock-skills:grilling`/`domain-modeling` ran this session (not `superpowers`); `superpowers` also offers an equivalent brainstorming/domain-modeling pair if preferred in a future session.
