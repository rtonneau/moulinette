//! Shared output planning and exclusive writing, independent of the UI toolkit.
use crate::{
    conversion::{self, Format, Selection},
    mpa::{self, MpaFile},
};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufReader, Write},
    path::{Path, PathBuf},
};

pub fn load(path: &Path) -> Result<MpaFile, String> {
    let input = File::open(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    mpa::read(BufReader::new(input), mpa::ParseOptions::default()).map_err(|e| e.to_string())
}

pub fn filenames(
    input: &Path,
    dir: &Path,
    format: Format,
    indices: &[usize],
) -> Result<Vec<PathBuf>, String> {
    if dir.as_os_str().is_empty() {
        return Err("Choose an output directory".into());
    }
    let basename = input
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("legacy output naming requires a Unicode input basename; use --output")?;
    indices
        .iter()
        .map(|&i| {
            conversion::legacy_filename(basename, format, i + 1)
                .map(|name| dir.join(name))
                .map_err(|e| e.to_string())
        })
        .collect()
}

pub fn render(
    file: &MpaFile,
    format: Format,
    selection: Selection,
    indices: &[usize],
    paths: Vec<PathBuf>,
) -> Result<Vec<(PathBuf, Vec<u8>)>, String> {
    if indices.is_empty() || indices.len() != paths.len() {
        return Err("Choose at least one spectrum".into());
    }
    let mut outputs = Vec::new();
    for (&index, path) in indices.iter().zip(paths) {
        if outputs.iter().any(|(previous, _)| previous == &path) {
            return Err("Duplicate output destination".into());
        }
        let spectrum = file
            .spectra
            .get(index)
            .ok_or("Spectrum selection is out of range")?;
        let bytes = conversion::convert(spectrum, format, selection).map_err(|e| e.to_string())?;
        if path
            .try_exists()
            .map_err(|e| format!("cannot inspect {}: {e}", path.display()))?
        {
            return Err(format!(
                "output {} already exists; choose a new path",
                path.display()
            ));
        }
        outputs.push((path, bytes));
    }
    Ok(outputs)
}

/// Validate/render the entire batch before writing. create_new also protects against races.
/// Filesystem failures may leave partial output; callers receive that warning in the error.
pub fn write(
    outputs: Vec<(PathBuf, Vec<u8>)>,
    directory: Option<&Path>,
) -> Result<Vec<PathBuf>, String> {
    if let Some(dir) = directory {
        fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    let mut generated = Vec::new();
    for (path, bytes) in outputs {
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| {
                format!(
                    "cannot create {}: {e}. Earlier files in this batch may have been written.",
                    path.display()
                )
            })?;
        output.write_all(&bytes).map_err(|e| {
            format!(
                "cannot write {}: {e}. Partial files may remain.",
                path.display()
            )
        })?;
        generated.push(path);
    }
    Ok(generated)
}
