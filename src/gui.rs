#![deny(unsafe_code)]
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use moulinette::{gui_state, lst::ListFile, mpa::MpaFile, workflow};
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

/// Validate the list-mode panel's current rows and fields.
fn list_parameters(ui: &MainWindow) -> Result<gui_state::ListParameters, String> {
    let rois: Vec<RoiRow> = ui.get_rois().iter().collect();
    let windows: Vec<TimeWindowRow> = ui.get_time_windows().iter().collect();
    gui_state::list_mode_parameters(
        &rois
            .iter()
            .map(|r| gui_state::RoiFields {
                name: r.name.as_str(),
                adc: r.adc.as_str(),
                low: r.low.as_str(),
                high: r.high.as_str(),
                unit: r.unit,
            })
            .collect::<Vec<_>>(),
        ui.get_bin_width().as_str(),
        &windows
            .iter()
            .map(|w| (w.start_s.as_str(), w.end_s.as_str()))
            .collect::<Vec<_>>(),
        ui.get_output_path().as_str(),
    )
}

fn joined(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join("\n")
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
        ui.set_list_mode(list_mode);
        ui.set_drop_hint("".into());
        ui.set_busy(true);
        ui.set_loaded(false);
        ui.set_has_results(false);
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
                                ui.set_results("No conversion yet.".into());
                                state.file = Some(Arc::new(file));
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
        std::thread::spawn(move || {
            let result = workflow::render(&file, format, selection, &indices, paths)
                .and_then(|outputs| workflow::write(outputs, Some(&dir)));
            let _ = sender.send((dir, result));
        });
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
        let Some(file) = shared.borrow().list.clone() else {
            report(&ui, Err("Open a list-mode file first".into()));
            return;
        };
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
        let weak = ui.as_weak();
        let shared = shared.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = gui_state::list_mode_run(&file, &params);
            let _ = sender.send((params.output_dir, result));
        });
        let timer = timer_analyze.clone();
        let timer_callback = Rc::downgrade(&timer);
        timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(50),
            move || {
                let (dir, result) = match receiver.try_recv() {
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
                match result {
                    Ok(paths) => {
                        ui.set_results(joined(&paths).into());
                        ui.set_has_results(true);
                        shared.borrow_mut().result_directory = Some(dir);
                        report(
                            &ui,
                            Ok(format!(
                                "Analysis complete · {} files generated.",
                                paths.len()
                            )),
                        );
                    }
                    Err(e) => {
                        ui.set_results("Analysis failed. See the message below.".into());
                        report(&ui, Err(e));
                    }
                }
            },
        );
    });

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

        ui.set_list_mode(false);
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(100));
        assert_eq!(labelled("Run list-mode analysis"), 0);
        assert_eq!(
            labelled("Convert measurement"),
            1,
            "converter footer restored"
        );
    }
}
