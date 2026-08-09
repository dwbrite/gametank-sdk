use ratatui::{
    Frame,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    widgets::{Block, Borders},
};

use crate::{
    action::ComponentAction,
    component::Component,
    components::{ControlDeck, pattern_editor::PatternEditor},
    file::{NUM_INSTRUMENTS, TrackerFile, TuningData},
    scheme::SCHEME,
    tracker::PATTERN_TABLE_WIDTH,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppFocus {
    ControlDeck,
    PatternEditor,
}

impl AppFocus {
    fn next(self) -> Self {
        match self {
            AppFocus::ControlDeck => AppFocus::PatternEditor,
            AppFocus::PatternEditor => AppFocus::ControlDeck,
        }
    }
}

pub struct TrackerContainer {
    active_focus: AppFocus,
    control_deck: ControlDeck,
    pattern_editor: PatternEditor,
}

impl TrackerContainer {
    pub fn init(file: &TrackerFile) -> Self {
        let mut control_deck = ControlDeck::init();
        control_deck.set_names(&file.instrument_names());
        control_deck.set_sample_rate(file.sample_rate);
        let mut pattern_editor = PatternEditor::init();
        pattern_editor.set_tuning(file.tuning.clone());
        let mut tc = TrackerContainer {
            control_deck,
            pattern_editor,
            active_focus: AppFocus::ControlDeck,
        };
        tc.update_tuning_context(&file.tuning);
        tc
    }

    pub fn get_instrument_names(&self) -> [String; NUM_INSTRUMENTS] {
        self.control_deck.get_names()
    }

    pub fn get_bpm(&self) -> f64 {
        self.control_deck.get_bpm()
    }

    pub fn pattern_idx(&self) -> u8 {
        self.pattern_editor.pattern_idx
    }

    pub fn set_playing(&mut self, playing: bool) {
        self.control_deck.playing = playing;
        self.pattern_editor.playing = playing;
    }

    pub fn mark_export_success(&mut self) {
        self.control_deck.mark_export_success();
    }

    pub fn current_row(&self) -> u8 {
        self.pattern_editor.sel_y
    }

    pub fn set_current_row(&mut self, row: u8) {
        self.pattern_editor.sel_y = row;
    }

    pub fn apply_tuning(&mut self, tuning: &TuningData) {
        self.pattern_editor.set_tuning(tuning.clone());
        self.update_tuning_context(tuning);
    }

    fn update_tuning_context(&mut self, tuning: &TuningData) {
        let scale_size = tuning.scale.len();
        let note_count = tuning.notes.len();
        let note_keys: Vec<&String> = tuning.notes.keys().collect();
        let assigned_indices = tuning
            .key_assignments
            .keys()
            .filter_map(|note| note_keys.iter().position(|k| *k == note))
            .collect();
        self.control_deck
            .set_tuning_context(scale_size, note_count, assigned_indices);
    }

    fn bubble_actions(&mut self, actions: Vec<ComponentAction>) -> Vec<ComponentAction> {
        let mut bubbled = Vec::new();
        for action in actions {
            match action {
                ComponentAction::RequestFocus => {
                    self.active_focus = AppFocus::ControlDeck;
                }
                other => bubbled.push(other),
            }
        }
        bubbled
    }
}

impl Component for TrackerContainer {
    fn update(&mut self, events: Vec<Event>, file: &mut TrackerFile) -> Vec<ComponentAction> {
        let mut actions_out = Vec::new();
        let mut keyboard_pass: Vec<Event> = Vec::with_capacity(events.len());
        let mut mouse_pass: Vec<Event> = Vec::new();

        for e in events {
            match &e {
                Event::Key(KeyEvent {
                    code: KeyCode::Tab,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if self.active_focus == AppFocus::ControlDeck {
                        self.control_deck.blur_all();
                    }
                    self.active_focus = self.active_focus.next();
                }
                Event::Mouse(_) => mouse_pass.push(e),
                _ => keyboard_pass.push(e),
            }
        }

        if !mouse_pass.is_empty() {
            let mut pe_focus = false;
            for e in &mouse_pass {
                if self.pattern_editor.on_mouse_event(e) {
                    pe_focus = true;
                }
            }
            let actions = self.control_deck.update(mouse_pass, file);
            let got_focus = actions
                .iter()
                .any(|a| matches!(a, ComponentAction::RequestFocus));
            if got_focus {
                self.active_focus = AppFocus::ControlDeck;
            } else if pe_focus {
                self.active_focus = AppFocus::PatternEditor;
            }
            actions_out.extend(self.bubble_actions(actions));
        }

        match self.active_focus {
            AppFocus::ControlDeck => {
                let actions = self.control_deck.update(keyboard_pass, file);
                actions_out.extend(self.bubble_actions(actions));
            }
            AppFocus::PatternEditor => {
                self.pattern_editor.update(keyboard_pass, file);
            }
        }

        self.pattern_editor
            .set_transpose(self.control_deck.get_transpose());

        actions_out
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, file: &TrackerFile) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(1),
                Constraint::Length(7),
                Constraint::Fill(1),
            ])
            .split(area);

        let header_area = layout[0];
        let deck_area = layout[1];
        let editor_area = layout[2];

        let header = Block::new()
            .bg(SCHEME.true_dark_color(SCHEME.black[3]))
            .borders(Borders::TOP)
            .title(" Gametank GO! | TRACKER ")
            .title_alignment(Alignment::Center)
            .italic()
            .fg(SCHEME.orange[3]);

        let background = Block::new().bg(SCHEME.true_dark_color(SCHEME.black[0]));

        frame.render_widget(header, header_area);
        self.control_deck.render(frame, deck_area, file);
        frame.render_widget(background, editor_area);
        self.pattern_editor.render(frame, editor_area, file);

        let focused_area = match self.active_focus {
            AppFocus::ControlDeck => deck_area,
            AppFocus::PatternEditor => editor_area,
        };
        render_focus_indicator(frame, focused_area);

        self.control_deck.render_popup(frame, area);
    }
}

fn render_focus_indicator(frame: &mut Frame, area: Rect) {
    let style = Style::new().fg(SCHEME.red[3]);
    let left_x = area.x.saturating_sub(1) + (area.width.saturating_sub(PATTERN_TABLE_WIDTH)) / 2;
    let right_x = left_x + PATTERN_TABLE_WIDTH.min(area.width);
    let buf = frame.buffer_mut();
    for y in area.y..area.bottom() {
        buf.set_string(left_x, y, "│", style);
        if right_x > left_x {
            buf.set_string(right_x, y, "│", style);
        }
    }
}
