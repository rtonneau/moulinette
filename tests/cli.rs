//! These launch only the newly built Rust application, never the reference EXE.
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "moulinette-rust-tests-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn cli(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_moulinette"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap()
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        // Only flat test-created files; no recursive deletion.
        for entry in fs::read_dir(&self.0).unwrap() {
            fs::remove_file(entry.unwrap().path()).unwrap();
        }
        fs::remove_dir(&self.0).unwrap();
    }
}
#[test]
fn exact_output_no_overwrite_and_unsupported() {
    let w = Workspace::new();
    fs::write(
        w.0.join("input.mpa"),
        b"[ADC1]\ncaloff=1.25\ncalfact=2.5\n[DATA8,2]\n10\n20\n",
    )
    .unwrap();
    let out = w.cli(&[
        "convert",
        "input.mpa",
        "--format",
        "ey",
        "--output",
        "out.txt",
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        fs::read(w.0.join("out.txt")).unwrap(),
        b"     3.750     10\r\n     6.250     20\r\n"
    );
    assert!(
        !w.cli(&[
            "convert",
            "input.mpa",
            "--format",
            "xy",
            "--output",
            "out.txt"
        ])
        .status
        .success()
    );
    assert_eq!(
        fs::read(w.0.join("out.txt")).unwrap(),
        b"     3.750     10\r\n     6.250     20\r\n"
    );
    let out = w.cli(&[
        "convert",
        "input.mpa",
        "--format",
        "rbs",
        "--output",
        "unsupported.txt",
    ]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("unsupported"));
    assert!(!w.0.join("unsupported.txt").exists());
}
#[test]
fn multiple_spectra_and_legacy_filenames() {
    let w = Workspace::new();
    fs::write(w.0.join("input.mpa"), b"[DATA7,1]\n11\n[DATA0,1]\n22\n").unwrap();
    assert!(
        !w.cli(&[
            "convert",
            "input.mpa",
            "--format",
            "xy",
            "--output",
            "ambiguous.txt"
        ])
        .status
        .success()
    );
    assert!(!w.0.join("ambiguous.txt").exists());
    assert!(
        w.cli(&[
            "convert",
            "input.mpa",
            "--format",
            "xy",
            "--output-dir",
            "."
        ])
        .status
        .success()
    );
    assert_eq!(
        fs::read(w.0.join("input.XYb")).unwrap(),
        b"       0     11\r\n"
    );
    assert_eq!(
        fs::read(w.0.join("input.XYc")).unwrap(),
        b"       0     22\r\n"
    );
    assert!(
        w.cli(&[
            "convert",
            "input.mpa",
            "--format",
            "xy",
            "--spectrum",
            "2",
            "--output",
            "selected.txt"
        ])
        .status
        .success()
    );
    assert_eq!(
        fs::read(w.0.join("selected.txt")).unwrap(),
        b"       0     22\r\n"
    );
}
#[test]
fn batch_validation_before_output_and_format_listing() {
    let w = Workspace::new();
    fs::write(
        w.0.join("input.mpa"),
        b"[ADC1]\ncaloff=0\ncalfact=1\n[DATA0,1]\n11\n[DATA1,1]\n22\n",
    )
    .unwrap();
    assert!(
        !w.cli(&[
            "convert",
            "input.mpa",
            "--format",
            "ey",
            "--output-dir",
            "."
        ])
        .status
        .success()
    );
    assert!(!w.0.join("input.EYb").exists());
    let out = w.cli(&["formats"]);
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("xy   supported"));
    assert!(text.contains("pro  unsupported"));
}
