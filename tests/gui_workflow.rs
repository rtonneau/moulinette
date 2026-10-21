use moulinette::{
    conversion::Format,
    gui_state,
    mpa::{self, MpaFile},
    workflow,
};
use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);

fn le(words: &[u16]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_le_bytes()).collect()
}

/// A minimal synthetic `.lst` fixture: one linear ADC (energy == channel)
/// with four events on ADC1 at channels 25, 30, 10 and 99, the same layout
/// used by `gui_state`'s and the `list` CLI's own fixtures.
fn synthetic_lst() -> Vec<u8> {
    let mut bytes =
        b"[ADC1]\r\ncaloff=0\r\ncalfact=1\r\ntimerreduce=100\r\nnchannels=4096\r\n[LISTDATA]"
            .to_vec();
    bytes.extend(le(&[0x0001, 0x0000, 0x0019]));
    bytes.extend(le(&[0x0000, 0x4000]));
    bytes.extend(le(&[0x0001, 0x0000, 0x001E]));
    for _ in 0..11 {
        bytes.extend(le(&[0x0000, 0x4000]));
    }
    bytes.extend(le(&[0x0001, 0x0000, 0x000A]));
    bytes.extend(le(&[0x0000, 0x4000]));
    bytes.extend(le(&[0x0001, 0x0000, 0x0063]));
    bytes
}

/// Ticket 3: loading a `.lst` file on the Convert screen reuses the
/// converter panel over a Ticket 2 histogram instead of an `MpaFile`
/// produced by `workflow::load`.
#[test]
fn lst_convert_writes_histogram_as_xy() {
    let dir = std::env::temp_dir().join(format!(
        "moulinette-lst-convert-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&dir).unwrap();
    let input = dir.join("fixture.lst");
    fs::write(&input, synthetic_lst()).unwrap();

    // Mirrors gui_state::load_list + the gui.rs Convert-screen branch: parse
    // the .lst file, then wrap Ticket 2's histogram as a plain `MpaFile` so
    // workflow::render/conversion::convert run unchanged.
    let list_file = gui_state::load_list(&input).unwrap();
    let spectra = gui_state::histogram_list_file(&list_file);
    assert_eq!(spectra.len(), 1);
    let wrapped = MpaFile {
        sections: Vec::new(),
        spectra: spectra.clone(),
    };

    // Select the one ADC (spectrum index 0) and convert to XY, exactly as
    // the "Spectra to convert" dropdown would. Mode 0 is `Format::Xy`
    // (`gui_state::MODES[0]`, as asserted by `mode_mapping_and_validation`).
    let (format, selection, indices) =
        gui_state::parameters(0, 0, "0", "", wrapped.spectra.len()).unwrap();
    assert_eq!(format, Format::Xy);
    let paths = workflow::filenames(&input, &dir, format, &indices).unwrap();
    let rendered = workflow::render(&wrapped, format, selection, &indices, paths.clone()).unwrap();
    let written = workflow::write(rendered, None).unwrap();
    assert_eq!(written, paths);

    // The written file's contents must match Ticket 2's histogram, converted
    // directly, not just the wrapped MpaFile's own round trip.
    let expected = moulinette::conversion::convert(&spectra[0], format, selection).unwrap();
    assert_eq!(fs::read(&written[0]).unwrap(), expected);

    for path in written {
        fs::remove_file(path).unwrap();
    }
    fs::remove_file(&input).unwrap();
    fs::remove_dir(dir).unwrap();
}

#[test]
fn dropped_measurement_rejects_invalid_batches_and_busy_work() {
    let dir = std::env::temp_dir().join(format!(
        "moulinette-drop-tests-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&dir).unwrap();
    let file = dir.join("measurement.MPA");
    let other = dir.join("measurement.txt");
    let folder = dir.join("folder.mpa");
    fs::write(&file, b"[DATA0,1]\n1\n").unwrap();
    fs::write(&other, b"text").unwrap();
    fs::create_dir(&folder).unwrap();
    let paths = vec![file.clone()];
    assert_eq!(gui_state::dropped_measurement(&paths, false).unwrap(), file);
    assert!(
        gui_state::dropped_measurement(&paths, true)
            .unwrap_err()
            .contains("Working")
    );
    for paths in [
        vec![],
        vec![file.clone(), file.clone()],
        vec![other.clone()],
        vec![folder.clone()],
        vec![dir.join("missing.mpa")],
    ] {
        assert!(gui_state::dropped_measurement(&paths, false).is_err());
    }
    fs::remove_file(file).unwrap();
    fs::remove_file(other).unwrap();
    fs::remove_dir(folder).unwrap();
    fs::remove_dir(dir).unwrap();
}

#[test]
fn mode_mapping_and_validation() {
    for (index, (expected, _, _)) in gui_state::MODES.iter().enumerate() {
        let (format, selection, indices) =
            gui_state::parameters(index as i32, 2, "1", "7", 3).unwrap();
        assert_eq!(format, *expected);
        assert_eq!(indices, vec![1]);
        assert_eq!(selection.start, if format == Format::Tof { 0 } else { 1 });
        assert_eq!(
            selection.end,
            if format == Format::Tof { None } else { Some(7) }
        );
        // `ChargeNormalized` (ticket 08) is deliberately the one MODES
        // entry that reports unsupported through the generic one-spectrum
        // `convert` path: it needs a second, user-chosen monitor spectrum
        // that path has no room for. The Convert screen calls
        // `gui_state::charge_normalized` directly for it instead of
        // `workflow::render`'s `convert` loop.
        assert!(format.unsupported_reason().is_none() || format == Format::ChargeNormalized);
    }
    assert_eq!(
        gui_state::parameters(0, 0, "0", "", 3).unwrap().2,
        vec![0, 1, 2]
    );
    for (mode, spectrum, start, end, count) in [
        // MODES now has 9 entries (0..=8, ticket 08's `ChargeNormalized`
        // appended as the 9th); 9 is the first out-of-range index.
        (9, 0, "0", "", 1),
        (-1, 0, "0", "", 1),
        (0, 2, "0", "", 1),
        (0, -1, "0", "", 1),
        (0, 0, "-1", "", 1),
        (0, 0, "0", "x", 1),
        (0, 0, "0", "", 0),
    ] {
        assert!(gui_state::parameters(mode, spectrum, start, end, count).is_err());
    }
    assert!(gui_state::parameters(6, 0, "ignored", "ignored", 1).is_ok());
}

#[test]
fn preview_matches_actual_filenames_and_errors_propagate() {
    let input = Path::new("sample.extra.mpa");
    let paths = gui_state::preview(input, Path::new("output"), 0, 0, 2).unwrap();
    assert_eq!(
        paths,
        vec![
            Path::new("output").join("sample.XYb"),
            Path::new("output").join("sample.XYc")
        ]
    );
    assert!(gui_state::preview(input, Path::new(""), 0, 0, 1).is_err());
    assert!(gui_state::preview(input, Path::new("output"), 0, 0, 26).is_err());
    let file = mpa::parse(b"[DATA0,1]\n10\n").unwrap();
    let params = gui_state::parameters(1, 0, "0", "", 1).unwrap();
    let error = workflow::render(
        &file,
        params.0,
        params.1,
        &params.2,
        vec![Path::new("unused.EYb").into()],
    )
    .unwrap_err();
    assert!(error.contains("CALFACT"));
    let params = gui_state::parameters(0, 0, "2", "1", 1).unwrap();
    assert!(
        workflow::render(
            &file,
            params.0,
            params.1,
            &params.2,
            vec![Path::new("unused.XYb").into()]
        )
        .unwrap_err()
        .contains("invalid inclusive channel range")
    );
    assert!(
        mpa::parse(b"[DATA0,2]\n1\n")
            .unwrap_err()
            .to_string()
            .contains("expected 2")
    );
}

#[test]
fn overwrite_preflight_and_race_protection() {
    let dir = std::env::temp_dir().join(format!(
        "moulinette-gui-tests-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&dir).unwrap();
    let input = dir.join("input.mpa");
    fs::write(&input, b"[DATA0,1]\n1\n[DATA1,1]\n2\n").unwrap();
    let file = workflow::load(&input).unwrap();
    let (format, selection, indices) = gui_state::parameters(0, 0, "0", "", 2).unwrap();
    let paths = workflow::filenames(&input, &dir, format, &indices).unwrap();
    fs::write(&paths[1], b"protected").unwrap();
    assert!(
        workflow::render(&file, format, selection, &indices, paths.clone())
            .unwrap_err()
            .contains("already exists")
    );
    assert!(!paths[0].exists());
    fs::remove_file(&paths[1]).unwrap();
    let rendered = workflow::render(&file, format, selection, &indices, paths.clone()).unwrap();
    fs::write(&paths[0], b"raced").unwrap();
    assert!(workflow::write(rendered, Some(&dir)).is_err());
    assert_eq!(fs::read(&paths[0]).unwrap(), b"raced");
    assert!(!paths[1].exists());
    fs::remove_file(&paths[0]).unwrap();
    let rendered = workflow::render(&file, format, selection, &indices, paths.clone()).unwrap();
    assert_eq!(workflow::write(rendered, Some(&dir)).unwrap(), paths);
    assert_eq!(fs::read(&paths[0]).unwrap(), b"       0     1\r\n");
    for path in paths {
        fs::remove_file(path).unwrap();
    }
    fs::remove_file(&input).unwrap();
    assert!(workflow::load(&input).unwrap_err().contains("cannot read"));
    fs::remove_dir(dir).unwrap();
}

/// Ticket 5/7: the `.mpa` Analyze-screen ROI run (`gui_state::mpa_analyze`)
/// reports the same statistics `roi_statistics` (Ticket 4) computes
/// directly, and builds an in-window `chart::LineChart` (Ticket 6/7) that
/// renders to a correctly sized buffer.
#[test]
fn mpa_analyze_matches_roi_statistics_and_renders_a_correctly_sized_chart() {
    let file =
        mpa::parse(b"[ADC1]\ncaloff=0\ncalfact=1\n[DATA0,9]\n0\n0\n0\n10\n20\n10\n0\n0\n0\n")
            .unwrap();
    let rois = [gui_state::RoiSpec {
        name: "peak".into(),
        adc: 1,
        low: 3.0,
        high: 7.0,
        unit: gui_state::RoiUnit::Kev,
    }];

    let expected = gui_state::roi_statistics(&file.spectra[0], &rois).unwrap();
    let analysis = gui_state::mpa_analyze(&file, 0, &rois).unwrap();
    assert_eq!(analysis.stats, expected);
    assert_eq!(
        gui_state::format_roi_statistics(&analysis.stats),
        gui_state::format_roi_statistics(&expected)
    );

    // `selected_spectrum` follows the "Spectra to convert" combo's
    // convention: 0 means "All spectra", resolved to the first spectrum.
    let analysis_selected = gui_state::mpa_analyze(&file, 1, &rois).unwrap();
    assert_eq!(analysis_selected.stats, expected);

    let mut chart = analysis.chart;
    let image = chart.render().unwrap();
    assert_eq!((image.width, image.height), moulinette::chart::DEFAULT_SIZE);
    assert_eq!(
        image.rgb.len() as u32,
        image.width * image.height * 3,
        "RGB8 buffer has 3 bytes per pixel"
    );

    assert!(gui_state::mpa_analyze(&file, 5, &rois).is_err());
}

#[test]
fn inspection_preserves_parser_associations_and_records() {
    let file =
        mpa::parse(b"starttime=2026\n[ADC1]\ncalfact=2\nlivetime=100\n[DATA99,1]\n1\n").unwrap();
    let text = gui_state::inspection(&file, 0);
    assert!(text.contains("ADC1"));
    assert!(text.contains("DATA identifier: 99"));
    assert!(text.contains("CALFACT = 2"));
    assert!(text.contains("LIVETIME = 100"));
    assert!(text.contains("STARTTIME = 2026"));
}
