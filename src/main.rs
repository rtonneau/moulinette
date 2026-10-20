#![forbid(unsafe_code)]
use moulinette::{
    conversion::{self, Format, Selection},
    lst, plot,
    rate::{self, Roi},
    workflow,
};
use std::{env, ffi::OsString, path::PathBuf, process::ExitCode};

const HELP: &str = "Moulinette — static-analysis-based native MPA converter\n\nUsage:\n  moulinette formats\n  moulinette convert INPUT --format FORMAT --output FILE [--spectrum N]\n  moulinette convert INPUT --format FORMAT --output-dir DIR [--spectrum N]\n  moulinette list INPUT --roi NAME:ADC:LOW:HIGH[:unit] [--roi ...] --output-dir DIR [--dt-ms N]\n\nOptions:\n  --spectrum N  One-based DATA occurrence (required for single output with multiple spectra)\n  --start N     Zero-based inclusive first channel (default 0)\n  --end N       Zero-based inclusive last channel (default final channel)\n  --roi NAME:ADC:LOW:HIGH[:unit]  ROI on one ADC's spectrum (repeatable, at least one required);\n                unit is 'ch' (channel bounds) or 'kev' (energy bounds, default)\n  --dt-ms N     Time-bin width in milliseconds (default 15000)\n\nExisting output files are never overwritten. TOF always emits all channels.\n";

#[derive(Debug)]
struct Options {
    input: PathBuf,
    format: Format,
    output: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    spectrum: Option<usize>,
    selection: Selection,
}
fn integer(value: &std::ffi::OsStr, option: &str) -> Result<usize, String> {
    value
        .to_str()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| format!("{option} requires a nonnegative integer"))
}
fn options(args: Vec<OsString>) -> Result<Options, String> {
    let mut args = args.into_iter();
    let input = args.next().ok_or("convert requires an input path")?;
    if input.to_string_lossy().starts_with("--") {
        return Err("convert requires an input path before options".into());
    }
    let mut format = None;
    let mut output = None;
    let mut output_dir = None;
    let mut spectrum = None;
    let mut start = None;
    let mut end = None;
    let mut seen = std::collections::HashSet::new();
    while let Some(option) = args.next() {
        let name = option.to_str().ok_or("option names must be ASCII")?;
        if ![
            "--format",
            "--output",
            "--output-dir",
            "--spectrum",
            "--start",
            "--end",
        ]
        .contains(&name)
        {
            return Err(format!("unknown option {name:?}"));
        }
        if !seen.insert(option.clone()) {
            return Err(format!("duplicate option {name}"));
        }
        let value = args
            .next()
            .ok_or_else(|| format!("{name} requires a value"))?;
        match name {
            "--format" => {
                format = Some(
                    value
                        .to_str()
                        .ok_or("format name must be ASCII")?
                        .parse::<Format>()
                        .map_err(|e| e.to_string())?,
                )
            }
            "--output" => output = Some(PathBuf::from(value)),
            "--output-dir" => output_dir = Some(PathBuf::from(value)),
            "--spectrum" => {
                let n = integer(&value, name)?;
                if n == 0 {
                    return Err("--spectrum is one-based; choose 1 or greater".into());
                }
                spectrum = Some(n);
            }
            "--start" => start = Some(integer(&value, name)?),
            "--end" => end = Some(integer(&value, name)?),
            _ => unreachable!(),
        }
    }
    if output.is_some() == output_dir.is_some() {
        return Err("choose exactly one of --output FILE or --output-dir DIR".into());
    }
    Ok(Options {
        input: input.into(),
        format: format.ok_or("--format is required; use 'formats' to list choices")?,
        output,
        output_dir,
        spectrum,
        selection: Selection {
            start: start.unwrap_or(0),
            end,
        },
    })
}

/// Unit a `--roi` value's LOW/HIGH bounds are expressed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoiUnit {
    Channel,
    Kev,
}

/// One parsed `--roi NAME:ADC:LOW:HIGH[:unit]` value.
#[derive(Debug, Clone, PartialEq)]
struct RoiSpec {
    name: String,
    adc: u8,
    low: f64,
    high: f64,
    unit: RoiUnit,
}

/// Parse one `--roi` value: `NAME:ADC:LOW:HIGH[:unit]`, `unit` defaulting
/// to `kev` when omitted.
fn parse_roi_spec(value: &str) -> Result<RoiSpec, String> {
    let parts: Vec<&str> = value.split(':').collect();
    if parts.len() != 4 && parts.len() != 5 {
        return Err(format!(
            "--roi value {value:?} must have the form NAME:ADC:LOW:HIGH[:unit]"
        ));
    }
    let name = parts[0];
    if name.is_empty() {
        return Err(format!("--roi value {value:?} has an empty NAME"));
    }
    let adc = parts[1]
        .parse::<u8>()
        .map_err(|_| format!("--roi value {value:?} has a non-numeric or out-of-range ADC"))?;
    let low = parts[2]
        .parse::<f64>()
        .map_err(|_| format!("--roi value {value:?} has a non-numeric LOW bound"))?;
    let high = parts[3]
        .parse::<f64>()
        .map_err(|_| format!("--roi value {value:?} has a non-numeric HIGH bound"))?;
    let unit = match parts.get(4).copied() {
        None | Some("kev") => RoiUnit::Kev,
        Some("ch") => RoiUnit::Channel,
        Some(other) => {
            return Err(format!(
                "--roi value {value:?} has unknown unit {other:?}; expected 'ch' or 'kev'"
            ));
        }
    };
    Ok(RoiSpec {
        name: name.to_string(),
        adc,
        low,
        high,
        unit,
    })
}

#[derive(Debug)]
struct ListOptions {
    input: PathBuf,
    rois: Vec<RoiSpec>,
    dt_ms: u64,
    output_dir: PathBuf,
}

fn list_options(args: Vec<OsString>) -> Result<ListOptions, String> {
    let mut args = args.into_iter();
    let input = args.next().ok_or("list requires an input path")?;
    if input.to_string_lossy().starts_with("--") {
        return Err("list requires an input path before options".into());
    }
    let mut rois = Vec::new();
    let mut dt_ms = None;
    let mut output_dir = None;
    let mut seen = std::collections::HashSet::new();
    while let Some(option) = args.next() {
        let name = option.to_str().ok_or("option names must be ASCII")?;
        if !["--roi", "--dt-ms", "--output-dir"].contains(&name) {
            return Err(format!("unknown option {name:?}"));
        }
        if name != "--roi" && !seen.insert(name.to_string()) {
            return Err(format!("duplicate option {name}"));
        }
        let value = args
            .next()
            .ok_or_else(|| format!("{name} requires a value"))?;
        match name {
            "--roi" => {
                let value_str = value.to_str().ok_or("--roi value must be ASCII")?;
                rois.push(parse_roi_spec(value_str)?);
            }
            "--dt-ms" => dt_ms = Some(integer(&value, name)? as u64),
            "--output-dir" => output_dir = Some(PathBuf::from(value)),
            _ => unreachable!(),
        }
    }
    if rois.is_empty() {
        return Err("list requires at least one --roi".into());
    }
    Ok(ListOptions {
        input: input.into(),
        rois,
        dt_ms: dt_ms.unwrap_or(15_000),
        output_dir: output_dir.ok_or("--output-dir is required")?,
    })
}

fn run_list(opts: ListOptions) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read(&opts.input)
        .map_err(|e| format!("cannot read {}: {e}", opts.input.display()))?;
    let file = lst::parse(&bytes).map_err(|e| e.to_string())?;

    let mut series: Vec<(String, Vec<rate::RateBin>)> = Vec::new();
    for spec in &opts.rois {
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
                        format!("no ADC{} header found for --roi {}", spec.adc, spec.name)
                    })?;
                Roi::from_energy(header, spec.low, spec.high, spec.name.clone())
                    .map_err(|e| e.to_string())?
            }
        };
        let bins = rate::time_binned_rate(&file.events, spec.adc, Some(&roi), 0, None, opts.dt_ms);
        series.push((spec.name.clone(), bins));
    }

    let output_dir = &opts.output_dir;
    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("cannot create {}: {e}", output_dir.display()))?;

    let mut csv_paths = Vec::new();
    for (name, _) in &series {
        csv_paths.push(output_dir.join(format!("{name}.csv")));
    }
    let png_path = output_dir.join("rate_vs_time.png");
    for (i, path) in csv_paths.iter().enumerate() {
        if csv_paths[..i].contains(path) || path == &png_path {
            return Err(format!("duplicate output destination {}", path.display()).into());
        }
    }

    for (path, (_, bins)) in csv_paths.iter().zip(&series) {
        plot::write_rate_csv(bins, path)?;
        println!("{}", path.display());
    }
    plot::rate_vs_time(&series, &png_path)?;
    println!("{}", png_path.display());
    Ok(())
}

fn run_convert(opts: Options) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(reason) = opts.format.unsupported_reason() {
        return Err(conversion::ConversionError::Unsupported {
            format: opts.format,
            reason,
        }
        .into());
    }
    let file = workflow::load(&opts.input)?;
    let indices = match opts.spectrum {
        Some(n) if n <= file.spectra.len() => vec![n - 1],
        Some(n) => {
            return Err(format!(
                "--spectrum {n} is out of range; input has {} DATA sections",
                file.spectra.len()
            )
            .into());
        }
        None if opts.output.is_some() && file.spectra.len() > 1 => {
            return Err(format!(
                "input has {} spectra; choose --spectrum N for --output, or use --output-dir",
                file.spectra.len()
            )
            .into());
        }
        None => (0..file.spectra.len()).collect(),
    };
    let paths = if let Some(path) = opts.output {
        vec![path]
    } else {
        workflow::filenames(
            &opts.input,
            opts.output_dir.as_ref().expect("validated output-dir"),
            opts.format,
            &indices,
        )?
    };
    let outputs = workflow::render(&file, opts.format, opts.selection, &indices, paths)?;
    for path in workflow::write(outputs, opts.output_dir.as_deref())? {
        println!("{}", path.display());
    }
    Ok(())
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let Some(command) = args.next() else {
        print!("{HELP}");
        return Ok(());
    };
    match command.to_str() {
        Some("--help" | "-h" | "help") => {
            print!("{HELP}");
            Ok(())
        }
        Some("formats") => {
            if args.next().is_some() {
                return Err("formats takes no arguments".into());
            }
            for format in Format::ALL {
                match format.unsupported_reason() {
                    Some(reason) => println!("{:<4} unsupported: {reason}", format.name()),
                    None => println!("{:<4} supported", format.name()),
                }
            }
            Ok(())
        }
        Some("convert") => {
            let args: Vec<_> = args.collect();
            if args.len() == 1 && args[0] == "--help" {
                print!("{HELP}");
                return Ok(());
            }
            run_convert(options(args)?)
        }
        Some("list") => {
            let args: Vec<_> = args.collect();
            if args.len() == 1 && args[0] == "--help" {
                print!("{HELP}");
                return Ok(());
            }
            run_list(list_options(args)?)
        }
        _ => Err("unknown command; use 'help', 'formats', 'convert' or 'list'".into()),
    }
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(text: &str) -> Vec<OsString> {
        text.split_whitespace().map(OsString::from).collect()
    }
    #[test]
    fn options_and_errors() {
        let opts = options(args(
            "input.mpa --format xy --output result.txt --start 1 --end 3 --spectrum 2",
        ))
        .unwrap();
        assert_eq!(opts.spectrum, Some(2));
        assert_eq!(opts.selection.end, Some(3));
        for text in [
            "input.mpa --output x",
            "input.mpa --format xy",
            "input.mpa --format xy --output x --output-dir y",
            "input.mpa --format xy --output x --start -1",
            "input.mpa --format xy --output x --spectrum 0",
            "input.mpa --format xy --output x --format ey",
            "input.mpa --format xy --output x --typo 1",
        ] {
            assert!(options(args(text)).is_err(), "{text}");
        }
    }

    #[test]
    fn list_options_valid_invocation() {
        let opts = list_options(args(
            "input.lst --roi peak:1:20:35:kev --dt-ms 2000 --output-dir out",
        ))
        .unwrap();
        assert_eq!(opts.input, PathBuf::from("input.lst"));
        assert_eq!(opts.rois.len(), 1);
        assert_eq!(opts.rois[0].name, "peak");
        assert_eq!(opts.rois[0].adc, 1);
        assert_eq!(opts.rois[0].low, 20.0);
        assert_eq!(opts.rois[0].high, 35.0);
        assert_eq!(opts.rois[0].unit, RoiUnit::Kev);
        assert_eq!(opts.dt_ms, 2000);
        assert_eq!(opts.output_dir, PathBuf::from("out"));
    }

    #[test]
    fn list_options_default_unit_and_dt_ms() {
        let opts = list_options(args("input.lst --roi peak:1:20:35 --output-dir out")).unwrap();
        assert_eq!(opts.rois[0].unit, RoiUnit::Kev);
        assert_eq!(opts.dt_ms, 15_000);
    }

    #[test]
    fn list_options_missing_roi_errors() {
        let err = list_options(args("input.lst --output-dir out")).unwrap_err();
        assert!(err.contains("--roi"), "{err}");
    }

    #[test]
    fn list_options_malformed_roi_errors() {
        for text in [
            "input.lst --roi peak:1:20 --output-dir out",
            "input.lst --roi peak:x:20:35 --output-dir out",
            "input.lst --roi peak:1:x:35 --output-dir out",
            "input.lst --roi peak:1:20:35:bogus --output-dir out",
            "input.lst --roi :1:20:35 --output-dir out",
        ] {
            assert!(list_options(args(text)).is_err(), "{text}");
        }
    }

    #[test]
    fn list_options_duplicate_output_dir_errors() {
        let err = list_options(args(
            "input.lst --roi peak:1:20:35 --output-dir out --output-dir out2",
        ))
        .unwrap_err();
        assert!(err.contains("duplicate"), "{err}");
    }

    fn le(words: &[u16]) -> Vec<u8> {
        words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    /// A synthetic `.lst` fixture with one ADC (linear, `caloff=0`,
    /// `calfact=1`, so energy == channel) and four events on ADC1:
    /// channel 25 at t=0ms, channel 30 at t=100ms (both inside a [20,35]
    /// ROI), channel 10 at t=1200ms and channel 99 at t=1300ms (both
    /// outside it). `timerreduce=100` keeps the fixture small.
    fn synthetic_lst_fixture() -> Vec<u8> {
        let mut bytes =
            b"[ADC1]\r\ncaloff=0\r\ncalfact=1\r\ntimerreduce=100\r\nnchannels=4096\r\n[LISTDATA]"
                .to_vec();
        bytes.extend(le(&[0x0001, 0x0000, 0x0019])); // t=0, channel 25
        bytes.extend(le(&[0x0000, 0x4000])); // +100ms -> t=100
        bytes.extend(le(&[0x0001, 0x0000, 0x001E])); // t=100, channel 30
        for _ in 0..11 {
            bytes.extend(le(&[0x0000, 0x4000])); // +11*100ms -> t=1200
        }
        bytes.extend(le(&[0x0001, 0x0000, 0x000A])); // t=1200, channel 10
        bytes.extend(le(&[0x0000, 0x4000])); // +100ms -> t=1300
        bytes.extend(le(&[0x0001, 0x0000, 0x0063])); // t=1300, channel 99
        bytes
    }

    #[test]
    fn list_integration_writes_exact_csv() {
        let dir =
            std::env::temp_dir().join(format!("moulinette-cli-list-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input_path = dir.join("fixture.lst");
        std::fs::write(&input_path, synthetic_lst_fixture()).unwrap();
        let output_dir = dir.join("out");

        let cli_args = vec![
            OsString::from(input_path.as_os_str()),
            OsString::from("--roi"),
            OsString::from("peak:1:20:35:kev"),
            OsString::from("--dt-ms"),
            OsString::from("1000"),
            OsString::from("--output-dir"),
            OsString::from(output_dir.as_os_str()),
        ];
        run_list(list_options(cli_args).unwrap()).unwrap();

        let csv = std::fs::read_to_string(output_dir.join("peak.csv")).unwrap();
        assert_eq!(
            csv,
            "time_s,rate,rate_err,counts\n0.5,2,1.4142135623730951,2\n1.5,0,0,0\n"
        );
        let png_metadata = std::fs::metadata(output_dir.join("rate_vs_time.png")).unwrap();
        assert!(png_metadata.len() > 0);

        std::fs::remove_dir_all(&dir).ok();
    }
}
