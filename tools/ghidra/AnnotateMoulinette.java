// Applies evidence-based names to an analyst COPY of the existing project.
// Run with -process moulinetteb.exe -noanalysis -postScript AnnotateMoulinette.java.
// @category Moulinette
import ghidra.app.script.GhidraScript;
import ghidra.program.model.symbol.SourceType;
import ghidra.program.model.listing.Function;

public class AnnotateMoulinette extends GhidraScript {
    private void function(long address, String name, String comment) throws Exception {
        Function f = getFunctionAt(toAddr(address));
        if (f == null) { println("SKIPPED missing function at " + Long.toHexString(address)); return; }
        String previous = f.getName();
        f.setName(name, SourceType.USER_DEFINED);
        f.setComment(comment + "\nRecovered static role; original source names unknown.");
        println("FUNCTION " + Long.toHexString(address) + " " + previous + " -> " + name);
    }
    private void global(long address, String name) throws Exception {
        createLabel(toAddr(address), name, true, SourceType.USER_DEFINED);
        println("GLOBAL " + Long.toHexString(address) + " " + name);
    }
    public void run() throws Exception {
        if (!"642a46517364a73f4fe4ab3906f94078562c92201e149fbdbc2f9fd4704f0e55".equalsIgnoreCase(currentProgram.getExecutableSHA256())) {
            throw new IllegalStateException("Unexpected executable identity; refusing annotations");
        }
        println("Image base=" + currentProgram.getImageBase() + "; existing function count=" + currentProgram.getFunctionManager().getFunctionCount());
        if (currentProgram.getImageBase().getOffset() != 0x400000L) { throw new IllegalStateException("Unexpected image base"); }
        if (getFunctionAt(toAddr(0x406be6)) == null) {
            println("The saved project lacks analyzed reader functions; analyzing only the analyst copy before naming.");
            analyzeAll(currentProgram);
        }
        function(0x406be6, "read_mpa_and_dispatch", "CONFIRMED: unit 10, ADC/DATA prefix recognition; metadata character tables; output dispatch. docs/mpa_format.md");
        function(0x401d7e, "convert_selected_files_dialog110", "CONFIRMED: => ID 1010 callback, selected file iteration; both ASCII labels map to ASC, which reader does not dispatch.");
        function(0x409ac7, "convert_selected_files_dialog101", "CONFIRMED: legacy => ID 1010 callback, distinct 1CO/8CO selections.");
        function(0x4015d8, "show_conversion_dialog110", "CONFIRMED: initializes resource 110; callbacks including 1072 -> 40cb38 via thunk.");
        function(0x408fd9, "show_conversion_dialog101", "CONFIRMED: resource 101 legacy selector.");
        function(0x4118a0, "write_scientific_spectrum_and_ini", "CONFIRMED: RBS/ERD/NRA common writer; metadata dialog 102; VERSION 1.2; output unit 20; configured heading and experiment INI files updated through unit 30.");
        function(0x4163ca, "write_tof_text", "CONFIRMED: TOF dispatch; all channels as 20-character text, (1X,A).");
        function(0x41662d, "write_pro_tmca", "CONFIRMED: PRO dispatch; C V VERSION 2.07; time *1000 truncated; TRUETI and DEADTI reuse runtime.");
        function(0x41753c, "write_one_column_text", "CONFIRMED: PIX/PIG/MON/1CO dispatch; selected channels; (1X,A).");
        function(0x4178bb, "write_eight_column_text", "CONFIRMED: 8CO dispatch; (8A10); emits full eight-item last row without a tail guard.");
        function(0x417ed8, "write_channel_yield", "CONFIRMED: XY dispatch; (I8,5X,A), coordinate = one-based array index - 1.");
        function(0x41827c, "write_energy_yield", "CONFIRMED: EY dispatch; (F10.3,5X,A), coordinate = one-based array index * REAL4 slope + REAL4 offset. CALFACT2 unused here.");
        function(0x40bc08, "uppercase_ascii_in_place", "CONFIRMED: ASCII a-z converted by subtracting 32; does not decode multibyte characters.");
        function(0x40cb38, "show_conversion_options", "CONFIRMED: resource 109, callbacks for Full/First/ROI.");
        function(0x40d540, "accept_conversion_options", "CONFIRMED: stores RBS/ERD/NRA/Other mode 1/2/3 and associated text fields.");
        function(0x415c17, "accept_heading_once", "CONFIRMED: reads heading edits and selected bounds, closes dialog 102.");
        function(0x415f5f, "accept_heading_for_all", "CONFIRMED: sets 0x70d494=1, reads heading edits and bounds.");
        function(0x416263, "prepare_numeric_header_text", "CONFIRMED: adjusts leading blanks; replaces oversized decimal text with 0. or adds decimal point when missing. Limit from caller is 7.");
        function(0x41cb00, "rtl_len_trim_spaces", "CONFIRMED: strips trailing spaces; return in EAX despite bad recovered void signature.");
        function(0x41cb60, "rtl_character_equal_blank_padded", "CONFIRMED: equality with trailing blank padding; register-based Fortran convention.");
        function(0x41ceac, "rtl_find_first_character_in_set", "CONFIRMED: zero-based first match, -1 if absent.");
        global(0x4d46c0, "mpa_adc_metadata_index");
        global(0x4d46cc, "mpa_data_section_ordinal");
        global(0x4d46d8, "mpa_value_offset");
        global(0x4d46dc, "mpa_trimmed_line_length");
        global(0x4d46e0, "mpa_output_type_by_adc");
        global(0x4d73fc, "mpa_line_text80");
        global(0x4d396c, "mpa_declared_count_text80");
        global(0x4d46c4, "mpa_calibration_slope_real4");
        global(0x4d46c8, "mpa_calibration_offset_real4");
        global(0x4d3868, "selected_start_channel_i32");
        global(0x4d3864, "selected_stop_channel_i32");
        global(0x4d395c, "conversion_mode_other_i32");
        global(0x4d3960, "conversion_mode_nra_i32");
        global(0x4d3964, "conversion_mode_erd_i32");
        global(0x4d3968, "conversion_mode_rbs_i32");
        global(0x4d3afc, "current_caloff_text80");
        global(0x4d3aac, "current_calfact_text80");
        global(0x4d3a5c, "current_calfact2_text80");
        global(0x4d39bc, "current_livetime_text80");
        global(0x4d3a0c, "current_runtime_text80");
        global(0x4d3e1c, "current_sample_text80");
        global(0x4d3dcc, "current_particle_text80");
        global(0x4d3cdc, "current_incident_angle_text80");
        global(0x4d3bec, "current_detector_angle_text80");
        global(0x70d494, "heading_yes_to_all_flag");
        println("Annotations complete; Ghidra saves the analyst project copy.");
    }
}
