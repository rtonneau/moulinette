//! Rate-vs-time CSV export and PNG rendering for `.lst` acquisition-rate
//! series computed by `rate::time_binned_rate`. Kept out of `rate.rs` so
//! that `rate`/`lst` stay free of drawing concerns.
use std::{
    fmt,
    fs::OpenOptions,
    io::{self, Write},
    path::Path,
};

use plotters::prelude::*;
use plotters::style::Palette;

use crate::rate::RateBin;

/// Errors rendering a rate-vs-time PNG.
#[derive(Debug)]
pub enum PlotError {
    /// `output_path` already exists; existing output files are never
    /// overwritten (mirrors `workflow::render`'s refusal).
    AlreadyExists(std::path::PathBuf),
    /// Checking whether `output_path` exists failed.
    Io(io::Error),
    /// Drawing with `plotters` failed.
    Draw(String),
}

impl fmt::Display for PlotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlotError::AlreadyExists(path) => {
                write!(
                    f,
                    "output {} already exists; choose a new path",
                    path.display()
                )
            }
            PlotError::Io(e) => write!(f, "cannot inspect output path: {e}"),
            PlotError::Draw(e) => write!(f, "cannot render plot: {e}"),
        }
    }
}

impl std::error::Error for PlotError {}

/// Render one labeled line per named series on a single shared y-axis to a
/// PNG at `output_path`, via `plotters`' `BitMapBackend`.
///
/// Never overwrites an existing file at `output_path`.
pub fn rate_vs_time(
    series: &[(String, Vec<RateBin>)],
    output_path: &Path,
) -> Result<(), PlotError> {
    // `plotters-bitmap`'s `present()` saves via `image`'s `File::create`-equivalent
    // semantics, which would silently overwrite an existing file. Atomically claim
    // the path first (mirroring `workflow::write`'s `create_new`), then let
    // `present()` overwrite the empty placeholder we now own.
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_path)
    {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            return Err(PlotError::AlreadyExists(output_path.to_path_buf()));
        }
        Err(e) => return Err(PlotError::Io(e)),
    }

    let x_max = series
        .iter()
        .flat_map(|(_, bins)| bins.iter().map(|b| b.time_s))
        .fold(0f64, f64::max);
    let y_max = series
        .iter()
        .flat_map(|(_, bins)| bins.iter().map(|b| b.rate))
        .fold(0f64, f64::max);

    let root = BitMapBackend::new(output_path, (1200, 800)).into_drawing_area();
    root.fill(&WHITE)
        .map_err(|e| PlotError::Draw(e.to_string()))?;

    let mut chart = ChartBuilder::on(&root)
        .caption("Acquisition rate vs. time", ("sans-serif", 24))
        .margin(15)
        .x_label_area_size(45)
        .y_label_area_size(60)
        .build_cartesian_2d(0f64..x_max.max(1.0), 0f64..(y_max * 1.1).max(1.0))
        .map_err(|e| PlotError::Draw(e.to_string()))?;

    chart
        .configure_mesh()
        .x_desc("Acquisition Time [s]")
        .y_desc("Acquisition Rate [Yield/s]")
        .draw()
        .map_err(|e| PlotError::Draw(e.to_string()))?;

    for (i, (name, bins)) in series.iter().enumerate() {
        let color = Palette99::pick(i).to_rgba();
        let points: Vec<(f64, f64)> = bins.iter().map(|b| (b.time_s, b.rate)).collect();
        chart
            .draw_series(LineSeries::new(points, color))
            .map_err(|e| PlotError::Draw(e.to_string()))?
            .label(name)
            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color));
    }

    chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.8))
        .border_style(BLACK)
        .draw()
        .map_err(|e| PlotError::Draw(e.to_string()))?;

    root.present().map_err(|e| PlotError::Draw(e.to_string()))?;
    Ok(())
}

/// Write `bins` as CSV (header `time_s,rate,rate_err,counts`, one row per
/// bin) to `output_path`.
///
/// Never overwrites an existing file at `output_path` (uses the same
/// `create_new` refusal as `workflow::write`).
pub fn write_rate_csv(bins: &[RateBin], output_path: &Path) -> Result<(), io::Error> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_path)?;
    file.write_all(b"time_s,rate,rate_err,counts\n")?;
    for bin in bins {
        writeln!(
            file,
            "{},{},{},{}",
            bin.time_s, bin.rate, bin.rate_err, bin.counts
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rate::RateBin;
    use std::fs;

    fn bin(time_s: f64, rate: f64, rate_err: f64, counts: u64) -> RateBin {
        RateBin {
            time_s,
            rate,
            rate_err,
            counts,
        }
    }

    #[test]
    fn write_rate_csv_writes_header_and_rows() {
        let dir = std::env::temp_dir().join(format!("moulinette-plot-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rates.csv");
        let bins = vec![bin(0.5, 2.0, 2.0_f64.sqrt(), 2), bin(1.5, 1.0, 1.0, 1)];
        write_rate_csv(&bins, &path).unwrap();
        let contents = fs::read_to_string(&path).unwrap();
        assert_eq!(
            contents,
            "time_s,rate,rate_err,counts\n0.5,2,1.4142135623730951,2\n1.5,1,1,1\n"
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_rate_csv_refuses_to_overwrite() {
        let dir =
            std::env::temp_dir().join(format!("moulinette-plot-test-ov-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rates.csv");
        fs::write(&path, b"existing").unwrap();
        let err = write_rate_csv(&[], &path).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rate_vs_time_refuses_to_overwrite() {
        let dir =
            std::env::temp_dir().join(format!("moulinette-plot-test-png-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rates.png");
        fs::write(&path, b"existing").unwrap();
        let err = rate_vs_time(&[], &path).unwrap_err();
        assert!(matches!(err, PlotError::AlreadyExists(p) if p == path));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rate_vs_time_renders_a_png() {
        let dir =
            std::env::temp_dir().join(format!("moulinette-plot-test-ok-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rates.png");
        let series = vec![
            ("roi-a".to_string(), vec![bin(0.5, 2.0, 2.0_f64.sqrt(), 2)]),
            ("roi-b".to_string(), vec![bin(0.5, 1.0, 1.0, 1)]),
        ];
        rate_vs_time(&series, &path).unwrap();
        let metadata = fs::metadata(&path).unwrap();
        assert!(metadata.len() > 0);
        fs::remove_dir_all(&dir).ok();
    }
}
