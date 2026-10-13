#![forbid(unsafe_code)]
use moulinette::{
    conversion::{self, Format, Selection},
    workflow,
};
use std::{env, ffi::OsString, path::PathBuf, process::ExitCode};

const HELP: &str = "Moulinette — static-analysis-based native MPA converter\n\nUsage:\n  moulinette formats\n  moulinette convert INPUT --format FORMAT --output FILE [--spectrum N]\n  moulinette convert INPUT --format FORMAT --output-dir DIR [--spectrum N]\n\nOptions:\n  --spectrum N  One-based DATA occurrence (required for single output with multiple spectra)\n  --start N     Zero-based inclusive first channel (default 0)\n  --end N       Zero-based inclusive last channel (default final channel)\n\nExisting output files are never overwritten. TOF always emits all channels.\n";

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
        _ => Err("unknown command; use 'help', 'formats' or 'convert'".into()),
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
}
