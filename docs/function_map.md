# Function map

Static findings, 2026-10-13. Addresses are linked virtual addresses, image base
0x400000. Names are analyst annotations, not original symbols. CONFIRMED is a
direct static observation; confidence in the scientific interpretation may be
lower. Callers below include thunk-resolved or callback registration edges,
because REA's direct caller list can omit both.

| Address / analyst name | Callers / triggering edge | Relevant callees | Purpose | Confidence |
| --- | --- | --- | --- | --- |
| 0x406a50, application initialization | QuickWin startup; caller identity not exhaustively traced | 0x40bdf0, 0x4069e0, 0x422be0, 0x4015d8 | Load config, default conversion modes, collect selected files, show dialog110 | CONFIRMED instructions; initial selection interaction UNKNOWN |
| 0x4015d8, show_conversion_dialog110 | 0x406a50 via thunk 0x4012df | QuickWin dialog RTL 0x473497, 0x47750a, 0x477965, 0x476845 | Resource110, choices, callbacks | CONFIRMED |
| 0x401d7e, convert_selected_files_dialog110 | Callback registered at 0x401b2b for control1010 | Reader thunk0x401195 -> 0x406be6; 0x4082d1, 0x408a31; 0x405d3b | Selected-file conversion; both ASCII menu labels map to ASC | CONFIRMED |
| 0x408fd9, show_conversion_dialog101 | Alternate UI route, reachability from startup UNKNOWN | QuickWin dialog RTL, 0x405965, 0x405d3b | Legacy resource101, broader output choices | CONFIRMED role; runtime reachability UNKNOWN |
| 0x409ac7, convert_selected_files_dialog101 | Callback registered at 0x40988b for control1010 | 0x406be6, 0x4082d1, 0x408a31 | Legacy selected-file conversion; distinct 1CO and 8CO codes | CONFIRMED |
| 0x406be6, read_mpa_and_dispatch | 0x401d7e via0x401195, direct0x409ac7 | Seven writers below through thunks; string and I/O RTL; 0x40bc08 | MPA metadata scanner and per-section dispatch | CONFIRMED |
| 0x4118a0, write_scientific_spectrum_and_ini | Reader at0x407fbf/0x40800e/0x40805d via0x40117c; may also be called from other input readers | 0x415c17, 0x415f5f as callbacks; 0x416263; I/O RTL | RBS/NRA/ERD header, selected counts, heading/experiment INI updates | CONFIRMED |
| 0x4163ca, write_tof_text | Reader0x4080a0 via0x4011db | I/O RTL, LEN_TRIM | TOF full-spectrum one-column output | CONFIRMED |
| 0x41662d, write_pro_tmca | Reader0x408169 via0x4011d6 | I/O RTL, string helpers | PRO / RNRA C V header and selected counts | CONFIRMED |
| 0x41753c, write_one_column_text | Reader0x4080e3/0x408126/0x4081ac via0x401294 | I/O RTL, LEN_TRIM | PIX/PIG/MON/1CO cropped text output | CONFIRMED |
| 0x4178bb, write_eight_column_text | Reader0x4081ef via0x4012bc | I/O RTL, LEN_TRIM | 8CO output (8A10), no partial-row guard | CONFIRMED |
| 0x417ed8, write_channel_yield | Reader0x408232 via0x401028 | I/O RTL, LEN_TRIM | XY (I8,5X,A), zero-based coordinate | CONFIRMED |
| 0x41827c, write_energy_yield | Reader0x408280 via0x401235 | I/O RTL, LEN_TRIM | EY (F10.3,5X,A), one-based calibrated coordinate | CONFIRMED |
| 0x40bc08, uppercase_ascii_in_place | Reader, GUI converters, scientific writer | 0x41cb00, character-copy helpers | ASCII a-z only, in-place | CONFIRMED |
| 0x40bdf0, load configuration | 0x406a50 via0x4011f4 | I/O RTL, character helpers | moulinette.ini; paths and four heading filenames | CONFIRMED |
| 0x40cb38, show_conversion_options | Main control1072 registration0x401d0c via0x40115e | 0x40d540 and radio/enable callbacks; QuickWin RTL | Resource109 | CONFIRMED |
| 0x40d540, accept_conversion_options | Resource109 OK callback registration0x40cf0f | 0x477ef1, 0x478159, 0x4768ba | Store RBS/ERD/NRA/Other modes and limits | CONFIRMED |
| 0x415c17, accept_heading_once | Resource102 Yes ID1, registration0x4136e0 | 0x478159, 0x41cef0, internal reads, 0x4768ba | Read edits/bounds, close heading dialog | CONFIRMED |
| 0x415f5f, accept_heading_for_all | Resource102 Yes to all ID2, registration0x413705 | Same heading getter helpers | Also sets 0x70d494=1 | CONFIRMED |
| 0x416263, prepare_numeric_header_text | Scientific writer at0x413967 and later numeric header fields | Left-adjust helper0x41cef0, first-character search | Limit7 numeric-text fallback; decimal handling | CONFIRMED instructions; intent INFERRED |
| 0x421290 | Reader and writers | RTL open/setup helpers, Windows filesystem operations transitively | OPEN statement engine | INFERRED role, exact DEC symbol UNKNOWN |
| 0x4233b0 | Reader metadata loop | RTL unit state helpers | EOF enquiry | INFERRED role |
| 0x423520 / 0x423a70 | Reader/writers | RTL format/input conversion | Begin external formatted READ / transfer and finish | INFERRED role |
| 0x424af0 | Reader and writers | RTL internal-unit setup and parser | Internal formatted READ from characters | INFERRED role |
| 0x426090 / 0x426690 | Writers | 0x4360b0, 0x42bda0, RTL file output | Begin formatted WRITE / transfer and finish | INFERRED role |
| 0x425e20 | Reader/writers | RTL unit-close helpers | CLOSE | INFERRED role |
| 0x41cb00, rtl_len_trim_spaces | Reader and text writers | none | Return trailing-space-trimmed length in EAX | CONFIRMED; decompiler's void signature wrong |
| 0x41cb60, rtl_character_equal_blank_padded | Metadata/GUI/type comparisons | none | Fortran character equality, including remaining blanks | CONFIRMED |
| 0x41ceac, rtl_find_first_character_in_set | Reader, naming/date helpers | none | Zero-based first match; -1 if absent | CONFIRMED |
| 0x4360b0 | Formatted read/write transfer | 0x496a00 descriptor table, RTL callbacks | Decode compiler format bytecode | CONFIRMED |
| 0x42bda0 | Write transfer engine | 0x437750/0x438530 floats; character/integer formatting | Dispatch field output | CONFIRMED |
| 0x437750 | Format output dispatcher | Decimal conversion0x4410a0, allocation helpers | Floating output formatting | CONFIRMED code; exact rounding needs runtime |

## Evidence and saved annotations

Each writer except the large scientific dossier has a retained `0xADDRESS.json`
with assembly, pseudocode and references in [evidence/phase2](evidence/phase2).
The scientific dossier exceeded the MCP transport size; it was retained without
truncation as Evidence `ev_035765cfffacbd3560fbe4b2e8272339619fd407de6b0f1306c9dfd7074d4397`
in `rea-evidence-bundle.json`. Its smaller `pseudo-0x4118a0.json` and
`asm-writer.json` were separately consumed. The bundle contains 89 records;
its zero tracked REA unknown records does not mean the reconstruction has no
unknowns. The Markdown documents enumerate the remaining uncertainties.

Reusable REA state is saved outside res/ at
`analysis/phase2.rea-analysis.snapshot.json`. REA's Windows P0 provider does not
support database annotation. Instead, the existing saved project was copied to
`analysis/ghidra/moulinette2.gpr` plus its .rep folder. That saved project initially
had 147 functions but lacked the reader/writers; necessary analysis was completed
on the copy before naming. [AnnotateMoulinette.java](../tools/ghidra/AnnotateMoulinette.java)
renamed 21 functions and labeled 26 globals, with SHA-256 and image-base checks.
The successful naming/save log is [ghidra-annotations.log](evidence/phase2/ghidra-annotations.log).
Source-style local variables were not renamed where stack recovery was unreliable.
Open the analyst copy in Ghidra to see the persistent global-variable names.

## GUI mapping and limits

Resources were decoded structurally with [inspect-dialogs.ps1](../tools/inspect-dialogs.ps1),
not just searched for printable strings. `dialog-controls.json` contains IDs,
classes, styles and positions. The executable's nine embedded resources include
dialogs and icons; no embedded menu tree was supplied. QuickWin creates menus
and dialogs at runtime, so generic File/Open menu sequencing remains unobserved.

| GUI surface | Callback / routing | Mode and output |
| --- | --- | --- |
| Dialog110 => button1010 | 0x401d7e -> 0x406be6 for MPA | RBS/ERD/NRA/TOF/PIXE/PIGE/RNRA selections; XY; ASCII mapped to ASC; AUTOMATIC |
| Dialog101 => button1010 | 0x409ac7 -> 0x406be6 | Distinct 1CO/8CO; XY/EY; RBS/ERD/NRA/TOF/PIX/PIG/PRO; CAP/GAZ/AUT choices also displayed |
| Dialog110 Param. conv.1072 | thunk0x40115e -> 0x40cb38 | Show resource109 |
| Dialog109 OK1 | 0x40d540 | Set modes1/2/3; close dialog |
| Dialog102 Yes1 | 0x415c17 | Accept current scientific heading |
| Dialog102 Yes to all2 | 0x415f5f | Accept heading; suppress later heading prompts using flag0x70d494 |
| Finish1 / Cancel2 | Finish callback installed in dialog110, GUI close handling | Do not assume Finish performs conversion; => has the confirmed conversion callback |

Resource109 groups: RBS radios1048/1049/1050, ERD1053/1054/1055,
NRA1059/1060/1061 select Full spectrum / First points / Roi. Other1065/1066/1067
select Respect file / User / Roi; 1068 supplies the User count and 1069/1070
the limits. The legacy reader does not use instrument roimin/roimax to implement
these GUI ROI settings.

**CONFIRMED discrepancy:** dialog110's ASCII(8col.) and ASCII(1col.) labels
both map to ASC at 0x40201a..0x40208d; ASC has no MPA dispatch. Dialog101 maps
them to 8CO and 1CO. **INFERRED:** those choices in dialog110 may generate no
MPA output. Runtime tests must not substitute the legacy path and then claim
the main GUI path is equivalent. Similarly, automatic exact-equality labels
and advertised CAP/GAZ choices must be characterized rather than guessed.
