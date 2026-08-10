use std::{path::PathBuf, thread::sleep, time::Duration};

use ratatui::{
    DefaultTerminal,
    crossterm::{
        event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
        execute,
    },
    layout::Rect,
};

use crate::{
    action::ComponentAction,
    component::Component,
    components::{
        CommandPalette, ExportConfirmModal, FileBrowser, InstrumentEditor, QuitConfirmModal,
        TrackerContainer, TuningEditor,
    },
    config::{GttConfig, build_key_assignments, config_to_tuning_keys},
    export,
    file::{NUM_INSTRUMENTS, TrackerFile},
    player::Player,
};

pub struct App {
    input_path: PathBuf,
    terminal: DefaultTerminal,
    tracker_container: TrackerContainer,
    file_data: TrackerFile,
    config: GttConfig,
    command_palette: CommandPalette,
    instrument_editor: InstrumentEditor,
    tuning_editor: TuningEditor,
    file_browser: FileBrowser,
    quit_confirm: QuitConfirmModal,
    export_confirm: ExportConfirmModal,
    player: Option<Player>,
}

impl App {
    pub fn new(input_path: Option<PathBuf>) -> color_eyre::Result<Self> {
        let no_file = input_path.is_none();
        let input_path = input_path.unwrap_or_else(|| PathBuf::from("track.gtt"));
        let mut file_data = if input_path.exists() {
            TrackerFile::load(&input_path).map_err(|e| color_eyre::eyre::eyre!("{}", e))?
        } else {
            TrackerFile::empty()
        };
        let config: GttConfig = confy::load("gtt", None).unwrap_or_default();
        file_data.tuning.key_assignments = config_to_tuning_keys(&config.bindings.key_assignments);
        let terminal = ratatui::init();
        execute!(std::io::stdout(), event::EnableMouseCapture)?;
        let tracker_container = TrackerContainer::init(&file_data);
        let player = Player::new(120.0, file_data.sample_rate);
        let mut file_browser = FileBrowser::new();
        if no_file {
            file_browser.visible = true;
            file_browser.open_file_explorer();
        }
        Ok(Self {
            input_path,
            terminal,
            tracker_container,
            file_data,
            config,
            command_palette: CommandPalette::init(),
            instrument_editor: InstrumentEditor::init(),
            tuning_editor: TuningEditor::init(),
            file_browser,
            quit_confirm: QuitConfirmModal::init(),
            export_confirm: ExportConfirmModal::init(),
            player,
        })
    }

    fn write_file(&mut self) -> std::io::Result<()> {
        let names = self.tracker_container.get_instrument_names();
        for (i, name) in names.iter().enumerate() {
            self.file_data.instruments[i].name = name.clone();
        }
        self.file_data.save(&self.input_path)
    }

    fn export_dir(&self) -> PathBuf {
        let stem = self
            .input_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "tracker".to_string());
        let parent = self
            .input_path
            .parent()
            .unwrap_or(std::path::Path::new("."));
        parent.join(format!("{}-export", stem))
    }

    fn run_export(&self, export_dir: &PathBuf) -> std::io::Result<()> {
        let bpm = self.tracker_container.get_bpm();
        let stem = self
            .input_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "track".to_string());
        export::export_all(&self.file_data, bpm, &stem, export_dir)
    }

    pub fn run(mut self) -> std::io::Result<()> {
        let _ = poll_events();
        let result = self.event_loop();
        execute!(std::io::stdout(), event::DisableMouseCapture).ok();
        result
    }

    fn event_loop(&mut self) -> std::io::Result<()> {
        loop {
            sleep(Duration::from_millis(16));

            if let Some(player) = &self.player
                && player.is_playing()
            {
                let row = player.current_row() as u8;
                self.tracker_container.set_current_row(row);
            }

            let events = poll_events();

            let actions = if self.quit_confirm.visible {
                self.quit_confirm.update(events, &mut self.file_data)
            } else if self.export_confirm.visible {
                self.export_confirm.update(events, &mut self.file_data)
            } else if self.instrument_editor.visible {
                self.instrument_editor.update(events, &mut self.file_data)
            } else if self.tuning_editor.visible {
                self.tuning_editor.update(events, &mut self.file_data)
            } else if self.file_browser.visible {
                self.file_browser.update(events, &mut self.file_data)
            } else if self.command_palette.visible {
                self.command_palette.update(events, &mut self.file_data)
            } else {
                let mut tc_events = Vec::with_capacity(events.len());
                for e in events {
                    if matches!(
                        &e,
                        Event::Key(KeyEvent {
                            code: KeyCode::Char(':'),
                            kind: KeyEventKind::Press,
                            modifiers: KeyModifiers::NONE,
                            ..
                        })
                    ) {
                        self.command_palette.visible = true;
                    } else {
                        tc_events.push(e);
                    }
                }
                self.tracker_container
                    .update(tc_events, &mut self.file_data)
            };

            if self.handle_actions(actions)? {
                return Ok(());
            }

            let tc = &mut self.tracker_container;
            let cp = &mut self.command_palette;
            let ie = &mut self.instrument_editor;
            let te = &mut self.tuning_editor;
            let fb = &mut self.file_browser;
            let qc = &mut self.quit_confirm;
            let ec = &mut self.export_confirm;
            let file = &self.file_data;
            self.terminal.draw(|f| {
                let area = f.area();
                tc.render(f, area, file);
                if cp.visible {
                    cp.render(
                        f,
                        Rect {
                            y: area.bottom().saturating_sub(1),
                            height: 1,
                            ..area
                        },
                        file,
                    );
                }
                if ie.visible {
                    ie.render(f, area, file);
                }
                if te.visible {
                    te.render(f, area, file);
                }
                if fb.visible {
                    fb.render(f, area, file);
                }
                if qc.visible {
                    qc.render(f, area, file);
                }
                if ec.visible {
                    ec.render(f, area, file);
                }
            })?;
        }
    }

    fn handle_actions(&mut self, actions: Vec<ComponentAction>) -> std::io::Result<bool> {
        for action in actions {
            match action {
                ComponentAction::Quit => return Ok(true),
                ComponentAction::SaveAndQuit => {
                    self.write_file()?;
                    return Ok(true);
                }
                ComponentAction::SaveFile => {
                    self.write_file()?;
                }
                ComponentAction::Export => {
                    let export_dir = self.export_dir();
                    if export_dir.exists() {
                        self.export_confirm.open(export_dir);
                    } else {
                        let export_dir = export_dir.clone();
                        if let Err(_) = self.run_export(&export_dir) {
                            self.export_confirm.open_error(export_dir);
                        } else {
                            self.tracker_container.mark_export_success();
                        }
                    }
                }
                ComponentAction::ConfirmExport(path) => {
                    let _ = std::fs::remove_dir_all(&path);
                    match self.run_export(&path) {
                        Ok(()) => {
                            self.export_confirm.visible = false;
                            self.tracker_container.mark_export_success();
                        }
                        Err(_) => {
                            self.export_confirm.set_error();
                        }
                    }
                }
                ComponentAction::Play => {
                    if let Some(player) = &self.player {
                        if player.is_playing() {
                            player.pause();
                            self.tracker_container.set_playing(false);
                        } else {
                            let row = self.tracker_container.current_row() as usize;
                            let pattern = self
                                .file_data
                                .current_pattern(self.tracker_container.pattern_idx())
                                .clone();
                            player.update_pattern(pattern);
                            for i in 0..NUM_INSTRUMENTS.min(8) {
                                player.update_waveform(i, self.file_data.instrument_waveform(i));
                            }
                            player.update_tuning_notes(self.file_data.tuning.notes.clone());
                            player.set_bpm(self.tracker_container.get_bpm());
                            player.play(row);
                            self.tracker_container.set_playing(true);
                        }
                    }
                }
                ComponentAction::OpenInstrumentEditor(idx) => {
                    let name = self.tracker_container.get_instrument_names()[idx].clone();
                    let waveform = self.file_data.instrument_waveform(idx);
                    self.instrument_editor.open(idx, &name, &waveform);
                }
                ComponentAction::OpenTuningEditor => {
                    self.tuning_editor.open(&self.file_data.tuning);
                }
                ComponentAction::OpenFileBrowser => {
                    self.file_browser.visible = true;
                    if self.file_browser.file_explorer.is_none() {
                        self.file_browser.open_file_explorer();
                    }
                }
                ComponentAction::OpenFile(path) => match TrackerFile::load(&path) {
                    Ok(file_data) => {
                        self.file_data = file_data;
                        self.input_path = path;
                        self.tracker_container = TrackerContainer::init(&self.file_data);
                        if let Some(player) = &mut self.player {
                            player.set_sample_rate(self.file_data.sample_rate);
                            player.pause();
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to load file: {}", e);
                    }
                },
                ComponentAction::CreateNewFile(path) => {
                    self.file_data = TrackerFile::empty();
                    self.input_path = path;
                    self.tracker_container = TrackerContainer::init(&self.file_data);
                    if let Some(player) = &mut self.player {
                        player.set_sample_rate(self.file_data.sample_rate);
                        player.pause();
                    }
                }
                ComponentAction::OpenQuitConfirm => {
                    self.quit_confirm.open();
                }
                ComponentAction::InstrumentSaved(idx, waveform) => {
                    self.file_data.instruments[idx].waveform = waveform.to_vec();
                }
                ComponentAction::TuningSaved(tuning) => {
                    self.file_data.tuning = tuning.clone();
                    self.tracker_container.apply_tuning(&tuning);
                    self.config.bindings.key_assignments =
                        build_key_assignments(&tuning.key_assignments);
                    let _ = confy::store("gtt", None, &self.config);
                }
                ComponentAction::SetSampleRate(rate) => {
                    self.file_data.sample_rate = rate;
                    if let Some(player) = &self.player {
                        player.set_sample_rate(rate);
                    }
                }
                ComponentAction::RequestFocus => {}
            }
        }
        Ok(false)
    }
}

fn poll_events() -> Vec<Event> {
    let mut events = vec![];
    while let Ok(true) = event::poll(Duration::from_millis(0)) {
        if let Ok(e) = event::read() {
            events.push(e);
        }
    }
    events
}
