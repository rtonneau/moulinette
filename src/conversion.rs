//! Reconstructed writers. Evidence and formatting limits: docs/conversion_algorithm.md.
use crate::mpa::{Spectrum, trim_spaces};
use std::fmt;
use std::str::FromStr;

/// Seven writer families, with legacy scientific/one-column mode aliases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Xy,
    Ey,
    OneColumn,
    Pix,
    Pig,
    Mon,
    Tof,
    EightColumn,
    Pro,
    Rbs,
    Nra,
    Erd,
    /// Ratio of each channel's counts to a user-chosen monitor ADC/
    /// spectrum's total counts over the run (ADR 0002's integrated-charge
    /// proxy), optionally scaled to counts-per-µC. This needs a second
    /// spectrum (the monitor) alongside the one `convert` receives, so it
    /// is registered here (name/legacy filename/CLI `formats` listing)
    /// but reported unsupported through the generic one-spectrum
    /// `convert` path; the real computation is
    /// `gui_state::charge_normalized`, called directly by the GUI's
    /// Convert screen for this format only.
    ChargeNormalized,
}
impl Format {
    pub const ALL: [Self; 13] = [
        Self::Xy,
        Self::Ey,
        Self::OneColumn,
        Self::Pix,
        Self::Pig,
        Self::Mon,
        Self::Tof,
        Self::EightColumn,
        Self::Pro,
        Self::Rbs,
        Self::Nra,
        Self::Erd,
        Self::ChargeNormalized,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Xy => "xy",
            Self::Ey => "ey",
            Self::OneColumn => "1co",
            Self::Pix => "pix",
            Self::Pig => "pig",
            Self::Mon => "mon",
            Self::Tof => "tof",
            Self::EightColumn => "8co",
            Self::Pro => "pro",
            Self::Rbs => "rbs",
            Self::Nra => "nra",
            Self::Erd => "erd",
            Self::ChargeNormalized => "chg",
        }
    }
    pub fn unsupported_reason(self) -> Option<&'static str> {
        match self {
            Self::Pro => Some(
                "PRO header defaults, numeric E10.4 formatting and header-failure behavior are not reconstructed reliably",
            ),
            Self::Rbs | Self::Nra | Self::Erd => Some(
                "scientific headers depend on missing templates, GUI defaults and unresolved numeric-text sanitization",
            ),
            Self::ChargeNormalized => Some(
                "needs a second, user-chosen monitor ADC/spectrum alongside the one this function receives; use the GUI's Convert screen charge-normalization panel, or call gui_state::charge_normalized directly",
            ),
            _ => None,
        }
    }
}
impl FromStr for Format {
    type Err = ConversionError;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.to_ascii_lowercase();
        match text.as_str() {
            "energy-yield" => Ok(Self::Ey),
            "pixe" => Ok(Self::Pix),
            "pige" => Ok(Self::Pig),
            _ => Self::ALL
                .into_iter()
                .find(|f| f.name() == text)
                .ok_or(ConversionError::UnknownFormat(text)),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionError {
    InvalidRange {
        start: usize,
        end: usize,
        channels: usize,
    },
    InconsistentSpectrum,
    UnknownFormat(String),
    Unsupported {
        format: Format,
        reason: &'static str,
    },
    MissingMetadata {
        adc: usize,
        key: &'static str,
    },
    InvalidMetadata {
        adc: usize,
        key: &'static str,
        value: Vec<u8>,
    },
    NonFiniteEnergy {
        channel: usize,
    },
    PartialEightColumnRow {
        points: usize,
    },
    InvalidFilename,
    /// `gui_state::charge_normalized`'s monitor ADC/spectrum accumulated
    /// zero counts over the run, so the monitor-count ratio (and any
    /// counts-per-µC scaling) is undefined; returned instead of dividing
    /// by zero. See ADR 0002.
    MonitorNoCounts,
    /// `gui_state::charge_normalized` could not parse a channel's count
    /// text as a finite number (via `gui_state::channel_count`); `reason`
    /// is that function's own diagnostic message.
    InvalidCount(String),
}
impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRange {
                start,
                end,
                channels,
            } => write!(
                f,
                "invalid inclusive channel range {start}..{end} for {channels} channels"
            ),
            Self::InconsistentSpectrum => {
                write!(f, "declared channel count differs from stored records")
            }
            Self::UnknownFormat(name) => {
                write!(f, "unknown format {name:?}; use 'formats' to list choices")
            }
            Self::Unsupported { format, reason } => {
                write!(f, "format {} is unsupported: {reason}", format.name())
            }
            Self::MissingMetadata { adc, key } => {
                write!(f, "ADC{adc}: missing {key} calibration metadata")
            }
            Self::InvalidMetadata { adc, key, value } => write!(
                f,
                "ADC{adc}: invalid finite REAL*4 {key} value {:?}",
                String::from_utf8_lossy(value)
            ),
            Self::NonFiniteEnergy { channel } => write!(
                f,
                "channel {channel}: calibrated coordinate overflows REAL*4"
            ),
            Self::PartialEightColumnRow { points } => write!(
                f,
                "8CO requires a multiple of eight points; selected {points}. Legacy partial-row bytes are unknown"
            ),
            Self::InvalidFilename => write!(
                f,
                "cannot derive safe legacy filename (empty basename or occurrence beyond supported suffix letters)"
            ),
            Self::MonitorNoCounts => write!(f, "monitor ADC recorded no counts"),
            Self::InvalidCount(reason) => write!(f, "{reason}"),
        }
    }
}
impl std::error::Error for ConversionError {}
/// Inclusive zero-based bounds; None selects through the final channel.
#[derive(Debug, Clone, Copy, Default)]
pub struct Selection {
    pub start: usize,
    pub end: Option<usize>,
}
pub(crate) fn bounds(s: &Spectrum, selection: Selection) -> Result<(usize, usize), ConversionError> {
    let channels = s.channels.len();
    if channels != s.declared_channels {
        return Err(ConversionError::InconsistentSpectrum);
    }
    let end = selection.end.unwrap_or(channels.saturating_sub(1));
    if channels == 0 || selection.start > end || end >= channels {
        return Err(ConversionError::InvalidRange {
            start: selection.start,
            end,
            channels,
        });
    }
    Ok((selection.start, end))
}
fn count_text(bytes: &[u8]) -> &[u8] {
    trim_spaces(&bytes[..bytes.len().min(20)])
}
/// I8, 5X, A; CRLF is an explicit implementation convention, not verified RTL bytes.
pub fn xy(s: &Spectrum, selection: Selection) -> Result<Vec<u8>, ConversionError> {
    let (start, end) = bounds(s, selection)?;
    let mut out = Vec::new();
    for i in start..=end {
        if i.to_string().len() > 8 {
            out.extend_from_slice(b"********");
        } else {
            out.extend_from_slice(format!("{i:>8}").as_bytes());
        }
        out.extend_from_slice(b"     ");
        out.extend_from_slice(count_text(&s.channels[i].bytes));
        out.extend_from_slice(b"\r\n");
    }
    Ok(out)
}

pub(crate) fn coefficient(s: &Spectrum, key: &'static str) -> Result<f32, ConversionError> {
    let bytes = s
        .metadata_value(key)
        .ok_or(ConversionError::MissingMetadata {
            adc: s.adc_index,
            key,
        })?;
    let value = std::str::from_utf8(bytes)
        .ok()
        .and_then(|t| {
            t.trim_matches(' ')
                .replace(['d', 'D'], "E")
                .parse::<f32>()
                .ok()
        })
        .filter(|v| v.is_finite());
    value.ok_or_else(|| ConversionError::InvalidMetadata {
        adc: s.adc_index,
        key,
        value: bytes.to_vec(),
    })
}
fn fixed_energy(value: f32) -> String {
    let text = format!("{value:.3}");
    if text.len() > 10 {
        "**********".into()
    } else {
        format!("{text:>10}")
    }
}
/// Float32 coefficients, wide intermediate, float32 final store. This emulates
/// the observed x87 operation sequence for ordinary inputs, not every precision
/// boundary. Counts remain text; no yield normalization is performed.
pub fn energy_counts(s: &Spectrum, selection: Selection) -> Result<Vec<u8>, ConversionError> {
    let (start, end) = bounds(s, selection)?;
    let slope = coefficient(s, "CALFACT")?;
    let offset = coefficient(s, "CALOFF")?;
    let mut out = Vec::new();
    for index in start..=end {
        let energy = ((index + 1) as f64 * f64::from(slope) + f64::from(offset)) as f32;
        if !energy.is_finite() {
            return Err(ConversionError::NonFiniteEnergy { channel: index });
        }
        out.extend_from_slice(fixed_energy(energy).as_bytes());
        out.extend_from_slice(b"     ");
        out.extend_from_slice(count_text(&s.channels[index].bytes));
        out.extend_from_slice(b"\r\n");
    }
    Ok(out)
}
pub fn one_column(s: &Spectrum, selection: Selection) -> Result<Vec<u8>, ConversionError> {
    let (start, end) = bounds(s, selection)?;
    let mut out = Vec::new();
    for record in &s.channels[start..=end] {
        out.push(b' ');
        out.extend_from_slice(count_text(&record.bytes));
        out.extend_from_slice(b"\r\n");
    }
    Ok(out)
}
/// Eight adjacent A10 fields. Partial rows are explicitly unsupported instead
/// of exposing uninitialized/neighboring values as the legacy routine can.
pub fn eight_column(s: &Spectrum, selection: Selection) -> Result<Vec<u8>, ConversionError> {
    let (start, end) = bounds(s, selection)?;
    let points = end - start + 1;
    if points % 8 != 0 {
        return Err(ConversionError::PartialEightColumnRow { points });
    }
    let mut out = Vec::new();
    for row in s.channels[start..=end].chunks_exact(8) {
        for record in row {
            let text = count_text(&record.bytes);
            out.extend(std::iter::repeat_n(
                b' ',
                10usize.saturating_sub(text.len()),
            ));
            out.extend_from_slice(&text[..text.len().min(10)]);
        }
        out.extend_from_slice(b"\r\n");
    }
    Ok(out)
}
pub fn convert(
    s: &Spectrum,
    format: Format,
    selection: Selection,
) -> Result<Vec<u8>, ConversionError> {
    if let Some(reason) = format.unsupported_reason() {
        return Err(ConversionError::Unsupported { format, reason });
    }
    match format {
        Format::Xy => xy(s, selection),
        Format::Ey => energy_counts(s, selection),
        Format::EightColumn => eight_column(s, selection),
        Format::Tof => one_column(s, Selection::default()),
        _ => one_column(s, selection),
    }
}
/// Explicit-mode seeded counter: occurrence 1 => .XYb, occurrence 2 => .XYc.
/// Applies first-dot replacement to the basename only, protecting parent paths.
/// No automatic classification or MON override is performed by this API.
pub fn legacy_filename(
    input_basename: &str,
    format: Format,
    occurrence: usize,
) -> Result<String, ConversionError> {
    let stem = input_basename.split('.').next().unwrap_or("");
    if stem.is_empty() || stem.contains(['/', '\\']) || !(1..=25).contains(&occurrence) {
        return Err(ConversionError::InvalidFilename);
    }
    let code = format.name().to_ascii_uppercase();
    let prefix = &code[..2];
    let suffix = char::from(b'a' + occurrence as u8);
    Ok(format!("{stem}.{prefix}{suffix}"))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpa::parse;
    #[test]
    fn independently_calculated_energy_and_selection() {
        let f=parse(b"[ADC1]\ncaloff=1.25\ncalfact=2.5\ncalfact2=999\ncaluse=0\nlivetime=100\n[DATA8,3]\n10\n020\n30\n").unwrap();
        // One-based i: 1*2.5+1.25=3.75, 2*2.5+1.25=6.25, 3*2.5+1.25=8.75.
        assert_eq!(
            energy_counts(&f.spectra[0], Selection::default()).unwrap(),
            b"     3.750     10\r\n     6.250     020\r\n     8.750     30\r\n"
        );
        assert_eq!(
            energy_counts(
                &f.spectra[0],
                Selection {
                    start: 1,
                    end: Some(1)
                }
            )
            .unwrap(),
            b"     6.250     020\r\n"
        );
    }
    #[test]
    fn coefficient_quantization_negative_slope_and_d_exponent() {
        let f = parse(b"[ADC1]\ncaloff=16777217\ncalfact=-1D0\n[DATA0,2]\n1\n2\n").unwrap();
        // 16777217 rounds to 16777216 in REAL*4. Output overflows F10.3.
        assert_eq!(coefficient(&f.spectra[0], "CALOFF").unwrap(), 16777216.0);
        assert_eq!(
            energy_counts(&f.spectra[0], Selection::default()).unwrap(),
            b"**********     1\r\n**********     2\r\n"
        );
        let f = parse(b"[ADC1]\ncaloff=0\ncalfact=-0.125\n[DATA0,2]\n1\n2\n").unwrap();
        assert_eq!(
            energy_counts(&f.spectra[0], Selection::default()).unwrap(),
            b"    -0.125     1\r\n    -0.250     2\r\n"
        );
    }
    #[test]
    fn float_metadata_diagnostics_and_overflow() {
        let f = parse(b"[DATA0,1]\n1\n").unwrap();
        assert!(matches!(
            energy_counts(&f.spectra[0], Selection::default()),
            Err(ConversionError::MissingMetadata { key: "CALFACT", .. })
        ));
        for value in ["nan", "inf", "not-a-number", "1e99"] {
            let input = format!("[ADC1]\ncaloff=0\ncalfact={value}\n[DATA0,1]\n1\n");
            let f = parse(input.as_bytes()).unwrap();
            assert!(matches!(
                energy_counts(&f.spectra[0], Selection::default()),
                Err(ConversionError::InvalidMetadata { .. })
            ));
        }
        let f = parse(b"[ADC1]\ncaloff=0\ncalfact=3e38\n[DATA0,2]\n1\n2\n").unwrap();
        assert!(matches!(
            energy_counts(&f.spectra[0], Selection::default()),
            Err(ConversionError::NonFiniteEnergy { channel: 1 })
        ));
    }
    #[test]
    fn one_column_aliases_and_tof_ignores_crop() {
        let f = parse(b"[DATA0,3]\n 01 \n2E3\n\xb0\n").unwrap();
        for format in [Format::OneColumn, Format::Pix, Format::Pig, Format::Mon] {
            assert_eq!(
                convert(
                    &f.spectra[0],
                    format,
                    Selection {
                        start: 1,
                        end: Some(1)
                    }
                )
                .unwrap(),
                b" 2E3\r\n"
            );
        }
        assert_eq!(
            convert(
                &f.spectra[0],
                Format::Tof,
                Selection {
                    start: 999,
                    end: Some(0)
                }
            )
            .unwrap(),
            b"  01\r\n 2E3\r\n \xb0\r\n"
        );
    }
    #[test]
    fn eight_column_exact_fields_and_partial_rejection() {
        let f =
            parse(b"[DATA0,9]\n1\n22\n333\n4444\n55555\n666666\n7777777\n88888888\n9\n").unwrap();
        let bytes = eight_column(
            &f.spectra[0],
            Selection {
                start: 0,
                end: Some(7),
            },
        )
        .unwrap();
        assert_eq!(
            bytes,
            b"         1        22       333      4444     55555    666666   7777777  88888888\r\n"
        );
        assert!(matches!(
            eight_column(&f.spectra[0], Selection::default()),
            Err(ConversionError::PartialEightColumnRow { points: 9 })
        ));
        let f = parse(b"[DATA0,8]\n123456789012345\n\n 1\n4\n5\n6\n7\n8\n").unwrap();
        let bytes = eight_column(&f.spectra[0], Selection::default()).unwrap();
        assert_eq!(&bytes[..30], b"1234567890                   1");
    }
    #[test]
    fn unsupported_inventory_and_legacy_names() {
        let f = parse(b"[DATA0,1]\n1\n").unwrap();
        for format in [
            Format::Pro,
            Format::Rbs,
            Format::Nra,
            Format::Erd,
            Format::ChargeNormalized,
        ] {
            assert!(matches!(
                convert(&f.spectra[0], format, Selection::default()),
                Err(ConversionError::Unsupported { .. })
            ));
        }
        // `ChargeNormalized`'s legacy prefix ("CH") must not collide with
        // any other registered format's first two letters.
        let prefixes: Vec<String> = Format::ALL
            .iter()
            .map(|f| f.name().to_ascii_uppercase()[..2.min(f.name().len())].to_string())
            .collect();
        let charge_prefix = &prefixes[prefixes.len() - 1];
        assert_eq!(charge_prefix, "CH");
        assert_eq!(
            prefixes.iter().filter(|p| *p == charge_prefix).count(),
            1,
            "CH collides with another format's legacy prefix: {prefixes:?}"
        );
        assert_eq!(
            legacy_filename("sample.mpa", Format::ChargeNormalized, 1).unwrap(),
            "sample.CHb"
        );
        assert_eq!(
            legacy_filename("sample.extra.mpa", Format::Xy, 1).unwrap(),
            "sample.XYb"
        );
        assert_eq!(
            legacy_filename("sample.mpa", Format::OneColumn, 2).unwrap(),
            "sample.1Cc"
        );
        assert_eq!(
            legacy_filename("sample.mpa", Format::Rbs, 1).unwrap(),
            "sample.RBb"
        );
        assert!(legacy_filename(".mpa", Format::Xy, 1).is_err());
        assert!(legacy_filename("sample", Format::Xy, 26).is_err());
    }
    #[test]
    fn sample_outputs_have_expected_record_counts() {
        let f = parse(include_bytes!("../res/15032501.mpa")).unwrap();
        for s in &f.spectra {
            for format in [
                Format::Xy,
                Format::Ey,
                Format::OneColumn,
                Format::Tof,
                Format::EightColumn,
            ] {
                let bytes = convert(s, format, Selection::default()).unwrap();
                let records = bytes.iter().filter(|&&b| b == b'\n').count();
                assert_eq!(
                    records,
                    if format == Format::EightColumn {
                        s.declared_channels / 8
                    } else {
                        s.declared_channels
                    }
                );
            }
        }
    }
    #[test]
    fn exact_xy_crop_and_bounds() {
        let f = parse(b"[DATA77,3]\n 0001  \n123456789012345678901234\n\n").unwrap();
        assert_eq!(
            xy(&f.spectra[0], Selection::default()).unwrap(),
            b"       0      0001\r\n       1     12345678901234567890\r\n       2     \r\n"
        );
        assert_eq!(
            xy(
                &f.spectra[0],
                Selection {
                    start: 1,
                    end: Some(1)
                }
            )
            .unwrap(),
            b"       1     12345678901234567890\r\n"
        );
        assert!(
            xy(
                &f.spectra[0],
                Selection {
                    start: 0,
                    end: Some(3)
                }
            )
            .is_err()
        );
    }
}
