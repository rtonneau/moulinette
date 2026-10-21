//! Interactive in-window line chart: `plotters` renders into an in-memory
//! RGB buffer (`BitMapBackend::with_buffer`) for the current view range,
//! and the view range follows mouse-wheel zoom and click-drag pan. The
//! pixel/data mapping functions are free functions so they can be tested
//! without a window; `gui.rs` only scales cursor positions from the
//! displaying element's logical size to buffer pixels (`element_to_buffer`).
//!
//! The chart styling mirrors `plot::rate_vs_time` (caption, axis
//! descriptions, one `Palette99` colour per series, legend); the PNG file
//! output stays in `plot.rs` and never depends on this module's view state.
use std::ops::Range;

use plotters::prelude::*;
use plotters::style::Palette;

use crate::kde::gaussian_kde;
use crate::rate::RateBin;

/// Buffer size used before the displaying element reports its own size.
pub const DEFAULT_SIZE: (u32, u32) = (900, 400);
/// Smallest buffer rendered; below this the axis labels leave no plot area.
pub const MIN_SIZE: (u32, u32) = (240, 160);
/// Largest buffer rendered (a 4K-wide chart at most).
pub const MAX_SIZE: (u32, u32) = (4096, 4096);
/// Narrowest view span, as a fraction of the full-data span, zoom can reach.
const MIN_ZOOM: f64 = 1e-6;
/// Widest view span, as a multiple of the full-data span, zoom can reach.
const MAX_ZOOM: f64 = 1e3;

/// A `[min, max]` interval of one data axis, with `min < max`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisRange {
    pub min: f64,
    pub max: f64,
}

impl AxisRange {
    pub fn span(&self) -> f64 {
        self.max - self.min
    }

    fn contains(&self, v: f64) -> bool {
        v >= self.min && v <= self.max
    }

    /// Scale the span by `factor` around `anchor` (which keeps its relative
    /// position), clamping the new span to `[MIN_ZOOM, MAX_ZOOM]` times
    /// `home_span`.
    fn zoomed(&self, anchor: f64, factor: f64, home_span: f64) -> AxisRange {
        let span = (self.span() * factor).clamp(home_span * MIN_ZOOM, home_span * MAX_ZOOM);
        let scale = span / self.span();
        AxisRange {
            min: anchor + (self.min - anchor) * scale,
            max: anchor + (self.max - anchor) * scale,
        }
    }

    fn shifted(&self, delta: f64) -> AxisRange {
        AxisRange {
            min: self.min + delta,
            max: self.max + delta,
        }
    }
}

/// The visible data window of a chart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewRange {
    pub x: AxisRange,
    pub y: AxisRange,
}

/// The plotting rectangle inside the buffer, in buffer pixels: `left` and
/// `right` are the columns of `x.min` and `x.max`, `bottom` and `top` the
/// rows of `y.min` and `y.max` (pixel rows grow downwards).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotArea {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl PlotArea {
    /// The area `plotters` maps a `Cartesian2d` chart onto, given its
    /// plotting area's pixel range: `ChartBuilder::build_cartesian_2d`
    /// maps `x.min` to `x0`, `x.max` to `x1 - 1`, `y.max` to `y0` and
    /// `y.min` to `y1 - 1` (plotters 0.3.7, `chart/builder.rs`).
    pub fn from_pixel_range((x, y): (Range<i32>, Range<i32>)) -> PlotArea {
        PlotArea {
            left: f64::from(x.start),
            top: f64::from(y.start),
            right: f64::from(x.end - 1),
            bottom: f64::from(y.end - 1),
        }
    }

    pub fn contains(&self, (px, py): (f64, f64)) -> bool {
        px >= self.left && px <= self.right && py >= self.top && py <= self.bottom
    }
}

/// Map a data point to buffer pixels through `view` onto `area`. This is
/// the continuous form of `plotters`' own mapping, which floors the result
/// to whole pixels.
pub fn data_to_pixel(view: &ViewRange, area: &PlotArea, (x, y): (f64, f64)) -> (f64, f64) {
    (
        area.left + (x - view.x.min) / view.x.span() * (area.right - area.left),
        area.bottom - (y - view.y.min) / view.y.span() * (area.bottom - area.top),
    )
}

/// Map a buffer pixel back to data coordinates: the inverse of
/// `data_to_pixel`.
pub fn pixel_to_data(view: &ViewRange, area: &PlotArea, (px, py): (f64, f64)) -> (f64, f64) {
    (
        view.x.min + (px - area.left) / (area.right - area.left) * view.x.span(),
        view.y.min + (area.bottom - py) / (area.bottom - area.top) * view.y.span(),
    )
}

/// Map a position in the displaying element's own (logical) coordinates to
/// buffer pixels, for an element of size `element` showing a buffer of
/// size `buffer` stretched to fill it.
pub fn element_to_buffer(
    (x, y): (f64, f64),
    element: (f64, f64),
    buffer: (u32, u32),
) -> (f64, f64) {
    (
        x * f64::from(buffer.0) / element.0,
        y * f64::from(buffer.1) / element.1,
    )
}

/// The inverse of `element_to_buffer`.
pub fn buffer_to_element(
    (px, py): (f64, f64),
    element: (f64, f64),
    buffer: (u32, u32),
) -> (f64, f64) {
    (
        px * element.0 / f64::from(buffer.0),
        py * element.1 / f64::from(buffer.1),
    )
}

/// Clipped segment endpoints, each flagged for whether it is the original,
/// unclipped endpoint.
struct ClippedSegment {
    start: (f64, f64),
    end: (f64, f64),
    start_is_original: bool,
    end_is_original: bool,
}

/// Clip the segment `a`-`b` to `view` (Liang-Barsky).
fn clip_segment(view: &ViewRange, a: (f64, f64), b: (f64, f64)) -> Option<ClippedSegment> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [
        (-dx, a.0 - view.x.min),
        (dx, view.x.max - a.0),
        (-dy, a.1 - view.y.min),
        (dy, view.y.max - a.1),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
        }
    }
    if t0 > t1 {
        return None;
    }
    let at = |t: f64| (a.0 + t * dx, a.1 + t * dy);
    let start = if t0 <= 0.0 { a } else { at(t0) };
    let end = if t1 >= 1.0 { b } else { at(t1) };
    Some(ClippedSegment {
        start,
        end,
        start_is_original: t0 <= 0.0,
        end_is_original: t1 >= 1.0,
    })
}

/// Split a polyline into the runs visible inside `view`, clipping segments
/// that cross its edges. `plotters` clamps out-of-range points onto the
/// plot edge instead of clipping them, which would draw spurious lines
/// along the edges once the view is zoomed or panned.
fn visible_runs(points: &[(f64, f64)], view: &ViewRange) -> Vec<Vec<(f64, f64)>> {
    let mut runs: Vec<Vec<(f64, f64)>> = Vec::new();
    // Whether the last run ends at the previous segment's unclipped end.
    let mut open = false;
    for pair in points.windows(2) {
        match clip_segment(view, pair[0], pair[1]) {
            Some(seg) => {
                match runs.last_mut() {
                    Some(run) if open && seg.start_is_original => run.push(seg.end),
                    _ => runs.push(vec![seg.start, seg.end]),
                }
                open = seg.end_is_original;
            }
            None => open = false,
        }
    }
    runs
}

/// Raw RGB8 pixels of a rendered chart (row-major, 3 bytes per pixel).
pub struct RgbImage {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}

#[cfg(feature = "gui")]
impl RgbImage {
    /// Wrap the pixels as a `slint::Image` for an `image` property.
    pub fn to_slint_image(&self) -> slint::Image {
        slint::Image::from_rgb8(
            slint::SharedPixelBuffer::<slint::Rgb8Pixel>::clone_from_slice(
                &self.rgb,
                self.width,
                self.height,
            ),
        )
    }
}

/// A shaded vertical band drawn behind the series (e.g. one ROI's
/// resolved channel or energy range), spanning the full visible y-range
/// at render time so it stays visible through zoom and pan, the way
/// `gui_state`'s spectrum-with-ROI-bands view needs its bands to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    pub x: AxisRange,
    /// `Palette99` index used for the band's fill color.
    pub color_index: usize,
}

/// The data point nearest to a hovered pixel.
#[derive(Debug, Clone, PartialEq)]
pub struct Hover<'a> {
    pub series: &'a str,
    pub x: f64,
    pub y: f64,
    /// The point's position in buffer pixels.
    pub pixel: (f64, f64),
}

/// A line chart of named series with a zoomable, pannable view range.
pub struct LineChart {
    caption: String,
    x_desc: String,
    y_desc: String,
    series: Vec<(String, Vec<(f64, f64)>)>,
    /// Shaded bands drawn behind the series, e.g. one per ROI.
    bands: Vec<Band>,
    home: ViewRange,
    view: ViewRange,
    size: (u32, u32),
    /// Plotting area of the last render at the current size and view;
    /// `None` until rendered, so no interaction maps through a stale area.
    area: Option<PlotArea>,
}

impl LineChart {
    /// A chart showing `home` initially (and after `reset`). `home` must
    /// have positive, finite spans.
    pub fn new(
        caption: &str,
        x_desc: &str,
        y_desc: &str,
        series: Vec<(String, Vec<(f64, f64)>)>,
        home: ViewRange,
    ) -> LineChart {
        LineChart {
            caption: caption.into(),
            x_desc: x_desc.into(),
            y_desc: y_desc.into(),
            series,
            bands: Vec::new(),
            home,
            view: home,
            size: DEFAULT_SIZE,
            area: None,
        }
    }

    /// Attach shaded bands (e.g. one per ROI) drawn behind the series.
    pub fn with_bands(mut self, bands: Vec<Band>) -> LineChart {
        self.bands = bands;
        self
    }

    /// The in-window counterpart of the `.mpa` Analyze screen's static
    /// spectrum-with-ROI-bands placeholder (Ticket 5): a single "Counts"
    /// series over the spectrum's channel indices, plus one `Band` per ROI
    /// so each stays shaded through pan/zoom instead of only at the home
    /// view.
    pub fn spectrum_with_bands(counts: &[f64], bands: Vec<Band>) -> LineChart {
        let n = counts.len().max(1);
        let y_max = (counts.iter().cloned().fold(0.0f64, f64::max) * 1.1).max(1.0);
        let points: Vec<(f64, f64)> = counts.iter().enumerate().map(|(i, &c)| (i as f64, c)).collect();
        LineChart::new(
            "Spectrum with ROI bands",
            "Channel",
            "Counts",
            vec![("Counts".to_string(), points)],
            ViewRange {
                x: AxisRange {
                    min: 0.0,
                    max: n as f64,
                },
                y: AxisRange {
                    min: 0.0,
                    max: y_max,
                },
            },
        )
        .with_bands(bands)
    }

    /// The in-window counterpart of `plot::violin`'s per-group shapes: one
    /// Gaussian-KDE density curve per named group (e.g. one per ROI/time-
    /// window pair), density plotted against the shared value domain, so
    /// it reuses `LineChart`'s zoom/pan/hover exactly like `rate_vs_time`
    /// does rather than drawing a mirrored violin polygon. The mirrored
    /// polygon shape itself stays in `plot::violin`'s PNG output, which
    /// this never touches.
    pub fn violin(groups: &[(String, Vec<f64>)]) -> LineChart {
        let all_values: Vec<f64> = groups.iter().flat_map(|(_, v)| v.iter().copied()).collect();
        let (mut y_min, mut y_max) = if all_values.is_empty() {
            (0.0, 1.0)
        } else {
            (
                all_values.iter().cloned().fold(f64::INFINITY, f64::min),
                all_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            )
        };
        if y_max <= y_min {
            y_min -= 1.0;
            y_max += 1.0;
        }
        let span = y_max - y_min;
        let steps = 200usize;
        let eval_points: Vec<f64> = (0..=steps)
            .map(|i| y_min + span * (i as f64) / (steps as f64))
            .collect();

        let mut series = Vec::with_capacity(groups.len());
        let mut density_max = 0.0f64;
        for (name, values) in groups {
            let densities = gaussian_kde(values, &eval_points);
            density_max = density_max.max(densities.iter().cloned().fold(0.0, f64::max));
            series.push((
                name.clone(),
                eval_points
                    .iter()
                    .zip(&densities)
                    .map(|(&v, &d)| (v, d))
                    .collect(),
            ));
        }
        LineChart::new(
            "Rate distribution per window",
            "Rate [Yield/s]",
            "Density",
            series,
            ViewRange {
                x: AxisRange {
                    min: y_min,
                    max: y_max,
                },
                y: AxisRange {
                    min: 0.0,
                    max: (density_max * 1.1).max(1e-9),
                },
            },
        )
    }

    /// The in-window counterpart of `plot::rate_vs_time`: the same series,
    /// axis descriptions and initial axis ranges.
    pub fn rate_vs_time(series: &[(String, Vec<RateBin>)]) -> LineChart {
        let x_max = series
            .iter()
            .flat_map(|(_, bins)| bins.iter().map(|b| b.time_s))
            .fold(0f64, f64::max);
        let y_max = series
            .iter()
            .flat_map(|(_, bins)| bins.iter().map(|b| b.rate))
            .fold(0f64, f64::max);
        LineChart::new(
            "Acquisition rate vs. time",
            "Acquisition Time [s]",
            "Acquisition Rate [Yield/s]",
            series
                .iter()
                .map(|(name, bins)| {
                    (
                        name.clone(),
                        bins.iter().map(|b| (b.time_s, b.rate)).collect(),
                    )
                })
                .collect(),
            ViewRange {
                x: AxisRange {
                    min: 0.0,
                    max: x_max.max(1.0),
                },
                y: AxisRange {
                    min: 0.0,
                    max: (y_max * 1.1).max(1.0),
                },
            },
        )
    }

    pub fn view(&self) -> ViewRange {
        self.view
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// The plotting area of the last render, if still current.
    pub fn area(&self) -> Option<PlotArea> {
        self.area
    }

    /// Set the buffer size (clamped to `MIN_SIZE`..=`MAX_SIZE`). Returns
    /// whether it changed, in which case the chart must be re-rendered
    /// before it can be interacted with again.
    pub fn set_size(&mut self, width: u32, height: u32) -> bool {
        let size = (
            width.clamp(MIN_SIZE.0, MAX_SIZE.0),
            height.clamp(MIN_SIZE.1, MAX_SIZE.1),
        );
        if size == self.size {
            return false;
        }
        self.size = size;
        self.area = None;
        true
    }

    /// Return to the initial view range.
    pub fn reset(&mut self) {
        self.view = self.home;
    }

    /// Scale the view by `factor` (below 1 zooms in) around the data point
    /// under buffer pixel `pixel`. Over the plot area both axes zoom; left
    /// of it (the y-axis labels) only y zooms; below it (the x-axis labels)
    /// only x zooms.
    pub fn zoom(&mut self, pixel: (f64, f64), factor: f64) {
        let Some(area) = self.area else {
            return;
        };
        if !(factor.is_finite() && factor > 0.0) {
            return;
        }
        let (x, y) = pixel_to_data(&self.view, &area, pixel);
        if pixel.0 >= area.left {
            self.view.x = self.view.x.zoomed(x, factor, self.home.x.span());
        }
        if pixel.1 <= area.bottom {
            self.view.y = self.view.y.zoomed(y, factor, self.home.y.span());
        }
    }

    /// Pan so that the data point under buffer pixel `from` moves to `to`.
    pub fn pan(&mut self, from: (f64, f64), to: (f64, f64)) {
        let Some(area) = self.area else {
            return;
        };
        let a = pixel_to_data(&self.view, &area, from);
        let b = pixel_to_data(&self.view, &area, to);
        self.view.x = self.view.x.shifted(a.0 - b.0);
        self.view.y = self.view.y.shifted(a.1 - b.1);
    }

    /// The visible data point nearest (in pixels) to buffer pixel `pixel`,
    /// or `None` when `pixel` is outside the plot area.
    pub fn nearest(&self, pixel: (f64, f64)) -> Option<Hover<'_>> {
        let area = self.area?;
        if !area.contains(pixel) {
            return None;
        }
        let mut best: Option<(f64, Hover<'_>)> = None;
        for (name, points) in &self.series {
            for &(x, y) in points {
                if !(self.view.x.contains(x) && self.view.y.contains(y)) {
                    continue;
                }
                let p = data_to_pixel(&self.view, &area, (x, y));
                let distance = (p.0 - pixel.0).powi(2) + (p.1 - pixel.1).powi(2);
                if best.as_ref().is_none_or(|(d, _)| distance < *d) {
                    best = Some((
                        distance,
                        Hover {
                            series: name,
                            x,
                            y,
                            pixel: p,
                        },
                    ));
                }
            }
        }
        best.map(|(_, hover)| hover)
    }

    /// Tooltip text for `hover`: the series name and the point's exact
    /// values under the axis descriptions.
    pub fn tooltip(&self, hover: &Hover) -> String {
        format!(
            "{}\n{}: {}\n{}: {}",
            hover.series, self.x_desc, hover.x, self.y_desc, hover.y
        )
    }

    /// Render the current view at the current size, and remember the
    /// plotting area `plotters` laid out for later pixel/data mapping.
    pub fn render(&mut self) -> Result<RgbImage, String> {
        let (width, height) = self.size;
        let view = self.view;
        let mut rgb = vec![0u8; width as usize * height as usize * 3];
        let area;
        {
            let root = BitMapBackend::with_buffer(&mut rgb, (width, height)).into_drawing_area();
            root.fill(&WHITE).map_err(|e| e.to_string())?;
            let mut chart = ChartBuilder::on(&root)
                .caption(&self.caption, ("sans-serif", 20))
                .margin(10)
                .x_label_area_size(40)
                .y_label_area_size(60)
                .build_cartesian_2d(view.x.min..view.x.max, view.y.min..view.y.max)
                .map_err(|e| e.to_string())?;
            chart
                .configure_mesh()
                .x_desc(self.x_desc.as_str())
                .y_desc(self.y_desc.as_str())
                .draw()
                .map_err(|e| e.to_string())?;
            for band in &self.bands {
                if band.x.max <= view.x.min || band.x.min >= view.x.max {
                    continue;
                }
                let lo = band.x.min.max(view.x.min);
                let hi = band.x.max.min(view.x.max);
                let color = Palette99::pick(band.color_index).to_rgba();
                chart
                    .draw_series(std::iter::once(Rectangle::new(
                        [(lo, view.y.min), (hi, view.y.max)],
                        color.mix(0.25).filled(),
                    )))
                    .map_err(|e| e.to_string())?;
            }
            for (i, (name, points)) in self.series.iter().enumerate() {
                let color = Palette99::pick(i).to_rgba();
                chart
                    .draw_series(
                        visible_runs(points, &view)
                            .into_iter()
                            .map(|run| PathElement::new(run, color)),
                    )
                    .map_err(|e| e.to_string())?
                    .label(name)
                    .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color));
            }
            chart
                .configure_series_labels()
                .background_style(WHITE.mix(0.8))
                .border_style(BLACK)
                .draw()
                .map_err(|e| e.to_string())?;
            area = PlotArea::from_pixel_range(chart.plotting_area().get_pixel_range());
            root.present().map_err(|e| e.to_string())?;
        }
        self.area = Some(area);
        Ok(RgbImage { width, height, rgb })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 1e-9;

    fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < TOLERANCE && (a.1 - b.1).abs() < TOLERANCE
    }

    fn view(x: (f64, f64), y: (f64, f64)) -> ViewRange {
        ViewRange {
            x: AxisRange { min: x.0, max: x.1 },
            y: AxisRange { min: y.0, max: y.1 },
        }
    }

    // A 900x400 buffer whose plot area leaves room for labels.
    const AREA: PlotArea = PlotArea {
        left: 70.0,
        top: 40.0,
        right: 889.0,
        bottom: 349.0,
    };

    #[test]
    fn pixel_data_mapping_round_trips_with_known_ranges() {
        let view = view((-20.0, 380.0), (0.5, 1500.0));
        // Corners and centre of the view land on the area's edges/centre.
        assert!(close(data_to_pixel(&view, &AREA, (-20.0, 0.5)), (70.0, 349.0)));
        assert!(close(data_to_pixel(&view, &AREA, (380.0, 1500.0)), (889.0, 40.0)));
        assert!(close(
            data_to_pixel(&view, &AREA, (180.0, 750.25)),
            (479.5, 194.5)
        ));
        assert!(close(pixel_to_data(&view, &AREA, (70.0, 349.0)), (-20.0, 0.5)));
        assert!(close(pixel_to_data(&view, &AREA, (889.0, 40.0)), (380.0, 1500.0)));
        // Round trips, inside and outside the area, in both directions.
        for i in 0..=20 {
            for j in 0..=20 {
                let pixel = (f64::from(i) * 45.0 - 3.25, f64::from(j) * 20.0 + 0.5);
                let back = data_to_pixel(&view, &AREA, pixel_to_data(&view, &AREA, pixel));
                assert!(close(back, pixel), "{pixel:?} -> {back:?}");
                let point = (f64::from(i) * 21.5 - 40.0, f64::from(j) * 80.0 - 7.0);
                let mapped = pixel_to_data(&view, &AREA, data_to_pixel(&view, &AREA, point));
                assert!(
                    (mapped.0 - point.0).abs() < 1e-9 * 400.0
                        && (mapped.1 - point.1).abs() < 1e-9 * 1500.0,
                    "{point:?} -> {mapped:?}"
                );
            }
        }
    }

    #[test]
    fn element_buffer_scaling_round_trips() {
        let element = (612.5, 272.0);
        let buffer = (1225, 544);
        assert!(close(
            element_to_buffer((306.25, 136.0), element, buffer),
            (612.5, 272.0)
        ));
        for (x, y) in [(0.0, 0.0), (17.3, 250.1), (612.5, 272.0)] {
            let back = buffer_to_element(element_to_buffer((x, y), element, buffer), element, buffer);
            assert!(close(back, (x, y)));
        }
    }

    #[test]
    fn chart_mapping_matches_plotters_own_pixels() {
        // The rendered plot area plus `data_to_pixel` must agree with how
        // `plotters` itself places data points (which floors to pixels).
        let (width, height) = (700u32, 350u32);
        let view = view((2.0, 50.0), (-3.0, 12.0));
        let mut rgb = vec![0u8; width as usize * height as usize * 3];
        let root = BitMapBackend::with_buffer(&mut rgb, (width, height)).into_drawing_area();
        let chart = ChartBuilder::on(&root)
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(60)
            .build_cartesian_2d(view.x.min..view.x.max, view.y.min..view.y.max)
            .unwrap();
        let area = PlotArea::from_pixel_range(chart.plotting_area().get_pixel_range());
        for point in [(2.0, -3.0), (50.0, 12.0), (13.7, 4.2), (49.9, -2.95)] {
            let (px, py) = chart.backend_coord(&point);
            let ours = data_to_pixel(&view, &area, point);
            assert!(
                (ours.0 - f64::from(px)).abs() <= 1.0 && (ours.1 - f64::from(py)).abs() <= 1.0,
                "{point:?}: plotters ({px}, {py}), ours {ours:?}"
            );
        }
    }

    fn sample_chart() -> LineChart {
        let bins = |rates: &[f64]| -> Vec<RateBin> {
            rates
                .iter()
                .enumerate()
                .map(|(i, &rate)| RateBin {
                    time_s: i as f64 + 0.5,
                    rate,
                    rate_err: rate.sqrt(),
                    counts: rate as u64,
                })
                .collect()
        };
        LineChart::rate_vs_time(&[
            ("roi-a".into(), bins(&[2.0, 4.0, 6.0, 8.0, 10.0])),
            ("roi-b".into(), bins(&[1.0, 1.0, 9.0, 1.0, 1.0])),
        ])
    }

    #[test]
    fn rate_chart_home_range_matches_png_and_render_sets_area() {
        let mut chart = sample_chart();
        assert_eq!(chart.view(), view((0.0, 4.5), (0.0, 11.0)));
        assert!(chart.area().is_none());
        let image = chart.render().unwrap();
        assert_eq!((image.width, image.height), DEFAULT_SIZE);
        assert_eq!(image.rgb.len(), 900 * 400 * 3);
        let area = chart.area().unwrap();
        assert!(area.left > 0.0 && area.right < 900.0 && area.left < area.right);
        assert!(area.top > 0.0 && area.bottom < 400.0 && area.top < area.bottom);
        assert!(chart.set_size(500, 300));
        assert!(chart.area().is_none(), "resizing invalidates the area");
        assert!(!chart.set_size(500, 300));
        assert!(chart.set_size(10, 10_000));
        assert_eq!(chart.size(), (MIN_SIZE.0, MAX_SIZE.1));
    }

    #[test]
    fn chart_zoom_keeps_the_cursor_point_fixed() {
        let mut chart = sample_chart();
        chart.render().unwrap();
        let area = chart.area().unwrap();
        let cursor = (area.left + 0.25 * (area.right - area.left), area.top + 100.0);
        let before = pixel_to_data(&chart.view(), &area, cursor);
        chart.zoom(cursor, 0.5);
        let after = pixel_to_data(&chart.view(), &area, cursor);
        assert!(close(before, after), "{before:?} vs {after:?}");
        assert!((chart.view().x.span() - 2.25).abs() < TOLERANCE);
        assert!((chart.view().y.span() - 5.5).abs() < TOLERANCE);

        // Below the plot (x-axis labels): only x zooms.
        let y_span = chart.view().y.span();
        chart.zoom((area.left + 10.0, area.bottom + 20.0), 2.0);
        assert!((chart.view().x.span() - 4.5).abs() < TOLERANCE);
        assert!((chart.view().y.span() - y_span).abs() < TOLERANCE);
        // Left of the plot (y-axis labels): only y zooms.
        chart.zoom((area.left - 30.0, area.top + 10.0), 2.0);
        assert!((chart.view().x.span() - 4.5).abs() < TOLERANCE);
        assert!((chart.view().y.span() - 2.0 * y_span).abs() < TOLERANCE);

        // Zoom is bounded, and ignores nonsense factors.
        for _ in 0..200 {
            chart.zoom(cursor, 0.1);
        }
        assert!(chart.view().x.span() >= 4.5 * MIN_ZOOM * (1.0 - 1e-9));
        let current = chart.view();
        chart.zoom(cursor, f64::NAN);
        chart.zoom(cursor, -1.0);
        assert_eq!(chart.view(), current);
        chart.reset();
        assert_eq!(chart.view(), view((0.0, 4.5), (0.0, 11.0)));
    }

    #[test]
    fn chart_pan_moves_data_with_the_cursor() {
        let mut chart = sample_chart();
        chart.render().unwrap();
        let area = chart.area().unwrap();
        let from = (area.left + 100.0, area.top + 50.0);
        let to = (from.0 + 40.0, from.1 - 30.0);
        let grabbed = pixel_to_data(&chart.view(), &area, from);
        chart.pan(from, to);
        assert!(close(pixel_to_data(&chart.view(), &area, to), grabbed));
        assert!(chart.view().x.min < 0.0, "dragging right reveals earlier times");
        assert!(chart.view().y.min < 0.0, "dragging up reveals lower rates");
        assert!((chart.view().x.span() - 4.5).abs() < TOLERANCE, "pan keeps spans");
    }

    #[test]
    fn chart_hover_finds_the_nearest_visible_point() {
        let mut chart = sample_chart();
        chart.render().unwrap();
        let area = chart.area().unwrap();
        let peak = data_to_pixel(&chart.view(), &area, (2.5, 9.0));
        let hover = chart.nearest((peak.0 + 3.0, peak.1 - 2.0)).unwrap();
        assert_eq!((hover.series, hover.x, hover.y), ("roi-b", 2.5, 9.0));
        assert!(close(hover.pixel, peak));
        assert_eq!(
            chart.tooltip(&hover),
            "roi-b\nAcquisition Time [s]: 2.5\nAcquisition Rate [Yield/s]: 9"
        );
        let low = data_to_pixel(&chart.view(), &area, (0.5, 1.0));
        assert_eq!(chart.nearest(low).unwrap().y, 1.0);
        assert!(chart.nearest((area.left - 5.0, peak.1)).is_none());
        // Points panned out of view are never reported.
        chart.zoom(peak, 0.01);
        chart.render().unwrap();
        let area = chart.area().unwrap();
        let hover = chart.nearest((area.left, area.bottom)).unwrap();
        assert_eq!((hover.x, hover.y), (2.5, 9.0));
    }

    #[test]
    fn chart_clips_lines_to_the_view() {
        let view = view((0.0, 10.0), (0.0, 10.0));
        let runs = visible_runs(
            &[(-5.0, 5.0), (5.0, 5.0), (8.0, 8.0), (20.0, 8.0), (30.0, 8.0), (5.0, 2.0)],
            &view,
        );
        let expected = [
            vec![(0.0, 5.0), (5.0, 5.0), (8.0, 8.0), (10.0, 8.0)],
            vec![(10.0, 3.2), (5.0, 2.0)],
        ];
        assert_eq!(runs.len(), expected.len(), "{runs:?}");
        for (run, want) in runs.iter().zip(&expected) {
            assert_eq!(run.len(), want.len(), "{runs:?}");
            assert!(run.iter().zip(want).all(|(&a, &b)| close(a, b)), "{runs:?}");
        }
        assert!(visible_runs(&[(20.0, 1.0), (30.0, 1.0)], &view).is_empty());
        assert!(visible_runs(&[(1.0, 1.0)], &view).is_empty());
    }

    #[test]
    fn spectrum_chart_hover_shows_channel_and_count() {
        let mut chart = LineChart::spectrum_with_bands(&[2.0, 9.0, 3.0], Vec::new());
        assert_eq!(chart.view(), view((0.0, 3.0), (0.0, 9.9)));
        chart.render().unwrap();
        let area = chart.area().unwrap();
        let peak = data_to_pixel(&chart.view(), &area, (1.0, 9.0));
        let hover = chart.nearest(peak).unwrap();
        assert_eq!((hover.series, hover.x, hover.y), ("Counts", 1.0, 9.0));
        let tooltip = chart.tooltip(&hover);
        assert!(tooltip.contains("Channel: 1"), "{tooltip}");
        assert!(tooltip.contains("Counts: 9"), "{tooltip}");
    }

    #[test]
    fn spectrum_chart_band_shades_its_channel_range_through_zoom() {
        // All-zero counts so the only non-white pixels come from the band.
        let bands = vec![Band {
            x: AxisRange { min: 3.0, max: 6.0 },
            color_index: 0,
        }];
        let mut chart = LineChart::spectrum_with_bands(&[0.0; 10], bands.clone());
        assert_eq!(chart.bands, bands, "bands are stored, not just drawn once");

        let pixel_at = |image: &RgbImage, (px, py): (f64, f64)| {
            let (x, y) = (px.round() as usize, py.round() as usize);
            let i = (y * image.width as usize + x) * 3;
            (image.rgb[i], image.rgb[i + 1], image.rgb[i + 2])
        };
        let image = chart.render().unwrap();
        let area = chart.area().unwrap();
        let mid_y = (chart.view().y.min + chart.view().y.max) / 2.0;
        let inside = data_to_pixel(&chart.view(), &area, (4.5, mid_y));
        assert_ne!(
            pixel_at(&image, inside),
            (255, 255, 255),
            "the band tints the area behind the flat (all-zero) line"
        );

        // The band stays visible (still tinted, now at the zoomed-in pixel
        // position) after zooming in on a point inside it.
        chart.zoom(inside, 0.5);
        let image = chart.render().unwrap();
        let area = chart.area().unwrap();
        let mid_y = (chart.view().y.min + chart.view().y.max) / 2.0;
        let inside = data_to_pixel(&chart.view(), &area, (4.5, mid_y));
        assert_ne!(
            pixel_at(&image, inside),
            (255, 255, 255),
            "the band is still shaded after zooming"
        );
    }

    #[test]
    fn violin_chart_hover_shows_group_label_and_density() {
        let groups = vec![
            ("0-10".to_string(), vec![1.0, 2.0, 2.0, 3.0]),
            ("10-20".to_string(), vec![4.0, 5.0, 5.0, 6.0]),
        ];
        let mut chart = LineChart::violin(&groups);
        assert_eq!(chart.view().x, AxisRange { min: 1.0, max: 6.0 });
        chart.render().unwrap();
        let area = chart.area().unwrap();
        // Land exactly on one point of "0-10"'s own density curve (the
        // shared eval domain's left edge, x = 1.0) so the nearest match is
        // unambiguous: "10-20"'s density there is near zero, far from this
        // exact value, while "0-10"'s point is an exact (distance-zero) hit.
        let expected_density = gaussian_kde(&groups[0].1, &[1.0])[0];
        let target = data_to_pixel(&chart.view(), &area, (1.0, expected_density));
        let hover = chart.nearest(target).unwrap();
        assert_eq!(hover.series, "0-10");
        assert!((hover.x - 1.0).abs() < 1e-9);
        assert!((hover.y - expected_density).abs() < 1e-9);
        let tooltip = chart.tooltip(&hover);
        assert!(tooltip.contains("0-10"), "{tooltip}");
        assert!(tooltip.contains("Density:"), "{tooltip}");
    }

    #[test]
    fn spectrum_and_violin_charts_reuse_the_same_line_chart_pan_zoom_hover() {
        // Both in-window views (Ticket 7) are built from `LineChart`
        // (Ticket 6) rather than each reimplementing pan/zoom/hover: a
        // shared helper exercises the same sequence of calls on each and
        // checks they respond identically, since there is only one
        // implementation of `zoom`/`pan`/`reset`/`nearest` for both to call.
        fn exercise(chart: &mut LineChart) {
            chart.render().unwrap();
            let area = chart.area().unwrap();
            let home = chart.view();
            let cursor = (
                area.left + 0.3 * (area.right - area.left),
                area.top + 0.3 * (area.bottom - area.top),
            );
            chart.zoom(cursor, 0.5);
            assert_ne!(chart.view(), home, "zoom narrows the shared view state");
            chart.render().unwrap();
            chart.pan(cursor, (cursor.0 + 5.0, cursor.1 + 5.0));
            chart.render().unwrap();
            assert!(chart.area().is_some(), "render always records the plot area");
            chart.reset();
            assert_eq!(chart.view(), home, "reset returns to the home view");
        }

        let mut spectrum = LineChart::spectrum_with_bands(
            &[1.0, 5.0, 9.0, 4.0, 1.0],
            vec![Band {
                x: AxisRange { min: 1.0, max: 3.0 },
                color_index: 0,
            }],
        );
        let mut violin = LineChart::violin(&[
            ("w1".to_string(), vec![1.0, 2.0, 2.0, 3.0]),
            ("w2".to_string(), vec![4.0, 5.0, 5.0, 6.0]),
        ]);
        exercise(&mut spectrum);
        exercise(&mut violin);
    }
}
