//! Toolkit-independent GUI settings mapping and inspection text, plus the
//! list-mode (`.lst`) panel's validation and analysis orchestration over
//! `lst`, `rate` and `plot`.
use crate::{
    conversion::{Format, Selection},
    lst::{self, ListFile},
    mpa::{MpaFile, SectionKind},
    plot,
    rate::{self, RateBin, Roi},
    workflow,
};
use std::path::{Path, PathBuf};

/// Validate a complete native drop before changing the loaded measurement.
pub fn dropped_measurement(paths: &[PathBuf], busy: bool) -> Result<&Path, String> {
    if busy {
        return Err("Working — drop the file again when finished.".into());
    }
    let [path] = paths else {
        return Err("Drop exactly one .mpa or .lst measurement file.".into());
    };
    if !path.is_file()
        || !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("mpa") || ext.eq_ignore_ascii_case("lst"))
    {
        return Err("Drop one .mpa or .lst file, not a folder or another file type.".into());
    }
    Ok(path)
}

/// Whether `path` names a list-mode (`.lst`) file, which the GUI shows in
/// the list-mode analysis panel instead of the converter panel.
pub fn is_list_mode(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("lst"))
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

/// Unit a list-mode ROI row's low/high bounds are expressed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoiUnit {
    /// Energy bounds, resolved through the ADC's linear calibration.
    Kev,
    /// Inclusive channel bounds.
    Channel,
}

/// ROI unit choices in the order the GUI offers them; a row's unit index
/// selects from this table, like `MODES` for the converter's format.
pub const ROI_UNITS: [(RoiUnit, &str); 2] = [(RoiUnit::Kev, "keV"), (RoiUnit::Channel, "Channel")];

/// Default list-mode time-bin width, matching the CLI's `--dt-ms` default.
pub const DEFAULT_BIN_WIDTH_MS: u64 = 15_000;

pub const LIST_MODE_WARNINGS: &str = "keV bounds use only CALOFF and CALFACT; non-zero CALFACT2/CALFACT3 is refused. Rates are ROI counts per second in each time bin, without dead-time correction. The last bin can be partial.";

/// Raw text of one ROI row of the list-mode panel.
#[derive(Debug, Clone, Copy)]
pub struct RoiFields<'a> {
    pub name: &'a str,
    pub adc: &'a str,
    pub low: &'a str,
    pub high: &'a str,
    /// Index into `ROI_UNITS`.
    pub unit: i32,
}

/// One validated ROI row.
#[derive(Debug, Clone, PartialEq)]
pub struct RoiSpec {
    pub name: String,
    /// 1-based ADC number, matching `lst::AdcHeader::adc_index`.
    pub adc: u8,
    pub low: f64,
    pub high: f64,
    pub unit: RoiUnit,
}

/// One validated half-open `[start_s, end_s)` time window, in seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeWindow {
    pub start_s: f64,
    pub end_s: f64,
}

/// Validated settings of the list-mode analysis panel.
#[derive(Debug, Clone, PartialEq)]
pub struct ListParameters {
    pub rois: Vec<RoiSpec>,
    pub dt_ms: u64,
    pub windows: Vec<TimeWindow>,
    pub output_dir: PathBuf,
}

/// Characters that cannot appear in a ROI name, since the name becomes an
/// output filename.
const FORBIDDEN_NAME_CHARACTERS: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

fn bound(text: &str, row: usize, which: &str) -> Result<f64, String> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("ROI {row}: {which} bound must be a number"))
}

fn roi(fields: &RoiFields, row: usize) -> Result<RoiSpec, String> {
    let name = fields.name.trim();
    if name.is_empty() {
        return Err(format!("ROI {row}: name is required"));
    }
    if name.contains(FORBIDDEN_NAME_CHARACTERS) || name.chars().any(char::is_control) {
        return Err(format!(
            "ROI {row}: name {name:?} cannot be used as a filename"
        ));
    }
    let adc = fields
        .adc
        .trim()
        .parse::<u8>()
        .ok()
        .filter(|adc| (1..=8).contains(adc))
        .ok_or_else(|| format!("ROI {row}: ADC must be an integer from 1 to 8"))?;
    let unit = usize::try_from(fields.unit)
        .ok()
        .and_then(|n| ROI_UNITS.get(n))
        .ok_or_else(|| format!("ROI {row}: invalid bound unit"))?
        .0;
    let low = bound(fields.low, row, "low")?;
    let high = bound(fields.high, row, "high")?;
    if low > high {
        return Err(format!(
            "ROI {row}: low bound must not exceed the high bound"
        ));
    }
    if unit == RoiUnit::Channel && low < 0.0 {
        return Err(format!("ROI {row}: channel bounds must be nonnegative"));
    }
    Ok(RoiSpec {
        name: name.to_string(),
        adc,
        low,
        high,
        unit,
    })
}

fn time_window((start, end): (&str, &str), row: usize) -> Result<TimeWindow, String> {
    let seconds = |text: &str, which: &str| {
        text.trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or_else(|| format!("Time window {row}: {which} must be a number of seconds"))
    };
    let start_s = seconds(start, "start")?;
    let end_s = seconds(end, "end")?;
    if start_s >= end_s {
        return Err(format!("Time window {row}: start must be before end"));
    }
    Ok(TimeWindow { start_s, end_s })
}

/// Validate the list-mode panel's inputs: ROI rows, time-bin width in
/// milliseconds, optional time windows (start/end seconds) and output
/// directory. The directory may not exist yet; it is created on run.
pub fn list_mode_parameters(
    rois: &[RoiFields],
    bin_width_ms: &str,
    windows: &[(&str, &str)],
    output_dir: &str,
) -> Result<ListParameters, String> {
    if rois.is_empty() {
        return Err("Add at least one ROI".into());
    }
    let rois = rois
        .iter()
        .enumerate()
        .map(|(i, fields)| roi(fields, i + 1))
        .collect::<Result<Vec<_>, _>>()?;
    for (i, spec) in rois.iter().enumerate() {
        // Case-insensitive: Windows filenames are, and names become filenames.
        if rois[..i]
            .iter()
            .any(|other| other.name.eq_ignore_ascii_case(&spec.name))
        {
            return Err(format!(
                "ROI {}: name {:?} is already used",
                i + 1,
                spec.name
            ));
        }
    }
    let dt_ms = bin_width_ms
        .trim()
        .parse::<u64>()
        .ok()
        .filter(|&dt| dt > 0)
        .ok_or("Time-bin width must be a positive integer number of milliseconds")?;
    let windows = windows
        .iter()
        .enumerate()
        .map(|(i, &window)| time_window(window, i + 1))
        .collect::<Result<Vec<_>, _>>()?;
    let output_dir = PathBuf::from(output_dir.trim());
    if output_dir.as_os_str().is_empty() {
        return Err("Choose an output directory".into());
    }
    if output_dir.exists() && !output_dir.is_dir() {
        return Err(format!(
            "Output directory {} is not a folder",
            output_dir.display()
        ));
    }
    Ok(ListParameters {
        rois,
        dt_ms,
        windows,
        output_dir,
    })
}

/// Files a list-mode run writes, in writing order: one rate CSV per ROI,
/// the shared `rate_vs_time.png`, then one violin PNG per ROI when at
/// least one time window is given (the same layout as the `list` CLI).
pub fn list_mode_outputs(params: &ListParameters) -> Vec<PathBuf> {
    let dir = &params.output_dir;
    let mut paths: Vec<PathBuf> = params
        .rois
        .iter()
        .map(|roi| dir.join(format!("{}.csv", roi.name)))
        .collect();
    paths.push(dir.join("rate_vs_time.png"));
    if !params.windows.is_empty() {
        paths.extend(
            params
                .rois
                .iter()
                .map(|roi| dir.join(format!("{}_violin.png", roi.name))),
        );
    }
    paths
}

/// Read and parse a `.lst` list-mode file.
pub fn load_list(path: &Path) -> Result<ListFile, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    lst::parse(&bytes).map_err(|e| format!("cannot parse {}: {e}", path.display()))
}

/// Summary of a parsed list-mode file for the inspection panel.
pub fn list_inspection(file: &ListFile) -> String {
    let last_ms = file.events.iter().map(|e| e.time_ms).max().unwrap_or(0);
    let mut text = format!(
        "{} events  ·  last event at {} s\n\n",
        file.events.len(),
        last_ms as f64 / 1000.0
    );
    for header in &file.adc_headers {
        let events = file
            .events
            .iter()
            .filter(|e| {
                usize::from(header.adc_index)
                    .checked_sub(1)
                    .and_then(|bit| e.adc_values.get(bit))
                    .is_some_and(Option::is_some)
            })
            .count();
        let value = |v: Option<f64>| v.map_or_else(|| "missing".to_string(), |v| v.to_string());
        text.push_str(&format!(
            "ADC{}  ·  {} channels  ·  {} events\nCALOFF = {}\nCALFACT = {}\n",
            header.adc_index,
            header.n_channels,
            events,
            value(header.caloff),
            value(header.calfact)
        ));
        for (key, v) in [("CALFACT2", header.calfact2), ("CALFACT3", header.calfact3)] {
            if let Some(v) = v {
                text.push_str(&format!("{key} = {v}\n"));
            }
        }
        text.push('\n');
    }
    if file.adc_headers.is_empty() {
        text.push_str("No [ADCn] header sections; only channel-bounded ROIs can be used.\n");
    }
    text
}

/// Run the list-mode analysis on a parsed file: resolve each ROI, compute
/// its time-binned rate, then write the rate CSVs, the rate-vs-time PNG and
/// (with time windows) one violin PNG per ROI. Every destination is checked
/// before anything is written, and each writer also refuses existing files.
pub fn list_mode_run(file: &ListFile, params: &ListParameters) -> Result<Vec<PathBuf>, String> {
    let mut series: Vec<(String, Vec<RateBin>)> = Vec::new();
    for spec in &params.rois {
        let roi = match spec.unit {
            RoiUnit::Channel => Roi::from_channels(
                spec.adc,
                spec.low.round() as u32,
                spec.high.round() as u32,
                spec.name.clone(),
            ),
            RoiUnit::Kev => {
                let header = file
                    .adc_headers
                    .iter()
                    .find(|h| h.adc_index == spec.adc)
                    .ok_or_else(|| {
                        format!(
                            "ROI {}: no ADC{} header found for energy bounds",
                            spec.name, spec.adc
                        )
                    })?;
                Roi::from_energy(header, spec.low, spec.high, spec.name.clone())
                    .map_err(|e| format!("ROI {}: {e}", spec.name))?
            }
        };
        let bins =
            rate::time_binned_rate(&file.events, spec.adc, Some(&roi), 0, None, params.dt_ms);
        series.push((spec.name.clone(), bins));
    }

    let paths = list_mode_outputs(params);
    for (i, path) in paths.iter().enumerate() {
        if paths[..i].contains(path) {
            return Err(format!("duplicate output destination {}", path.display()));
        }
        if path
            .try_exists()
            .map_err(|e| format!("cannot inspect {}: {e}", path.display()))?
        {
            return Err(format!(
                "output {} already exists; choose a new path",
                path.display()
            ));
        }
    }
    let dir = &params.output_dir;
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;

    let partial = |e: String| format!("{e}. Earlier files in this run may have been written.");
    let mut generated = Vec::new();
    let mut paths = paths.into_iter();
    for (_, bins) in &series {
        let path = paths.next().expect("one CSV per ROI");
        plot::write_rate_csv(bins, &path)
            .map_err(|e| partial(format!("cannot write {}: {e}", path.display())))?;
        generated.push(path);
    }
    let path = paths.next().expect("rate-vs-time PNG");
    plot::rate_vs_time(&series, &path).map_err(|e| partial(e.to_string()))?;
    generated.push(path);
    if !params.windows.is_empty() {
        for (_, bins) in &series {
            let path = paths.next().expect("one violin PNG per ROI");
            let groups: Vec<(String, Vec<f64>)> = params
                .windows
                .iter()
                .map(|w| {
                    let values = bins
                        .iter()
                        .filter(|b| b.time_s >= w.start_s && b.time_s < w.end_s)
                        .map(|b| b.rate)
                        .collect();
                    (format!("{}:{}", w.start_s, w.end_s), values)
                })
                .collect();
            plot::violin(&groups, &path).map_err(|e| partial(e.to_string()))?;
            generated.push(path);
        }
    }
    Ok(generated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        sync::atomic::{AtomicUsize, Ordering},
    };
    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "moulinette-gui-state-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        dir
    }

    fn fields<'a>(
        name: &'a str,
        adc: &'a str,
        low: &'a str,
        high: &'a str,
        unit: i32,
    ) -> RoiFields<'a> {
        RoiFields {
            name,
            adc,
            low,
            high,
            unit,
        }
    }

    fn le(words: &[u16]) -> Vec<u8> {
        words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    /// One linear ADC (energy == channel), `timerreduce=100`, with ADC1
    /// events at channel 25 (t=0ms), 30 (t=100ms), 10 (t=1200ms) and 99
    /// (t=1300ms); the same layout as the `list` CLI's integration fixture.
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

    #[test]
    fn dropped_measurement_accepts_lst_and_mpa() {
        let dir = temp_dir("drop");
        let list = dir.join("run.lst");
        let upper = dir.join("RUN2.LST");
        let mpa = dir.join("measurement.mpa");
        for path in [&list, &upper, &mpa] {
            fs::write(path, b"x").unwrap();
        }
        for path in [&list, &upper, &mpa] {
            let paths = vec![path.clone()];
            assert_eq!(dropped_measurement(&paths, false).unwrap(), path);
        }
        assert!(
            dropped_measurement(std::slice::from_ref(&list), true)
                .unwrap_err()
                .contains("Working")
        );
        assert!(is_list_mode(&list) && is_list_mode(&upper) && !is_list_mode(&mpa));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn dropped_measurement_still_rejects_folders_and_other_extensions() {
        let dir = temp_dir("drop-reject");
        let list = dir.join("run.lst");
        let other = dir.join("run.lst.txt");
        let folder = dir.join("folder.lst");
        fs::write(&list, b"x").unwrap();
        fs::write(&other, b"x").unwrap();
        fs::create_dir(&folder).unwrap();
        for paths in [
            vec![],
            vec![list.clone(), list.clone()],
            vec![other.clone()],
            vec![folder.clone()],
            vec![dir.join("missing.lst")],
        ] {
            let error = dropped_measurement(&paths, false).unwrap_err();
            assert!(error.contains(".mpa or .lst"), "{error}");
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn list_mode_parameters_accepts_valid_input() {
        let params = list_mode_parameters(
            &[
                fields(" peak ", "1", "20", "35.5", 0),
                fields("bg", "2", "100", "200", 1),
            ],
            " 2000 ",
            &[("0", "1.5"), ("1.5", "3")],
            "out",
        )
        .unwrap();
        assert_eq!(
            params,
            ListParameters {
                rois: vec![
                    RoiSpec {
                        name: "peak".into(),
                        adc: 1,
                        low: 20.0,
                        high: 35.5,
                        unit: RoiUnit::Kev,
                    },
                    RoiSpec {
                        name: "bg".into(),
                        adc: 2,
                        low: 100.0,
                        high: 200.0,
                        unit: RoiUnit::Channel,
                    },
                ],
                dt_ms: 2000,
                windows: vec![
                    TimeWindow {
                        start_s: 0.0,
                        end_s: 1.5
                    },
                    TimeWindow {
                        start_s: 1.5,
                        end_s: 3.0
                    },
                ],
                output_dir: PathBuf::from("out"),
            }
        );
        assert_eq!(
            list_mode_outputs(&params),
            vec![
                Path::new("out").join("peak.csv"),
                Path::new("out").join("bg.csv"),
                Path::new("out").join("rate_vs_time.png"),
                Path::new("out").join("peak_violin.png"),
                Path::new("out").join("bg_violin.png"),
            ]
        );
        let no_windows =
            list_mode_parameters(&[fields("peak", "1", "20", "35", 0)], "15000", &[], "out")
                .unwrap();
        assert!(no_windows.windows.is_empty());
        assert_eq!(list_mode_outputs(&no_windows).len(), 2);
    }

    #[test]
    fn list_mode_parameters_requires_an_roi() {
        let error = list_mode_parameters(&[], "15000", &[], "out").unwrap_err();
        assert!(error.contains("ROI"), "{error}");
    }

    #[test]
    fn list_mode_parameters_rejects_malformed_rois() {
        for row in [
            fields("peak", "1", "x", "35", 0),
            fields("peak", "1", "20", "", 0),
            fields("peak", "1", "NaN", "35", 0),
            fields("peak", "1", "40", "35", 0),
            fields("peak", "1", "-1", "35", 1),
            fields("", "1", "20", "35", 0),
            fields("a/b", "1", "20", "35", 0),
            fields("peak", "0", "20", "35", 0),
            fields("peak", "9", "20", "35", 0),
            fields("peak", "x", "20", "35", 0),
            fields("peak", "1", "20", "35", 2),
            fields("peak", "1", "20", "35", -1),
        ] {
            let error = list_mode_parameters(&[row], "15000", &[], "out").unwrap_err();
            assert!(error.starts_with("ROI 1"), "{row:?}: {error}");
        }
        let error = list_mode_parameters(
            &[
                fields("peak", "1", "20", "35", 0),
                fields("PEAK", "1", "40", "50", 0),
            ],
            "15000",
            &[],
            "out",
        )
        .unwrap_err();
        assert!(
            error.contains("ROI 2") && error.contains("already used"),
            "{error}"
        );
    }

    #[test]
    fn list_mode_parameters_rejects_bad_bin_width_and_windows() {
        let row = [fields("peak", "1", "20", "35", 0)];
        for width in ["", "0", "-5", "1.5", "x"] {
            assert!(
                list_mode_parameters(&row, width, &[], "out").is_err(),
                "{width}"
            );
        }
        for window in [("1", "1"), ("2", "1"), ("x", "1"), ("0", "")] {
            let error = list_mode_parameters(&row, "15000", &[window], "out").unwrap_err();
            assert!(error.starts_with("Time window 1"), "{window:?}: {error}");
        }
    }

    #[test]
    fn list_mode_parameters_rejects_invalid_output_directory() {
        let row = [fields("peak", "1", "20", "35", 0)];
        assert!(
            list_mode_parameters(&row, "15000", &[], "  ")
                .unwrap_err()
                .contains("output directory")
        );
        let dir = temp_dir("not-a-folder");
        let file = dir.join("file.txt");
        fs::write(&file, b"x").unwrap();
        assert!(
            list_mode_parameters(&row, "15000", &[], file.to_str().unwrap())
                .unwrap_err()
                .contains("not a folder")
        );
        assert!(
            list_mode_parameters(&row, "15000", &[], dir.join("new").to_str().unwrap()).is_ok()
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn list_mode_run_writes_outputs_and_never_overwrites() {
        let dir = temp_dir("run");
        let input = dir.join("fixture.lst");
        fs::write(&input, synthetic_lst()).unwrap();
        let file = load_list(&input).unwrap();
        let inspection = list_inspection(&file);
        assert!(inspection.contains("4 events"), "{inspection}");
        assert!(inspection.contains("ADC1"), "{inspection}");
        let out = dir.join("out");
        let params = list_mode_parameters(
            &[
                fields("peak", "1", "20", "35", 0),
                fields("low", "1", "0", "15", 1),
            ],
            "1000",
            &[("0", "1"), ("1", "2")],
            out.to_str().unwrap(),
        )
        .unwrap();
        let generated = list_mode_run(&file, &params).unwrap();
        assert_eq!(generated, list_mode_outputs(&params));
        assert_eq!(
            fs::read_to_string(out.join("peak.csv")).unwrap(),
            "time_s,rate,rate_err,counts\n0.5,2,1.4142135623730951,2\n1.5,0,0,0\n"
        );
        assert_eq!(
            fs::read_to_string(out.join("low.csv")).unwrap(),
            "time_s,rate,rate_err,counts\n0.5,0,0,0\n1.5,1,1,1\n"
        );
        for path in &generated {
            assert!(fs::metadata(path).unwrap().len() > 0, "{}", path.display());
        }

        // A second run refuses before writing anything: the protected PNG
        // keeps its content and the removed CSV is not recreated.
        fs::remove_file(out.join("peak.csv")).unwrap();
        fs::write(out.join("rate_vs_time.png"), b"protected").unwrap();
        let error = list_mode_run(&file, &params).unwrap_err();
        assert!(error.contains("already exists"), "{error}");
        assert!(!out.join("peak.csv").exists());
        assert_eq!(
            fs::read(out.join("rate_vs_time.png")).unwrap(),
            b"protected"
        );

        let missing_header = list_mode_parameters(
            &[fields("peak", "2", "20", "35", 0)],
            "1000",
            &[],
            dir.join("other").to_str().unwrap(),
        )
        .unwrap();
        assert!(
            list_mode_run(&file, &missing_header)
                .unwrap_err()
                .contains("no ADC2 header")
        );
        assert!(!dir.join("other").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn load_list_reports_unreadable_and_malformed_files() {
        let dir = temp_dir("load");
        assert!(
            load_list(&dir.join("missing.lst"))
                .unwrap_err()
                .contains("cannot read")
        );
        let bad = dir.join("bad.lst");
        fs::write(&bad, b"[ADC1]\r\nno marker").unwrap();
        assert!(load_list(&bad).unwrap_err().contains("LISTDATA"));
        fs::remove_dir_all(dir).unwrap();
    }
}
