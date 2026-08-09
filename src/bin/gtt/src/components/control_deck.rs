use std::time::{Duration, Instant};

use rat_widget::button::{Button, ButtonState};
use rat_widget::choice::{Choice, ChoiceState};
use rat_widget::text::HasScreenCursor;
use rat_widget::text_input::{TextInput, TextInputState, handle_events};
use ratatui::{
    Frame,
    crossterm::event::{
        Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind,
    },
    layout::{Constraint, Layout, Position, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Paragraph},
};

use crate::{
    action::ComponentAction,
    component::Component,
    file::{NUM_INSTRUMENTS, TrackerFile},
    scheme::SCHEME,
    tracker::PATTERN_TABLE_WIDTH,
};

const BTN_W: u16 = 3;

struct InstrumentEntry {
    open_button: ButtonState,
    name_input: TextInputState,
    name_snapshot: String,
}

impl InstrumentEntry {
    fn new(name: &str) -> Self {
        let mut name_input = TextInputState::new();
        name_input.set_value(name);
        Self {
            open_button: ButtonState::new(),
            name_input,
            name_snapshot: name.to_string(),
        }
    }
}

const SAMPLE_RATE_OPTIONS: [u8; 5] = [0xFF, 0xEF, 0xD0, 0xB7, 0xA8];
const SAMPLE_RATE_LABELS: [&str; 5] = ["14kHz", "16kHz", "22kHz", "32kHz", "44kHz"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Row {
    Bpm,
    Beats,
    Trans,
    SampleRate,
    ActionPlay,
    ActionNewOpen,
    Instrument(usize),
    ActionTuning,
    ActionQuit,
    ActionSave,
    ActionExport,
}

impl Row {
    fn is_setting(self) -> bool {
        matches!(self, Row::Bpm | Row::Beats | Row::Trans)
    }

    fn is_action(self) -> bool {
        matches!(
            self,
            Row::ActionPlay
                | Row::ActionNewOpen
                | Row::ActionTuning
                | Row::ActionQuit
                | Row::ActionSave
                | Row::ActionExport
        )
    }

    fn label(self) -> &'static str {
        match self {
            Row::Bpm => "BPM:  ",
            Row::Beats => "Beats: ",
            Row::Trans => "Trans: ",
            Row::SampleRate => "Rate:  ",
            Row::Instrument(_)
            | Row::ActionPlay
            | Row::ActionNewOpen
            | Row::ActionTuning
            | Row::ActionQuit
            | Row::ActionSave
            | Row::ActionExport => "",
        }
    }

    fn range(self) -> (u16, u16) {
        match self {
            Row::Bpm => (1, 399),
            Row::Beats => (0, 255),
            Row::Trans
            | Row::SampleRate
            | Row::Instrument(_)
            | Row::ActionPlay
            | Row::ActionNewOpen
            | Row::ActionTuning
            | Row::ActionQuit
            | Row::ActionSave
            | Row::ActionExport => (0, 0),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Col {
    Input,
    Plus,
    Minus,
    Open,
}

impl Col {
    fn next(self, row: Row) -> Self {
        if row == Row::SampleRate {
            return Col::Input;
        }
        if row.is_setting() {
            match self {
                Col::Input => Col::Plus,
                Col::Plus => Col::Minus,
                _ => Col::Input,
            }
        } else if row.is_action() {
            Col::Open
        } else {
            match self {
                Col::Open => Col::Input,
                _ => Col::Open,
            }
        }
    }

    fn prev(self, row: Row) -> Self {
        if row == Row::SampleRate {
            return Col::Input;
        }
        if row.is_setting() {
            match self {
                Col::Minus => Col::Plus,
                Col::Plus => Col::Input,
                _ => Col::Minus,
            }
        } else if row.is_action() {
            Col::Open
        } else {
            match self {
                Col::Input => Col::Open,
                _ => Col::Input,
            }
        }
    }
}

fn default_col(row: Row) -> Col {
    match row {
        Row::Bpm | Row::Beats | Row::Trans | Row::SampleRate => Col::Input,
        Row::Instrument(_)
        | Row::ActionPlay
        | Row::ActionNewOpen
        | Row::ActionTuning
        | Row::ActionQuit
        | Row::ActionSave
        | Row::ActionExport => Col::Open,
    }
}

pub struct ControlDeck {
    action_export_btn: ButtonState,
    action_new_open_btn: ButtonState,
    action_play_btn: ButtonState,
    action_quit_btn: ButtonState,
    action_save_btn: ButtonState,
    action_tuning_btn: ButtonState,
    bpm_input: TextInputState,
    bpm_minus: ButtonState,
    bpm_plus: ButtonState,
    bpm_snapshot: String,
    editing: bool,
    instruments: [InstrumentEntry; NUM_INSTRUMENTS],
    pub playing: bool,
    rate_state: ChoiceState<usize>,
    sample_rate: u8,
    rows_input: TextInputState,
    rows_minus: ButtonState,
    rows_plus: ButtonState,
    rows_snapshot: String,
    save_feedback_until: Option<Instant>,
    export_feedback_until: Option<Instant>,
    selected_col: Col,
    selected_row: Row,
    trans_input: TextInputState,
    trans_minus: ButtonState,
    trans_plus: ButtonState,
    trans_snapshot: String,
    trans_assigned_indices: Vec<usize>,
    trans_note_count: usize,
    trans_scale_size: usize,
}

impl ControlDeck {
    pub fn init() -> Self {
        let mut bpm_input = TextInputState::new();
        bpm_input.set_value("120");
        let mut rows_input = TextInputState::new();
        rows_input.set_value("64");
        let mut trans_input = TextInputState::new();
        trans_input.set_value("0");
        Self {
            selected_row: Row::Bpm,
            selected_col: Col::Input,
            editing: false,
            bpm_input,
            bpm_snapshot: "120".to_string(),
            bpm_plus: ButtonState::new(),
            bpm_minus: ButtonState::new(),
            rows_input,
            rows_snapshot: "64".to_string(),
            rows_plus: ButtonState::new(),
            rows_minus: ButtonState::new(),
            trans_input,
            trans_snapshot: "0".to_string(),
            trans_plus: ButtonState::new(),
            trans_minus: ButtonState::new(),
            trans_assigned_indices: Vec::new(),
            trans_note_count: 0,
            trans_scale_size: 0,
            instruments: std::array::from_fn(|i| {
                InstrumentEntry::new(&format!("instrument_{}", i + 1))
            }),
            playing: false,
            action_tuning_btn: ButtonState::new(),
            action_new_open_btn: ButtonState::new(),
            action_play_btn: ButtonState::new(),
            action_quit_btn: ButtonState::new(),
            action_save_btn: ButtonState::new(),
            action_export_btn: ButtonState::new(),
            rate_state: {
                let mut s = ChoiceState::<usize>::new();
                s.set_value(2usize);
                s
            },
            sample_rate: 0xD0,
            save_feedback_until: None,
            export_feedback_until: None,
        }
    }

    pub fn get_names(&self) -> [String; NUM_INSTRUMENTS] {
        std::array::from_fn(|i| self.instruments[i].name_input.value::<String>())
    }

    pub fn set_names(&mut self, names: &[String; NUM_INSTRUMENTS]) {
        for i in 0..NUM_INSTRUMENTS {
            self.instruments[i].name_input.set_value(names[i].clone());
            self.instruments[i].name_snapshot = names[i].clone();
        }
    }

    pub fn get_sample_rate(&self) -> u8 {
        self.sample_rate
    }

    pub fn set_sample_rate(&mut self, rate: u8) {
        self.sample_rate = rate;
        let idx = SAMPLE_RATE_OPTIONS
            .iter()
            .position(|&r| r == rate)
            .unwrap_or(2);
        self.rate_state.set_value(idx);
    }

    fn sample_rate_index(&self) -> usize {
        self.rate_state.value()
    }

    pub fn blur_all(&mut self) {
        if self.editing {
            self.confirm_editing();
        }
        self.bpm_input.focus.set(false);
        self.bpm_plus.focus.set(false);
        self.bpm_minus.focus.set(false);
        self.rows_input.focus.set(false);
        self.rows_plus.focus.set(false);
        self.rows_minus.focus.set(false);
        self.trans_input.focus.set(false);
        self.trans_plus.focus.set(false);
        self.trans_minus.focus.set(false);
        self.rate_state.focus.set(false);
        for entry in &mut self.instruments {
            entry.open_button.focus.set(false);
            entry.name_input.focus.set(false);
        }
        self.action_tuning_btn.focus.set(false);
        self.action_new_open_btn.focus.set(false);
        self.action_play_btn.focus.set(false);
        self.action_quit_btn.focus.set(false);
        self.action_save_btn.focus.set(false);
        self.action_export_btn.focus.set(false);
    }

    pub fn get_bpm(&self) -> f64 {
        self.bpm_input
            .value::<String>()
            .parse::<f64>()
            .unwrap_or(120.0)
            .max(1.0)
    }

    pub fn mark_export_success(&mut self) {
        self.export_feedback_until = Some(Instant::now() + Duration::from_secs(3));
    }

    pub fn get_transpose(&self) -> i32 {
        self.trans_input
            .value::<String>()
            .parse::<i32>()
            .unwrap_or(0)
    }

    pub fn set_tuning_context(
        &mut self,
        scale_size: usize,
        note_count: usize,
        assigned_indices: Vec<usize>,
    ) {
        self.trans_scale_size = scale_size;
        self.trans_note_count = note_count;
        self.trans_assigned_indices = assigned_indices;
        let current = self.get_transpose();
        if !self.is_transpose_valid(current) {
            self.trans_input.set_value("0");
            self.trans_snapshot = "0".to_string();
        }
    }

    fn is_transpose_valid(&self, t: i32) -> bool {
        if self.trans_scale_size == 0 {
            return t == 0;
        }
        let shift = t * self.trans_scale_size as i32;
        self.trans_assigned_indices.iter().any(|&idx| {
            let shifted = idx as i32 + shift;
            shifted >= 0 && shifted < self.trans_note_count as i32
        })
    }

    fn update_focus_states(&mut self) {
        let row = self.selected_row;
        let col = self.selected_col;
        let editing = self.editing;
        self.bpm_input
            .focus
            .set(row == Row::Bpm && col == Col::Input && editing);
        self.bpm_plus.focus.set(row == Row::Bpm && col == Col::Plus);
        self.bpm_minus
            .focus
            .set(row == Row::Bpm && col == Col::Minus);
        self.rows_input
            .focus
            .set(row == Row::Beats && col == Col::Input && editing);
        self.rows_plus
            .focus
            .set(row == Row::Beats && col == Col::Plus);
        self.rows_minus
            .focus
            .set(row == Row::Beats && col == Col::Minus);
        self.trans_input
            .focus
            .set(row == Row::Trans && col == Col::Input && editing);
        self.trans_plus
            .focus
            .set(row == Row::Trans && col == Col::Plus);
        self.trans_minus
            .focus
            .set(row == Row::Trans && col == Col::Minus);
        self.rate_state.focus.set(row == Row::SampleRate);
        for i in 0..NUM_INSTRUMENTS {
            let is_row = row == Row::Instrument(i);
            self.instruments[i]
                .open_button
                .focus
                .set(is_row && col == Col::Open);
            self.instruments[i]
                .name_input
                .focus
                .set(is_row && col == Col::Input && editing);
        }
        self.action_tuning_btn.focus.set(row == Row::ActionTuning);
        self.action_new_open_btn
            .focus
            .set(row == Row::ActionNewOpen);
        self.action_play_btn.focus.set(row == Row::ActionPlay);
        self.action_quit_btn.focus.set(row == Row::ActionQuit);
        self.action_save_btn.focus.set(row == Row::ActionSave);
        self.action_export_btn.focus.set(row == Row::ActionExport);
    }

    fn current_input_mut(&mut self) -> &mut TextInputState {
        match self.selected_row {
            Row::Bpm => &mut self.bpm_input,
            Row::Beats => &mut self.rows_input,
            Row::Trans => &mut self.trans_input,
            Row::Instrument(i) => &mut self.instruments[i].name_input,
            Row::SampleRate
            | Row::ActionPlay
            | Row::ActionNewOpen
            | Row::ActionTuning
            | Row::ActionQuit
            | Row::ActionSave
            | Row::ActionExport => {
                unreachable!("action rows have no text input")
            }
        }
    }

    fn take_snapshot(&mut self) {
        match self.selected_row {
            Row::Bpm => self.bpm_snapshot = self.bpm_input.value::<String>(),
            Row::Beats => self.rows_snapshot = self.rows_input.value::<String>(),
            Row::Trans => self.trans_snapshot = self.trans_input.value::<String>(),
            Row::Instrument(i) => {
                let v = self.instruments[i].name_input.value::<String>();
                self.instruments[i].name_snapshot = v;
            }
            Row::ActionPlay
            | Row::ActionNewOpen
            | Row::ActionTuning
            | Row::ActionQuit
            | Row::ActionSave
            | Row::ActionExport
            | Row::SampleRate => {}
        }
    }

    fn restore_snapshot(&mut self) {
        match self.selected_row {
            Row::Bpm => {
                let s = self.bpm_snapshot.clone();
                self.bpm_input.set_value(s);
            }
            Row::Beats => {
                let s = self.rows_snapshot.clone();
                self.rows_input.set_value(s);
            }
            Row::Trans => {
                let s = self.trans_snapshot.clone();
                self.trans_input.set_value(s);
            }
            Row::Instrument(i) => {
                let s = self.instruments[i].name_snapshot.clone();
                self.instruments[i].name_input.set_value(s);
            }
            Row::ActionPlay
            | Row::ActionNewOpen
            | Row::ActionTuning
            | Row::ActionQuit
            | Row::ActionSave
            | Row::ActionExport
            | Row::SampleRate => {}
        }
        self.editing = false;
    }

    fn start_editing(&mut self) {
        if self.selected_row == Row::SampleRate
            || self.selected_row == Row::ActionPlay
            || self.selected_row == Row::ActionNewOpen
            || self.selected_row == Row::ActionTuning
            || self.selected_row == Row::ActionQuit
            || self.selected_row == Row::ActionSave
            || self.selected_row == Row::ActionExport
        {
            return;
        }
        self.take_snapshot();
        self.editing = true;
    }

    fn confirm_editing(&mut self) {
        match self.selected_row {
            Row::Bpm => {
                let (min, max) = Row::Bpm.range();
                let raw = self.bpm_input.value::<String>();
                let clamped = raw.parse::<u16>().map(|v| v.clamp(min, max)).unwrap_or(min);
                let s = clamped.to_string();
                self.bpm_input.set_value(s.clone());
                self.bpm_snapshot = s;
            }
            Row::Beats => {
                let (min, max) = Row::Beats.range();
                let raw = self.rows_input.value::<String>();
                let clamped = raw.parse::<u16>().map(|v| v.clamp(min, max)).unwrap_or(min);
                let s = clamped.to_string();
                self.rows_input.set_value(s.clone());
                self.rows_snapshot = s;
            }
            Row::Trans => {
                let raw = self.trans_input.value::<String>();
                let parsed = raw.parse::<i32>().unwrap_or(0);
                let valid_val = if self.is_transpose_valid(parsed) {
                    parsed
                } else {
                    self.trans_snapshot.parse::<i32>().unwrap_or(0)
                };
                let s = valid_val.to_string();
                self.trans_input.set_value(s.clone());
                self.trans_snapshot = s;
            }
            Row::Instrument(i) => {
                let val = self.instruments[i].name_input.value::<String>();
                self.instruments[i].name_snapshot = val;
            }
            Row::ActionPlay
            | Row::ActionNewOpen
            | Row::ActionTuning
            | Row::ActionQuit
            | Row::ActionSave
            | Row::ActionExport
            | Row::SampleRate => {}
        }
        self.editing = false;
    }

    fn get_setting_value(&self, row: Row) -> u16 {
        let (min, max) = row.range();
        let raw = match row {
            Row::Bpm => self.bpm_input.value::<String>(),
            Row::Beats => self.rows_input.value::<String>(),
            _ => return min,
        };
        raw.parse::<u16>().unwrap_or(min).clamp(min, max)
    }

    fn set_setting_value(&mut self, row: Row, value: u16) {
        let s = value.to_string();
        match row {
            Row::Bpm => {
                self.bpm_input.set_value(s.clone());
                self.bpm_snapshot = s;
            }
            Row::Beats => {
                self.rows_input.set_value(s.clone());
                self.rows_snapshot = s;
            }
            _ => {}
        }
    }

    fn increment(&mut self) {
        if self.selected_row == Row::Trans {
            let next = self.get_transpose() + 1;
            if self.is_transpose_valid(next) {
                let s = next.to_string();
                self.trans_input.set_value(s.clone());
                self.trans_snapshot = s;
            }
            return;
        }
        let row = self.selected_row;
        let (_, max) = row.range();
        let val = self.get_setting_value(row).saturating_add(1).min(max);
        self.set_setting_value(row, val);
    }

    fn decrement(&mut self) {
        if self.selected_row == Row::Trans {
            let next = self.get_transpose() - 1;
            if self.is_transpose_valid(next) {
                let s = next.to_string();
                self.trans_input.set_value(s.clone());
                self.trans_snapshot = s;
            }
            return;
        }
        let row = self.selected_row;
        let (min, _) = row.range();
        let val = self.get_setting_value(row).saturating_sub(1).max(min);
        self.set_setting_value(row, val);
    }

    fn handle_mouse_click(&mut self, pos: Position, actions: &mut Vec<ComponentAction>) -> bool {
        if self.rate_state.is_popup_active() {
            let popup_area = self.rate_state.popup.area;
            if popup_area.contains(pos) {
                let offset = self.rate_state.offset();
                for (i, &item_area) in self.rate_state.item_areas.iter().enumerate() {
                    if item_area.contains(pos) {
                        let idx = offset + i;
                        self.rate_state.set_value(idx);
                        self.rate_state.set_popup_active(false);
                        if idx < SAMPLE_RATE_OPTIONS.len() {
                            self.sample_rate = SAMPLE_RATE_OPTIONS[idx];
                            actions.push(ComponentAction::SetSampleRate(self.sample_rate));
                        }
                        return true;
                    }
                }
                return true;
            }
            self.rate_state.set_popup_active(false);
        }

        let rate_area = self.rate_state.area;
        if rate_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::SampleRate;
            self.selected_col = Col::Input;
            self.rate_state.flip_popup_active();
            self.update_focus_states();
            return true;
        }

        let bpm_plus_area = self.bpm_plus.area;
        let bpm_minus_area = self.bpm_minus.area;
        let rows_plus_area = self.rows_plus.area;
        let rows_minus_area = self.rows_minus.area;
        let bpm_input_area = self.bpm_input.area;
        let rows_input_area = self.rows_input.area;
        let trans_plus_area = self.trans_plus.area;
        let trans_minus_area = self.trans_minus.area;
        let trans_input_area = self.trans_input.area;

        if bpm_plus_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::Bpm;
            self.selected_col = Col::Plus;
            self.increment();
            self.update_focus_states();
            return true;
        }
        if bpm_minus_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::Bpm;
            self.selected_col = Col::Minus;
            self.decrement();
            self.update_focus_states();
            return true;
        }
        if rows_plus_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::Beats;
            self.selected_col = Col::Plus;
            self.increment();
            self.update_focus_states();
            return true;
        }
        if rows_minus_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::Beats;
            self.selected_col = Col::Minus;
            self.decrement();
            self.update_focus_states();
            return true;
        }
        if trans_plus_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::Trans;
            self.selected_col = Col::Plus;
            self.increment();
            self.update_focus_states();
            return true;
        }
        if trans_minus_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::Trans;
            self.selected_col = Col::Minus;
            self.decrement();
            self.update_focus_states();
            return true;
        }
        if bpm_input_area.contains(pos) {
            if self.editing && self.selected_row == Row::Bpm && self.selected_col == Col::Input {
                return true;
            }
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::Bpm;
            self.selected_col = Col::Input;
            self.start_editing();
            self.update_focus_states();
            return true;
        }
        if rows_input_area.contains(pos) {
            if self.editing && self.selected_row == Row::Beats && self.selected_col == Col::Input {
                return true;
            }
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::Beats;
            self.selected_col = Col::Input;
            self.start_editing();
            self.update_focus_states();
            return true;
        }
        if trans_input_area.contains(pos) {
            if self.editing && self.selected_row == Row::Trans && self.selected_col == Col::Input {
                return true;
            }
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::Trans;
            self.selected_col = Col::Input;
            self.start_editing();
            self.update_focus_states();
            return true;
        }

        let play_area = self.action_play_btn.area;
        if play_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::ActionPlay;
            self.selected_col = Col::Open;
            actions.push(ComponentAction::Play);
            self.update_focus_states();
            return true;
        }

        for i in 0..NUM_INSTRUMENTS {
            let open_area = self.instruments[i].open_button.area;
            let name_area = self.instruments[i].name_input.area;
            if open_area.contains(pos) {
                if self.editing {
                    self.confirm_editing();
                }
                self.selected_row = Row::Instrument(i);
                self.selected_col = Col::Open;
                actions.push(ComponentAction::OpenInstrumentEditor(i));
                self.update_focus_states();
                return true;
            }
            if name_area.contains(pos) {
                if self.editing
                    && self.selected_row == Row::Instrument(i)
                    && self.selected_col == Col::Input
                {
                    return true;
                }
                if self.editing {
                    self.confirm_editing();
                }
                self.selected_row = Row::Instrument(i);
                self.selected_col = Col::Input;
                self.start_editing();
                self.update_focus_states();
                return true;
            }
        }

        let tuning_area = self.action_tuning_btn.area;
        let new_open_area = self.action_new_open_btn.area;
        let quit_area = self.action_quit_btn.area;
        let save_area = self.action_save_btn.area;
        let export_area = self.action_export_btn.area;

        if tuning_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::ActionTuning;
            self.selected_col = Col::Open;
            actions.push(ComponentAction::OpenTuningEditor);
            self.update_focus_states();
            return true;
        }
        if new_open_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::ActionNewOpen;
            self.selected_col = Col::Open;
            actions.push(ComponentAction::OpenFileBrowser);
            self.update_focus_states();
            return true;
        }
        if quit_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::ActionQuit;
            self.selected_col = Col::Open;
            actions.push(ComponentAction::OpenQuitConfirm);
            self.update_focus_states();
            return true;
        }
        if save_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::ActionSave;
            self.selected_col = Col::Open;
            actions.push(ComponentAction::SaveFile);
            self.save_feedback_until = Some(Instant::now() + Duration::from_secs(3));
            self.update_focus_states();
            return true;
        }
        if export_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::ActionExport;
            self.selected_col = Col::Open;
            actions.push(ComponentAction::Export);
            self.update_focus_states();
            self.update_focus_states();
            return true;
        }

        false
    }
}

impl Component for ControlDeck {
    fn update(&mut self, events: Vec<Event>, _file: &mut TrackerFile) -> Vec<ComponentAction> {
        let mut actions = Vec::new();

        for event in &events {
            match event {
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column,
                    row,
                    ..
                }) => {
                    if self.handle_mouse_click(
                        Position {
                            x: *column,
                            y: *row,
                        },
                        &mut actions,
                    ) {
                        actions.push(ComponentAction::RequestFocus);
                    }
                }
                Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if self.editing {
                        match code {
                            KeyCode::Esc => {
                                self.restore_snapshot();
                                self.update_focus_states();
                            }
                            KeyCode::Enter => {
                                self.confirm_editing();
                                self.update_focus_states();
                            }
                            KeyCode::Char(c)
                                if self.selected_row.is_setting()
                                    && !c.is_ascii_digit()
                                    && !(self.selected_row == Row::Trans && *c == '-') => {}
                            _ => {
                                let state = self.current_input_mut();
                                state.focus.set(true);
                                let _ = handle_events(state, true, event);
                            }
                        }
                    } else if self.selected_row == Row::SampleRate
                        && self.rate_state.is_popup_active()
                    {
                        match code {
                            KeyCode::Up => {
                                let cur = self.rate_state.value();
                                if cur > 0 {
                                    self.rate_state.set_value(cur - 1);
                                }
                            }
                            KeyCode::Down => {
                                let cur = self.rate_state.value();
                                let next = (cur + 1).min(SAMPLE_RATE_OPTIONS.len() - 1);
                                self.rate_state.set_value(next);
                            }
                            KeyCode::Enter | KeyCode::Esc => {
                                self.rate_state.set_popup_active(false);
                                let idx = self.rate_state.value();
                                if idx < SAMPLE_RATE_OPTIONS.len() {
                                    self.sample_rate = SAMPLE_RATE_OPTIONS[idx];
                                    actions.push(ComponentAction::SetSampleRate(self.sample_rate));
                                }
                            }
                            _ => {}
                        }
                    } else {
                        match code {
                            KeyCode::Up => {
                                let new_row = match self.selected_row {
                                    Row::Bpm => Row::Bpm,
                                    Row::Beats => Row::Bpm,
                                    Row::Trans => Row::Beats,
                                    Row::SampleRate => Row::Trans,
                                    Row::ActionPlay => Row::SampleRate,
                                    Row::Instrument(0) => Row::ActionPlay,
                                    Row::Instrument(i) => Row::Instrument(i - 1),
                                    Row::ActionTuning => Row::Instrument(NUM_INSTRUMENTS - 1),
                                    Row::ActionNewOpen => Row::ActionTuning,
                                    Row::ActionQuit => Row::ActionNewOpen,
                                    Row::ActionSave => Row::ActionQuit,
                                    Row::ActionExport => Row::ActionSave,
                                };
                                self.selected_row = new_row;
                                self.selected_col = default_col(new_row);
                                self.update_focus_states();
                            }
                            KeyCode::Down => {
                                let new_row = match self.selected_row {
                                    Row::Bpm => Row::Beats,
                                    Row::Beats => Row::Trans,
                                    Row::Trans => Row::SampleRate,
                                    Row::SampleRate => Row::ActionPlay,
                                    Row::ActionPlay => Row::Instrument(0),
                                    Row::Instrument(i) if i < NUM_INSTRUMENTS - 1 => {
                                        Row::Instrument(i + 1)
                                    }
                                    Row::Instrument(_) => Row::ActionTuning,
                                    Row::ActionTuning => Row::ActionNewOpen,
                                    Row::ActionNewOpen => Row::ActionQuit,
                                    Row::ActionQuit => Row::ActionSave,
                                    Row::ActionSave => Row::ActionExport,
                                    Row::ActionExport => Row::ActionExport,
                                };
                                self.selected_row = new_row;
                                self.selected_col = default_col(new_row);
                                self.update_focus_states();
                            }
                            KeyCode::Left => {
                                let row = self.selected_row;
                                self.selected_col = self.selected_col.prev(row);
                                self.update_focus_states();
                            }
                            KeyCode::Right => {
                                let row = self.selected_row;
                                self.selected_col = self.selected_col.next(row);
                                self.update_focus_states();
                            }
                            KeyCode::Enter => {
                                if self.selected_row == Row::SampleRate {
                                    self.rate_state.flip_popup_active();
                                } else {
                                    match self.selected_col {
                                        Col::Input => self.start_editing(),
                                        Col::Plus => self.increment(),
                                        Col::Minus => self.decrement(),
                                        Col::Open => match self.selected_row {
                                            Row::ActionPlay => {
                                                actions.push(ComponentAction::Play);
                                            }
                                            Row::ActionNewOpen => {
                                                actions.push(ComponentAction::OpenFileBrowser);
                                            }
                                            Row::Instrument(i) => {
                                                actions
                                                    .push(ComponentAction::OpenInstrumentEditor(i));
                                            }
                                            Row::ActionTuning => {
                                                actions.push(ComponentAction::OpenTuningEditor);
                                            }
                                            Row::ActionQuit => {
                                                actions.push(ComponentAction::OpenQuitConfirm);
                                            }
                                            Row::ActionSave => {
                                                actions.push(ComponentAction::SaveFile);
                                                self.save_feedback_until =
                                                    Some(Instant::now() + Duration::from_secs(3));
                                            }
                                            Row::ActionExport => {
                                                actions.push(ComponentAction::Export);
                                            }
                                            _ => {}
                                        },
                                    }
                                }
                                self.update_focus_states();
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        actions
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, _file: &TrackerFile) {
        let bg = SCHEME.true_dark_color(SCHEME.black[3]);
        frame.render_widget(Block::new().style(Style::new().bg(bg)), area);

        let sel_row = self.selected_row;
        let sel_col = self.selected_col;
        let editing = self.editing;

        let label_w = 7u16;
        let input_w = [Row::Bpm, Row::Beats]
            .iter()
            .map(|r| r.range().1.to_string().len() as u16)
            .max()
            .unwrap_or(3)
            .max(4);

        let inner_margin = (area.width.saturating_sub(PATTERN_TABLE_WIDTH)) / 2 + 1;
        let working_area = Rect {
            x: area.x + inner_margin,
            y: area.y,
            width: PATTERN_TABLE_WIDTH.saturating_sub(2).min(area.width),
            height: area.height,
        };

        let [settings_col, inst_left_col, inst_right_col, actions_col] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .areas(working_area);

        let default_style = Style::new().bg(bg).fg(SCHEME.white[2]);
        let selected_style = Style::new()
            .bg(bg)
            .fg(SCHEME.orange[3])
            .add_modifier(Modifier::BOLD);
        let btn_base = Style::new().bg(bg).fg(SCHEME.white[2]);
        let btn_focus = Style::new()
            .bg(SCHEME.orange[3])
            .fg(SCHEME.black[0])
            .add_modifier(Modifier::BOLD);

        let setting_rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(settings_col);

        for (row_area, row) in [
            (setting_rows[0], Row::Bpm),
            (setting_rows[1], Row::Beats),
            (setting_rows[2], Row::Trans),
        ] {
            let row_sel = row == sel_row;
            let input_focused = row_sel && sel_col == Col::Input;
            let plus_focused = row_sel && sel_col == Col::Plus;
            let minus_focused = row_sel && sel_col == Col::Minus;

            let [label_area, input_area, _, plus_area, minus_area] = Layout::horizontal([
                Constraint::Length(label_w),
                Constraint::Length(input_w),
                Constraint::Length(1),
                Constraint::Length(BTN_W),
                Constraint::Length(BTN_W),
            ])
            .areas(row_area);

            frame.render_widget(
                Paragraph::new(row.label()).style(if row_sel {
                    selected_style
                } else {
                    default_style
                }),
                label_area,
            );

            let input_state = match row {
                Row::Bpm => &mut self.bpm_input,
                Row::Beats => &mut self.rows_input,
                Row::Trans => &mut self.trans_input,
                _ => unreachable!(),
            };
            input_state.focus.set(input_focused && editing);
            frame.render_stateful_widget(
                TextInput::new().style(if input_focused {
                    selected_style
                } else {
                    default_style
                }),
                input_area,
                input_state,
            );
            if input_focused
                && editing
                && let Some((cx, cy)) = input_state.screen_cursor()
            {
                frame.set_cursor_position((cx, cy));
            }

            let (plus_state, minus_state) = match row {
                Row::Bpm => (&mut self.bpm_plus, &mut self.bpm_minus),
                Row::Beats => (&mut self.rows_plus, &mut self.rows_minus),
                Row::Trans => (&mut self.trans_plus, &mut self.trans_minus),
                _ => unreachable!(),
            };
            plus_state.focus.set(plus_focused);
            minus_state.focus.set(minus_focused);
            let plus_line =
                Line::from("[+]").style(if plus_focused { btn_focus } else { btn_base });
            let minus_line =
                Line::from("[-]").style(if minus_focused { btn_focus } else { btn_base });
            frame.render_stateful_widget(
                Button::new(plus_line)
                    .style(btn_base)
                    .focus_style(btn_focus),
                plus_area,
                plus_state,
            );
            frame.render_stateful_widget(
                Button::new(minus_line)
                    .style(btn_base)
                    .focus_style(btn_focus),
                minus_area,
                minus_state,
            );
        }

        let rate_row_area = setting_rows[3];
        let rate_row_sel = sel_row == Row::SampleRate;
        let [rate_label_area, rate_value_area, _] = Layout::horizontal([
            Constraint::Length(label_w),
            Constraint::Length(8),
            Constraint::Fill(1),
        ])
        .areas(rate_row_area);

        frame.render_widget(
            Paragraph::new(Row::SampleRate.label()).style(if rate_row_sel {
                selected_style
            } else {
                default_style
            }),
            rate_label_area,
        );

        let rate_items: Vec<(usize, Line)> = SAMPLE_RATE_OPTIONS
            .iter()
            .enumerate()
            .map(|(i, _)| (i, Line::from(SAMPLE_RATE_LABELS[i])))
            .collect();
        let (rate_widget, _rate_popup) = Choice::new()
            .items(rate_items)
            .style(if rate_row_sel {
                selected_style
            } else {
                default_style
            })
            .select_style(
                Style::default()
                    .bg(SCHEME.orange[3])
                    .fg(SCHEME.black[0])
                    .add_modifier(Modifier::BOLD),
            )
            .focus_style(selected_style)
            .popup_len(SAMPLE_RATE_OPTIONS.len() as u16)
            .into_widgets();
        self.rate_state.focus.set(rate_row_sel);
        frame.render_stateful_widget(rate_widget, rate_value_area, &mut self.rate_state);

        const PLAY_BTN_W: u16 = 7;
        let play_focused = sel_row == Row::ActionPlay;
        let play_label = if self.playing { "[Pause]" } else { "[Play]" };
        let play_area = Rect {
            x: setting_rows[5].x,
            y: setting_rows[5].y,
            width: PLAY_BTN_W.min(setting_rows[5].width),
            height: 1,
        };
        self.action_play_btn.focus.set(play_focused);
        frame.render_stateful_widget(
            Button::new(Line::from(play_label).style(if play_focused {
                btn_focus
            } else {
                btn_base
            }))
            .style(btn_base)
            .focus_style(btn_focus),
            play_area,
            &mut self.action_play_btn,
        );

        let left_inst_rows = Layout::vertical([Constraint::Length(1); 6]).split(inst_left_col);
        let right_inst_rows = Layout::vertical([Constraint::Length(1); 6]).split(inst_right_col);

        for idx in 0..6usize {
            let cell_area = left_inst_rows[idx];
            let is_row = sel_row == Row::Instrument(idx);
            let open_focused = is_row && sel_col == Col::Open;
            let input_focused = is_row && sel_col == Col::Input;

            let [btn_area, _, input_area] = Layout::horizontal([
                Constraint::Length(BTN_W),
                Constraint::Length(1),
                Constraint::Fill(1),
            ])
            .areas(cell_area);

            let open_line =
                Line::from("[⚙]").style(if open_focused { btn_focus } else { btn_base });
            self.instruments[idx].open_button.focus.set(open_focused);
            frame.render_stateful_widget(
                Button::new(open_line)
                    .style(btn_base)
                    .focus_style(btn_focus),
                btn_area,
                &mut self.instruments[idx].open_button,
            );

            self.instruments[idx]
                .name_input
                .focus
                .set(input_focused && editing);
            frame.render_stateful_widget(
                TextInput::new().style(if input_focused {
                    selected_style
                } else {
                    default_style
                }),
                input_area,
                &mut self.instruments[idx].name_input,
            );

            if input_focused
                && editing
                && let Some((cx, cy)) = self.instruments[idx].name_input.screen_cursor()
            {
                frame.set_cursor_position((cx, cy));
            }
        }

        for vis in 0..5usize {
            let idx = vis + 6;
            let cell_area = right_inst_rows[vis];
            let is_row = sel_row == Row::Instrument(idx);
            let open_focused = is_row && sel_col == Col::Open;
            let input_focused = is_row && sel_col == Col::Input;

            let [btn_area, _, input_area] = Layout::horizontal([
                Constraint::Length(BTN_W),
                Constraint::Length(1),
                Constraint::Fill(1),
            ])
            .areas(cell_area);

            let open_line =
                Line::from("[⚙]").style(if open_focused { btn_focus } else { btn_base });
            self.instruments[idx].open_button.focus.set(open_focused);
            frame.render_stateful_widget(
                Button::new(open_line)
                    .style(btn_base)
                    .focus_style(btn_focus),
                btn_area,
                &mut self.instruments[idx].open_button,
            );

            self.instruments[idx]
                .name_input
                .focus
                .set(input_focused && editing);
            frame.render_stateful_widget(
                TextInput::new().style(if input_focused {
                    selected_style
                } else {
                    default_style
                }),
                input_area,
                &mut self.instruments[idx].name_input,
            );

            if input_focused
                && editing
                && let Some((cx, cy)) = self.instruments[idx].name_input.screen_cursor()
            {
                frame.set_cursor_position((cx, cy));
            }
        }

        let tuning_focused = sel_row == Row::ActionTuning;
        const TUNING_BTN_W: u16 = 15;
        let tuning_btn_area = Rect {
            x: right_inst_rows[5].x,
            y: right_inst_rows[5].y,
            width: TUNING_BTN_W.min(right_inst_rows[5].width),
            height: 1,
        };
        self.action_tuning_btn.focus.set(tuning_focused);
        frame.render_stateful_widget(
            Button::new(Line::from("[Tuning editor]").style(if tuning_focused {
                btn_focus
            } else {
                btn_base
            }))
            .style(btn_base)
            .focus_style(btn_focus),
            tuning_btn_area,
            &mut self.action_tuning_btn,
        );

        let action_rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .split(actions_col);

        const NEW_OPEN_BTN_W: u16 = 11;
        const QUIT_BTN_W: u16 = 6;
        const EXPORT_BTN_W: u16 = 8;

        let new_open_focused = sel_row == Row::ActionNewOpen;
        let quit_focused = sel_row == Row::ActionQuit;
        let save_focused = sel_row == Row::ActionSave;
        let export_focused = sel_row == Row::ActionExport;

        let showing_saved = self.save_feedback_until.is_some_and(|t| t > Instant::now());
        let save_label = if showing_saved { "[Saved!]" } else { "[Save]" };
        let save_btn_w = save_label.len() as u16;

        let showing_exported = self
            .export_feedback_until
            .is_some_and(|t| t > Instant::now());
        let export_label = if showing_exported {
            "[Exported]"
        } else {
            "[Export]"
        };
        let export_btn_w = export_label.len() as u16;

        let new_open_area = Rect {
            x: action_rows[0].x,
            y: action_rows[0].y,
            width: NEW_OPEN_BTN_W.min(action_rows[0].width),
            height: 1,
        };
        let quit_area = Rect {
            x: action_rows[1].x,
            y: action_rows[1].y,
            width: QUIT_BTN_W.min(action_rows[1].width),
            height: 1,
        };
        let save_area = Rect {
            x: action_rows[2].x,
            y: action_rows[2].y,
            width: save_btn_w.min(action_rows[2].width),
            height: 1,
        };
        let export_area = Rect {
            x: action_rows[3].x,
            y: action_rows[3].y,
            width: export_btn_w.min(action_rows[3].width),
            height: 1,
        };

        self.action_new_open_btn.focus.set(new_open_focused);
        self.action_quit_btn.focus.set(quit_focused);
        self.action_save_btn.focus.set(save_focused);
        self.action_export_btn.focus.set(export_focused);

        frame.render_stateful_widget(
            Button::new(Line::from("[New/Open]").style(if new_open_focused {
                btn_focus
            } else {
                btn_base
            }))
            .style(btn_base)
            .focus_style(btn_focus),
            new_open_area,
            &mut self.action_new_open_btn,
        );
        frame.render_stateful_widget(
            Button::new(Line::from("[Quit]").style(if quit_focused {
                btn_focus
            } else {
                btn_base
            }))
            .style(btn_base)
            .focus_style(btn_focus),
            quit_area,
            &mut self.action_quit_btn,
        );
        frame.render_stateful_widget(
            Button::new(Line::from(save_label).style(if save_focused {
                btn_focus
            } else {
                btn_base
            }))
            .style(btn_base)
            .focus_style(btn_focus),
            save_area,
            &mut self.action_save_btn,
        );
        frame.render_stateful_widget(
            Button::new(Line::from(export_label).style(if export_focused {
                btn_focus
            } else {
                btn_base
            }))
            .style(btn_base)
            .focus_style(btn_focus),
            export_area,
            &mut self.action_export_btn,
        );
    }
}

impl ControlDeck {
    pub fn render_popup(&mut self, frame: &mut Frame, boundary: Rect) {
        if !self.rate_state.is_popup_active() {
            return;
        }
        let bg = SCHEME.true_dark_color(SCHEME.black[3]);
        let rate_items: Vec<(usize, Line)> = SAMPLE_RATE_OPTIONS
            .iter()
            .enumerate()
            .map(|(i, _)| (i, Line::from(SAMPLE_RATE_LABELS[i])))
            .collect();
        let (_, rate_popup) = Choice::new()
            .items(rate_items)
            .style(Style::new().bg(bg).fg(SCHEME.white[2]))
            .select_style(
                Style::default()
                    .bg(SCHEME.orange[3])
                    .fg(SCHEME.black[0])
                    .add_modifier(Modifier::BOLD),
            )
            .popup_len(SAMPLE_RATE_OPTIONS.len() as u16)
            .popup_boundary(boundary)
            .into_widgets();
        frame.render_stateful_widget(rate_popup, boundary, &mut self.rate_state);
    }
}
