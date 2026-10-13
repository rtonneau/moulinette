# Moulinette: initial reverse-engineering report

Historical Phase 1 reconnaissance, 2026-10-13. At that stage no converter had been implemented
and the original executable has not been launched. Static evidence establishes
format and UI candidates, not conversion equivalence.

## Phase 4 status and execution boundary

The user explicitly prohibits executing moulinetteb.exe, configuring a VM or
Windows Sandbox, and requesting reference executions. All earlier experiment
plans in this report and reference_experiments.md are superseded. Saved REA
evidence, Ghidra assembly/decompilation and MPA fixtures are the specification
sources. Static inspection never executes the target.

The native Rust replacement provides an ordered byte-preserving MPA parser,
XY and EY conversion, one-column count modes, TOF and complete-row 8CO output.
PRO and RBS/NRA/ERD remain explicitly unsupported where header/runtime or
missing-template behavior cannot be reconstructed reliably. The seven writer
families and remaining uncertainties are consolidated in
[conversion_algorithm.md](conversion_algorithm.md).

Three claims remain distinct: binary instructions and descriptors confirm
specified static behavior; automated Rust tests verify the replacement against
that specification and independently calculated fixtures; original byte-level
equivalence and exceptional RTL behavior remain experimentally unvalidated.
No test result is claimed to establish equivalence. No scientific formula is
invented to fill an evidence gap. Resources under res/ remain immutable.

Phase 4 validation on stable Rust/Cargo 1.93.1: `cargo fmt`,
`cargo clippy --all-targets -- -D warnings`, `cargo test` and
`cargo build --release` pass. Seventeen tests cover thirteen library cases,
CLI argument validation and three executable integration cases. Integration
tests launch only Cargo's newly built Rust binary. Resource SHA-256 values
still match the historical table below. Rust 1.85 is the declared minimum;
the minimum version was checked by Clippy's compatibility lint, not a separate
1.85 toolchain run.

REA/Ghidra was used only for static inspection. The Phase 4 EY assembly query
returned evidence `ev_b0b50ec85295d9597beacbfb4e86782b3edea140266a169d12ef2d87566dcaf9`
from Ghidra 12.1.4 for the matching executable hash. The retained Phase 2
[EY assembly](evidence/phase2/0x41827c.json.asm.txt) shows the decisive
FILD/FMUL/FADD/FSTP sequence at 0x41852d..0x418549.
The native analysis session was closed after inspection. No original execution,
VM, Sandbox configuration or reference run was requested.

## Repository and immutable reference material (historical)

The initial repository contains Cargo.toml (edition 2024, no dependencies), a
placeholder src/lib.rs, and these three resources. No example outputs, scientific
documentation, or heading templates were found in the repository file inventory.
No applicable AGENTS.md was found in the workspace or the checked C:/DEV ancestors.

| Resource | Bytes | SHA-256 |
| --- | ---: | --- |
| res/moulinetteb.exe | 782336 | 642a46517364a73f4fe4ab3906f94078562c92201e149fbdbc2f9fd4704f0e55 |
| res/15032501.mpa | 96819 | f479738b4bc90e1b0ca9e8756810ae90ad110d414deed9f639e555b76d9bd0e7 |
| res/moulinette.ini | 954 | 2495015b3c41e9772c6f2a636509cf7a1f1c058345c237e1016d6a8ae209f801 |

All inspection reads res/ without modifying it. Generated evidence is under
[evidence/phase1](evidence/phase1). Reproduce local static inspection from the
repository root with `powershell -NoProfile -File tools/reconnaissance.ps1`.
This requires an existing llvm-objdump on PATH; it never launches the target.
The script is an inspection utility, not the proposed reusable MPA parser.

## Binary identity and runtime

Confirmed by REA open_binary and independently by LLVM PE inspection:

| Property | Finding |
| --- | --- |
| Binary format / machine | PE32, COFF i386 (machine 0x014c), native x86 |
| Subsystem | Windows GUI (2), not a console subsystem executable |
| Image base | 0x00400000 |
| Entry point | RVA 0x000481b0; linked VA 0x004481b0 |
| PE timestamp | 2002-03-19 12:46:15 as printed by LLVM; a header value, not verified build history |
| Linker version | 5.0 |
| Symbols | COFF symbols stripped; no export directory |
| Debug strings | Debug/moulinette.exe; U:\\fortran90\\filetreatment\\Debug\\moulinette.pdb |
| CLR header | Absent; no managed .NET runtime indicated |
| Imported DLLs | USER32.dll, GDI32.dll, comdlg32.dll, KERNEL32.dll |

See [PE headers and imports](evidence/phase1/pe-headers-imports.txt) and
[sections](evidence/phase1/sections.txt). The four imported DLLs are Windows
components; no external Fortran runtime DLL appears in the import table.
Dynamic loading remains possible: LoadLibraryA and GetProcAddress are imported.

Raw ASCII strings provide strong evidence of Digital Visual Fortran / DEC RTL
and QuickWin, with Microsoft C runtime support linked into the executable:

| File offset | Observed text |
| --- | --- |
| 0x95520 | Visual Fortran run-time error |
| 0x973ac | Microsoft Visual C++ Runtime Library |
| 0x9d258 | @(#)DEC Fortran RTL Message Catalog |
| 0xa62f8 | DIGITAL (R) QuickWin Version 5.00 |
| 0xa6384 | Visual Fortran 5.00 |
| 0x99bf4 | Version 24/08/2000 |
| 0x99c07 | Mathot Gilles |

Inference: a Digital Visual Fortran 5-era QuickWin application with statically
linked runtime code. These version strings do not prove the exact compiler
version used for every object. The embedded application version and PE timestamp
are distinct observations.

## REA capabilities and limitations (initial pass; superseded below)

REA successfully bound the requested executable and identified x86 PE with the
same SHA-256 as the local hash. `inspect_pe_resources` also succeeded, producing
Evidence `ev_33ba78ccada1e02239eaab6495b59c651eb9d95f40fd22a06828698b0a9b961f`.
The complete result is retained in [rea-pe-resources.json](evidence/phase1/rea-pe-resources.json).
Its coverage is 21 examined entries and nine resources: seven dialogs, one icon
and one icon group, all with language ID 2060 (0x080c). REA leaves dialog payloads
opaque; the separate local UTF-16 printable-run inspection is not REA decompilation.

REA binary_overview, search_strings and search_procedures returned
`provider_unavailable`. Diagnostics reported:

- Ghidra: GHIDRA_INSTALL_DIR unset; Ghidra 12.1.x installation and Java unavailable
  to the provider.
- Hopper: unsupported on win32.
- IDA: REA_IDA_MCP_CONFIG not configured.

Consequently analyzed symbols, procedure inventory, decompilation and provider
cross-references remain unavailable. No provider was installed or configured.
Local LLVM disassembly remains usable. No application function has yet been
identified confidently, and no scientific algorithm is claimed recovered.

## Expected functionality and interaction

Confirmed static UI evidence, with runtime behavior still unobserved:

- Dialog 101 contains input/output file and directory selectors, Finish/Cancel,
  and View/Rename/Delete/MkDir/RmDir controls. Dialog 110 additionally includes
  Param. conv. This is substantial evidence of interactive file selection.
- Dialog 103 exposes input/output/INI paths and RBS, ERD, NRA and experiment INI
  filenames.
- Dialog 109 exposes RBS/ERD/NRA/Other conversion choices including Full spectrum,
  First points, Roi, Min/Max, Respect file and User.
- Dialog 102 exposes Sample, Particle Type, Incident Energy (MeV), Current (nA),
  Incident Angle (degrees), Preset Charge (microcoulombs), Detector Type, Detector
  Angle (degrees), Detector Omega (msr), Detector Resolution (keV), and Start/Stop
  Channel. Units here are explicit labels, not reconstructed calculations.

See [dialog text](evidence/phase1/dialog-text.txt). Printable runs can include
incidental bytes; they do not establish control IDs, defaults, or event handlers.

Application-looking strings in the initialized .data section give tracing seeds:

| File offset / linked VA | Text / significance |
| --- | --- |
| 0x99d24 / 0x49b324 | [DATA: spectrum-section recognition seed |
| 0x99d34 / 0x49b334 | Adjacent LIVETIME and RUNTIME literals |
| 0x99d44 / 0x49b344 | Adjacent CALFACT2 and CALFACT literals |
| 0x99d54 / 0x49b354 | CALOFF |
| 0x99ddc / 0x49b3dc | CMLINE1 |
| 0x99de4 / 0x49b3e4 | CMLINE0 |
| 0x99f7c / 0x49b57c | *.MPA |
| 0x9a418 / 0x49ba18 | moulinette.ini |

The VA mapping above applies to these file-backed .data bytes only (offset +
0x401600), not to arbitrary offsets elsewhere in the executable. The previous
report incorrectly used +0x401000; current Ghidra string addresses correct it.
Adjacent Fortran literals are not necessarily single tokens or C strings.

Other input filter strings include *.MP, *.SPC, *.DAT, *.ASC, *.CAP and *.TXT.
Output candidates include *.RB?, *.ER?, *.NR?, *.PI?, *.PR?, *.8C?, *.1C?, *.XY?,
*.EY? and *.GAZ; labels include ASCII (1col.), ORIGIN; ASCII (8col.), TMCA;
En Yield; PIXE; PIGE; RNRA; AUTOMATIC. These are menu/filter candidates, not proof
that every format is implemented or of its output layout/naming rules.

The supplied moulinette.ini sets INPUT_PATH to Z:\\, OUTPUT_PATH to
X:\\Mesures_Sigma\\28Si\\travail, INI_PATH to U:\\data\\mpa3, and names RBS.INI,
NRA.INI, ERD.INI and EXP.INI. Those four templates are absent from res/.

Error strings include invalid path name (offset 0x99a44), Out of memory
(0x99a5c), and error dialog not found (0x99f9c). The large DEC runtime error
catalog is generic runtime evidence: its messages do not establish which errors
the application catches or displays.

## Command-line handling and entry point

[Entry disassembly](evidence/phase1/entry-disassembly.txt) shows initialization,
command-line scanning including quote/whitespace handling, and a call to
0x00461ef0 at 0x004482fd. This is a startup/GUI-runtime candidate, not yet the
identified Fortran conversion main routine. The PE imports GetCommandLineA and
GetStartupInfoA. Neither imports nor generic startup scanning prove a supported
batch conversion CLI. Supported arguments, interactive sequencing, console
output, error dialogs and exit codes all remain unobserved.

## Sample observations

The sample is line-oriented text with 25,053 CRLF pairs. It contains three bytes
0xb0 in detector labels; it is not valid UTF-8 as supplied. Those bytes match the
degree sign in Windows-1252/Latin-1; exact producer encoding is unknown.
`mpafmt=asc` and `fmt=asc` are present in its global header. This sample provides
no evidence about binary MPA variants, endianness or Fortran unformatted records.

Eight ADC metadata sections precede four data sections. ADC1–ADC4 have active=1;
ADC5–ADC8 have active=0. Each data line is one nonnegative decimal integer.

| Data section | Header line | Declared / observed channels | Sum | Maximum | Corresponding ADC TOTALSUM |
| --- | ---: | --- | ---: | ---: | ---: |
| DATA0 | 218 | 8192 / 8192 | 2102300 | 1440 | ADC1: 2102300 |
| DATA1 | 8411 | 8192 / 8192 | 3304096 | 2386 | ADC2: 3304096 |
| DATA2 | 16604 | 8192 / 8192 | 713758 | 64860 | ADC3: 713758 |
| DATA3 | 24797 | 256 / 256 | 139955 | 139224 | ADC4: 139955 |

All four spectra begin/end with zero and have minimum zero. See
[sample summary](evidence/phase1/sample-summary.json). The zero-based DATA to
one-based ADC correspondence is strongly supported by matching ranges and sums
in this sample; recognition and indexing inside the executable remain untraced.

Metadata includes caloff, calfact, calfact2, calfact3, calunit, caluse, realtime,
runtime, livetime, ROISUM, ROINET and comment lines. All observed calibrations are
identity-valued with caluse=0. A negative ROINET occurs in ADC1; metadata must not
be presumed unsigned simply because the observed channel counts are nonnegative.
No calibration, dead-time correction, normalization or unit conversion formula
can yet be concluded from this sample or the binary strings.

## Execution feasibility

The host is Windows x64, so this x86 executable is a plausible WOW64 candidate.
Its static imports are ordinary Windows DLLs. Actual loader success and runtime
compatibility have not been tested. The executable is a QuickWin GUI; redirecting
stdin/stdout may not exercise its interaction model.

No execution safety guarantee follows from static metadata. UI resources include
file deletion/rename/directory removal and the imports include DeleteFileA,
MoveFileA, RemoveDirectoryA and CreateProcessA. These are capabilities, not evidence
of malicious behavior or startup side effects. The supplied configuration points
outside the workspace. Launching it in res/ is therefore unsuitable for reference
testing. A temporary working directory alone is not a security isolation boundary.

Proposed controlled execution: fresh per-case experiment directory, copies of
the executable/sample, a configuration targeting only that directory, and a
snapshot of its file tree before/after. Preserve original hashes, impose bounded
timeouts, capture exit codes and UI observations, and avoid destructive UI controls.
Use a disposable Windows sandbox/VM if host filesystem access must be constrained.
No current run, generated reference output or exit code is claimed.

## Proposed next investigation

1. Make a deep provider available, preferably the REA-supported Ghidra 12.1.x and
   compatible Java, or an existing IDA MCP connection. This is an environment
   prerequisite, not an inferred request to install software globally. LLVM can
   continue providing manual disassembly in the meantime.
2. Trace the listed string VAs and conversion dialog handlers to application
   routines. Follow configuration loading, file opening, case normalization,
   [DATA recognition, metadata extraction and channel loops. Record function
   boundaries/addresses, evidence and clear pseudocode separately from hypotheses.
3. Establish the GUI's defaults and obtain baseline outputs in controlled runs.
   Determine what happens when the four external heading templates are absent;
   recover built-in defaults before assuming additional files are required.
4. Characterize each selectable output family and the automatic detector mapping.
   Test channel boundaries, first-points/ROI behavior, rounding, field widths,
   newline convention, file suffixes, and overwrite/error behavior.
5. Change one input field at a time in copied/synthetic fixtures: distinguish
   runtime/realtime/livetime; vary calibration coefficients and flags; use channel
   sentinels to establish order and inclusive/exclusive bounds. Probe missing,
   malformed, duplicated and truncated sections with bounded execution.
6. Implement separate Rust parser, conversion, formatting and CLI modules only
   once supported behavior is specified. Preserve raw metadata where semantics
   are unknown. Build deterministic fixtures and compare byte-for-byte outputs;
   justify numeric tolerances only where measured floating behavior requires them.
7. Expand this report and write mpa_format.md, conversion_algorithm.md and README.md
   as findings and the replacement become supported by evidence.

## Phase 1 checklist and open questions

| Question | Status |
| --- | --- |
| Repository and executable identification | Answered |
| Samples, configuration, outputs and documentation inventory | Answered within repository |
| Architecture, format, static dependencies | Answered |
| Likely compiler/runtime | Strongly inferred from explicit embedded runtime strings |
| REA target and resources | Answered |
| REA strings, symbols, functions and xrefs | Initial blocker resolved in current Phase1 verification below |
| Interactive selection and conversion options | Partially answered by static dialog text |
| Arguments, errors, defaults and exit codes | Unresolved; no reference execution |
| Safe/feasible execution | Plausible host compatibility; controlled-run conditions identified, untested |
| Conversion algorithms and precise output formats | Unresolved |

The reference-output collection is still empty. There is no scientific or
behavioral equivalence claim at this stage.

## Current Phase 1 verification: Ghidra available

The current REA session reports Ghidra 12.1.4 with a compatible Java 25 JDK.
The earlier provider-unavailable finding above is historical and no longer a
blocker. Opened res/moulinetteb.exe with provider ghidra and the existing matching
analysis snapshot; no files under res/ were written. Recomputed all three
reference hashes and confirmed the values above. The test_ghidra executable is
an identical copy, not a second version. The repository now also contains a
Ghidra project and snapshot, inspection tools, and this prior report; it still
contains only one MPA input and no reference conversion outputs. Rust remains
the Cargo starter library, with no converter.

Current retained evidence under evidence/phase1:

| Observation | Evidence / file |
| --- | --- |
| 1,194 analyzed procedures, 1,466 defined strings, eight mapped segments including Headers | ev_1b49f8c25316029c200b64f850a703f529953edfcd4566fdfff2834a1449751f; rea-overview-current.json |
| Metadata and configuration strings at corrected VAs | ev_8131ac669e5c1f54d9bbb169acc41cd2e8aeb4bec5ea01c8e94469476310a6e1; rea-strings-current.json |
| [DATA at 0x49b324 referenced from 0x407a20 and 0x407a26 | ev_97d4265a8e4ba8fdbdbe6a8d8adf1d94d7dc79dd5f3cca8506049fb47c96e2c5; rea-data-xrefs.json |
| LIVETIME literal referenced from 0x407934 | ev_2d1f3a912cbb854e73dcf07656bb11b920d8703a0638d6ba45d8e3e8536b25b1; rea-time-xrefs.json |
| moulinette.ini references at 0x40be94, 0x40bebc, 0x4102d5, 0x4102f4 | ev_13c0f686374643ef1a6786d8c0770189b635f9c057e6b8afc4c3aae120985ded; rea-config-xrefs.json |
| Reader candidate decompilation | ev_21ae729f3471567686fd4ef8ab9242df6e3a46f0db97f178b2038320f5cbe487; rea-parser-pseudocode.json |

Ghidra resolves the DATA references to FUN_00406be6, with an analyzed inclusive
body 0x406be6..0x4082d0 (5,867 bytes, 172 blocks; procedure_info Evidence
ev_8805e74aca9f57f8c6806214165ed9931fe962c484dc4c31b3047a8b517920eb).
Its pseudocode compares a four-byte prefix with 0x4344415b (little-endian [ADC)
and compares five bytes with [DATA. It uses 80-byte text buffers, maintains a
spectrum counter, and dispatches to routines including 0x4118a0, 0x4163ca,
0x41662d, 0x41753c, 0x4178bb, 0x417ed8, and 0x41827c. These are concrete
Phase 2 tracing seeds, not verified names or specifications of the output formats.

The decompiler warns of incomplete stack tracking, overlapping globals, and
removed unreachable blocks. Missing argument displays and inferred pointer
types must be checked against assembly and Fortran runtime calling conventions.
An 80-byte buffer observation does not prove an accepted line-length limit.
No exact numeric transformation, error policy, CLI contract, or output formatting
is established by this initial function inspection.

The revised strategy is to trace this reader and its output callees directly;
provider installation is no longer necessary. In parallel with static work in
the next phase, prepare isolated experiment copies and local configuration,
then obtain baseline GUI conversions and compare file trees, bytes, and exit
codes. Missing heading templates remain an experimental question. Use copied
sentinel spectra and change one field at a time to establish calibration,
channel bounds, rounding, and malformed-input behavior before implementing Rust.

The Windows x64 host is compatible in principle with x86 execution through
WOW64; target execution and GUI compatibility remain untested. Static analysis
does not establish confinement. Experiment copies must avoid the original INI's
external drive paths and destructive file-management controls. No executable
was launched during this verification, and no original resources were modified.
Phase 1 now has working REA strings, procedure metadata, xrefs, and preliminary
decompilation; command-line semantics and runtime behavior remain unresolved.

## Phase 2: static conversion reconstruction

Status: static investigation and experiment preparation completed for this pass,
2026-10-13. Original executable execution remains unperformed; essential runtime
input/output behavior is not yet characterized. No Rust implementation started.
CONFIRMED denotes direct static/sample evidence, INFERRED denotes interpretation
or a predicted runtime outcome, and UNKNOWN denotes an unresolved question.

Detailed specifications and evidence:

- [mpa_format.md](mpa_format.md): parsing pseudocode, metadata tables and limits.
- [conversion_algorithm.md](conversion_algorithm.md): all seven MPA-dispatched
  writers, output descriptors, precision and scientific transformations.
- [function_map.md](function_map.md): names, addresses, callback/call edges,
  confidence and saved Ghidra annotations.
- [reference_experiments.md](reference_experiments.md): trust assessment,
  exact manual GUI steps and validation matrix.

### Significant confirmed findings

| Finding | Supporting evidence |
| --- | --- |
| Reader0x406be6 recognizes first4 bytes [ADC and first5 bytes [DATA; keys use blank-padded equality; ASCII uppercase only | reader.asm.txt; ev_d6946758dad406ce0c5d496eb805a21025ce8d28064d8ad7b671611d7f0babdd; upper.json, compare.json |
| ADC index is parsed; DATA numeric suffix is ignored; metadata is retrieved by DATA occurrence | 0x407993..0x407aa5 and metadata-copy loops to0x407ccd |
| Metadata is mostly character*80; channel counts are read as character*20 by writers | Reader descriptor0x49b228 and writer0x49caa4 transfer descriptors; seven writer dossiers |
| CALOFF/CALFACT are read into REAL4 globals even before text output dispatch | Reader0x407c02..0x407c7e; slope0x4d46c4, offset0x4d46c8 |
| Automatic classification is exact full-label equality, not prefix matching | Reader0x40703e..0x4072cf; rtl_character_equal_blank_padded0x41cb60 |
| XY coordinates are zero-based; EY uses one-based index*slope+offset | XY0x41817b..0x4181a6; EY0x41852d..0x41854f; ev_b0b50ec85295d9597beacbfb4e86782b3edea140266a169d12ef2d87566dcaf9 |
| Output FORMATs: (1X,A), (8A10), (I8,5X,A), (F10.3,5X,A) | Compiled descriptors0x49c87c/0x49c8e8/0x49c950/0x49c9c8; format table0x496a00, dispatch0x42bda0; format-region.json and float-jump-table.json |
| PRO time fields multiply by1000, explicitly truncate toward zero; TRUETI/DEADTI reuse RUNTIME; REALTI uses LIVETIME | 0x416a08..0x416c63, float1000.0 at0x49cd3c; ev_6d7c7f6ed56a78e7422502adf227b444ec3138095eb7c1eb5339ac7fd86c9f29 |
| Scientific writer subtracts180.0 from detector angle only for RBS | 0x413ccb..0x413cea, constant0x49cb08; asm-writer.json |
| Scientific conversions read and can update configured heading/experiment INI files | Writer unit30 read/open/write sequences, INI_PATH0x4d3f0c and heading-name globals; pseudo-0x4118a0.json |
| Eight-column writer always copies8 values per row without a tail guard | 0x417b18..0x417e86; ev_dd113ed7ee80d00e8208a8ea1041702deba470057225ec1206e83143283eb052 |
| Main dialog110 => callback0x401d7e maps both ASCII captions to ASC; ASC absent from reader dispatch | main-asm.json / selected-asm.json at0x40201a..0x40208d; reader0x407f7a..0x408280 |
| Main Param.conv.1072 routes through thunk0x40115e to0x40cb38/resource109; heading resource102 Yes/Yes-to-all callbacks recovered | main-callback-map.json, structural dialog-controls.json, options-init.json, asm-writer.json |

No universal normalization, dead-time correction, cross-section calculation,
channel rebinning or count scaling was found in the inspected writer paths.
This is a bounded static conclusion, not a claim about every scientific format
or unrelated input reader. EY's second column is original count text. The
scientific interpretation of180-degree adjustment, literal header constants,
and the destination particle-code semantics must not be invented. Literal
particle spelling-to-code mappings are now tabulated in conversion_algorithm.md.

### Decompiler verification and persistent analysis

Several recovered calls omit arguments, float values appear as pointers, and
LEN_TRIM appears void despite its EAX result. These are calling-convention
artifacts. Metadata destinations, prefix checks, REAL4 storage, x87 coordinate
arithmetic and truncation were checked against assembly. PRO's decompiler
ROUND expression was corrected to explicit control-word truncation. Compiled
FORMAT descriptors were followed through the RTL table and floating dispatch
rather than treated as printable source FORMAT strings.

The scientific dossier completed but exceeded the MCP transport budget. Its
full record was exported without repeating analyze_function. The evidence bundle
at [phase2/rea-evidence-bundle.json](evidence/phase2/rea-evidence-bundle.json)
contains89 records and about23MB. The reproducible REA snapshot is
`analysis/phase2.rea-analysis.snapshot.json`, outside res/. Both retain limitations.
Neither is original Fortran source.

REA Windows P0 cannot mutate its Ghidra database. To satisfy persistent naming
while preserving res/, copied the existing saved project to
`analysis/ghidra/moulinette2.gpr`. That saved project had147 functions and lacked
the reader/writer boundaries present in REA's ephemeral analysis, so the copy
required analysis before annotations. Named21 functions and26 global variables;
the successful save is logged in [ghidra-annotations.log](evidence/phase2/ghidra-annotations.log).
The script checks binary SHA-256 and image base. Unreliably recovered locals
remain unnamed. Original files and project under res/ were not edited.

### Predictions and remaining unknowns

**INFERRED:** the supplied labels may make AUTOMATIC produce no spectrum files.
Both main-GUI ASCII choices may likewise produce no MPA outputs, because of
their ASC code. These are important fidelity questions, not excuses to silently
substitute a guessed converter. The legacy GUI's1CO/8CO mappings are different;
its runtime accessibility remains UNKNOWN.

**INFERRED:** explicit-mode first outputs may start at duplicate suffix b because
the type counter includes initialized slot0; AUT-classified first modes can retain
the literal3-character extension. Exact output filenames/overwrite effects are
not runtime-confirmed.

**UNKNOWN:** successful baseline bytes; initial GUI selection sequencing;
missing-template outcome/defaults; particle-code semantics and GUI-edit casing;
full byte-level header formatting; precise CRLF/trailing-space handling;
REAL4 half-way formatting; safe channel capacity; malformed-input exit codes,
partial-file retention and stateful missing metadata. No full equivalence claim.

### Execution preparation and checks

Host is64-bit with SysWOW64 present, observed build26100/24H2. Its registry
ProductName is stale/ambiguous; loader compatibility is plausible rather than
tested. Signature status is NotSigned. Static analysis does not establish
trustworthiness. No target launch, malware scan that could quarantine originals,
or global software installation was performed. See the linked experiment guide
for Microsoft documentation and the disposable Sandbox/VM precautions.

Preparation creates independent case copies, Latin-1-preserving variants,
restricted .wsb files and a separately guarded manual GUI runner. The runner
captures filesystem manifests (including heading INI updates), stdout/stderr
where available, exit code, launch errors and timeout termination. It has no
automatic target launch during preparation.

Checks performed: PowerShell parser checks for the3 inspection/experiment scripts;
baseline input SHA-256 equality; all4 Sentinel9 headers and channel sequences;
calibration/time substitutions; original executable/sample/INI hashes unchanged;
Ghidra function/global annotation log and save. Earlier truncated fixture removed
only final CRLF; a separate truncated-data fixture removes the final channel
record while retaining its declaration. No reference runtime test is claimed.

Next action is the priority-ordered experiment matrix. Resolve baseline success,
the main/legacy GUI discrepancies, numeric headers and malformed-input behavior
before beginning Rust implementation.
