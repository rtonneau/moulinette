//! Byte-oriented MPA-3 list-mode (`.lst`) parser. See docs/lst_format.md for
//! the header dialect and binary record layout this module implements.
use std::fmt;

use crate::mpa::trim_spaces;

/// Per-ADC calibration/metadata declared in the `.lst` ASCII header.
///
/// Values are stored exactly as read from the header; ticket 2 decides how
/// `calfact2`/`calfact3` affect ROI resolution (v1 only implements linear
/// `caloff`/`calfact` calibration elsewhere in the app).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdcHeader {
    /// The literal number following `ADC` in the `[ADCn]` section header
    /// (1-based, matching `src/mpa.rs`'s `SectionKind::Adc` convention).
    pub adc_index: u8,
    pub caloff: Option<f64>,
    pub calfact: Option<f64>,
    pub calfact2: Option<f64>,
    pub calfact3: Option<f64>,
    pub n_channels: u32,
}

/// One decoded event record from the binary list-data stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    /// Accumulated timer clock, in ms, at the time this event was recorded.
    pub time_ms: u64,
    /// One slot per possible ADC (bits 0-7 of the event's ADC mask); `Some`
    /// for ADCs flagged active in this event, `None` otherwise.
    pub adc_values: [Option<u16>; 8],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListFile {
    pub adc_headers: Vec<AdcHeader>,
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LstParseError {
    /// The ASCII header never contained a `[LISTDATA]` marker.
    MissingListData,
    /// A 16-bit word (identified by its 0-based index into the binary word
    /// stream) was neither a timer word, a sync marker, nor a valid event
    /// header.
    UnrecognizedWord { word_index: usize },
    /// EOF was reached while reading an event's RTC words or ADC channel
    /// values; `word_index` is the index of the word the parser was
    /// attempting to read.
    TruncatedEvent { word_index: usize },
}

impl fmt::Display for LstParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LstParseError::MissingListData => write!(f, "missing [LISTDATA] marker"),
            LstParseError::UnrecognizedWord { word_index } => {
                write!(f, "unrecognized word at word index {word_index}")
            }
            LstParseError::TruncatedEvent { word_index } => {
                write!(
                    f,
                    "truncated event: unexpected EOF at word index {word_index}"
                )
            }
        }
    }
}

impl std::error::Error for LstParseError {}

fn strip_cr(line: &[u8]) -> &[u8] {
    if line.last() == Some(&b'\r') {
        &line[..line.len() - 1]
    } else {
        line
    }
}

/// Parse a positive `ADCn` section index (the digits following `ADC`, with
/// the trailing `]` already stripped by the caller).
fn adc_section_index(digits: &[u8]) -> Option<u8> {
    std::str::from_utf8(digits).ok()?.trim().parse::<u8>().ok()
}

/// Locate the first case-insensitive occurrence of `[LISTDATA]`, returning
/// the byte offset just past it (the start of the binary event stream).
fn find_list_data_end(bytes: &[u8]) -> Option<usize> {
    const MARKER: &[u8] = b"[LISTDATA]";
    if bytes.len() < MARKER.len() {
        return None;
    }
    (0..=bytes.len() - MARKER.len()).find_map(|start| {
        let window = &bytes[start..start + MARKER.len()];
        window
            .iter()
            .zip(MARKER)
            .all(|(b, m)| b.to_ascii_uppercase() == *m)
            .then_some(start + MARKER.len())
    })
}

/// Parse the ASCII header: `[ADCn]` sections and their `caloff`/`calfact`/
/// `calfact2`/`calfact3`/`nchannels` key=value lines, plus the global
/// `timerreduce` key. See docs/lst_format.md for the full dialect.
fn parse_header(header: &[u8]) -> (Vec<AdcHeader>, u64) {
    let mut adc_headers: Vec<AdcHeader> = Vec::new();
    let mut current: Option<usize> = None;
    let mut timer_reduce: u64 = 1;
    for raw_line in header.split(|&b| b == b'\n') {
        let line = trim_spaces(strip_cr(raw_line));
        if line.is_empty() {
            continue;
        }
        if line[0] == b'[' {
            current = None;
            let upper = line.to_ascii_uppercase();
            if upper.starts_with(b"[ADC")
                && line.ends_with(b"]")
                && let Some(idx) = adc_section_index(&line[4..line.len() - 1])
            {
                let pos = match adc_headers.iter().position(|a| a.adc_index == idx) {
                    Some(pos) => pos,
                    None => {
                        adc_headers.push(AdcHeader {
                            adc_index: idx,
                            caloff: None,
                            calfact: None,
                            calfact2: None,
                            calfact3: None,
                            n_channels: 2048,
                        });
                        adc_headers.len() - 1
                    }
                };
                current = Some(pos);
            }
            continue;
        }
        let Some(eq) = line.iter().position(|&b| b == b'=') else {
            continue;
        };
        let key = trim_spaces(&line[..eq]).to_ascii_lowercase();
        let value = trim_spaces(&line[eq + 1..]);
        let Ok(value_str) = std::str::from_utf8(value) else {
            continue;
        };
        let value_str = value_str.trim();
        if key == b"timerreduce" {
            if let Ok(v) = value_str.parse::<u64>() {
                timer_reduce = v;
            }
            continue;
        }
        let Some(adc) = current.map(|pos| &mut adc_headers[pos]) else {
            continue;
        };
        match key.as_slice() {
            b"caloff" => adc.caloff = value_str.parse::<f64>().ok(),
            b"calfact" => adc.calfact = value_str.parse::<f64>().ok(),
            b"calfact2" => adc.calfact2 = value_str.parse::<f64>().ok(),
            b"calfact3" => adc.calfact3 = value_str.parse::<f64>().ok(),
            b"nchannels" => {
                if let Ok(v) = value_str.parse::<u32>() {
                    adc.n_channels = v;
                }
            }
            _ => {}
        }
    }
    (adc_headers, timer_reduce)
}

/// Parse the binary list-data word stream (little-endian 16-bit words)
/// following the header, given the header's `timerreduce` (ms per timer
/// word).
fn parse_events(data: &[u8], timer_reduce: u64) -> Result<Vec<Event>, LstParseError> {
    let words: Vec<u16> = data
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let mut events = Vec::new();
    let mut time_ms: u64 = 0;
    let mut i = 0usize;
    while i + 1 < words.len() {
        let low = words[i];
        let high = words[i + 1];
        if high == 0x4000 {
            time_ms += timer_reduce;
            i += 2;
            continue;
        }
        if low == 0xFFFF && high == 0xFFFF {
            i += 2;
            continue;
        }
        if (high >> 14) & 1 != 0 {
            return Err(LstParseError::UnrecognizedWord { word_index: i });
        }
        let dummy = (high >> 15) & 1 != 0;
        let rtc_present = (high >> 12) & 1 != 0;
        let adc_mask = (low & 0xFF) as u8;
        let mut cursor = i + 2;
        if rtc_present {
            if cursor + 2 >= words.len() {
                return Err(LstParseError::TruncatedEvent { word_index: cursor });
            }
            cursor += 3;
        }
        if dummy {
            if cursor >= words.len() {
                return Err(LstParseError::TruncatedEvent { word_index: cursor });
            }
            cursor += 1;
        }
        let mut adc_values: [Option<u16>; 8] = [None; 8];
        for bit in 0..8u8 {
            if (adc_mask >> bit) & 1 != 0 {
                if cursor >= words.len() {
                    return Err(LstParseError::TruncatedEvent { word_index: cursor });
                }
                adc_values[bit as usize] = Some(words[cursor]);
                cursor += 1;
            }
        }
        events.push(Event {
            time_ms,
            adc_values,
        });
        i = cursor;
    }
    Ok(events)
}

/// Parse a `.lst` file's ASCII header (up to and including the
/// `[LISTDATA]` marker) and its following binary event stream.
pub fn parse(bytes: &[u8]) -> Result<ListFile, LstParseError> {
    let header_end = find_list_data_end(bytes).ok_or(LstParseError::MissingListData)?;
    let (adc_headers, timer_reduce) = parse_header(&bytes[..header_end]);
    let events = parse_events(&bytes[header_end..], timer_reduce)?;
    Ok(ListFile {
        adc_headers,
        events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn le(words: &[u16]) -> Vec<u8> {
        words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    #[test]
    fn header_only_zero_events() {
        let f =
            parse(b"[ADC1]\r\ncaloff=1.5\r\ncalfact=2.5\r\nnchannels=4096\r\n[LISTDATA]").unwrap();
        assert_eq!(
            f.adc_headers,
            vec![AdcHeader {
                adc_index: 1,
                caloff: Some(1.5),
                calfact: Some(2.5),
                calfact2: None,
                calfact3: None,
                n_channels: 4096,
            }]
        );
        assert!(f.events.is_empty());
    }

    #[test]
    fn single_timer_word_advances_clock_without_event() {
        let mut bytes = b"[LISTDATA]".to_vec();
        bytes.extend(le(&[0x0000, 0x4000]));
        let f = parse(&bytes).unwrap();
        assert!(f.events.is_empty());
    }

    #[test]
    fn timer_words_accumulate_onto_running_clock() {
        let mut bytes = b"[ADC1]\r\ntimerreduce=5\r\n[LISTDATA]".to_vec();
        // Two timer words (5ms each), then an event with ADC0 active.
        bytes.extend(le(&[0x0000, 0x4000]));
        bytes.extend(le(&[0x0000, 0x4000]));
        bytes.extend(le(&[0x0001, 0x0000, 0x002A]));
        let f = parse(&bytes).unwrap();
        assert_eq!(f.events.len(), 1);
        assert_eq!(f.events[0].time_ms, 10);
        assert_eq!(f.events[0].adc_values[0], Some(0x002A));
    }

    #[test]
    fn sync_marker_is_skipped() {
        let mut bytes = b"[LISTDATA]".to_vec();
        bytes.extend(le(&[0xFFFF, 0xFFFF]));
        let f = parse(&bytes).unwrap();
        assert!(f.events.is_empty());
    }

    #[test]
    fn event_with_one_active_adc_no_rtc_no_dummy() {
        let mut bytes = b"[LISTDATA]".to_vec();
        bytes.extend(le(&[0x0001, 0x0000, 0x1234]));
        let f = parse(&bytes).unwrap();
        assert_eq!(f.events.len(), 1);
        let ev = f.events[0];
        assert_eq!(ev.time_ms, 0);
        assert_eq!(
            ev.adc_values,
            [Some(0x1234), None, None, None, None, None, None, None]
        );
    }

    #[test]
    fn event_with_rtc_present() {
        let mut bytes = b"[LISTDATA]".to_vec();
        // mask = ADC0 active, high bit12 set (RTC present).
        bytes.extend(le(&[0x0001, 0x1000, 0x0001, 0x0002, 0x0003, 0x00AB]));
        let f = parse(&bytes).unwrap();
        assert_eq!(f.events.len(), 1);
        assert_eq!(f.events[0].adc_values[0], Some(0x00AB));
    }

    #[test]
    fn event_with_dummy_flag() {
        let mut bytes = b"[LISTDATA]".to_vec();
        // mask = ADC0 active, high bit15 set (dummy word inserted).
        bytes.extend(le(&[0x0001, 0x8000, 0xDEAD, 0x00CD]));
        let f = parse(&bytes).unwrap();
        assert_eq!(f.events.len(), 1);
        assert_eq!(f.events[0].adc_values[0], Some(0x00CD));
    }

    #[test]
    fn event_with_three_active_adcs() {
        let mut bytes = b"[LISTDATA]".to_vec();
        // mask = 0b0000_0111 => ADC0, ADC1, ADC2 active.
        bytes.extend(le(&[0x0007, 0x0000, 0x0010, 0x0020, 0x0030]));
        let f = parse(&bytes).unwrap();
        assert_eq!(f.events.len(), 1);
        assert_eq!(
            f.events[0].adc_values,
            [
                Some(0x0010),
                Some(0x0020),
                Some(0x0030),
                None,
                None,
                None,
                None,
                None
            ]
        );
    }

    #[test]
    fn truncated_event_eof_mid_read() {
        let mut bytes = b"[LISTDATA]".to_vec();
        // mask = ADC0 and ADC1 active, but only one channel value follows.
        bytes.extend(le(&[0x0003, 0x0000, 0x0099]));
        let err = parse(&bytes).unwrap_err();
        assert_eq!(err, LstParseError::TruncatedEvent { word_index: 3 });
    }

    #[test]
    fn unrecognized_word_errors() {
        let mut bytes = b"[LISTDATA]".to_vec();
        // high has bit14 set but isn't exactly 0x4000, so it's neither a
        // timer word nor a valid event header; low isn't a sync marker.
        bytes.extend(le(&[0x0000, 0x4001]));
        let err = parse(&bytes).unwrap_err();
        assert_eq!(err, LstParseError::UnrecognizedWord { word_index: 0 });
    }

    #[test]
    fn missing_list_data_marker() {
        let err = parse(b"[ADC1]\r\ncaloff=1\r\n").unwrap_err();
        assert_eq!(err, LstParseError::MissingListData);
    }
}
