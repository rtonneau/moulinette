# Session: Adding new list mode file conversion. Example in 'D:\OneDrive - Université de Namur\JupyterLab\PFAS/PIGE/test_mpa3kit.ipynb' using custom mpa3kit python library accessible at 'C:\DEV\python\mpa3kit'. The soft should be able to load .lst files and also make some plot of acquisition rate versus time (like in cell 11) and also violin plots like cell 13. If possible to use a similar approach as 'import ruptures as rpt' to detect changes of rate, it is a plus

**Date:** 2026-10-20T08:26:10.876Z
**Status:** Grill phase complete

## Problem Statement

Moulinette currently only converts aggregated `.mpa` histogram files into text spectrum formats. It cannot read MPA-3 "list-mode" `.lst` files, which record every detector event individually (timestamp + per-ADC channel value) instead of a pre-binned histogram. A reference Python workflow (the `mpa3kit` library plus a PIGE analysis notebook) already does this kind of analysis — loading `.lst` files and producing a rate-vs-time plot and a violin plot of binned acquisition rate, with an optional (notebook-only, not yet required here) `ruptures` changepoint-detection step. Moulinette needs a native-Rust equivalent of the load + rate-vs-time + violin-plot capability, consistent with its existing "native Rust, no Python runtime dependency" design.

## Context & Constraints

- **Current behavior:** Moulinette parses `.mpa` ASCII-header + `[DATAn,N]` histogram files (`src/mpa.rs`) and writes them to XY/EY/1CO/PIX/PIG/MON/TOF/8CO text formats (`src/conversion.rs`) via a shared `workflow` module used by both the CLI (`moulinette convert`) and the Slint GUI. The GUI validates a dropped file by checking its extension is `.mpa` (`gui_state::dropped_measurement`) and shows one converter panel with a `MODES` list of output formats. There is no plotting dependency or interactive viewer anywhere in the app today; `cargo build --release --no-default-features --bin moulinette` is documented as a dependency-free CLI-only build (only the `gui` feature pulls in slint/rfd/winsafe).
- **Pain point:** `.lst` list-mode files can't be loaded at all, and there is no way to compute or visualize acquisition-rate-over-time or its distribution without leaving Rust for the Python/`mpa3kit` workflow.
- **Dependencies:** Reuses moulinette's existing ASCII-header/section-record parsing conventions (ADC sections, `CALOFF`/`CALFACT` key=value pairs) for the `.lst` header, since `.lst` and `.mpa` share the same header dialect up to `.lst`'s `[LISTDATA]` marker. Adds a new unconditional dependency, the `plotters` crate (pure-Rust, safe, PNG/SVG backends), for rendering the rate-vs-time line plot and the violin plot (violin shape via a hand-rolled Gaussian KDE, since `plotters` has no built-in violin chart type). This ends the "dependency-free CLI-only build" property — recorded in ADR 0001.
- **Tech stack:** Rust (edition 2024), Slint 1.18.1 GUI, `plotters` (new). Reference material: `mpa3kit` Python library (`C:\DEV\python\mpa3kit\src\mpa3kit\parser.py`, `adc.py`) and the PIGE notebook `test_mpa3kit.ipynb` (cells ~8-14 for rate-vs-time/violin, cells ~17-24 for the out-of-scope `ruptures` changepoint example).

### `.lst` binary format (from `mpa3kit`'s working parser)

- ASCII header identical in style to `.mpa` (`[ADCn]` sections, `caloff`/`calfact`/`calfact2`/`calfact3`/`nchannels`/`roi`/`timerreduce` key=value lines), terminated by a `[LISTDATA]` marker.
- Binary body: little-endian 16-bit words, read two at a time (`low`, `high`):
  - **Timer word:** `high == 0x4000` → accumulate `timer_reduce` ms (header's `timerreduce` key, default 1 ms per word) onto the running clock; advance 2 words.
  - **Sync marker:** `low == 0xFFFF && high == 0xFFFF` → skip, advance 2 words.
  - **Event record:** otherwise (decided by bit 14 of `high`). `low & 0xFF` is an 8-bit ADC-active bitmask (up to 8 ADCs, bits 0-7). `high` flags: bit 15 = dummy word inserted, bit 12 = RTC (real-time-counter) present. If RTC present, the next 3 words form a 48-bit RTC value (consumed for stream alignment only — not surfaced in v1, since the "time" axis for all analyses is the accumulated timer-word clock, not RTC). If a dummy word was inserted, skip one word. Then read one 16-bit channel value per bit set in the ADC mask, in order, appending one `Event { time_ms, adc_values }` per record.
  - Anything else is an unrecognized word — in v1 this is a hard parse error (see "parser strictness" decision below), unlike `mpa3kit`'s lenient skip-and-count behavior.
- Calibration: `energy = CALOFF + CALFACT*channel` (+ optional `CALFACT2*channel^2 + CALFACT3*channel^3` in `mpa3kit`, but moulinette v1 only implements the linear terms, matching its existing EY-format calibration).
- An ROI is a channel range on one ADC's spectrum, given directly as channel bounds or as an energy range resolved via that ADC's calibration.
- Core derived series ("time-binned rate"): events within an ROI (or a whole ADC, if no ROI given) are grouped into fixed-width time bins; each bin yields `(time_s, rate_counts_per_s, poisson_error, raw_counts)`.

## Success Metrics

- `moulinette list <input.lst> --roi NAME:ADC:LOW:HIGH ... --dt-ms N --window START:END ... --output-dir DIR` parses a `.lst` file and writes, per requested ROI/ADC series: one CSV (`time_s, rate, rate_err, counts`) and one rate-vs-time PNG plot (all requested series as labeled lines on one shared y-axis).
- The same command (or an equivalent flag) also writes one violin-plot PNG per ROI, with one KDE-shaped violin per user-defined time window for that ROI (not hardcoded to exactly two).
- Dropping a `.lst` file onto the existing Slint GUI window switches it into a new list-mode analysis panel (ROI list editor, bin-width field, time-window editor, output directory, same "convert → write files" worker-thread pattern as today's converter panel); dropping a `.mpa` file keeps today's converter panel.
- A malformed/truncated `.lst` file (bad header, missing `[LISTDATA]`, truncated event mid-read, unrecognized word) produces a structured parse error, not a silent partial result.
- A `.lst` file whose active ADC declares nonzero `CALFACT2`/`CALFACT3` and is referenced by an energy-bounded ROI produces a clear error directing the user to channel-based ROI bounds instead.
- Unit tests cover the `.lst` binary parser using hand-built synthetic byte fixtures (timer words, sync markers, event records with and without RTC/dummy flags, ASCII header parsing) in the style of `mpa.rs`'s existing tests — no large real-world `.lst` sample is committed to the repo.
- README documents the new `list` command and that the CLI-only build (`--no-default-features`) now carries `plotters` as a real dependency, updating the "dependency-free CLI-only build" claim.

## Architecture & Approach

- New module(s) for list-mode parsing (binary event-stream reader reusing/sharing the existing header/section-record parsing primitives from `src/mpa.rs` where it makes sense) and for the rate/ROI/time-binning computation, independent of `src/conversion.rs`'s spectrum-text writers (no new writer `Format` variant — `.lst` never becomes an XY/EY/etc. text file).
- A small from-scratch Gaussian KDE routine backs the violin rendering; `plotters` handles both the line chart (rate-vs-time) and the polygon-based violin drawing.
- CLI: a new top-level `list` subcommand in `src/main.rs`, parallel to `formats`/`convert`, taking the input `.lst` path, a repeatable ROI flag, a bin-width flag (default 15000 ms), a repeatable time-window flag (for the violin plot; defaults to a reasonable two-window split if the user gives none — exact default behavior left to planning), and an output directory — writing one CSV + two PNGs (rate-vs-time, violin) following the existing `workflow::write`-style "refuse to overwrite" semantics.
- GUI: extend `gui_state::dropped_measurement` to accept `.lst` as well as `.mpa`, and add a new panel (list-mode analysis) that the main window switches to based on the dropped file's extension, built the same way as today's converter panel (worker thread, busy spinner, no overwriting existing files).
- Changepoint detection (`ruptures`-equivalent) is explicitly deferred to a follow-up ticket; no code for it ships in this effort.

## Assumptions & Trade-offs

- `plotters` becomes an unconditional dependency (not gated behind the `gui` feature), because `list` is core CLI functionality like `convert`. This deliberately ends the documented "dependency-free CLI-only build" property — see ADR 0001 (`.work/adr/0001-plotters-unconditional-dependency.md`).
- Energy-based ROI selection only supports linear calibration (`CALOFF`/`CALFACT`); files relying on `CALFACT2`/`CALFACT3` for a referenced ADC must use channel-based ROI bounds instead, or this ticket's scope would have to grow to add polynomial calibration support (explicitly rejected for v1).
- The parser is strict: any unrecognized word, truncated event, or EOF mid-read is a hard parse error. This is a deliberate behavior difference from `mpa3kit`'s lenient skip-and-count approach, chosen to match `mpa.rs`'s existing strictness and the project's "never guess" ethos.
- RTC (real-time-counter) words are parsed only far enough to keep the byte stream aligned; their value is not surfaced anywhere in v1, since every analysis uses the accumulated timer-word clock as its time axis.
- Test fixtures are synthetic (hand-built byte sequences), not derived from the one known real-world sample (`mpa3kit`'s bundled `examples/26080505.lst`, ~15 MB) — that file is not committed to the repo.

## Open Questions

- Exact CLI flag syntax for ROI and time-window specification (e.g. `NAME:ADC:LOW:HIGH` vs. separate flags) is left to the planning/ticket-breakdown stage, not fixed here.
- Default time-window(s) for the violin plot when the user supplies none is left to planning.

## Notes

- Follow-up ticket (explicitly out of scope here): `ruptures`-equivalent changepoint detection on time-binned counts (the notebook uses `rpt.Pelt(model="rbf").fit(counts).predict(pen=10)`), to be designed and scoped separately once load + rate-vs-time + violin plots ship.
- Domain terms (list-mode file, region of interest, time-binned rate) are recorded in `.work/GLOSSARY.md`.
- The `plotters`-dependency trade-off is recorded in ADR 0001, `.work/adr/0001-plotters-unconditional-dependency.md`.
