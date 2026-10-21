# Implementation Plan

**Session:** Clean/review current state of the GUI layout. Need a home screen to decide whether Analyze or conversion will be performed. both .lst files and .mpa files should be convertible and analysable. Analysis part should be able to display a dynamic graph
**Date:** 2026-10-21T10:30:23.234Z
**Estimated effort:** 4-6 days

## Strategy

Build the shell first (home screen + navigation), then land each cross-format capability as a vertical slice (library function with tests, then its GUI wiring), then build the interactive-chart rendering module once and apply it to every visualization in one pass per view. This keeps every ticket shippable on its own: early tickets may leave a path explicitly marked "not yet supported" in the status text rather than half-wiring a feature, and later tickets remove those messages as they land. Charge normalization comes last since it depends on the `.lst`-histogram conversion path and the chart module both existing already.

## Tickets Overview

- **Ticket 1:** Home screen with Analyze/Convert cards and a Home link; file loading now screen-aware, routes into the existing converter/list-mode panels unchanged (cross-format combinations not yet wired show a clear "not supported yet" status instead of silently misbehaving).
- **Ticket 2:** Library function histogramming a `.lst` file's per-ADC events into channel-count spectra, with unit tests.
- **Ticket 3:** Wire the Ticket 2 histogram into the Convert screen so `.lst` files produce XY/EY/1CO/PIX/PIG/MON/TOF/8CO output through the existing converter panel.
- **Ticket 4:** Library function computing per-ROI statistics (weighted centroid, std-dev width, gross and linear-background-subtracted net counts) over an `.mpa` spectrum, with unit tests.
- **Ticket 5:** Wire Ticket 4's ROI statistics into the Analyze screen for `.mpa` files: ROI editor (reusing the existing row UI), stats table, and the spectrum rendered with ROI bands highlighted.
- **Ticket 6:** Interactive chart rendering module (pan/zoom/hover over a `plotters`-rendered buffer shown through a Slint `Image`), applied first to replace the `.lst` rate-vs-time PNG-only output with a live in-window chart.
- **Ticket 7:** Apply the Ticket 6 chart module to the violin (time-window) chart and to Ticket 5's `.mpa` spectrum+ROI-bands view.
- **Ticket 8:** Monitor-ADC and counts-per-µC calibration data model, the Charge-normalized (µC) conversion format, and its UI section in the Convert screen for both `.mpa` and `.lst`-via-histogram.
- **Ticket 9:** Charge-axis analysis series (ROI rate against running monitor-ADC counts) as an opt-in chart in the Analyze screen for `.lst`, rendered through the Ticket 6 chart module.

## Sequencing Rationale

Ticket 1 must land first since every later ticket wires into the screens it creates. Tickets 2-3 and 4-5 are independent vertical slices (library then GUI) that can be built in either order; library tickets (2, 4) have no GUI dependency and are ordered before their own wiring ticket only. Ticket 6 needs an existing chart (the `.lst` rate-vs-time one, already present) to prove itself against before Ticket 7 reuses it for the violin and the new `.mpa` spectrum view from Ticket 5. Tickets 8-9 depend on Ticket 3 (the `.lst` histogram path they normalize) and Ticket 6 (the chart module Ticket 9 reuses), so they come last.

## Risks & Mitigation

- **Risk:** Slint has no built-in interactive charting widget, so Ticket 6's buffer-rasterize-and-pointer-translate approach is unproven in this codebase. → **Mitigation:** Prototype against the simplest existing chart (rate-vs-time, one line series) in Ticket 6 before Ticket 7 extends it to the violin and spectrum views.
- **Risk:** Linear-background net-count subtraction (Ticket 4) can go negative or unstable for a very narrow or noisy ROI. → **Mitigation:** Clamp net counts at zero and cover it with a unit test using a synthetic narrow/noisy ROI.
- **Risk:** Monitor-ADC total counts could be zero (e.g., ROI/ADC never fired), making the charge ratio undefined. → **Mitigation:** Ticket 8 returns a structured error ("monitor ADC recorded no counts") instead of dividing by zero, covered by a unit test.

## Assumptions

- The existing `RoiFields`/`RoiSpec` validation in `gui_state.rs` is reused as-is for both the `.mpa` ROI-statistics editor (Ticket 5) and the monitor-ADC picker (Ticket 8), since both already constrain to ADC 1-8.
- `plotters`' `BitMapBackend` can render into an in-memory buffer (not just a file) for Ticket 6; this is confirmed by `plotters`' own API (`BitMapBackend::with_buffer`) and does not require a new dependency.
- Existing CSV/PNG file outputs for `.lst` analysis (rate CSV, `rate_vs_time.png`, violin PNGs) keep their exact current paths and format; Tickets 6-7 add an in-window view alongside them, not a replacement of the file outputs.
