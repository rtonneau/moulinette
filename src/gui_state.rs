//! Toolkit-independent GUI settings mapping and inspection text.
use crate::{
    conversion::{Format, Selection},
    mpa::{MpaFile, SectionKind},
    workflow,
};
use std::path::{Path, PathBuf};

/// Validate a complete native drop before changing the loaded measurement.
pub fn dropped_measurement(paths: &[PathBuf], busy: bool) -> Result<&Path, String> {
    if busy {
        return Err("Working — drop the file again when finished.".into());
    }
    let [path] = paths else {
        return Err("Drop exactly one .mpa measurement file.".into());
    };
    if !path.is_file()
        || !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("mpa"))
    {
        return Err("Drop one .mpa file, not a folder or another file type.".into());
    }
    Ok(path)
}

pub const MODES: [(Format, &str, &str); 8] = [
    (
        Format::Xy,
        "XY · Channel / counts",
        "Zero-based channel coordinates and original count text.",
    ),
    (
        Format::Ey,
        "EY · Energy / counts",
        "Uses ADC CALFACT and CALOFF. Counts are preserved; no yield normalization.",
    ),
    (
        Format::OneColumn,
        "1CO · One-column counts",
        "One count record per line.",
    ),
    (
        Format::Pix,
        "PIX · PIXE counts",
        "One-column counts with the legacy PIX filename extension.",
    ),
    (
        Format::Pig,
        "PIG · PIGE counts",
        "One-column counts with the legacy PIG filename extension.",
    ),
    (
        Format::Mon,
        "MON · Monitor counts",
        "One-column counts with the legacy MON filename extension.",
    ),
    (
        Format::Tof,
        "TOF · Full-spectrum counts",
        "Always writes every channel; channel cropping is unavailable.",
    ),
    (
        Format::EightColumn,
        "8CO · Eight-column counts",
        "Eight fixed-width fields per row. Selection must contain a multiple of eight channels.",
    ),
];

pub fn parameters(
    mode: i32,
    spectrum: i32,
    start: &str,
    end: &str,
    count: usize,
) -> Result<(Format, Selection, Vec<usize>), String> {
    let format = MODES
        .get(usize::try_from(mode).map_err(|_| "Invalid conversion mode")?)
        .ok_or("Invalid conversion mode")?
        .0;
    let indices = if spectrum == 0 {
        (0..count).collect()
    } else {
        let n = usize::try_from(spectrum).map_err(|_| "Invalid spectrum")?;
        if n > count {
            return Err("Spectrum selection is out of range".into());
        }
        vec![n - 1]
    };
    if indices.is_empty() {
        return Err("No spectra loaded".into());
    }
    let selection = if format == Format::Tof {
        Selection::default()
    } else {
        Selection {
            start: start
                .trim()
                .parse()
                .map_err(|_| "First channel must be a nonnegative integer")?,
            end: if end.trim().is_empty() {
                None
            } else {
                Some(
                    end.trim()
                        .parse()
                        .map_err(|_| "Last channel must be a nonnegative integer or blank")?,
                )
            },
        }
    };
    Ok((format, selection, indices))
}

pub fn preview(
    input: &Path,
    dir: &Path,
    mode: i32,
    spectrum: i32,
    count: usize,
) -> Result<Vec<PathBuf>, String> {
    let (format, _, indices) = parameters(mode, spectrum, "0", "", count)?;
    workflow::filenames(input, dir, format, &indices)
}

pub fn inspection(file: &MpaFile, selected: i32) -> String {
    let mut text = String::new();
    for (i, s) in file.spectra.iter().enumerate() {
        if selected > 0 && selected as usize != i + 1 {
            continue;
        }
        text.push_str(&format!(
            "Spectrum {}  ·  ADC{}  ·  {} channels\nDATA identifier: {}\n",
            i + 1,
            s.adc_index,
            s.declared_channels,
            String::from_utf8_lossy(&s.data_identifier)
        ));
        for record in &s.metadata {
            if let Some((key, value)) = record.key_value() {
                text.push_str(&format!(
                    "{} = {}\n",
                    String::from_utf8_lossy(&key),
                    String::from_utf8_lossy(value).trim()
                ));
            }
        }
        text.push('\n');
    }
    for section in &file.sections {
        if matches!(section.kind, SectionKind::Global | SectionKind::Other) {
            for record in &section.records {
                if let Some((key, value)) = record.key_value() {
                    text.push_str(&format!(
                        "File record: {} = {}\n",
                        String::from_utf8_lossy(&key),
                        String::from_utf8_lossy(value).trim()
                    ));
                }
            }
        }
    }
    text
}

pub fn warnings(format: Format) -> &'static str {
    match format {
        Format::Ey => {
            "EY uses only CALFACT and CALOFF; higher-order coefficients are ignored. Counts are not normalized. Energy field overflow is represented by asterisks."
        }
        Format::EightColumn => {
            "8CO count fields are limited to 10 bytes; longer count text is truncated."
        }
        _ => {
            "Count text is limited to the legacy first 20 bytes. Output uses legacy filenames and CRLF line endings."
        }
    }
}
