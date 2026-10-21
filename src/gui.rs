#![deny(unsafe_code)]
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use moulinette::{
    chart,
    conversion::Format,
    gui_state,
    lst::ListFile,
    mpa::{MpaFile, Spectrum},
    workflow,
};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::{cell::RefCell, path::PathBuf, rc::Rc, sync::Arc};
// Slint-generated bindings require unsafe code; handwritten code does not.
#[allow(unsafe_code)]
mod presentation {
    slint::include_modules!();
}
use presentation::{MainWindow, RoiRow, TimeWindowRow};

#[derive(Default)]
struct State {
    input: PathBuf,
    file: Option<Arc<MpaFile>>,
    list: Option<Arc<ListFile>>,
    result_directory: Option<PathBuf>,
    /// The in-window rate-vs-time chart of the last list-mode run.
    rate_chart: Option<chart::LineChart>,
    /// The in-window per-window-group density ("violin") chart of the last
    /// list-mode run, when it configured at least one time window.
    violin_chart: Option<chart::LineChart>,
    /// The in-window spectrum-with-ROI-bands chart of the last `.mpa`
    /// Analyze run.
    spectrum_chart: Option<chart::LineChart>,
}

/// One list-mode analysis run's outcome, sent from the worker thread
/// (`on_analyze`'s `.lst` branch) back to the UI timer: the written
/// files' result plus the two in-window charts (rate-vs-time always,
/// violin only when at least one time window is configured), each
/// already rendered at its default size off the UI thread.
struct ListAnalysisOutcome {
    output_dir: PathBuf,
    written: Result<Vec<PathBuf>, String>,
    rate_chart: Option<(chart::LineChart, Result<chart::RgbImage, String>)>,
    violin_chart: Option<(chart::LineChart, Result<chart::RgbImage, String>)>,
}

/// One `.mpa` Analyze-screen ROI run's outcome, sent from the worker
/// thread (`on_analyze`'s `.mpa` branch) back to the UI timer: the
/// statistics plus the in-window spectrum chart, already rendered at its
/// default size off the UI thread.
struct MpaAnalysisOutcome {
    stats: Vec<gui_state::RoiStatistics>,
    chart: chart::LineChart,
    image: Result<chart::RgbImage, String>,
}

/// `MainWindow::screen` values; see the property's doc comment in `main.slint`.
const SCREEN_HOME: i32 = 0;
const SCREEN_CONVERT: i32 = 1;
const SCREEN_ANALYZE: i32 = 2;

/// Reset to the home screen, clearing the loaded file and the busy/loaded/
/// has-results state the same way loading a new file does.
fn go_home(ui: &MainWindow, shared: &Rc<RefCell<State>>) {
    *shared.borrow_mut() = State::default();
    ui.set_screen(SCREEN_HOME);
    ui.set_list_mode(false);
    hide_rate_chart(ui);
    hide_violin_chart(ui);
    hide_spectrum_chart(ui);
    ui.set_busy(false);
    ui.set_loaded(false);
    ui.set_has_results(false);
    ui.set_filename("No measurement selected".into());
    ui.set_input_path("".into());
    ui.set_output_path("".into());
    ui.set_inspection("".into());
    ui.set_preview("".into());
    ui.set_results("".into());
    ui.set_warnings("".into());
    ui.set_drop_hint("".into());
    ui.set_error(false);
    ui.set_status("Open an MPA measurement or list-mode file to begin.".into());
}

/// A loaded input: an `.mpa` measurement or an `.lst` list-mode file.
enum Measurement {
    Mpa(MpaFile),
    List(ListFile),
}

fn blank_roi() -> RoiRow {
    RoiRow {
        adc: "1".into(),
        ..Default::default()
    }
}

/// Convert the shared ROI rows model into `gui_state::RoiFields`, shared by
/// the `.lst` list-mode panel and the `.mpa` Analyze-screen ROI editor.
fn roi_fields(rois: &[RoiRow]) -> Vec<gui_state::RoiFields<'_>> {
    rois.iter()
        .map(|r| gui_state::RoiFields {
            name: r.name.as_str(),
            adc: r.adc.as_str(),
            low: r.low.as_str(),
            high: r.high.as_str(),
            unit: r.unit,
        })
        .collect()
}

/// Validate the list-mode panel's current rows and fields.
fn list_parameters(ui: &MainWindow) -> Result<gui_state::ListParameters, String> {
    let rois: Vec<RoiRow> = ui.get_rois().iter().collect();
    let windows: Vec<TimeWindowRow> = ui.get_time_windows().iter().collect();
    gui_state::list_mode_parameters(
        &roi_fields(&rois),
        ui.get_bin_width().as_str(),
        &windows
            .iter()
            .map(|w| (w.start_s.as_str(), w.end_s.as_str()))
            .collect::<Vec<_>>(),
        ui.get_output_path().as_str(),
    )
}

/// Validate the `.mpa` Analyze-screen ROI editor's current rows (the same
/// rows model as the list-mode panel, without time windows/bin width).
fn mpa_roi_parameters(ui: &MainWindow) -> Result<Vec<gui_state::RoiSpec>, String> {
    let rois: Vec<RoiRow> = ui.get_rois().iter().collect();
    gui_state::mpa_roi_parameters(&roi_fields(&rois))
}

fn joined(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Labels for the Convert screen's "Monitor ADC/spectrum" dropdown
/// (charge normalization, ticket 08): one entry per loaded spectrum, in
/// the same order as `spectra` itself (0-based, unlike that dropdown's
/// "All spectra" first entry, since a monitor must be exactly one
/// spectrum).
fn monitor_adc_labels(spectra: &[Spectrum]) -> Vec<String> {
    spectra
        .iter()
        .map(|s| format!("ADC{} · {} channels", s.adc_index, s.declared_channels))
        .collect()
}

fn strings(values: impl IntoIterator<Item = String>) -> ModelRc<SharedString> {
    Rc::new(VecModel::from(
        values
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    ))
    .into()
}

fn refresh(ui: &MainWindow, state: &State) {
    let mode = ui.get_mode();
    if let Some((_, _, description)) = usize::try_from(mode)
        .ok()
        .and_then(|n| gui_state::MODES.get(n))
    {
        ui.set_description((*description).into());
    }
    if let Some(file) = &state.file {
        ui.set_inspection(gui_state::inspection(file, ui.get_spectrum()).into());
        let preview = gui_state::preview(
            &state.input,
            &PathBuf::from(ui.get_output_path().as_str()),
            mode,
            ui.get_spectrum(),
            file.spectra.len(),
        );
        ui.set_preview(
            match preview {
                Ok(paths) => joined(&paths),
                Err(e) => e,
            }
            .into(),
        );
    }
    if state.list.is_some() {
        ui.set_preview(
            match list_parameters(ui) {
                Ok(params) => joined(&gui_state::list_mode_outputs(&params)),
                Err(e) => e,
            }
            .into(),
        );
    }
}

fn report(ui: &MainWindow, result: Result<String, String>) {
    ui.set_busy(false);
    ui.set_error(result.is_err());
    ui.set_status(result.unwrap_or_else(|e| e).into());
}

/// Mouse-wheel zoom factor per logical pixel of scroll delta: winit reports
/// one wheel notch as 60 px, so a notch scales the view by about 0.84.
const ZOOM_RATE: f64 = 0.003;

fn hide_rate_chart(ui: &MainWindow) {
    ui.set_rate_chart_ready(false);
    ui.set_rate_chart_tooltip("".into());
}

fn show_rate_chart(ui: &MainWindow, chart: &mut chart::LineChart) -> Result<(), String> {
    let image = chart.render()?;
    ui.set_rate_chart(image.to_slint_image());
    ui.set_rate_chart_ready(true);
    Ok(())
}

/// Size the chart's buffer to the displaying area's `width` x `height`
/// (logical) in physical pixels, so it is shown unscaled, and re-render it
/// whenever that changed or no render matches the current size yet.
fn fit_rate_chart(
    ui: &MainWindow,
    chart: &mut chart::LineChart,
    width: f32,
    height: f32,
) -> Result<(), String> {
    if !(width > 0. && height > 0.) {
        return Ok(());
    }
    let scale = ui.window().scale_factor();
    let resized = chart.set_size(
        (width * scale).round() as u32,
        (height * scale).round() as u32,
    );
    if resized || chart.area().is_none() {
        show_rate_chart(ui, chart)?;
    }
    Ok(())
}

/// Show the tooltip and marker for the data point nearest to `(x, y)` in
/// the chart area's logical coordinates, or hide them.
fn hover_rate_chart(ui: &MainWindow, chart: &chart::LineChart, x: f32, y: f32, w: f32, h: f32) {
    let element = (f64::from(w), f64::from(h));
    let cursor = chart::element_to_buffer((f64::from(x), f64::from(y)), element, chart.size());
    match chart.nearest(cursor) {
        Some(hover) => {
            let (mx, my) = chart::buffer_to_element(hover.pixel, element, chart.size());
            ui.set_rate_chart_marker_x(mx as f32);
            ui.set_rate_chart_marker_y(my as f32);
            ui.set_rate_chart_tooltip(chart.tooltip(&hover).into());
        }
        None => ui.set_rate_chart_tooltip("".into()),
    }
}

/// Wire the rate-vs-time chart area's resize/zoom/pan/hover/reset callbacks
/// to `State::rate_chart`. Only the in-window view changes; the files of the
/// run are already written.
fn install_rate_chart(ui: &MainWindow, shared: &Rc<RefCell<State>>) {
    // Runs `f` on the current chart; a render failure is reported in the
    // status line rather than aborting the interaction.
    fn with_chart(
        weak: &slint::Weak<MainWindow>,
        shared: &Rc<RefCell<State>>,
        f: impl FnOnce(&MainWindow, &mut chart::LineChart) -> Result<(), String>,
    ) {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let mut state = shared.borrow_mut();
        let Some(chart) = state.rate_chart.as_mut() else {
            return;
        };
        if let Err(e) = f(&ui, chart) {
            report(&ui, Err(format!("Cannot draw the rate chart: {e}")));
        }
    }

    let (weak, state) = (ui.as_weak(), shared.clone());
    ui.on_rate_chart_resized(move |w, h| {
        with_chart(&weak, &state, |ui, chart| fit_rate_chart(ui, chart, w, h));
    });

    let (weak, state) = (ui.as_weak(), shared.clone());
    ui.on_rate_chart_zoom(move |x, y, delta, w, h| {
        with_chart(&weak, &state, |ui, chart| {
            fit_rate_chart(ui, chart, w, h)?;
            let cursor = chart::element_to_buffer(
                (f64::from(x), f64::from(y)),
                (f64::from(w), f64::from(h)),
                chart.size(),
            );
            // Scrolling up (positive delta) zooms in.
            chart.zoom(cursor, (-f64::from(delta) * ZOOM_RATE).exp());
            show_rate_chart(ui, chart)?;
            hover_rate_chart(ui, chart, x, y, w, h);
            Ok(())
        });
    });

    let (weak, state) = (ui.as_weak(), shared.clone());
    ui.on_rate_chart_pan(move |from_x, from_y, to_x, to_y, w, h| {
        with_chart(&weak, &state, |ui, chart| {
            fit_rate_chart(ui, chart, w, h)?;
            let element = (f64::from(w), f64::from(h));
            let from =
                chart::element_to_buffer((f64::from(from_x), f64::from(from_y)), element, chart.size());
            let to = chart::element_to_buffer((f64::from(to_x), f64::from(to_y)), element, chart.size());
            chart.pan(from, to);
            show_rate_chart(ui, chart)
        });
    });

    let (weak, state) = (ui.as_weak(), shared.clone());
    ui.on_rate_chart_hover(move |x, y, w, h| {
        with_chart(&weak, &state, |ui, chart| {
            fit_rate_chart(ui, chart, w, h)?;
            hover_rate_chart(ui, chart, x, y, w, h);
            Ok(())
        });
    });

    let (weak, state) = (ui.as_weak(), shared.clone());
    ui.on_rate_chart_reset(move || {
        with_chart(&weak, &state, |ui, chart| {
            chart.reset();
            ui.set_rate_chart_tooltip("".into());
            show_rate_chart(ui, chart)
        });
    });
}

fn hide_violin_chart(ui: &MainWindow) {
    ui.set_violin_chart_ready(false);
    ui.set_violin_chart_tooltip("".into());
}

fn show_violin_chart(ui: &MainWindow, chart: &mut chart::LineChart) -> Result<(), String> {
    let image = chart.render()?;
    ui.set_violin_chart(image.to_slint_image());
    ui.set_violin_chart_ready(true);
    Ok(())
}

/// Size the violin chart's buffer to the displaying area's size, like
/// `fit_rate_chart`.
fn fit_violin_chart(
    ui: &MainWindow,
    chart: &mut chart::LineChart,
    width: f32,
    height: f32,
) -> Result<(), String> {
    if !(width > 0. && height > 0.) {
        return Ok(());
    }
    let scale = ui.window().scale_factor();
    let resized = chart.set_size(
        (width * scale).round() as u32,
        (height * scale).round() as u32,
    );
    if resized || chart.area().is_none() {
        show_violin_chart(ui, chart)?;
    }
    Ok(())
}

/// Show the tooltip and marker for the nearest group/density point to
/// `(x, y)`, like `hover_rate_chart`.
fn hover_violin_chart(ui: &MainWindow, chart: &chart::LineChart, x: f32, y: f32, w: f32, h: f32) {
    let element = (f64::from(w), f64::from(h));
    let cursor = chart::element_to_buffer((f64::from(x), f64::from(y)), element, chart.size());
    match chart.nearest(cursor) {
        Some(hover) => {
            let (mx, my) = chart::buffer_to_element(hover.pixel, element, chart.size());
            ui.set_violin_chart_marker_x(mx as f32);
            ui.set_violin_chart_marker_y(my as f32);
            ui.set_violin_chart_tooltip(chart.tooltip(&hover).into());
        }
        None => ui.set_violin_chart_tooltip("".into()),
    }
}

/// Wire the violin (time-window density) chart area's resize/zoom/pan/
/// hover/reset callbacks to `State::violin_chart`, exactly like
/// `install_rate_chart` does for the rate-vs-time chart. Only the
/// in-window view changes; the per-ROI violin PNGs of the run (`plot::
/// violin`, via `list_mode_write`) are already written and untouched.
fn install_violin_chart(ui: &MainWindow, shared: &Rc<RefCell<State>>) {
    fn with_chart(
        weak: &slint::Weak<MainWindow>,
        shared: &Rc<RefCell<State>>,
        f: impl FnOnce(&MainWindow, &mut chart::LineChart) -> Result<(), String>,
    ) {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let mut state = shared.borrow_mut();
        let Some(chart) = state.violin_chart.as_mut() else {
            return;
        };
        if let Err(e) = f(&ui, chart) {
            report(&ui, Err(format!("Cannot draw the violin chart: {e}")));
        }
    }

    let (weak, state) = (ui.as_weak(), shared.clone());
    ui.on_violin_chart_resized(move |w, h| {
        with_chart(&weak, &state, |ui, chart| fit_violin_chart(ui, chart, w, h));
    });

    let (weak, state) = (ui.as_weak(), shared.clone());
    ui.on_violin_chart_zoom(move |x, y, delta, w, h| {
        with_chart(&weak, &state, |ui, chart| {
            fit_violin_chart(ui, chart, w, h)?;
            let cursor = chart::element_to_buffer(
                (f64::from(x), f64::from(y)),
                (f64::from(w), f64::from(h)),
                chart.size(),
            );
            chart.zoom(cursor, (-f64::from(delta) * ZOOM_RATE).exp());
            show_violin_chart(ui, chart)?;
            hover_violin_chart(ui, chart, x, y, w, h);
            Ok(())
        });
    });

    let (weak, state) = (ui.as_weak(), shared.clone());
    ui.on_violin_chart_pan(move |from_x, from_y, to_x, to_y, w, h| {
        with_chart(&weak, &state, |ui, chart| {
            fit_violin_chart(ui, chart, w, h)?;
            let element = (f64::from(w), f64::from(h));
            let from = chart::element_to_buffer(
                (f64::from(from_x), f64::from(from_y)),
                element,
                chart.size(),
            );
            let to =
                chart::element_to_buffer((f64::from(to_x), f64::from(to_y)), element, chart.size());
            chart.pan(from, to);
            show_violin_chart(ui, chart)
        });
    });

    let (weak, state) = (ui.as_weak(), shared.clone());
    ui.on_violin_chart_hover(move |x, y, w, h| {
        with_chart(&weak, &state, |ui, chart| {
            fit_violin_chart(ui, chart, w, h)?;
            hover_violin_chart(ui, chart, x, y, w, h);
            Ok(())
        });
    });

    let (weak, state) = (ui.as_weak(), shared.clone());
    ui.on_violin_chart_reset(move || {
        with_chart(&weak, &state, |ui, chart| {
            chart.reset();
            ui.set_violin_chart_tooltip("".into());
            show_violin_chart(ui, chart)
        });
    });
}

fn hide_spectrum_chart(ui: &MainWindow) {
    ui.set_spectrum_chart_ready(false);
    ui.set_spectrum_chart_tooltip("".into());
}

fn show_spectrum_chart(ui: &MainWindow, chart: &mut chart::LineChart) -> Result<(), String> {
    let image = chart.render()?;
    ui.set_spectrum_chart(image.to_slint_image());
    ui.set_spectrum_chart_ready(true);
    Ok(())
}

/// Size the spectrum chart's buffer to the displaying area's size, like
/// `fit_rate_chart`.
fn fit_spectrum_chart(
    ui: &MainWindow,
    chart: &mut chart::LineChart,
    width: f32,
    height: f32,
) -> Result<(), String> {
    if !(width > 0. && height > 0.) {
        return Ok(());
    }
    let scale = ui.window().scale_factor();
    let resized = chart.set_size(
        (width * scale).round() as u32,
        (height * scale).round() as u32,
    );
    if resized || chart.area().is_none() {
        show_spectrum_chart(ui, chart)?;
    }
    Ok(())
}

/// Show the tooltip and marker for the nearest channel/count point to
/// `(x, y)`, like `hover_rate_chart`.
fn hover_spectrum_chart(ui: &MainWindow, chart: &chart::LineChart, x: f32, y: f32, w: f32, h: f32) {
    let element = (f64::from(w), f64::from(h));
    let cursor = chart::element_to_buffer((f64::from(x), f64::from(y)), element, chart.size());
    match chart.nearest(cursor) {
        Some(hover) => {
            let (mx, my) = chart::buffer_to_element(hover.pixel, element, chart.size());
            ui.set_spectrum_chart_marker_x(mx as f32);
            ui.set_spectrum_chart_marker_y(my as f32);
            ui.set_spectrum_chart_tooltip(chart.tooltip(&hover).into());
        }
        None => ui.set_spectrum_chart_tooltip("".into()),
    }
}

/// Wire the `.mpa` spectrum-with-ROI-bands chart area's resize/zoom/pan/
/// hover/reset callbacks to `State::spectrum_chart`, exactly like
/// `install_rate_chart` does for the rate-vs-time chart. ROI bands stay
/// shaded through pan/zoom since `chart::LineChart::render` redraws them
/// clipped to the current view every time.
fn install_spectrum_chart(ui: &MainWindow, shared: &Rc<RefCell<State>>) {
    fn with_chart(
        weak: &slint::Weak<MainWindow>,
        shared: &Rc<RefCell<State>>,
        f: impl FnOnce(&MainWindow, &mut chart::LineChart) -> Result<(), String>,
    ) {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let mut state = shared.borrow_mut();
        let Some(chart) = state.spectrum_chart.as_mut() else {
            return;
        };
        if let Err(e) = f(&ui, chart) {
            report(&ui, Err(format!("Cannot draw the spectrum chart: {e}")));
        }
    }

    let (weak, state) = (ui.as_weak(), shared.clone());
    ui.on_spectrum_chart_resized(move |w, h| {
        with_chart(&weak, &state, |ui, chart| fit_spectrum_chart(ui, chart, w, h));
    });

    let (weak, state) = (ui.as_weak(), shared.clone());
    ui.on_spectrum_chart_zoom(move |x, y, delta, w, h| {
        with_chart(&weak, &state, |ui, chart| {
            fit_spectrum_chart(ui, chart, w, h)?;
            let cursor = chart::element_to_buffer(
                (f64::from(x), f64::from(y)),
                (f64::from(w), f64::from(h)),
                chart.size(),
            );
            chart.zoom(cursor, (-f64::from(delta) * ZOOM_RATE).exp());
            show_spectrum_chart(ui, chart)?;
            hover_spectrum_chart(ui, chart, x, y, w, h);
            Ok(())
        });
    });

    let (weak, state) = (ui.as_weak(), shared.clone());
    ui.on_spectrum_chart_pan(move |from_x, from_y, to_x, to_y, w, h| {
        with_chart(&weak, &state, |ui, chart| {
            fit_spectrum_chart(ui, chart, w, h)?;
            let element = (f64::from(w), f64::from(h));
            let from = chart::element_to_buffer(
                (f64::from(from_x), f64::from(from_y)),
                element,
                chart.size(),
            );
            let to =
                chart::element_to_buffer((f64::from(to_x), f64::from(to_y)), element, chart.size());
            chart.pan(from, to);
            show_spectrum_chart(ui, chart)
        });
    });

    let (weak, state) = (ui.as_weak(), shared.clone());
    ui.on_spectrum_chart_hover(move |x, y, w, h| {
        with_chart(&weak, &state, |ui, chart| {
            fit_spectrum_chart(ui, chart, w, h)?;
            hover_spectrum_chart(ui, chart, x, y, w, h);
            Ok(())
        });
    });

    let (weak, state) = (ui.as_weak(), shared.clone());
    ui.on_spectrum_chart_reset(move || {
        with_chart(&weak, &state, |ui, chart| {
            chart.reset();
            ui.set_spectrum_chart_tooltip("".into());
            show_spectrum_chart(ui, chart)
        });
    });
}

// Winit 0.30 delivers one event per file and no position with the drop.
// Read the Windows cursor in physical coordinates, then convert to Slint units.
#[cfg(target_os = "windows")]
fn drop_position(ui: &MainWindow) -> Option<(f32, f32)> {
    use slint::winit_030::WinitWindowAccessor;
    let cursor = winsafe::GetCursorPos().ok()?;
    ui.window().with_winit_window(|window| {
        let origin = window.inner_position().ok()?;
        let scale = window.scale_factor() as f32;
        Some((
            (cursor.x - origin.x) as f32 / scale,
            (cursor.y - origin.y) as f32 / scale,
        ))
    })?
}

#[cfg(target_os = "windows")]
fn install_file_drop(ui: &MainWindow, load: Rc<dyn Fn(PathBuf)>) {
    use slint::winit_030::{EventResult, WinitWindowAccessor, winit::event::WindowEvent};
    let hover_timer = Rc::new(slint::Timer::default());
    let drop_timer = Rc::new(slint::Timer::default());
    let pending = Rc::new(RefCell::new(Vec::<PathBuf>::new()));
    let weak = ui.as_weak();
    ui.window().on_winit_window_event(move |_, event| {
        let Some(ui) = weak.upgrade() else {
            return EventResult::Propagate;
        };
        match event {
            WindowEvent::HoveredFile(_) => {
                let weak = ui.as_weak();
                hover_timer.start(
                    slint::TimerMode::Repeated,
                    std::time::Duration::from_millis(50),
                    move || {
                        if let Some(ui) = weak.upgrade() {
                            let inside = drop_position(&ui)
                                .is_some_and(|(x, y)| ui.invoke_input_contains(x, y));
                            ui.set_drop_hint(if inside && ui.get_busy() {
                                "Working — drop the file again when finished.".into()
                            } else if inside {
                                "Release here to load one .mpa or .lst file.".into()
                            } else {
                                "".into()
                            });
                        }
                    },
                );
            }
            WindowEvent::HoveredFileCancelled => {
                hover_timer.stop();
                ui.set_drop_hint("".into());
            }
            WindowEvent::DroppedFile(path) => {
                hover_timer.stop();
                // Capture location and busy state at release, not when the timer fires.
                // Subsequent per-file events join this same batch without starting a load.
                if pending.borrow().is_empty() {
                    let inside =
                        drop_position(&ui).is_some_and(|(x, y)| ui.invoke_input_contains(x, y));
                    let busy = ui.get_busy();
                    let weak = ui.as_weak();
                    let pending = pending.clone();
                    let load = load.clone();
                    drop_timer.start(
                        slint::TimerMode::SingleShot,
                        std::time::Duration::from_millis(50),
                        move || {
                            let paths = std::mem::take(&mut *pending.borrow_mut());
                            let Some(ui) = weak.upgrade() else {
                                return;
                            };
                            ui.set_drop_hint("".into());
                            if !inside {
                                return;
                            }
                            match gui_state::dropped_measurement(&paths, busy || ui.get_busy()) {
                                Ok(path) => load(path.to_path_buf()),
                                Err(message) => ui.set_drop_hint(message.into()),
                            }
                        },
                    );
                }
                pending.borrow_mut().push(path.clone());
            }
            _ => {}
        }
        EventResult::Propagate
    });
}

#[cfg(not(target_os = "windows"))]
fn install_file_drop(_: &MainWindow, _: Rc<dyn Fn(PathBuf)>) {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = MainWindow::new()?;
    let state = Rc::new(RefCell::new(State::default()));
    let timer = Rc::new(slint::Timer::default());
    ui.set_formats(strings(
        gui_state::MODES
            .iter()
            .map(|(_, label, _)| label.to_string()),
    ));
    ui.set_spectra(strings(["All spectra".into()]));
    ui.set_roi_units(strings(
        gui_state::ROI_UNITS
            .iter()
            .map(|(_, label)| label.to_string()),
    ));
    ui.set_bin_width(gui_state::DEFAULT_BIN_WIDTH_MS.to_string().into());
    // Rows persist across loads so one ROI set can be reused for several files.
    let rois = Rc::new(VecModel::from(vec![blank_roi()]));
    let time_windows = Rc::new(VecModel::<TimeWindowRow>::default());
    ui.set_rois(rois.clone().into());
    ui.set_time_windows(time_windows.clone().into());
    refresh(&ui, &state.borrow());

    let weak = ui.as_weak();
    let shared = state.clone();
    ui.on_settings_changed(move || {
        if let Some(ui) = weak.upgrade() {
            refresh(&ui, &shared.borrow());
        }
    });

    let weak = ui.as_weak();
    let shared = state.clone();
    let timer_open = timer.clone();
    let load: Rc<dyn Fn(PathBuf)> = Rc::new(move |path| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        if ui.get_busy() {
            return;
        }
        let list_mode = gui_state::is_list_mode(&path);
        // On the Convert screen, a `.lst` file is histogrammed and shown
        // through the converter panel rather than the list-mode (ROI) panel,
        // so the UI-facing flag only reflects list-mode outside Convert.
        ui.set_list_mode(list_mode && ui.get_screen() != SCREEN_CONVERT);
        ui.set_drop_hint("".into());
        ui.set_busy(true);
        ui.set_loaded(false);
        ui.set_has_results(false);
        hide_rate_chart(&ui);
        hide_violin_chart(&ui);
        hide_spectrum_chart(&ui);
        ui.set_filename(
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .as_ref()
                .into(),
        );
        ui.set_input_path(path.to_string_lossy().as_ref().into());
        ui.set_inspection("".into());
        ui.set_preview("".into());
        ui.set_results(
            if list_mode {
                "No analysis yet."
            } else {
                "No conversion yet."
            }
            .into(),
        );
        ui.set_warnings("".into());
        *shared.borrow_mut() = State::default();
        ui.set_error(false);
        ui.set_status("Reading measurement…".into());
        let weak = ui.as_weak();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = if list_mode {
                gui_state::load_list(&path).map(Measurement::List)
            } else {
                workflow::load(&path).map(Measurement::Mpa)
            };
            let _ = sender.send((path, result));
        });
        // Timer polls only the channel; all Slint/state access stays on the event-loop thread.
        let timer = timer_open.clone();
        let timer_callback = Rc::downgrade(&timer);
        let shared = shared.clone();
        timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(50),
            move || {
                let (path, result) = match receiver.try_recv() {
                    Ok(value) => value,
                    Err(std::sync::mpsc::TryRecvError::Empty) => return,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        if let Some(timer) = timer_callback.upgrade() {
                            timer.stop();
                        }
                        if let Some(ui) = weak.upgrade() {
                            report(&ui, Err("Measurement worker stopped unexpectedly".into()));
                        }
                        return;
                    }
                };
                if let Some(timer) = timer_callback.upgrade() {
                    timer.stop();
                }
                let Some(ui) = weak.upgrade() else {
                    return;
                };
                match result {
                    Ok(measurement) => {
                        ui.set_filename(
                            path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .as_ref()
                                .into(),
                        );
                        ui.set_input_path(path.to_string_lossy().as_ref().into());
                        ui.set_output_path(
                            path.parent()
                                .unwrap_or_else(|| std::path::Path::new("."))
                                .to_string_lossy()
                                .as_ref()
                                .into(),
                        );
                        ui.set_loaded(true);
                        ui.set_has_results(false);
                        ui.set_warnings("".into());
                        // Loading an `.lst` file on the Convert screen is
                        // handled below via the Ticket 2 histogram; every
                        // other screen/extension combination is supported.
                        let screen = ui.get_screen();
                        let mut state = shared.borrow_mut();
                        state.input = path;
                        state.result_directory = None;
                        let message = match measurement {
                            Measurement::Mpa(file) => {
                                let count = file.spectra.len();
                                ui.set_spectra(strings(
                                    std::iter::once("All spectra".into()).chain(
                                        file.spectra.iter().enumerate().map(|(i, s)| {
                                            format!(
                                                "Spectrum {} · ADC{} · {} channels",
                                                i + 1,
                                                s.adc_index,
                                                s.declared_channels
                                            )
                                        }),
                                    ),
                                ));
                                ui.set_spectrum(0);
                                ui.set_monitor_adcs(strings(monitor_adc_labels(&file.spectra)));
                                ui.set_monitor_adc(0);
                                ui.set_results(
                                    if screen == SCREEN_ANALYZE {
                                        "No analysis yet."
                                    } else {
                                        "No conversion yet."
                                    }
                                    .into(),
                                );
                                state.file = Some(Arc::new(file));
                                format!("Measurement ready · {count} spectra.")
                            }
                            Measurement::List(file) if screen == SCREEN_CONVERT => {
                                // Reuse the converter panel over a Ticket 2
                                // histogram, one spectrum per ADC.
                                let wrapped = MpaFile {
                                    sections: Vec::new(),
                                    spectra: gui_state::histogram_list_file(&file),
                                };
                                let count = wrapped.spectra.len();
                                ui.set_spectra(strings(
                                    std::iter::once("All spectra".into()).chain(
                                        wrapped.spectra.iter().enumerate().map(|(i, s)| {
                                            format!(
                                                "Spectrum {} · ADC{} · {} channels",
                                                i + 1,
                                                s.adc_index,
                                                s.declared_channels
                                            )
                                        }),
                                    ),
                                ));
                                ui.set_spectrum(0);
                                ui.set_monitor_adcs(strings(monitor_adc_labels(&wrapped.spectra)));
                                ui.set_monitor_adc(0);
                                ui.set_results("No conversion yet.".into());
                                state.file = Some(Arc::new(wrapped));
                                format!("Measurement ready · {count} spectra.")
                            }
                            Measurement::List(file) => {
                                let count = file.events.len();
                                ui.set_inspection(gui_state::list_inspection(&file).into());
                                ui.set_results("No analysis yet.".into());
                                state.list = Some(Arc::new(file));
                                format!("List-mode file ready · {count} events.")
                            }
                        };
                        refresh(&ui, &state);
                        report(&ui, Ok(message));
                    }
                    Err(e) => report(&ui, Err(e)),
                }
            },
        );
    });

    let load_input = load.clone();
    let weak = ui.as_weak();
    ui.on_open_input(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        if ui.get_busy() {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Measurements", &["mpa", "lst"])
            .add_filter("MPA measurements", &["mpa"])
            .add_filter("List-mode files", &["lst"])
            .pick_file()
        {
            load_input(path);
        }
    });

    let weak = ui.as_weak();
    let shared = state.clone();
    ui.on_choose_output(move || {
        if let Some(ui) = weak.upgrade()
            && !ui.get_busy()
            && let Some(path) = rfd::FileDialog::new().pick_folder()
        {
            ui.set_output_path(path.to_string_lossy().as_ref().into());
            refresh(&ui, &shared.borrow());
        }
    });

    let weak = ui.as_weak();
    let shared = state.clone();
    let timer_convert = timer.clone();
    ui.on_convert(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        if ui.get_busy() {
            return;
        }
        let state = shared.borrow();
        let Some(file) = state.file.clone() else {
            report(&ui, Err("Open a measurement first".into()));
            return;
        };
        let params = gui_state::parameters(
            ui.get_mode(),
            ui.get_spectrum(),
            ui.get_first_channel().as_str(),
            ui.get_last_channel().as_str(),
            file.spectra.len(),
        );
        let (format, selection, indices) = match params {
            Ok(p) => p,
            Err(e) => {
                report(&ui, Err(e));
                return;
            }
        };
        let dir = PathBuf::from(ui.get_output_path().as_str());
        let paths = match workflow::filenames(&state.input, &dir, format, &indices) {
            Ok(p) => p,
            Err(e) => {
                report(&ui, Err(e));
                return;
            }
        };
        ui.set_busy(true);
        ui.set_error(false);
        ui.set_status("Validating and converting measurement…".into());
        ui.set_has_results(false);
        ui.set_results("Conversion in progress…".into());
        ui.set_warnings(gui_state::warnings(format).into());
        let weak = ui.as_weak();
        let shared = shared.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        // `Format::ChargeNormalized` (ticket 08) needs a second, user-
        // chosen monitor spectrum that `workflow::render`'s per-spectrum
        // `conversion::convert` loop has no room for (see
        // `conversion::Format::ChargeNormalized`'s doc comment and ADR
        // 0002), so it is written here directly via
        // `gui_state::charge_normalized` instead of going through that
        // loop; `workflow::filenames`/`workflow::write` (naming/writing,
        // independent of `convert`) are reused unchanged.
        if format == Format::ChargeNormalized {
            let monitor_index = usize::try_from(ui.get_monitor_adc()).unwrap_or(usize::MAX);
            let Some(monitor) = file.spectra.get(monitor_index).cloned() else {
                report(&ui, Err("Choose a monitor ADC/spectrum".into()));
                return;
            };
            let calibration = match gui_state::charge_calibration(
                monitor.adc_index as u8,
                ui.get_counts_per_uc().as_str(),
            ) {
                Ok(c) => c,
                Err(e) => {
                    report(&ui, Err(e));
                    return;
                }
            };
            std::thread::spawn(move || {
                let result = (|| {
                    let mut outputs = Vec::new();
                    for (&index, path) in indices.iter().zip(paths) {
                        if outputs.iter().any(|(p, _): &(PathBuf, Vec<u8>)| p == &path) {
                            return Err("Duplicate output destination".to_string());
                        }
                        let spectrum = file
                            .spectra
                            .get(index)
                            .ok_or("Spectrum selection is out of range")?;
                        let bytes = gui_state::charge_normalized(
                            spectrum,
                            selection,
                            &monitor,
                            calibration,
                        )
                        .map_err(|e| e.to_string())?;
                        outputs.push((path, bytes));
                    }
                    workflow::write(outputs, Some(&dir))
                })();
                let _ = sender.send((dir, result));
            });
        } else {
            std::thread::spawn(move || {
                let result = workflow::render(&file, format, selection, &indices, paths)
                    .and_then(|outputs| workflow::write(outputs, Some(&dir)));
                let _ = sender.send((dir, result));
            });
        }
        let timer = timer_convert.clone();
        let timer_callback = Rc::downgrade(&timer);
        timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(50),
            move || {
                let (dir, result) = match receiver.try_recv() {
                    Ok(value) => value,
                    Err(std::sync::mpsc::TryRecvError::Empty) => return,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        if let Some(timer) = timer_callback.upgrade() { timer.stop(); }
                        if let Some(ui) = weak.upgrade() { report(&ui, Err("Conversion worker stopped unexpectedly. Partial files may remain.".into())); }
                        return;
                    }
                };
                if let Some(timer) = timer_callback.upgrade() {
                    timer.stop();
                }
                let Some(ui) = weak.upgrade() else {
                    return;
                };
                match result {
                    Ok(paths) => {
                        ui.set_results(joined(&paths).into());
                        ui.set_has_results(true);
                        shared.borrow_mut().result_directory = Some(dir);
                        report(
                            &ui,
                            Ok(format!(
                                "Conversion complete · {} files generated.",
                                paths.len()
                            )),
                        );
                    }
                    Err(e) => {
                        ui.set_results("Conversion failed. See the message below.".into());
                        report(&ui, Err(e));
                    }
                }
            },
        );
    });

    let weak = ui.as_weak();
    let shared = state.clone();
    let model = rois.clone();
    ui.on_add_roi(move || {
        if let Some(ui) = weak.upgrade()
            && !ui.get_busy()
        {
            model.push(blank_roi());
            refresh(&ui, &shared.borrow());
        }
    });

    let weak = ui.as_weak();
    let shared = state.clone();
    ui.on_remove_roi(move |index| {
        if let Some(ui) = weak.upgrade()
            && !ui.get_busy()
            && let Ok(index) = usize::try_from(index)
            && index < rois.row_count()
        {
            rois.remove(index);
            refresh(&ui, &shared.borrow());
        }
    });

    let weak = ui.as_weak();
    let shared = state.clone();
    let model = time_windows.clone();
    ui.on_add_time_window(move || {
        if let Some(ui) = weak.upgrade()
            && !ui.get_busy()
        {
            model.push(TimeWindowRow::default());
            refresh(&ui, &shared.borrow());
        }
    });

    let weak = ui.as_weak();
    let shared = state.clone();
    ui.on_remove_time_window(move |index| {
        if let Some(ui) = weak.upgrade()
            && !ui.get_busy()
            && let Ok(index) = usize::try_from(index)
            && index < time_windows.row_count()
        {
            time_windows.remove(index);
            refresh(&ui, &shared.borrow());
        }
    });

    let weak = ui.as_weak();
    let shared = state.clone();
    let timer_analyze = timer.clone();
    ui.on_analyze(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        // The single busy flag is shared with loading and conversion, so no
        // two runs of any kind overlap.
        if ui.get_busy() {
            return;
        }
        let list_file = shared.borrow().list.clone();
        if let Some(file) = list_file {
            let params = match list_parameters(&ui) {
                Ok(p) => p,
                Err(e) => {
                    report(&ui, Err(e));
                    return;
                }
            };
            ui.set_busy(true);
            ui.set_error(false);
            ui.set_status("Computing rates and rendering plots…".into());
            ui.set_has_results(false);
            ui.set_results("Analysis in progress…".into());
            ui.set_warnings(gui_state::LIST_MODE_WARNINGS.into());
            hide_rate_chart(&ui);
            hide_violin_chart(&ui);
            shared.borrow_mut().rate_chart = None;
            shared.borrow_mut().violin_chart = None;
            let weak = ui.as_weak();
            let shared = shared.clone();
            let (sender, receiver) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                // The series are binned once, then feed both the written
                // files (`list_mode_write`, as in `list_mode_run`) and the
                // in-window charts, each rendered here at its default size.
                let outcome = match gui_state::rate_series(&file, &params) {
                    Ok(series) => {
                        let written = gui_state::list_mode_write(&series, &params);
                        let mut rate_chart = chart::LineChart::rate_vs_time(&series);
                        let rate_image = rate_chart.render();
                        let violin_chart = if params.windows.is_empty() {
                            None
                        } else {
                            let groups = gui_state::violin_series(&series, &params.windows);
                            let mut violin_chart = chart::LineChart::violin(&groups);
                            let violin_image = violin_chart.render();
                            Some((violin_chart, violin_image))
                        };
                        ListAnalysisOutcome {
                            output_dir: params.output_dir,
                            written,
                            rate_chart: Some((rate_chart, rate_image)),
                            violin_chart,
                        }
                    }
                    Err(e) => ListAnalysisOutcome {
                        output_dir: params.output_dir,
                        written: Err(e),
                        rate_chart: None,
                        violin_chart: None,
                    },
                };
                let _ = sender.send(outcome);
            });
            let timer = timer_analyze.clone();
            let timer_callback = Rc::downgrade(&timer);
            timer.start(
                slint::TimerMode::Repeated,
                std::time::Duration::from_millis(50),
                move || {
                    let outcome = match receiver.try_recv() {
                        Ok(value) => value,
                        Err(std::sync::mpsc::TryRecvError::Empty) => return,
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            if let Some(timer) = timer_callback.upgrade() {
                                timer.stop();
                            }
                            if let Some(ui) = weak.upgrade() {
                                report(
                                    &ui,
                                    Err(
                                        "Analysis worker stopped unexpectedly. Partial files may remain."
                                            .into(),
                                    ),
                                );
                            }
                            return;
                        }
                    };
                    if let Some(timer) = timer_callback.upgrade() {
                        timer.stop();
                    }
                    let Some(ui) = weak.upgrade() else {
                        return;
                    };
                    let ListAnalysisOutcome {
                        output_dir,
                        written,
                        rate_chart,
                        violin_chart,
                    } = outcome;
                    // The charts show the computed series even when writing
                    // the files failed (e.g. an output already exists).
                    let rate_chart_error = match rate_chart {
                        Some((chart, Ok(image))) => {
                            shared.borrow_mut().rate_chart = Some(chart);
                            ui.set_rate_chart(image.to_slint_image());
                            ui.set_rate_chart_ready(true);
                            None
                        }
                        Some((_, Err(e))) => Some(e),
                        None => None,
                    };
                    let violin_chart_error = match violin_chart {
                        Some((chart, Ok(image))) => {
                            shared.borrow_mut().violin_chart = Some(chart);
                            ui.set_violin_chart(image.to_slint_image());
                            ui.set_violin_chart_ready(true);
                            None
                        }
                        Some((_, Err(e))) => Some(e),
                        None => None,
                    };
                    match written {
                        Ok(paths) => {
                            ui.set_results(joined(&paths).into());
                            ui.set_has_results(true);
                            shared.borrow_mut().result_directory = Some(output_dir);
                            let mut message =
                                format!("Analysis complete · {} files generated.", paths.len());
                            if let Some(e) = rate_chart_error {
                                message.push_str(&format!(
                                    " The in-window rate chart could not be drawn: {e}"
                                ));
                            }
                            if let Some(e) = violin_chart_error {
                                message.push_str(&format!(
                                    " The in-window violin chart could not be drawn: {e}"
                                ));
                            }
                            report(&ui, Ok(message));
                        }
                        Err(e) => {
                            ui.set_results("Analysis failed. See the message below.".into());
                            report(&ui, Err(e));
                        }
                    }
                },
            );
            return;
        }
        let mpa_file = shared.borrow().file.clone();
        if let Some(file) = mpa_file {
            let rois = match mpa_roi_parameters(&ui) {
                Ok(r) => r,
                Err(e) => {
                    report(&ui, Err(e));
                    return;
                }
            };
            let selected_spectrum = ui.get_spectrum();
            ui.set_busy(true);
            ui.set_error(false);
            ui.set_status("Computing ROI statistics and rendering the spectrum chart…".into());
            ui.set_has_results(false);
            ui.set_results("Analysis in progress…".into());
            ui.set_warnings("".into());
            hide_spectrum_chart(&ui);
            shared.borrow_mut().spectrum_chart = None;
            let weak = ui.as_weak();
            let shared = shared.clone();
            let (sender, receiver) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                // The chart is rendered here, off the UI thread, at its
                // default size (the chart area resizes/re-renders it once
                // shown, like the rate-vs-time and violin charts).
                let result = gui_state::mpa_analyze(&file, selected_spectrum, &rois).map(
                    |analysis| {
                        let mut chart = analysis.chart;
                        let image = chart.render();
                        MpaAnalysisOutcome {
                            stats: analysis.stats,
                            chart,
                            image,
                        }
                    },
                );
                let _ = sender.send(result);
            });
            let timer = timer_analyze.clone();
            let timer_callback = Rc::downgrade(&timer);
            timer.start(
                slint::TimerMode::Repeated,
                std::time::Duration::from_millis(50),
                move || {
                    let result = match receiver.try_recv() {
                        Ok(value) => value,
                        Err(std::sync::mpsc::TryRecvError::Empty) => return,
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            if let Some(timer) = timer_callback.upgrade() {
                                timer.stop();
                            }
                            if let Some(ui) = weak.upgrade() {
                                report(
                                    &ui,
                                    Err("Analysis worker stopped unexpectedly".into()),
                                );
                            }
                            return;
                        }
                    };
                    if let Some(timer) = timer_callback.upgrade() {
                        timer.stop();
                    }
                    let Some(ui) = weak.upgrade() else {
                        return;
                    };
                    match result {
                        Ok(outcome) => {
                            ui.set_results(
                                gui_state::format_roi_statistics(&outcome.stats).into(),
                            );
                            let mut message = "ROI statistics computed.".to_string();
                            match outcome.image {
                                Ok(image) => {
                                    shared.borrow_mut().spectrum_chart = Some(outcome.chart);
                                    ui.set_spectrum_chart(image.to_slint_image());
                                    ui.set_spectrum_chart_ready(true);
                                }
                                Err(e) => {
                                    message.push_str(&format!(
                                        " The in-window chart could not be drawn: {e}"
                                    ));
                                }
                            }
                            report(&ui, Ok(message));
                        }
                        Err(e) => {
                            ui.set_results("Analysis failed. See the message below.".into());
                            report(&ui, Err(e));
                        }
                    }
                },
            );
            return;
        }
        report(&ui, Err("Open a list-mode or MPA measurement first".into()));
    });

    let weak = ui.as_weak();
    let shared = state.clone();
    ui.on_go_home(move || {
        if let Some(ui) = weak.upgrade() {
            if ui.get_busy() {
                return;
            }
            go_home(&ui, &shared);
        }
    });

    let weak = ui.as_weak();
    ui.on_pick_convert(move || {
        if let Some(ui) = weak.upgrade() {
            ui.set_screen(SCREEN_CONVERT);
        }
    });

    let weak = ui.as_weak();
    ui.on_pick_analyze(move || {
        if let Some(ui) = weak.upgrade() {
            ui.set_screen(SCREEN_ANALYZE);
        }
    });

    install_rate_chart(&ui, &state);
    install_violin_chart(&ui, &state);
    install_spectrum_chart(&ui, &state);

    let weak = ui.as_weak();
    ui.on_open_directory(move || {
        if let (Some(ui), Some(dir)) = (weak.upgrade(), state.borrow().result_directory.as_ref()) {
            #[cfg(target_os = "windows")]
            match std::process::Command::new("explorer.exe").arg(dir).spawn() {
                Ok(_) => {}
                Err(e) => report(&ui, Err(format!("Cannot open output folder: {e}"))),
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = dir;
                report(
                    &ui,
                    Err("Opening output folders is implemented for Windows only.".into()),
                );
            }
        }
    });
    install_file_drop(&ui, load);
    ui.run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrolling_moves_drop_target_and_reaches_convert_in_short_window() {
        slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
            i_slint_backend_testing::TestingBackendOptions {
                mock_time: true,
                renderer_name: Some("software".into()),
                ..Default::default()
            },
        )))
        .unwrap();
        let ui = MainWindow::new().unwrap();
        ui.set_screen(SCREEN_CONVERT);
        ui.window().set_size(slint::LogicalSize::new(760., 400.));
        ui.show().unwrap();
        ui.set_loaded(true);
        let clicked = Rc::new(std::cell::Cell::new(false));
        let result = clicked.clone();
        ui.on_convert(move || result.set(true));
        assert!(ui.invoke_input_contains(100., 130.));
        assert!(!ui.invoke_input_contains(0., 130.));
        assert!(!ui.invoke_input_contains(100., 399.));
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                position: slint::LogicalPosition::new(10., 200.),
                delta_x: 0.,
                delta_y: -5000.,
            });
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(500));
        assert!(
            !ui.invoke_input_contains(100., 130.),
            "drop target must follow the scrolled input panel"
        );
        let button =
            i_slint_backend_testing::ElementHandle::find_by_element_type_name(&ui, "Button")
                .last()
                .expect("footer Convert button");
        let position = button.absolute_position();
        let size = button.size();
        assert!(
            position.y >= 0. && position.y + size.height <= 400.,
            "Convert button must fit in viewport: {position:?} {size:?}"
        );
        button.mock_single_click(slint::platform::PointerEventButton::Left);
        assert!(
            clicked.get(),
            "Convert must be reachable after scrolling to the bottom"
        );
    }

    #[test]
    fn list_mode_panel_switches_and_edits_rows() {
        use i_slint_backend_testing::ElementHandle;
        // The Slint platform is per thread; each test sets its own.
        slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
            i_slint_backend_testing::TestingBackendOptions {
                mock_time: true,
                renderer_name: Some("software".into()),
                ..Default::default()
            },
        )))
        .unwrap();
        let ui = MainWindow::new().unwrap();
        ui.set_screen(SCREEN_ANALYZE);
        ui.window().set_size(slint::LogicalSize::new(760., 820.));
        ui.show().unwrap();
        let rois = Rc::new(VecModel::from(vec![blank_roi()]));
        ui.set_rois(rois.clone().into());
        ui.set_roi_units(strings(["keV".into(), "Channel".into()]));
        let model = rois.clone();
        ui.on_add_roi(move || model.push(blank_roi()));
        let model = rois.clone();
        ui.on_remove_roi(move |index| {
            model.remove(index as usize);
        });
        let analyzed = Rc::new(std::cell::Cell::new(false));
        let result = analyzed.clone();
        ui.on_analyze(move || result.set(true));
        ui.on_convert(|| panic!("list mode must not convert"));
        let labelled = |label: &str| ElementHandle::find_by_accessible_label(&ui, label).count();
        let placeholder = |text: &str| {
            ElementHandle::find_by_element_type_name(&ui, "LineEdit")
                .filter(|e| e.accessible_placeholder_text().as_deref() == Some(text))
                .collect::<Vec<_>>()
        };

        assert_eq!(labelled("Add ROI"), 0, "converter panel by default");
        assert_eq!(placeholder("Final channel").len(), 1);
        ui.set_list_mode(true);
        assert_eq!(labelled("Add ROI"), 1, "list-mode panel shown");
        assert!(placeholder("Final channel").is_empty(), "converter hidden");

        ElementHandle::find_by_accessible_label(&ui, "Add ROI")
            .next()
            .unwrap()
            .invoke_accessible_default_action();
        assert_eq!(rois.row_count(), 2);
        assert_eq!(placeholder("Name").len(), 2);
        // Typing into a row writes through to the Rust-owned model.
        placeholder("Name")[0].set_accessible_value("peak");
        placeholder("High")[1].set_accessible_value("35");
        assert_eq!(rois.row_data(0).unwrap().name, "peak");
        assert_eq!(rois.row_data(1).unwrap().high, "35");

        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(100));
        let remove = ElementHandle::find_by_accessible_label(&ui, "Remove")
            .last()
            .unwrap();
        let right = remove.absolute_position().x + remove.size().width;
        assert!(right <= 760., "ROI row must fit the minimum width: {right}");
        remove.invoke_accessible_default_action();
        assert_eq!(rois.row_count(), 1);
        assert_eq!(rois.row_data(0).unwrap().name, "peak");
        assert_eq!(placeholder("Name").len(), 1);

        ui.set_loaded(true);
        // Element search skips items scrolled out of view; scroll to the footer.
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                position: slint::LogicalPosition::new(10., 200.),
                delta_x: 0.,
                delta_y: -5000.,
            });
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(500));
        ElementHandle::find_by_accessible_label(&ui, "Run list-mode analysis")
            .next()
            .expect("footer button relabelled in list mode")
            .invoke_accessible_default_action();
        assert!(
            analyzed.get(),
            "footer button runs the analysis in list mode"
        );

        ui.set_screen(SCREEN_CONVERT);
        ui.set_list_mode(false);
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(100));
        assert_eq!(labelled("Run list-mode analysis"), 0);
        assert_eq!(
            labelled("Convert measurement"),
            1,
            "converter footer restored"
        );
    }

    #[test]
    fn home_screen_cards_then_convert_after_picking_a_card() {
        use i_slint_backend_testing::ElementHandle;
        slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
            i_slint_backend_testing::TestingBackendOptions {
                mock_time: true,
                renderer_name: Some("software".into()),
                ..Default::default()
            },
        )))
        .unwrap();
        let ui = MainWindow::new().unwrap();
        ui.window().set_size(slint::LogicalSize::new(760., 400.));
        ui.show().unwrap();

        assert_eq!(ui.get_screen(), SCREEN_HOME, "home screen by default");
        assert_eq!(
            ElementHandle::find_by_accessible_label(&ui, "Analyze").count(),
            1,
            "Analyze card present before any file loads"
        );
        assert_eq!(
            ElementHandle::find_by_accessible_label(&ui, "Convert").count(),
            1,
            "Convert card present before any file loads"
        );
        assert_eq!(
            ElementHandle::find_by_accessible_label(&ui, "Convert measurement").count(),
            0,
            "converter footer is not reachable from the home screen"
        );

        let weak = ui.as_weak();
        ui.on_pick_convert(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_screen(SCREEN_CONVERT);
            }
        });
        ElementHandle::find_by_accessible_label(&ui, "Convert")
            .next()
            .expect("Convert card")
            .invoke_accessible_default_action();
        assert_eq!(
            ui.get_screen(),
            SCREEN_CONVERT,
            "picking the Convert card switches screens"
        );
        assert_eq!(
            ElementHandle::find_by_accessible_label(&ui, "Analyze").count(),
            0,
            "home cards are gone once a screen is picked"
        );

        // Loading a .mpa file on the Convert screen must still reach the
        // existing Convert button, exactly like
        // scrolling_moves_drop_target_and_reaches_convert_in_short_window.
        ui.set_loaded(true);
        let clicked = Rc::new(std::cell::Cell::new(false));
        let result = clicked.clone();
        ui.on_convert(move || result.set(true));
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                position: slint::LogicalPosition::new(10., 200.),
                delta_x: 0.,
                delta_y: -5000.,
            });
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(500));
        let button = ElementHandle::find_by_element_type_name(&ui, "Button")
            .last()
            .expect("footer Convert button");
        let position = button.absolute_position();
        let size = button.size();
        assert!(
            position.y >= 0. && position.y + size.height <= 400.,
            "Convert button must fit in viewport: {position:?} {size:?}"
        );
        button.mock_single_click(slint::platform::PointerEventButton::Left);
        assert!(
            clicked.get(),
            "Convert must be reachable after picking the Convert card and loading an .mpa file"
        );
    }

    #[test]
    fn mpa_analyze_computes_roi_statistics_and_renders_chart() {
        use i_slint_backend_testing::ElementHandle;
        slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
            i_slint_backend_testing::TestingBackendOptions {
                mock_time: true,
                renderer_name: Some("software".into()),
                ..Default::default()
            },
        )))
        .unwrap();
        let ui = MainWindow::new().unwrap();
        ui.set_screen(SCREEN_ANALYZE);
        ui.window().set_size(slint::LogicalSize::new(760., 820.));
        ui.show().unwrap();

        // Synthetic `.mpa` fixture: one linear ADC (CALOFF=0, CALFACT=1)
        // with a small peak, the same channel layout Ticket 4's
        // `roi_statistics_flat_background_net_approx_gross` test uses.
        let file = Arc::new(
            moulinette::mpa::parse(
                b"[ADC1]\ncaloff=0\ncalfact=1\n[DATA0,9]\n0\n0\n0\n10\n20\n10\n0\n0\n0\n",
            )
            .unwrap(),
        );

        let rois = Rc::new(VecModel::from(vec![blank_roi()]));
        ui.set_rois(rois.clone().into());
        ui.set_roi_units(strings(["keV".into(), "Channel".into()]));
        ui.set_list_mode(false);
        ui.set_loaded(true);

        let model = rois.clone();
        ui.on_add_roi(move || model.push(blank_roi()));

        let weak = ui.as_weak();
        let shared_file = file.clone();
        ui.on_analyze(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let rois = mpa_roi_parameters(&ui).expect("valid ROIs");
            let analysis = gui_state::mpa_analyze(&shared_file, ui.get_spectrum(), &rois)
                .expect("analysis succeeds");
            ui.set_results(gui_state::format_roi_statistics(&analysis.stats).into());
            let mut chart = analysis.chart;
            let image = chart.render().expect("chart renders");
            ui.set_spectrum_chart(image.to_slint_image());
            ui.set_spectrum_chart_ready(true);
        });

        let placeholder = |text: &str| {
            ElementHandle::find_by_element_type_name(&ui, "LineEdit")
                .filter(|e| e.accessible_placeholder_text().as_deref() == Some(text))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            placeholder("Name").len(),
            1,
            "mpa ROI editor shown once a .mpa file is loaded on Analyze"
        );
        assert!(
            ElementHandle::find_by_accessible_label(&ui, "Add ROI")
                .next()
                .is_some(),
            "ROI editor reuses the shared Add ROI row widget"
        );

        // Add a second row through the widget, then configure the first row
        // as a keV-unit ROI over the peak: with CALOFF=0/CALFACT=1, low=3
        // and high=7 resolve to the 0-based channel range [2, 6].
        ElementHandle::find_by_accessible_label(&ui, "Add ROI")
            .next()
            .unwrap()
            .invoke_accessible_default_action();
        assert_eq!(rois.row_count(), 2);
        placeholder("Name")[0].set_accessible_value("peak");
        placeholder("Low")[0].set_accessible_value("3");
        placeholder("High")[0].set_accessible_value("7");
        rois.remove(1);

        // Drive the footer action exactly like
        // `list_mode_panel_switches_and_edits_rows` drives "Run list-mode
        // analysis", confirming the button is relabelled for this third
        // (`.mpa` on Analyze) case.
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                position: slint::LogicalPosition::new(10., 200.),
                delta_x: 0.,
                delta_y: -5000.,
            });
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(500));
        ElementHandle::find_by_accessible_label(&ui, "Compute ROI statistics")
            .next()
            .expect("footer button relabelled for .mpa Analyze")
            .invoke_accessible_default_action();

        // Ticket 4's own computation over the same ROI is the ground truth.
        let expected_rois = [gui_state::RoiSpec {
            name: "peak".into(),
            adc: 1,
            low: 3.0,
            high: 7.0,
            unit: gui_state::RoiUnit::Kev,
        }];
        let expected_stats = gui_state::roi_statistics(&file.spectra[0], &expected_rois).unwrap();
        assert_eq!(expected_stats[0].gross, 40.0);
        assert_eq!(expected_stats[0].net, 40.0);
        assert_eq!(expected_stats[0].centroid, 4.0);
        assert_eq!(
            ui.get_results().to_string(),
            gui_state::format_roi_statistics(&expected_stats),
            "stats table reflects Ticket 4's computed values"
        );
    }
}
