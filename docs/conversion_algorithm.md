# Reconstructed conversion algorithms

Static reconstruction and Rust implementation specification, 2026-10-13. Original execution is prohibited; historical reference-experiment proposals are superseded.
CONFIRMED findings are based on binary instructions, formatted-I/O descriptors,
and decoded resources. Runtime results, including literal file bytes, remain
unverified. Original source types/names were not recovered.

## Phase 4 implementation boundary

The original executable must not be launched. All proposals below for reference
runs, GUI experiments or runtime probes are historical unresolved questions;
they are not prerequisites or instructions for this implementation.

| Writer family | Rust support | Remaining boundary |
| --- | --- | --- |
| XY | Supported | I8 index, 5X, count text; no header |
| EY | Supported | F10.3 coordinate, 5X, count text; x87/RTL rounding unverified |
| PIX/PIG/MON/1CO | Supported | 1X plus selected count text; no header |
| TOF | Supported | 1X plus all counts; selection options do not apply |
| 8CO | Complete groups of eight | Reject selected intervals whose length is not divisible by eight |
| PRO | Explicitly unsupported | E10.4 RTL fidelity, missing-field defaults and header error recovery unresolved |
| RBS/NRA/ERD | Explicitly unsupported | Heading/default files absent; sanitizer and template writes unresolved |

Conversion applies character*20 truncation and trims trailing ASCII spaces.
Count text is not interpreted numerically. A10 fields add spaces on the left
to fill ten positions; longer values contribute their first ten bytes. There
are no explicit delimiters. Original RTL edge cases remain unverified.

EY reads coefficients as f32, evaluates multiplication and addition using f64
intermediates, then stores f32 before fixed formatting. This approximates the
observed x87 sequence; extended precision and fixed-format ties remain uncertain.
No beam, charge, time or solid-angle normalization is added. CRLF is a deliberate
Rust output convention, not an original-output byte capture. Invalid selections
are rejected instead of reproducing unchecked legacy memory reads.
Rust decimal formatting uses nearest-even rounding as supplied by Rust's
formatter, retains a negative zero sign and uses asterisks for an overwidth
I8/F10.3 field. The legacy decimal ties, signed-zero and overflow bytes are
unverified. Finite coefficients accept ordinary decimal/E notation and a D
exponent mapped to E; the full Fortran internal-read grammar (blank/null fields,
implicit exponents, repeat factors and malformed records) is not emulated.
Nonfinite coefficients or a coordinate overflowing f32 return structured errors.

Explicit-mode filename counters include initialized slot zero: first XY .XYb,
next .XYc; likewise EY .EYb, 1CO .1Cb, TOF .TOb, PIX .PIb and 8CO .8Cb.
Explicit --output bypasses generated names. Rust isolates the input basename
before finding its first dot, avoiding the legacy directory-dot buffer bug.
Static filename reconstruction does not establish original overwrite behavior.
The CLI refuses existing outputs. Batch rendering completes before file creation;
a later filesystem/write failure can still leave partial files, and batch output
is not an atomic transaction. Generated suffix letters are limited to DATA
occurrences 1 through 25; higher occurrences require explicit --output paths.

Rust requires calibration metadata only for EY. Legacy pre-dispatch calibration
reads are omitted for text modes. Automatic CMLINE1 classification and the MON
override are not implemented; explicit format selection determines the writer.
The alias energy-yield selects EY, while pixe and pige select PIX and PIG.

Synthetic and sample-based Rust tests establish implementation consistency
with these specified rules. They do not prove original executable equivalence.

## Reader and writer separation (static evidence)

```text
open input as formatted text, unit 10
initialize output type[0..8] from selected mode; data_ordinal = 0
while not end-of-file:
    read metadata record as character*80
    recognize key and store selected metadata as character*80 indexed by ADC
    on [ADC...]: internal-read the ADC index
    on [DATA...]:
        extract count text after comma; ignore numeric DATA suffix
        data_ordinal += 1
        load current metadata from ADC table[data_ordinal]
        internal-read CALOFF and CALFACT into 32-bit floats
        derive filename using current type and prior matching types
        rearrange date text
        dispatch writer(type[data_ordinal])
        # Writer reads exactly N channel records from the still-open unit 10.
close input
```

The scanner is `0x406be6`; supporting memory locations and recognition details
are in [mpa_format.md](mpa_format.md). Each dispatched writer processes one
spectrum independently; there is no channel-wise combination of spectra in
these seven routines. Many channel values remain character*20 rather than
numeric integers. This distinction matters for spaces, leading zeros, exponent
text, and values that would overflow a Rust integer.

## Complete reader dispatch inventory

All branches of the dispatch at `0x407f7a..0x408280` were inspected. These are
the seven application writer entry points reached by this MPA reader.

| Internal mode | Writer | Data array: field i begins at base + 20*i | Output records |
| --- | --- | --- | --- |
| RBS, NRA, ERD | 0x4118a0 | 0x4d9334 | Scientific numeric header, then selected text counts |
| TOF | 0x4163ca | 0x529d4c | All N counts as (1X,A) |
| PRO | 0x41662d | 0x57a670 | TMCA-style C V header, then selected counts |
| PIX, PIG, MON, 1CO | 0x41753c | 0x5caf88 | Selected counts as (1X,A) |
| 8CO | 0x4178bb | 0x61b940 | Eight fixed text columns, (8A10) |
| XY (blank-padded) | 0x417ed8 | 0x66c258 | (I8,5X,A): channel, original count text |
| EY (blank-padded) | 0x41827c | 0x6bcb70 | (F10.3,5X,A): calibrated coordinate, count text |

AUT, ASC, CAP, GAZ and other unmatched codes have no writer branch here.
That does not prove the application has no other input-format converters;
SPC/TXT and CAP readers at 0x4082d1 and 0x408a31 are outside this MPA path.

The format strings above are reconstructed from compiled descriptors, not
guessed from menu captions. `0x4360b0` decodes those descriptors using the table
at `0x496a00`; `0x42bda0` dispatches output formatting. Evidence is retained in
`format-region.json`, `format-table.json`, `format-decoder.json`,
`float-boundary.json`, and `float-jump-table.json` in [phase2](evidence/phase2/).
In particular, token 0x21 maps to floating format primitive 0x21 and the
formatter's fixed-point flag 2 at 0x42c0d3: F10.3. Token 0x1d maps to fixed
character widths, 0x24 to integer fields, 0x0e to X spacing, and 0x48 to A
using the passed character length. `decoded-formats.json` includes experimental
decodes at some arbitrary addresses; only the descriptor entry addresses cited
here are used as conclusions.

## Channel selection and text output

Let N be the internally parsed declared channel count, and S,T the selected
zero-based inclusive start/stop at 0x4d3868 / 0x4d3864.

```text
mode 1: S=0; T=N-1                  # Full spectrum / Other Respect file
mode 2: S=0; T=internal_read(K)-1    # First points / Other User
mode 3: S=internal_read(min); T=internal_read(max)  # Roi
read N character*20 records into writer-private array[1..N]
for i=S+1 .. T+1 inclusive:
    count_text = array[i][0 .. LEN_TRIM(array[i])]
    emit count_text according to the writer's FORMAT
```

RBS, ERD and NRA use separate option globals (0x4d3968, 0x4d3964,
0x4d3960); other writers use 0x4d395c. These all default to 1 in startup
0x406a50. TOF emits all N regardless of these options. PRO prints N in its
LENGTH header even when the subsequent output is cropped; the scientific writer
prints T-S+1 as its data length. No observed guard clamps the selected bounds
to [0,N-1]. Unknown runtime/memory outcomes require safe probes in isolation.

One-column writers add a leading blank through 1X. They remove trailing spaces
only, so original leading blanks can remain. Eight-column output uses eight
adjacent A10 fields (no comma or explicit delimiter). Its row loop advances i
by 8 and copies all eight fields even for a partial last row. **CONFIRMED:**
there is no last-row guard. **UNKNOWN:** the resulting stale/initial bytes for
positions beyond N, and whether cropped output exposes neighboring input values
past T. The nine-channel fixture tests both cases. Exact CRLF, trailing blanks
and final newline behavior require byte captures from the actual RTL.

## XY and energy/yield

**CONFIRMED (0x41817b..0x4181a6):** XY prints i-1 as I8, then five spaces and
trimmed count text. The first un-cropped channel coordinate is 0.

**CONFIRMED (0x41852d..0x41854f):** EY computes:

```text
E = float32(i * slope + offset)     # i is the one-based array index
emit E using F10.3, then 5X, then count_text using A
```

The input coefficients are stored as 32-bit floats. Assembly uses FILD for the
32-bit integer index, FMUL/FADD in x87 registers, and FSTP to a 32-bit float
before output formatting. It is unsafe to assume two separate float32 roundings
or a modern fused multiply-add is equivalent. With slope 2.5, offset 1.25,
the first coordinate is 3.750 rather than 1.250. CALFACT2, CALFACT3, CALUSE and
CALUNIT do not participate in this EY calculation. Counts are not divided by
live time, beam current, solid angle, charge, or energy interval in this routine.
Calling the second column a normalized yield would introduce an unsupported
scientific formula.

No numeric comparison tolerance is established without a reference run.
Aim for exact F10.3 bytes; if testing internal values later, separately account
for input float32 quantization and the final 0.001 formatting resolution.
Overflow to asterisks and half-way rounding need explicit experimental cases.

## PRO / TMCA-style header

**CONFIRMED:** 0x41662d writes these labels, in order, before count data:

1. C V VERSION = 2.07
2. C V NAME = (substring after the last Windows backslash in the output path)
3. C V ID = (current sample, from CMLINE2 or defaults)
4. C V REALTI = trunc_toward_zero(float32_livetime * 1000)
5. C V TRUETI = trunc_toward_zero(float32_runtime * 1000)
6. C V DEADTI = the same runtime-derived integer as TRUETI
7. C V CAL1 = parsed CALOFF
8. C V CAL2 = parsed CALFACT
9. C V CAL3 = parsed CALFACT2
10. C V ACMODE = Pha
11. C V SPECTYPE = Single
12. C V LENGTH = N
13. C V SUBNUMBER = 00000000
14. C V YAXIS = Energy keV
15. C V YAXIS = Counts

Header FORMAT descriptors at0x49c5f4, 0x49c600/0x49c61c,
0x49c64c/0x49c684/0x49c6a8, 0x49c6e0/0x49c718/0x49c750 and0x49c78c
give, respectively: (A), (A,1X,A), (A,1X,I8.8), (A,1X,E10.4),
and (A,I8.8) for the VERSION, NAME/ID, time, calibration and LENGTH records.
Time integers have at least8 digits (zero padding); LENGTH follows its label
without an extra X. These descriptors are retained in decoded-pro-formats.json;
the exponential primitive0x1f routes to formatter flags5 (exponential) rather
than fixed-point flags2 used by0x21. Counts still use (1X,A).

The duplicated YAXIS spelling and DEADTI's reused value are literal static
findings. Do not silently correct them to XAXIS or runtime-livetime.
The exact field spacing of these headers is not yet characterized end-to-end.
REALTI's source is LIVETIME, not the sample's separate REALTIME field.

Phase 4 assembly review resolves NAME's separator: the compare at
0x416797..0x4167af uses the literal byte 0x5c at 0x49cc6c, retaining the
position after each backslash. Forward slash is not included in this scan.
This is a bytewise Windows-path rule rather than a portable basename operation.

For truncation, the assembly explicitly sets x87 control-word rounding bits to
0xc00, performs FISTP into a 32-bit integer, then restores the control word.
The decompiler's `ROUND(...)` display is misleading here. Constant 0x49cd3c
contains float32 1000.0. This supports scaling by 1,000; milliseconds are an
**INFERRED** interpretation, not independently verified format semantics.

Header I/O checks status and jumps to 0x4171c3 after a failing step, skipping
later header fields. The count-reading/output path still follows. Blank/malformed
header metadata may therefore generate an incomplete header, rather than a
uniform hard failure. Input-open and count failures can take other RTL paths.

## RBS / ERD / NRA scientific writer

`0x4118a0` handles all three. It selects the corresponding heading filename
and channel-selection settings, reads defaults from INI_PATH, loads fields
missing from input, and presents HEADING resource 102 for edits. Fields supplied
by MPA and defaults must be tested together: the code compares string lengths
to 80 in some missing/default checks, so blank and untouched storage may behave
differently. The current repository does not supply the heading templates.

The primary output sequence is **CONFIRMED at statement level**:

| Record | Data | Reconstructed FORMAT / evidence |
| --- | --- | --- |
| 1 | VERSION 1.2 | (1X,A11), descriptor 0x49bf54 |
| 2 | Sample | (1X,A80), 0x49bf74 |
| 3 | Rearranged CMLINE0 date | (1X,A80), 0x49bf94 |
| 4 | Incident energy | (1X,F10.3), 0x49bfc8 |
| 5 | Preset charge | (1X,F10.3), 0x49bffc |
| 6 | CALOFF, CALFACT | (1X,2F10.3), 0x49c044 |
| 7 | Incident angle, adjusted detector angle | (1X,2F10.3), 0x49c08c |
| 8 | Detector solid angle | (1X,F10.3), 0x49c0c0 |
| 9 | Literal float32 1.0 | (1X,F10.3), 0x49c0e0; scientific meaning UNKNOWN |
| 10 | LIVETIME then RUNTIME text | (1X,A,6X,A), 0x49c100 |
| 11 | Selected start channel | (1X,I12), compiler-default INTEGER*4 width from RTL table |
| 12 | Detector resolution | (1X,F10.3), 0x49c154 |
| 13 | Current | (1X,F10.3), 0x49c188 |
| 14 | Literal integer 1 | (1X,I1), 0x49c1a8; meaning UNKNOWN |
| 15 | Particle code selected by comparisons | (1X,I1); literal spelling-to-code table below |
| 16 | Number of output points T-S+1 | (1X,I5), descriptor 0x49c408 |
| Following records | Selected original count text | (1X,A), descriptor 0x49c434 |

The last two descriptors are addressed by the retained writer assembly. Record11's
bare-I opcode uses RTL table0x496b2b[type9] =12 for INTEGER*4, confirmed from
the retained bytes and write-transfer switch. Runtime record treatment still
needs capture.
Particle-code comparisons at0x4141bd..0x4148fb use these literal spellings:

| Code | Accepted blank-padded spellings |
| --- | --- |
| 0 | 4HE++, HE++ |
| 1 | 4HE+, 4HE, HE, HE+ |
| 2 | 1H+, H+, 1H, H |
| 3 | 3HE+, 3HE |
| 4 | D+, D, 2H, 2H+ |
| 5 | 3HE++ |
| 1 | Any unmatched spelling, through fallback0x414924..0x41495d |

The mapping is **CONFIRMED** by string bytes (particle-literals.json) and
the immediate output integers in asm-writer.json. Physical meanings of these
destination codes are **UNKNOWN** until the destination format is identified
or reference documentation is supplied. Particle text is uppercased when read
from CMLINE3; case handling for subsequent manual edits needs a GUI experiment.

**CONFIRMED:** RBS subtracts float32 180.0 from the parsed detector angle at
0x413cde..0x413cea. ERD/NRA do not take that subtraction branch. The interpretation
of the source/destination angular convention is **UNKNOWN**. No counts are
changed by this header conversion.

Numeric header fields pass through `0x416263`: left-adjust text, locate the
decimal point/first blank, compare with caller limit 7, and in selected cases
replace the text with `0.` or insert a decimal point. Assembly in
`sanitize-asm.json` is more reliable than its recovered signature. This is an
observable numeric-text fallback, not a justified physical default. Large,
integer-only and exponent-form values need focused probes.

CMLINE0 is rearranged bytewise at 0x407e66..0x407ee9: swap its first two
two-character components, put '-' at positions 3 and 6, move two year bytes
from positions 9..10 into 7..8, and blank selected later positions. For the
sample prefix 03/25/2015 this yields 25-03-15; remaining date/time tail behavior
is bytewise, not a general date parser. No timestamp arithmetic is involved.

After writing the primary file, the routine reopens it to read its first
record, then opens unit 30 for output using paths assembled from INI_PATH and
the selected RBS/ERD/NRA heading filename; it also writes experiment metadata
through the configured EXP filename. **CONFIRMED:** this path can update the
heading/template files, not merely create a companion beside each spectrum.
There is no observed basename-plus-.ini rule. Template outputs contain named
DETECTOR_TYPE, DETECTOR_ANGLE, DETECTOR_OMEGA, DETECTOR_RESOLUTION and experiment
SAMPLE, PART_TYPE, INCIDENT_ENERGY, CURRENT, INCIDENT_ANGLE, PRESET_CHARGE records.
Whether missing templates permit a first successful run, overwrite/truncation
semantics and exact template spacing remain **UNKNOWN**. Experiment manifests
capture ini/ as well as output/ for this reason.

## Naming

**CONFIRMED (0x407cd7..0x407e61):** filenames are mutated in a fixed 275-byte
buffer. The reader finds the first dot, replaces the extension with the
three-character mode, and counts prior equal mode entries over indices
0..data_ordinal-1. If the count is nonzero, replace the third extension byte
with character 0x61+count. This is a type-based counter, not filesystem collision
detection. It does not establish overwrite protection.

Thus a first AUT-classified RBS can use .RBS and a second .RBb. Explicit RBS
initializes even type[0] to RBS, so its first output is statically predicted
to use .RBb, followed by .RBc. Same principle applies to .8C?, .1C?, .PI?,
.PR?, .ER?, .NR?, .XY?, .EY?. These are **INFERRED generated filenames until
runtime-tested**. A dot in a directory component can confuse the first-dot
algorithm; use a case directory without dots during initial tests.

## RTL and outstanding verification

Observed role assignments: 0x421290 opens; 0x423520 starts formatted external
read; 0x423a70 transfers read items/ends a read; 0x424af0 performs an internal
formatted read from character data; 0x426090 starts formatted write;
0x426690 transfers write items/ends a write; 0x425e20 closes;
0x4233b0 provides EOF status. These roles are strongly **INFERRED** from repeated
call sites and RTL bodies; original exported Fortran names were not recovered.

The high-priority discrepancies and exact experiments are listed in
[reference_experiments.md](reference_experiments.md). There is no claim of
full input/output equivalence until those experiments produce reference bytes.
