# MPA format and the legacy reader

Status: static reconstruction, 2026-10-13. CONFIRMED means observed in the
sample or binary, not runtime-tested. INFERRED means a likely interpretation;
UNKNOWN means no sufficient evidence. This specifies the converter's behavior,
not every format accepted by the instrument vendor.

## Phase 4 parser policy

The Rust library separates parsing from conversion, retains raw byte metadata,
unknown keys, section order and channel records, and avoids imposing UTF-8 on
instrument text. Each DATA occurrence snapshots the current matching one-based
ADC metadata; its numeric suffix is informational. Parsing does not insert
scientific defaults or interpret count records as integers.

The sample contains 8192, 8192, 8192 and 256 channels. The synthetic fixtures
exercise smaller counts, reordered DATA identifiers and metadata layouts.
The reusable Rust parser is not limited to the legacy nine type slots or the
unconfirmed capacity inferred from static array spacing. Missing data, invalid
section syntax and declared-count mismatches receive structured diagnostics.
ADC indices and declared channel counts must be positive. The default resource
limit is 1,000,000 channels per spectrum, configurable through ParseOptions;
this is a Rust allocation policy, not a recovered Fortran capacity. Blank and
nonnumeric channel records remain valid text. A section encountered before all
declared records have been consumed signals truncated data.
Conversion validates selected intervals before indexing and applies the observed
20-byte channel-field limit; metadata remains available for independent callers.

Record.bytes preserves the whole record after removing its CRLF/LF terminator.
Record.key_value exposes a legacy view of only the first 80 bytes, uppercases
ASCII keys and removes trailing ASCII spaces from the key. Spectrum metadata
lookup uses the last occurrence of a matching key. Metadata is snapshotted when
the corresponding DATA header is encountered; later ADC edits do not alter
already parsed spectra.
The scanner's current ADC association persists across DATA and unknown section
headers until another ADC header appears. Rust keeps that association for
metadata encountered between DATA blocks; global records before the first ADC
are preserved without inventing a per-ADC calibration inheritance rule.

Only EY conversion requires CALOFF and CALFACT in Rust. Legacy assembly parses
both before dispatching even text-only formats. Removing that unnecessary
precondition for text formats is a deliberate usability difference. No AUT
classification or explicit-mode CMLINE1 MON override is implemented: the CLI
format is applied directly to every selected spectrum.

Malformed-input rejection and missing-field diagnostics are Rust design choices.
The original scanner lacks equivalent observed guards and can preserve state
between files. That unsafe, stateful behavior is not reproduced. Historical
execution proposals below are superseded: no original reference run is allowed.

## Carrier and section structure (static evidence)

**CONFIRMED (sample):** `res/15032501.mpa` is byte-oriented, CRLF-delimited text.
It contains global key/value records, `[ADC1]` through `[ADC8]`, then
`[DATA0,8192 ]`, `[DATA1,8192 ]`, `[DATA2,8192 ]`, `[DATA3,256 ]`.
The four data sections contain one decimal count per line and exactly their
declared number of channels. Three bytes 0xb0 occur in detector labels, making
the sample invalid UTF-8. A Windows-1252/Latin-1 degree sign is an encoding
interpretation, not proof of the producer's code page.

**CONFIRMED (reader assembly):** the reader uses formatted character I/O on
Fortran unit 10. Its metadata buffer is 80 bytes at `0x4d73fc`. Channel readers
use character records of width 20. It does not select binary I/O when encountering
the sample's `mpafmt=asc`; the `MPAFMT` key is not recognized by this reader.
`FMT` is recognized and copied as a three-character field, but it is not a
binary-format switch. Binary MPA variants, encodings beyond byte text, and
unformatted record markers remain **UNKNOWN**.

## Recognition algorithm

Primary evidence: [reader assembly](evidence/phase2/reader.asm.txt), REA
`ev_d6946758dad406ce0c5d496eb805a21025ce8d28064d8ad7b671611d7f0babdd`;
string helpers are in `upper.json`, `trim.json`, `compare.json`, `index.json`.

1. Read an 80-character record. Find the first `=`; record an offset one past
   it, or zero if absent. Compute length excluding trailing ASCII spaces.
2. Uppercase ASCII `a` through `z` in the key/prefix region only using
   `0x40bc08`. Find `=` again. Construct the key from bytes preceding it.
   Equality uses Fortran blank padding, not C NUL termination.
3. Recognized metadata values are copied to fixed-width character fields;
   they are generally not parsed numerically at this point. `FMT`, `CMLINE1`,
   and `CMLINE3` have additional uppercase handling.
4. If the record begins with `[ADC` (four-byte compare at `0x407993`), parse
   the suffix starting at byte 4 with an internal integer read. The substring
   length excludes the presumed last closing bracket. Store its signed
   32-bit index at `0x4d46c0`. No bracket validation or index bounds check is
   visible in this application routine.
5. If the record begins with `[DATA` (five-byte comparison at `0x407a15`), find
   the first comma, extract the text after it while dropping the final
   nonblank byte (presumed `]`), and store the count text at `0x4d396c`.
6. Increment a separate data-section ordinal `0x4d46cc`. Use this ordinal to
   retrieve metadata and output type; the numeric suffix in `DATA0` is not
   parsed. Thus association is by occurrence, not by the DATA identifier.
7. Parse CALOFF and CALFACT to REAL*4 globals; generate a spectrum filename;
   dispatch the selected writer. That writer consumes the following N records
   from unit 10. Scan subsequent metadata/section records after it returns.

These prefix tests permit trailing characters; they do not implement a general
INI parser. Leading spaces before a section prefix do not match the literal
tests. A key's trailing spaces can compare equal through blank padding;
leading spaces are not removed by the reader. Exact tab handling remains
**UNKNOWN**. Character I/O normally pads short records and truncates long
ones; how this RTL consumes long records and handles premature EOF must be
experimentally checked before setting parser compatibility rules.

## Metadata recognized by this executable

Each table entry occupies 80 bytes. Address expressions below are
`base + ADC_index * 0x50`, with one-based ADC indices from the input.
The code does not store all instrument metadata.

| Key | Table base | Current field | Role / confidence |
| --- | --- | --- | --- |
| CMLINE0 | 0x4d712c | 0x4d3e6c | Date text, later rearranged; CONFIRMED |
| CMLINE1 | no general comment-text table | type table 0x4d46e0, 3 bytes per index | Output classification rather than sample name; CONFIRMED |
| CMLINE2 | 0x4d6e5c | 0x4d3e1c | Sample field, also GUI edit 1011; CONFIRMED |
| CMLINE3 | 0x4d6b8c | 0x4d3dcc | Particle type, uppercased; CONFIRMED |
| CMLINE4 | 0x4d68bc | 0x4d3d7c | Incident energy; CONFIRMED by GUI binding |
| CMLINE5 | 0x4d65ec | 0x4d3d2c | Current; CONFIRMED by GUI binding |
| CMLINE6 | 0x4d631c | 0x4d3cdc | Incident angle; CONFIRMED by GUI binding |
| CMLINE7 | 0x4d604c | 0x4d3c8c | Preset charge; CONFIRMED by GUI binding |
| CMLINE8 | 0x4d5d7c | 0x4d3c3c | Detector type; CONFIRMED by GUI binding |
| CMLINE9 | 0x4d5aac | 0x4d3bec | Detector angle; CONFIRMED by GUI binding |
| CMLIN10 | 0x4d57dc | 0x4d3b9c | Detector solid angle; spelling is CMLIN10, not CMLINE10; CONFIRMED |
| CMLIN11 | 0x4d550c | 0x4d3b4c | Detector resolution; spelling is CMLIN11; CONFIRMED |
| CALOFF | 0x4d523c | 0x4d3afc | REAL*4 offset at 0x4d46c8; CONFIRMED |
| CALFACT | 0x4d4f6c | 0x4d3aac | REAL*4 slope at 0x4d46c4; CONFIRMED |
| CALFACT2 | 0x4d4c9c | 0x4d3a5c | Text carried to PRO CAL3 header; CONFIRMED |
| RUNTIME | 0x4d49cc | 0x4d3a0c | Text carried to output; CONFIRMED |
| LIVETIME | 0x4d46fc | 0x4d39bc | Text carried to output; CONFIRMED |
| FMT | scalar 0x4d46b4, 3 bytes | passed to some writers | Stored uppercase; passed argument not used in inspected scientific writer; CONFIRMED within that routine |

`range`, `active`, `realtime`, `calfact3`, `calunit`, `caluse`, `roimin`,
`roimax`, `roi`, totals and sums are not handled by this reader's key dispatch.
Their meanings in the sample may be inferred from labels, but they must not
be silently promoted into conversion formulas. Counts are not checked against
TOTALSUM. ADC activity does not control DATA processing here.

## Automatic classification and section association

The default type from the caller is copied to nine entries (indices 0..8).
For `AUT`, CMLINE1's entire trimmed uppercase value is compared, with blank
padding, to RBS, NRA, ERD, TOF, PIXE, PIGE, RNRA. PIXE/PIGE/RNRA are reduced
to PIX/PIG/PRO. These are equality comparisons, not substring matching.
For an explicit type, CMLINE1 with first three value bytes `MON` overrides
that ADC to MON; the AUT branch does not perform this MON override.

Consequently the sample's `RBS135°`, `RBS165°`, `RD30°` and `1D` do not meet
the AUT equality checks. **INFERRED runtime outcome:** the unmodified sample
may produce no spectrum outputs under AUTOMATIC. No experiment has confirmed
this yet. Using exact labels is a separate, prepared experiment.

**CONFIRMED:** DATA occurrence 1 uses metadata index 1, occurrence 2 uses
index 2, etc. An input with DATA7 as its first data section still retrieves
ADC1 metadata. Reordering or missing sections can silently associate the wrong
calibration and labels. There is no observed spectrum combination or sum.

## Missing and malformed input

**CONFIRMED:** most fields remain text until needed by a writer. CALOFF and
CALFACT are numerically read before every writer dispatch, including text modes.
Unknown records are skipped. No application-side validation of ADC bounds,
channel bounds, negative N, maximum N, comma presence, numeric overflow, or
consistency is visible in the reader and seven writers.

The per-ADC metadata tables are not cleared in this reader on each new file.
Only selected current-field strings and type entries are reset. **INFERRED:**
missing metadata can depend on initial storage and prior files in the same
process. A universal missing-field default is therefore **UNKNOWN**.

Many I/O calls use flags 0x0784ff00 (without IOSTAT-style status handling) and
their return values are ignored. The RTL routes errors through 0x429d70 / 
0x429da0. **INFERRED:** malformed numeric metadata or insufficient records can
raise a runtime error, potentially after creating partial output files. Exact
dialogs, exit codes, cleanup and whether a final record needs CRLF are
**UNKNOWN**. The PRO header is different: its flagged reads/writes check status
and skip the remainder of the header after failure, then continue to data logic.

**UNKNOWN:** the true maximum channel count. Writer arrays use 20-byte strides
and roughly 330 kB blocks, suggesting capacity near 16,500 fields, but the source
declarations and exact limit have not been established. Do not use that
inference as a validated safe capacity.

See [conversion_algorithm.md](conversion_algorithm.md) for writer behavior and
[reference_experiments.md](reference_experiments.md) for validation cases.
