# Reference execution preparation

Status: prepared and syntax/fixture checked; no target process launched.
The Rust source and Cargo configuration are untouched.

## Compatibility and trust assessment

The host reports a 64-bit operating system, Windows build26100 and release24H2,
with SysWOW64/kernel32.dll present. Its registry ProductName still says
Windows10 Pro, so this is reported as the observed build rather than treating
that name as conclusive. The x86 PE and system imports are compatible in
principle with [Microsoft's documented WOW64 support](https://learn.microsoft.com/en-us/windows/win32/winprog64/running-32-bit-applications).
The actual loader and QuickWin GUI have not been tested.

Get-AuthenticodeSignature reports NotSigned. Hash identity matches the Phase1
binary; no provenance certificate or security assessment proves it trustworthy.
The inspected conversion path contains ordinary Fortran and GUI file I/O;
that bounded observation does not audit every routine, dynamic DLL load or
CreateProcess use. It also exposes Delete/Rename/RmDir GUI actions and rewrites
configured heading INI files. Host execution is therefore unnecessary for the
first experiments; use a disposable Windows Sandbox or VM.

Each case's .wsb disables networking, vGPU, clipboard, camera, microphone and
printer redirection. It maps only the copied case directory to
C:\MoulinetteCase with write access, needed to retain output. There is no res/
or project-wide mapped directory and no automatic logon command. The writable
case remains exposed to the executable; treat its contents as untrusted after
execution. This uses [Microsoft's supported Sandbox configuration options](https://learn.microsoft.com/en-us/windows/security/application-security/application-isolation/windows-sandbox/windows-sandbox-configure-using-wsb-file).
Sandbox availability/installation was not established or changed. If unavailable,
use a disposable VM with the same restricted case directory and disabled network.

## Prepared cases

Preparation copies, never moves, the executable and sample. It changes only
fixture copies and records original/fixture SHA-256 in logs/preparation.json.
All case paths are below `experiments/cases/` (ignored by Git).

| Directory | Modification |
| --- | --- |
| phase2-baseline | Original sample, byte-identical |
| phase2-labels | Labels RBS, RBS, ERD, MON replace RBS135°, RBS165°, RD30°, 1D; no numeric changes |
| phase2-sentinel9 | Four DATA sections each declare9 with distinct values1000..1008,2000..2008,3000..3008,4000..4008; active ADC ranges changed to9 |
| phase2-calibration | ADC1 slope2.5, offset1.25, quadratic9.5, cubic11, caluse1 |
| phase2-times | ADC1 livetime4.567, runtime12.345, realtime8.901 |
| phase2-missing-caloff | Remove all CALOFF records |
| phase2-truncated-data | Remove last channel record, retain DATA3's declared256 |
| phase2-truncated | Earlier prepared variant: remove only final CRLF; all channels still present |

Sentinel9 intentionally retains other instrument totals/ROI metadata to detect
whether the converter consults them. It is a diagnostic fixture, not a genuine
measurement. New cases can be generated reproducibly:

```powershell
powershell -NoProfile -File tools/prepare-reference-case.ps1 -Variant Baseline
powershell -NoProfile -File tools/prepare-reference-case.ps1 -Variant Calibration
```

The variant is chosen explicitly; existing directories are refused. No executable
is launched by preparation. The runner's target-launch path is separately guarded
by an explicit -Launch switch. Heading templates are absent from ini/ by design
for the first baseline. Do not synthesize scientifically plausible defaults and
call them legacy behavior. Add legitimate template copies only in separate cases
and record their hashes.

## Manual GUI procedure

1. On the host, open the chosen case's `reference.wsb` manually. This starts
   Windows Sandbox, not the legacy application. Leave the original res/ untouched.
2. Inside Sandbox, open PowerShell and run:

   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File C:\MoulinetteCase\run-reference-case.ps1 -Launch
   ```

   This regenerates moulinette.ini using guest input/output/ini paths. It starts
   the GUI with that working directory and begins a600-second capture window.
   Adjust -TimeoutSeconds if manual inspection needs longer.
3. Record the initial window and any file-selection prompt in logs/notes.txt.
   The exact first QuickWin selection screen is unobserved; if it offers Open
   or a browse dialog, select C:\MoulinetteCase\input\15032501.mpa. Do not enter
   original Z:, X:, U: paths. If no selection control is available, record the
   screen/menus instead of assuming a command-line interface.
4. When conversion dialog110 or101 appears, record which one you see. Select
   the input file and output directory C:\MoulinetteCase\output. Choose the
   requested output type. Click Param. conv. when available and record each
   group's Full/First/ROI or Respect/User/ROI settings. Use Full/Respect first.
5. Click the **=>** button (control1010) to invoke conversion. Finish and Cancel
   are distinct controls; do not use Finish as evidence that conversion ran.
6. For RBS/ERD/NRA, record missing-template messages. If HEADING appears, capture
   every field and Start/Stop values before editing. To test a valid header,
   create a fresh case and enter documented explicit values, then choose Yes
   or Yes to all and record which. Those are experiment choices, not recovered
   defaults. Keep all paths in the case directory.
7. Save screenshots/notes under logs/ if possible. QuickWin output may be
   internal window text and absent from redirected stdout/stderr. Record any
   runtime dialog verbatim and note which spectrum was being processed.
8. Exit the entire application through its UI. The runner writes before.json,
   after.json, run.json, stdout.txt and stderr.txt. run.json includes the exit
   code, launch error, timeout status, changed/created and deleted paths.
   A timeout termination is explicitly distinguished from a normal exit.
9. Close Sandbox only after checking capture completion; results remain in the
   mapped case directory. Make a fresh case for every new run so globals and
   rewritten INI files do not accidentally contaminate comparisons.

Avoid View/Rename/Delete/MkDir/RmDir during converter experiments. The runner
captures files in ini/ as well as output/; scientific conversion can update
heading defaults. No legacy outputs or exit codes have yet been collected.

## Exact validation matrix

| Priority / experiment | Controlled settings | Static prediction / question |
| --- | --- | --- |
| 1 Baseline GUI and trust boundary | Fresh baseline, full spectrum, AUTOMATIC | Observe initial workflow; sample labels fail exact classification; potentially no outputs |
| 2 Main ASCII discrepancy | Fresh baseline for each1col/8col choice in dialog110 | Both map to ASC; does this yield no output? Capture files and UI even if empty |
| 3 Legacy ASCII path | If dialog101 can be reached through a visible menu, repeat1col/8col using Sentinel9 | Expect (1X,A) or (8A10); determine last-row bytes, trailing blanks, newlines |
| 4 Direct recognized text mode | Baseline or Sentinel9, TOF or PIXE with Full | Establish successful output, channel order, count text, first explicit-mode suffix (index0 effect) |
| 5 Automatic labels | LabelsExact, AUTOMATIC | RBS and secondRBS / ERD classification; MON stays AUT on this branch; capture missing-template behavior |
| 6 Naming counter | Four same explicit modes in one file, then same case repeated only via fresh copies | Confirm .RBb/.RBc versus .RBS/.RBb; collisions, case, overwrite behavior |
| 7 XY versus EY | Sentinel9 and Calibration, each supported GUI path | XY first coordinate0; EY3.750,6.250,8.750; CALFACT2/caluse changes do not alter EY |
| 8 Crop bounds | Sentinel9, First/User count3; ROI min2 max4 | Expect3 values; XY2..4, EY one-based coordinates; 8CO may emit extra fields beyond T |
| 9 PRO times | TimeDistinct, RNRA/PRO | REALTI from livetime; TRUETI=DEADTI from runtime; truncation near integer boundary |
| 10 Scientific headers | Explicit RBS/ERD/NRA with same numeric metadata and recorded GUI edits | Detector angle135 produces -45 in RBS,135 in ERD/NRA; field widths/date/particle mapping; template updates |
| 11 Metadata association | Fresh fixture rename DATA0 to DATA7, then reorder DATA sections only | Reader uses ordinal rather than DATA ID; determine wrong-metadata propagation |
| 12 Missing/malformed | MissingCaloff and TruncatedData, then nonnumeric/negative/oversized values in fresh copies | Capture runtime errors, partial outputs and exit codes; no uniform fallback assumed |
| 13 Stateful missing fields | First valid file then missing-metadata file in same application; reverse order in another case | Detect retained metadata versus fresh-process defaults |
| 14 Numeric-text fidelity | Replace counts with leading zeros/blanks, signed text, more than20 bytes; use >10-byte values for8CO | Character I/O/truncation behavior; do not presume integer parsing |
| 15 REAL4 rounding | Coefficients not exactly representable, F10.3 half-way values, huge coordinates | Establish x87/finalfloat32 effect, rounding and asterisk fields |

The prepared calibration variant changes multiple related calibration flags to
make a clear probe; for causal attribution subsequently vary each coefficient
and flag separately. TimeDistinct separates three times in one probe; follow
with single-field variants. Boundary tests with invalid ADC/count sizes belong
only in disposable cases and should have short timeouts.

For each successful mode, retain raw input bytes, config/template files, UI
settings, output bytes, file hashes, filenames and exit status. Byte equivalence
is the default comparison; no floating tolerance or compatibility claim has
been established yet. Do not begin Rust implementation until at least the
selected modes have successful baseline captures and the two GUI discrepancies
have been resolved.
The filename/format predictions are static until a successful run captures the
actual records. Calibration metadata and heading templates must remain part of
each case's provenance; implementation is intentionally deferred.
