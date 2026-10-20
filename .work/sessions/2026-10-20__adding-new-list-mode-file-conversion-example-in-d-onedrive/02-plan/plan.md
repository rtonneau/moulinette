# Implementation Plan

**Session:** Adding new list mode file conversion. Example in 'D:\OneDrive - Université de Namur\JupyterLab\PFAS/PIGE/test_mpa3kit.ipynb' using custom mpa3kit python library accessible at 'C:\DEV\python\mpa3kit'. The soft should be able to load .lst files and also make some plot of acquisition rate versus time (like in cell 11) and also violin plots like cell 13. If possible to use a similar approach as 'import ruptures as rpt' to detect changes of rate, it is a plus
**Date:** 2026-10-20T08:56:36.116Z
**Estimated effort:** 3-4 days

## Strategy

Build bottom-up: the binary `.lst` parser first (no dependents, fully unit-testable with synthetic fixtures), then the ROI/rate-binning math on top of it (pure computation, also synthetic-fixture-testable), then the CLI surface with CSV+PNG output for the rate-vs-time plot only (the simplest visual deliverable, and the first point where `plotters` enters the build), then the violin plot (KDE + a second rendering path, reusing the same CLI), and finally the GUI panel (wiring the same parse/rate/plot functions into the existing Slint window). Each ticket produces something independently testable and committable; nothing in a later ticket needs to be reworked by an earlier one.

## Tickets Overview

- **Ticket 1:** Parse `.lst` files (ASCII header + binary event stream) into an in-memory `ListFile`/`Event` structure, with a strict, structured-error parser and synthetic byte-level unit tests.
- **Ticket 2:** ROI resolution (channel or linear-calibration energy bounds) and time-binned rate computation (`counts`, `rate`, Poisson `rate_err` per time bin) over a parsed `ListFile`.
- **Ticket 3:** Add the `plotters` dependency; implement the rate-vs-time line plot and per-series CSV writer; add the `moulinette list` CLI subcommand (rate-vs-time only); update the README's dependency-free CLI-only build claim.
- **Ticket 4:** Gaussian KDE + violin-plot rendering; wire a repeatable `--window` flag into `list` to produce one violin PNG per ROI.
- **Ticket 5:** GUI integration: a new list-mode analysis panel that activates when a `.lst` file is dropped, running the same parse/rate/plot pipeline on a worker thread; final README documentation pass for the whole feature.

## Sequencing Rationale

Parsing has no dependencies and is the riskiest piece to get byte-accurate, so it goes first and is fully pinned by tests before anything is built on top of it. Rate computation is pure math over the parser's output, so it is next and equally testable without any rendering or CLI concerns. The CLI ticket is split into rate-vs-time (ticket 3) before violin (ticket 4) because rate-vs-time is the simpler rendering path and is what first proves `plotters` integration end-to-end; violin adds KDE math on top of an already-working CLI. GUI (ticket 5) is last because it only wires existing, already-tested logic into the existing Slint window — it has no new computational logic of its own, so it can't block or be blocked by the others' correctness.

## Risks & Mitigation

- **Risk:** The `.lst` binary format's exact bit-layout (timer/sync/event discrimination, RTC/dummy flag positions) was reverse-engineered from a working Python parser (`mpa3kit`), not from a vendor spec; a subtle misreading could silently misparse real files. → **Mitigation:** Ticket 1's acceptance criteria require covering every record type (timer, sync, event with/without RTC, event with dummy flag, multi-ADC event) with hand-built byte fixtures whose expected output is computed by hand from the same bit-layout description now written into `docs/lst_format.md`, and the parser is strict (errors loudly on anything unrecognized) rather than silently continuing.
- **Risk:** Hand-rolled Gaussian KDE might not visually resemble matplotlib's violin shape closely enough to be useful. → **Mitigation:** Ticket 4 pins the KDE's bandwidth rule and asserts density values at known evaluation points against a small hand/`scipy`-cross-checked case, not just "a plot appears."
- **Risk:** Adding `plotters` unconditionally is a real, hard-to-reverse build change (ADR 0001). → **Mitigation:** Already decided and recorded during grilling; ticket 3 updates the README in the same commit that introduces the dependency, so the documented build property is never left stale.

## Assumptions

- `plotters`'s `BitMapBackend` and basic series/polygon drawing primitives are sufficient for both chart types; no other new crate is needed.
- The existing `workflow` module's "never overwrite existing output" and worker-thread/busy-spinner patterns can be reused as-is for the new CSV/PNG outputs and the GUI panel, rather than needing new infrastructure.
- No real-world `.lst` file is available inside the repo for integration testing; all tests use synthetic fixtures built from the bit-layout in `docs/lst_format.md`.
