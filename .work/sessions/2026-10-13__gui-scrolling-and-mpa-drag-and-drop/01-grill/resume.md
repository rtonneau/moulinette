# Session: GUI scrolling and MPA drag and drop

**Date:** 2026-10-13T15:07:03.815Z
**Status:** Grill phase complete

## Problem Statement

Make GUI controls reachable by scrolling and allow loading a measurement by dropping a file.

## Context & Constraints

- **Current behavior:** One main window without outer scrolling; native Open picker loads one measurement.
- **Pain point:** Content cannot scroll on shorter windows and files cannot be dropped.
- **Dependencies:** Preserve existing loading and conversion behavior.
- **Tech stack:** Rust and Slint 1.18.1.

## Success Metrics

- Whole-window vertical scrolling with wheel and scrollbar; shorter windows allowed with existing minimum width.
- Dropping exactly one .mpa file, case-insensitively, on the input panel loads it without conversion.
- Invalid and multiple drops show a clear message without clearing the measurement; folders rejected; outside drops ignored; busy drops refused with an indication.

## Architecture & Approach

Use a scrollable outer layout and an input-panel drop target. Share the Open loading path: replace measurement, select all spectra, reset output folder to input parent, retain format/channel settings. Parse failure leaves no measurement loaded.

## Assumptions & Trade-offs

Retain existing text-area scrolling. Support one measurement at a time and require retrying drops after busy work finishes.

## Open Questions

None.

## Notes

Approved by the user for bounded implementation on the current branch.
