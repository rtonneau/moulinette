//! ROI resolution and time-binned acquisition-rate computation for
//! `.lst` list-mode data. Rust port of `mpa3kit.parser.ListFileParser`'s
//! `time_binned_rate` and the linear term of `ADCInfo.energy`, scoped down
//! per the approved grill design: no sparse-matrix path, no quadratic/cubic
//! calibration (moulinette v1 only supports `caloff`/`calfact`).
use std::fmt;

use crate::lst::{AdcHeader, Event};

/// A channel-range region of interest on one ADC's spectrum.
///
/// `adc` matches `AdcHeader::adc_index`'s convention: the literal 1-based
/// number following `ADC` in the `.lst` header's `[ADCn]` section.
#[derive(Debug, Clone, PartialEq)]
pub struct Roi {
    pub adc: u8,
    pub start_channel: u32,
    pub end_channel: u32,
    pub name: String,
}

/// Errors resolving an energy-bounded ROI into channel bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateError {
    /// `caloff`/`calfact` are not both present on the given `AdcHeader`.
    MissingCalibration,
    /// `calfact2` or `calfact3` is present and non-zero on the given
    /// `AdcHeader`; moulinette v1 only supports linear calibration.
    UnsupportedCalibration,
}

impl fmt::Display for RateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RateError::MissingCalibration => {
                write!(f, "missing caloff/calfact calibration for ADC")
            }
            RateError::UnsupportedCalibration => {
                write!(f, "non-zero calfact2/calfact3 calibration is unsupported")
            }
        }
    }
}

impl std::error::Error for RateError {}

impl Roi {
    /// Build an ROI directly from channel bounds (inclusive).
    pub fn from_channels(adc: u8, start: u32, end: u32, name: String) -> Roi {
        let (start_channel, end_channel) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        Roi {
            adc,
            start_channel,
            end_channel,
            name,
        }
    }

    /// Build an ROI from an energy range (keV), resolved through
    /// `adc_header`'s linear calibration (`energy = caloff + calfact *
    /// channel`).
    pub fn from_energy(
        adc_header: &AdcHeader,
        low_kev: f64,
        high_kev: f64,
        name: String,
    ) -> Result<Roi, RateError> {
        let (Some(caloff), Some(calfact)) = (adc_header.caloff, adc_header.calfact) else {
            return Err(RateError::MissingCalibration);
        };
        let has_unsupported = |v: Option<f64>| v.is_some_and(|v| v != 0.0);
        if has_unsupported(adc_header.calfact2) || has_unsupported(adc_header.calfact3) {
            return Err(RateError::UnsupportedCalibration);
        }
        let chan_low = (low_kev - caloff) / calfact;
        let chan_high = (high_kev - caloff) / calfact;
        let (chan_low, chan_high) = if chan_low <= chan_high {
            (chan_low, chan_high)
        } else {
            (chan_high, chan_low)
        };
        let start_channel = chan_low.round() as u32;
        let end_channel = chan_high.round() as u32;
        Ok(Roi {
            adc: adc_header.adc_index,
            start_channel,
            end_channel,
            name,
        })
    }
}

/// One time bin of the acquisition-rate series.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RateBin {
    /// Bin center time, in seconds.
    pub time_s: f64,
    /// Counts per second.
    pub rate: f64,
    /// Poisson error on `rate`, in counts per second.
    pub rate_err: f64,
    /// Raw counts in this bin.
    pub counts: u64,
}

/// Compute time-binned acquisition rate for one ADC, optionally restricted
/// to an ROI's channel range.
///
/// `adc` matches `AdcHeader::adc_index`/`Roi::adc`'s 1-based convention;
/// `Event::adc_values` is indexed by the 0-based bit position of the
/// binary event stream's ADC mask, so `adc - 1` is used to index into it.
/// `start_ms`/the effective end are treated as a half-open `[start,
/// end)` window in milliseconds, matching `mpa3kit`'s convention; when
/// `end_ms` is `None`, the effective end is one past the last event's
/// `time_ms` (i.e. the window includes the last event).
pub fn time_binned_rate(
    events: &[Event],
    adc: u8,
    roi: Option<&Roi>,
    start_ms: u64,
    end_ms: Option<u64>,
    dt_ms: u64,
) -> Vec<RateBin> {
    let bit = adc.checked_sub(1).map(|b| b as usize);

    let end_excl = match end_ms {
        Some(v) => v,
        None => events
            .iter()
            .map(|e| e.time_ms)
            .max()
            .map(|m| m + 1)
            .unwrap_or(start_ms),
    };

    if end_excl <= start_ms || dt_ms == 0 {
        return Vec::new();
    }
    let duration = end_excl - start_ms;
    let n_bins = duration.div_ceil(dt_ms) as usize;
    let mut counts = vec![0u64; n_bins];

    for event in events {
        if event.time_ms < start_ms || event.time_ms >= end_excl {
            continue;
        }
        let Some(bit) = bit else { continue };
        if bit >= event.adc_values.len() {
            continue;
        }
        let Some(value) = event.adc_values[bit] else {
            continue;
        };
        if let Some(r) = roi {
            let value = value as u32;
            if value < r.start_channel || value > r.end_channel {
                continue;
            }
        }
        let bin_idx = ((event.time_ms - start_ms) / dt_ms) as usize;
        if bin_idx < n_bins {
            counts[bin_idx] += 1;
        }
    }

    let width_s = dt_ms as f64 / 1000.0;
    counts
        .into_iter()
        .enumerate()
        .map(|(i, counts)| {
            let time_s =
                (start_ms as f64 + (i as f64) * dt_ms as f64 + dt_ms as f64 / 2.0) / 1000.0;
            RateBin {
                time_s,
                rate: counts as f64 / width_s,
                rate_err: (counts as f64).sqrt() / width_s,
                counts,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(time_ms: u64, adc0: Option<u16>, adc1: Option<u16>) -> Event {
        let mut adc_values: [Option<u16>; 8] = [None; 8];
        adc_values[0] = adc0;
        adc_values[1] = adc1;
        Event {
            time_ms,
            adc_values,
        }
    }

    #[test]
    fn time_binned_rate_without_roi_counts_every_value() {
        let events = vec![
            event(0, Some(10), None),
            event(500, Some(20), None),
            event(1200, Some(30), None),
            event(1800, Some(40), None),
            event(2500, Some(50), None),
        ];
        let bins = time_binned_rate(&events, 1, None, 0, Some(3000), 1000);
        assert_eq!(
            bins,
            vec![
                RateBin {
                    time_s: 0.5,
                    rate: 2.0,
                    rate_err: 2.0_f64.sqrt(),
                    counts: 2
                },
                RateBin {
                    time_s: 1.5,
                    rate: 2.0,
                    rate_err: 2.0_f64.sqrt(),
                    counts: 2
                },
                RateBin {
                    time_s: 2.5,
                    rate: 1.0,
                    rate_err: 1.0,
                    counts: 1
                },
            ]
        );
    }

    #[test]
    fn time_binned_rate_with_roi_restricts_channel_range() {
        let events = vec![
            event(0, Some(10), None),    // out of ROI
            event(100, Some(25), None),  // in ROI
            event(1200, Some(30), None), // in ROI
            event(1300, Some(99), None), // out of ROI
        ];
        let roi = Roi::from_channels(1, 20, 35, "test-roi".to_string());
        let bins = time_binned_rate(&events, 1, Some(&roi), 0, Some(2000), 1000);
        assert_eq!(
            bins,
            vec![
                RateBin {
                    time_s: 0.5,
                    rate: 1.0,
                    rate_err: 1.0,
                    counts: 1
                },
                RateBin {
                    time_s: 1.5,
                    rate: 1.0,
                    rate_err: 1.0,
                    counts: 1
                },
            ]
        );
    }

    #[test]
    fn time_binned_rate_end_ms_none_runs_through_last_event() {
        let events = vec![event(0, Some(1), None), event(2500, Some(1), None)];
        let bins = time_binned_rate(&events, 1, None, 0, None, 1000);
        // Through the last event's time_ms (2500) inclusive => bins covering
        // [0,1000), [1000,2000), [2000,3000).
        assert_eq!(bins.len(), 3);
        assert_eq!(bins[2].counts, 1);
        assert_eq!(bins[2].time_s, 2.5);
    }

    #[test]
    fn roi_from_energy_resolves_known_linear_calibration() {
        let header = AdcHeader {
            adc_index: 2,
            caloff: Some(10.0),
            calfact: Some(0.5),
            calfact2: None,
            calfact3: None,
            n_channels: 4096,
        };
        // energy = 10 + 0.5*channel => channel = (energy - 10) / 0.5
        // low=20keV -> channel 20; high=60keV -> channel 100
        let roi = Roi::from_energy(&header, 20.0, 60.0, "peak".to_string()).unwrap();
        assert_eq!(
            roi,
            Roi {
                adc: 2,
                start_channel: 20,
                end_channel: 100,
                name: "peak".to_string(),
            }
        );
    }

    #[test]
    fn roi_from_energy_missing_calibration_errors() {
        let header = AdcHeader {
            adc_index: 1,
            caloff: None,
            calfact: Some(0.5),
            calfact2: None,
            calfact3: None,
            n_channels: 2048,
        };
        let err = Roi::from_energy(&header, 0.0, 10.0, "x".to_string()).unwrap_err();
        assert_eq!(err, RateError::MissingCalibration);
    }

    #[test]
    fn roi_from_energy_nonzero_calfact2_is_unsupported() {
        let header = AdcHeader {
            adc_index: 1,
            caloff: Some(0.0),
            calfact: Some(1.0),
            calfact2: Some(0.001),
            calfact3: None,
            n_channels: 2048,
        };
        let err = Roi::from_energy(&header, 0.0, 10.0, "x".to_string()).unwrap_err();
        assert_eq!(err, RateError::UnsupportedCalibration);
    }

    #[test]
    fn roi_from_energy_zero_calfact2_is_fine() {
        let header = AdcHeader {
            adc_index: 1,
            caloff: Some(0.0),
            calfact: Some(1.0),
            calfact2: Some(0.0),
            calfact3: Some(0.0),
            n_channels: 2048,
        };
        let roi = Roi::from_energy(&header, 5.0, 15.0, "x".to_string()).unwrap();
        assert_eq!(roi.start_channel, 5);
        assert_eq!(roi.end_channel, 15);
    }
}
