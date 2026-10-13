# Moulinette

Native Rust replacement for the statically analysed MPA path of moulinetteb.exe.
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
including files created between validation and writing.

Build and launch on Windows:

```powershell
cargo build --release --bins
.\target\release\moulinette-gui.exe
.\target\release\moulinette.exe formats
```

For development use `cargo run --bin moulinette-gui`. `cargo run` continues to
launch the CLI by default. For a dependency-free CLI-only build use
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
CLI-only builds need no external dependencies. Handwritten Rust avoids unsafe
code; generated Slint bindings use toolkit-managed unsafe code.
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

GUI limitations: no drag-and-drop, plots, cancellation or estimated progress.
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
