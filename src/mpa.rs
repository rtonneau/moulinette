//! Byte-oriented MPA text carrier. See docs/mpa_format.md for compatibility limits.
use std::{
    collections::HashMap,
    fmt,
    io::{self, BufRead, Cursor},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub line: usize,
    pub bytes: Vec<u8>,
}
impl Record {
    /// Legacy view of metadata: first 80 bytes, uppercase key, trailing spaces ignored.
    pub fn key_value(&self) -> Option<(Vec<u8>, &[u8])> {
        let bytes = &self.bytes[..self.bytes.len().min(80)];
        let eq = bytes.iter().position(|&b| b == b'=')?;
        Some((
            trim_spaces(&bytes[..eq]).to_ascii_uppercase(),
            &bytes[eq + 1..],
        ))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionKind {
    Global,
    Adc(usize),
    Data,
    Other,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub kind: SectionKind,
    pub records: Vec<Record>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spectrum {
    pub header: Record,
    /// Opaque DATA suffix, never used to select metadata.
    pub data_identifier: Vec<u8>,
    /// One-based DATA occurrence, also its associated ADC index.
    pub adc_index: usize,
    pub declared_channels: usize,
    /// Snapshot of ADC records when DATA was encountered.
    pub metadata: Vec<Record>,
    pub channels: Vec<Record>,
}
impl Spectrum {
    pub fn metadata_value(&self, key: &str) -> Option<&[u8]> {
        self.metadata.iter().rev().find_map(|r| {
            let (candidate, value) = r.key_value()?;
            candidate
                .eq_ignore_ascii_case(key.as_bytes())
                .then_some(value)
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MpaFile {
    pub sections: Vec<Section>,
    pub spectra: Vec<Spectrum>,
}
/// Resource policy, not a recovered limit of the Fortran program.
#[derive(Debug, Clone, Copy)]
pub struct ParseOptions {
    pub max_channels_per_spectrum: usize,
}
impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            max_channels_per_spectrum: 1_000_000,
        }
    }
}
#[derive(Debug)]
pub enum ParseErrorKind {
    Io(io::Error),
    InvalidSection,
    InvalidAdcIndex,
    InvalidChannelCount,
    ChannelLimit { count: usize, limit: usize },
    TruncatedData { expected: usize, found: usize },
    NoData,
}
#[derive(Debug)]
pub struct ParseError {
    pub line: usize,
    pub kind: ParseErrorKind,
}
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MPA line {}: ", self.line)?;
        match &self.kind {
            ParseErrorKind::Io(e) => write!(f, "{e}"),
            ParseErrorKind::InvalidSection => write!(f, "malformed ADC/DATA section header"),
            ParseErrorKind::InvalidAdcIndex => write!(f, "ADC index must be a positive integer"),
            ParseErrorKind::InvalidChannelCount => {
                write!(f, "channel count must be a positive integer")
            }
            ParseErrorKind::ChannelLimit { count, limit } => {
                write!(f, "{count} channels exceed parser limit {limit}")
            }
            ParseErrorKind::TruncatedData { expected, found } => {
                write!(f, "expected {expected} channel records, found {found}")
            }
            ParseErrorKind::NoData => write!(f, "no DATA sections found"),
        }
    }
}
impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            ParseErrorKind::Io(e) => Some(e),
            _ => None,
        }
    }
}
pub(crate) fn trim_spaces(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes.iter().rposition(|&b| b != b' ').map_or(0, |i| i + 1)]
}
fn positive_integer(bytes: &[u8]) -> Option<usize> {
    let n = std::str::from_utf8(bytes)
        .ok()?
        .trim_matches(' ')
        .parse::<usize>()
        .ok()?;
    (n > 0).then_some(n)
}
fn record(reader: &mut impl BufRead, line: &mut usize) -> Result<Option<Record>, ParseError> {
    let mut bytes = Vec::new();
    let n = reader
        .read_until(b'\n', &mut bytes)
        .map_err(|e| ParseError {
            line: *line + 1,
            kind: ParseErrorKind::Io(e),
        })?;
    if n == 0 {
        return Ok(None);
    }
    *line += 1;
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
    }
    Ok(Some(Record { line: *line, bytes }))
}
/// Parse CRLF/LF records without decoding values or counts. Unknown records survive.
/// Strict malformed-header/short-block checks intentionally replace unsafe legacy behavior.
pub fn read(mut reader: impl BufRead, options: ParseOptions) -> Result<MpaFile, ParseError> {
    let mut file = MpaFile {
        sections: vec![Section {
            kind: SectionKind::Global,
            records: vec![],
        }],
        spectra: vec![],
    };
    let mut line = 0;
    let mut current_adc = None;
    let mut metadata_by_adc: HashMap<usize, Vec<Record>> = HashMap::new();
    while let Some(r) = record(&mut reader, &mut line)? {
        let header = trim_spaces(&r.bytes);
        let upper = header.to_ascii_uppercase();
        let error = |kind| ParseError { line: r.line, kind };
        if upper.starts_with(b"[ADC") {
            if !header.ends_with(b"]") {
                return Err(error(ParseErrorKind::InvalidSection));
            }
            let index = positive_integer(&header[4..header.len() - 1])
                .ok_or_else(|| error(ParseErrorKind::InvalidAdcIndex))?;
            current_adc = Some(index);
            file.sections.push(Section {
                kind: SectionKind::Adc(index),
                records: vec![r],
            });
        } else if upper.starts_with(b"[DATA") {
            let comma = header
                .iter()
                .position(|&b| b == b',')
                .filter(|&i| i >= 5)
                .ok_or_else(|| error(ParseErrorKind::InvalidSection))?;
            if !header.ends_with(b"]") {
                return Err(error(ParseErrorKind::InvalidSection));
            }
            let count = positive_integer(&header[comma + 1..header.len() - 1])
                .ok_or_else(|| error(ParseErrorKind::InvalidChannelCount))?;
            if count > options.max_channels_per_spectrum {
                return Err(error(ParseErrorKind::ChannelLimit {
                    count,
                    limit: options.max_channels_per_spectrum,
                }));
            }
            let adc_index = file.spectra.len() + 1;
            let metadata = metadata_by_adc.get(&adc_index).cloned().unwrap_or_default();
            let mut spectrum = Spectrum {
                data_identifier: header[5..comma].to_vec(),
                adc_index,
                declared_channels: count,
                metadata,
                header: r.clone(),
                channels: vec![],
            };
            for found in 0..count {
                let Some(channel) = record(&mut reader, &mut line)? else {
                    return Err(ParseError {
                        line: line + 1,
                        kind: ParseErrorKind::TruncatedData {
                            expected: count,
                            found,
                        },
                    });
                };
                if channel.bytes.starts_with(b"[") {
                    return Err(ParseError {
                        line: channel.line,
                        kind: ParseErrorKind::TruncatedData {
                            expected: count,
                            found,
                        },
                    });
                }
                spectrum.channels.push(channel);
            }
            file.spectra.push(spectrum);
            file.sections.push(Section {
                kind: SectionKind::Data,
                records: vec![r],
            });
        } else if header.starts_with(b"[") {
            file.sections.push(Section {
                kind: SectionKind::Other,
                records: vec![r],
            });
        } else {
            // Like the scanner's ADC global, this association persists across
            // DATA and unrecognized section headers until another ADC is read.
            if let Some(adc) = current_adc {
                metadata_by_adc.entry(adc).or_default().push(r.clone());
            }
            file.sections
                .last_mut()
                .expect("global section exists")
                .records
                .push(r);
        }
    }
    if file.spectra.is_empty() {
        return Err(ParseError {
            line,
            kind: ParseErrorKind::NoData,
        });
    }
    Ok(file)
}
pub fn parse(bytes: &[u8]) -> Result<MpaFile, ParseError> {
    read(Cursor::new(bytes), ParseOptions::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scanner_adc_state_persists_after_data_and_unknown_headers() {
        let f = parse(
            b"[ADC2]\ncaloff=1\n[DATA9,1]\n10\ncaloff=2\n[IGNORED]\ncalfact=3\n[DATA1,1]\n20\n",
        )
        .unwrap();
        assert!(f.spectra[0].metadata.is_empty()); // DATA occurrence1 => absent ADC1
        assert_eq!(f.spectra[1].metadata_value("CALOFF"), Some(b"2".as_slice()));
        assert_eq!(
            f.spectra[1].metadata_value("CALFACT"),
            Some(b"3".as_slice())
        );
    }
    #[test]
    fn bytes_order_and_association() {
        let f = parse(b"unknown=x\r\n[ADC2]\r\ncaloff=9\r\n[ADC1]\r\nlabel=\xb0\r\ncaloff=1\r\nCALOFF=2\r\n[DATA7,2 ]\r\n 0001 \r\n1E3\r\n[DATA0,1]\r\n5").unwrap();
        assert_eq!(f.spectra[0].adc_index, 1);
        assert_eq!(f.spectra[0].data_identifier, b"7");
        assert_eq!(f.spectra[0].metadata_value("caloff"), Some(b"2".as_slice()));
        assert_eq!(
            f.spectra[0].metadata_value("label"),
            Some(b"\xb0".as_slice())
        );
        assert_eq!(f.spectra[0].channels[0].bytes, b" 0001 ");
        assert_eq!(f.spectra[1].metadata_value("CALOFF"), Some(b"9".as_slice()));
        assert_eq!(f.sections[0].records[0].bytes, b"unknown=x");
    }
    #[test]
    fn snapshot_and_width() {
        let mut b = b"[ADC1]\ncaloff=1\nlong=".to_vec();
        b.extend_from_slice(&[b'x'; 100]);
        b.extend_from_slice(b"\n[DATA,1]\n123\n[ADC1]\ncaloff=8\n");
        let f = parse(&b).unwrap();
        assert_eq!(f.spectra[0].metadata_value("CALOFF"), Some(b"1".as_slice()));
        assert_eq!(f.spectra[0].metadata_value("long").unwrap().len(), 75);
        assert_eq!(f.spectra[0].metadata[1].bytes.len(), 105);
    }
    #[test]
    fn diagnostic_errors() {
        for b in [b"[DATA0,2]\n1\n".as_slice(), b"[DATA0,2]\n1\n[ADC2]\n"] {
            let e = parse(b).unwrap_err();
            assert_eq!(e.line, 3);
            assert!(matches!(
                e.kind,
                ParseErrorKind::TruncatedData {
                    expected: 2,
                    found: 1
                }
            ));
        }
        assert!(matches!(
            parse(b"[ADC0]\n").unwrap_err().kind,
            ParseErrorKind::InvalidAdcIndex
        ));
        assert!(matches!(
            parse(b"[DATA0,-1]\n").unwrap_err().kind,
            ParseErrorKind::InvalidChannelCount
        ));
        assert!(matches!(
            parse(b"[DATA0 1]\n").unwrap_err().kind,
            ParseErrorKind::InvalidSection
        ));
        assert!(matches!(
            read(
                Cursor::new(b"[DATA0,9]\n"),
                ParseOptions {
                    max_channels_per_spectrum: 8
                }
            )
            .unwrap_err()
            .kind,
            ParseErrorKind::ChannelLimit { .. }
        ));
    }
    #[test]
    fn sample() {
        let f = parse(include_bytes!("../res/15032501.mpa")).unwrap();
        assert_eq!(
            f.spectra
                .iter()
                .map(|s| s.channels.len())
                .collect::<Vec<_>>(),
            [8192, 8192, 8192, 256]
        );
        assert_eq!(
            f.sections
                .iter()
                .filter(|s| matches!(s.kind, SectionKind::Adc(_)))
                .count(),
            8
        );
        assert_eq!(
            f.spectra[0].metadata_value("CMLINE1"),
            Some(b"RBS135\xb0".as_slice())
        );
        for s in f.spectra {
            assert_eq!(s.channels.len(), s.declared_channels);
            assert!(
                s.channels
                    .iter()
                    .all(|r| r.bytes.iter().all(u8::is_ascii_digit))
            );
        }
    }
}
