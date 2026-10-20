# Moulinette

Native Rust replacement for the statically analysed MPA path of moulinetteb.exe,
plus list-mode (`.lst`) acquisition-rate analysis.
Stable Rust 1.92 or newer and Cargo are required, with MSVC C++ build tools and
the Windows SDK for Windows MSVC builds. The original Windows
executable is never executed and is not a runtime dependency.

The Windows desktop GUI uses Slint 1.18.1 with a Fluent appearance, light/dark
themes and native file/folder dialogs. It inspects spectra, channel counts, ADC
identifiers and raw calibration/acquisition metadata when present. It offers
supported-format descriptions, all-spectra or single-spectrum selection, channel
bounds where supported, an output directory, filename previews, warnings,
generated-file lists and an Open output folder action. Parsing and conversion
run on worker threads with a busy spinner, without estimated percentages.
Concurrent conversions are prevented. Existing files are never overwritten,
including files created between validation and writing. Opening or dropping an
`.lst` list-mode file switches the window to the list-mode analysis panel
described below; opening an `.mpa` file switches back to the converter.

| Input | Extension | CLI | GUI |
|---|---|---|---|
| MPA measurement (spectra) | `.mpa` | `convert` to XY, EY, 1CO, PIX, PIG, MON, TOF, 8CO | Converter panel |
| MPA-3 list-mode events | `.lst` | `list`: per-ROI rate CSV, rate-vs-time PNG, optional violin PNGs | List-mode analysis panel |

Build and launch on Windows:

```powershell
cargo build --release --bins
.\target\release\moulinette-gui.exe
.\target\release\moulinette.exe formats
```

For development use `cargo run --bin moulinette-gui`. `cargo run` continues to
launch the CLI by default. For a CLI-only build without the GUI toolkit use
`cargo build --release --no-default-features --bin moulinette`.

```text
cargo run -- formats
cargo run -- convert res/15032501.mpa --format xy --spectrum 1 --output result.txt
cargo run -- convert res/15032501.mpa --format ey --spectrum 1 --output energy.txt
cargo run -- convert res/15032501.mpa --format xy --output-dir converted
```

Supported writer families: XY, energy/counts (EY), one-column counts
(1CO/PIX/PIG/MON), TOF and eight-column counts (8CO). PRO and RBS/NRA/ERD are
explicitly unsupported because their complete header/default behavior remains
uncertain. EY preserves counts; it does not calculate a normalized yield.

Select a one-based DATA occurrence using --spectrum when writing a single output
from a file containing multiple spectra. --start and --end select zero-based
inclusive channel bounds. --output-dir generates legacy mode extensions;
explicit --output uses the supplied path. Eight-column selections require
complete groups of eight; TOF emits the whole spectrum.
Existing output files are refused. Filesystem failures during writing may leave
partial files; multi-file output is not an atomic transaction.

Aliases include energy-yield for EY, pixe for PIX and pige for PIG. Automatic
classification from CMLINE1 is unsupported; select a format explicitly.

## List-mode (`.lst`) analysis

`moulinette list` parses an MPA-3 list-mode file (see
[the list-mode format notes](docs/lst_format.md)), computes the time-binned
acquisition rate inside each region of interest (ROI) and writes plots:

```text
moulinette list INPUT --roi NAME:ADC:LOW:HIGH[:unit] [--roi ...] --output-dir DIR [--dt-ms N] [--window START_S:END_S ...]
cargo run -- list run.lst --roi peak:1:20:35 --roi bg:2:100:200:ch --dt-ms 2000 --output-dir rates --window 0:60 --window 60:120
```

| Flag | Meaning |
|---|---|
| `--roi NAME:ADC:LOW:HIGH[:unit]` | Required, repeatable. ROI named NAME on ADC number ADC (as in the `[ADCn]` header). `unit` is `kev` (default: energy bounds resolved through CALOFF/CALFACT) or `ch` (inclusive channel bounds). |
| `--output-dir DIR` | Required. Created if missing. |
| `--dt-ms N` | Time-bin width in milliseconds (default 15000). |
| `--window START_S:END_S` | Optional, repeatable. Half-open time window in seconds, START_S < END_S. With at least one window, a violin plot of the per-bin rate distribution in each window is written per ROI. |

Outputs in DIR: `NAME.csv` per ROI (`time_s,rate,rate_err,counts`, one row per
bin, Poisson error), `rate_vs_time.png` with one line per ROI and, when windows
are given, `NAME_violin.png` per ROI. Existing files are never overwritten.
Energy ROIs require CALOFF and CALFACT for that ADC; non-zero CALFACT2/CALFACT3
is refused because only linear calibration is supported. Rates are counts per
second per bin without dead-time correction; the final bin can be partial.

The GUI's list-mode panel drives the same parser, rate and plotting code. After
an `.lst` file loads (parsing runs on a worker thread), the inspection panel
shows the event count and each ADC's channel count and calibration. Edit the
ROI list (name, ADC 1–8, low/high bounds, keV or Channel unit) with Add ROI and
Remove, set the time-bin width in milliseconds, optionally add violin time
windows (start/end seconds) with Add time window and Remove, then choose an
output directory. The expected file list, or the first validation error, updates
as you edit. Run list-mode analysis computes and writes everything on a worker
thread behind the same busy spinner and guard as conversion, so loading,
conversion and analysis never run concurrently. Every destination is checked
before anything is written, and each writer also refuses existing files. ROI
names become filenames, so they must be unique (case-insensitive) and contain no
path or reserved characters. ROI and window rows are kept when another `.lst`
file is loaded. Plots are written as PNG files and are not shown in the window.

The library preserves raw byte metadata, channel order and the observed
association of DATA occurrence 1 with ADC1, occurrence 2 with ADC2, and so on.
DATA suffixes do not choose calibration. Non-UTF-8 instrument text is retained.

Library entry points are `mpa::parse(&[u8])`, `mpa::read(BufRead, ParseOptions)`
and `conversion::convert(&Spectrum, Format, Selection)`. Parsing preserves raw
records; conversion returns output bytes or a structured error. `workflow` shares
loading, output planning, batch validation and exclusive writing between the
CLI and GUI. `gui_state` maps GUI settings without depending on Slint, allowing
direct testing. `src/gui.rs` owns state and worker orchestration; `ui/main.slint`
and `ui/components/` contain presentation compiled by `build.rs`.
CLI-only builds still depend on `plotters` (for `list`'s rate-vs-time PNG
output); they no longer need Slint, rfd or winsafe. Handwritten Rust avoids
unsafe code; generated Slint bindings use toolkit-managed unsafe code.
Calibration metadata is required only for EY. The parser accepts positive
channel counts up to a configurable default of 1,000,000 per spectrum; blank
and nonnumeric counts remain text. Metadata lookup applies the legacy first
80-byte view; conversion uses the first 20 count bytes and trims trailing spaces.

```text
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release --bins
```

Tests use synthetic inputs, existing samples and independently calculated
expectations. They verify the Rust implementation against static findings;
they do not prove equivalence with the original executable. Output uses CRLF;
legacy RTL rounding, x87 precision corner cases and malformed-input behavior
remain unvalidated. The replacement rejects unsafe selections.

The window scrolls vertically on smaller screens. On Windows, drop one `.mpa`
or `.lst` file onto the input panel to load it, just like Open; conversion or
analysis starts only when requested. Multiple files, folders and other file
types are rejected without replacing the measurement. Drops are unavailable
while work is running.

GUI limitations: no in-window plots, cancellation or estimated progress.
Select all spectra or one spectrum; arbitrary subsets are unavailable. Metadata
display uses lossy UTF-8 while the parser retains original bytes. The ADC
association follows DATA occurrence order, not the DATA suffix. EY uses CALFACT
and CALOFF from the file; calibration editing is unavailable. PRO, RBS, NRA and
ERD are unavailable in the GUI. Automatic classification and normalized yields
are not implemented. Legacy naming replaces the first dot in the basename and
supports occurrences 1–25; use CLI `--output` for other names/occurrences.
Multi-file writes are not atomic: I/O failures can leave earlier or partial files.
Native pickers are modal; folder opening is Windows-specific. Closing the GUI
while work is running can interrupt a batch. Outputs are rendered in memory
before writing, so large inputs can use substantial memory.

Slint has upstream licensing options; see [Slint licensing](https://slint.dev/license)
when distributing the application.

See [the investigation](docs/reverse_engineering.md),
[MPA parser specification](docs/mpa_format.md) and
[conversion algorithms and limitations](docs/conversion_algorithm.md).
Historical reference-execution plans are superseded by the no-execution decision.
