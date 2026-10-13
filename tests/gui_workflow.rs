use moulinette::{conversion::Format, gui_state, mpa, workflow};
use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);

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
        assert!(format.unsupported_reason().is_none());
    }
    assert_eq!(
        gui_state::parameters(0, 0, "0", "", 3).unwrap().2,
        vec![0, 1, 2]
    );
    for (mode, spectrum, start, end, count) in [
        (8, 0, "0", "", 1),
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
