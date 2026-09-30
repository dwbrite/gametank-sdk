use std::time::{Duration, Instant};

use rat_widget::button::{Button, ButtonState};
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
    tracker::{PATTERN_BEATS, PATTERN_TABLE_WIDTH},
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Row {
    Bpm,
    FxSpeed,
    Beats,
    Trans,
    ActionPlay,
    ActionNewOpen,
    Instrument(usize),
    ActionTuning,
    ActionQuit,
    ActionSave,
    ActionExport,
    PatternPrev,
    PatternNext,
    PatternNew,
    PatternCopy,
    PatternDelete,
    PatternZap,
}

impl Row {
    fn is_setting(self) -> bool {
        matches!(self, Row::Bpm | Row::FxSpeed | Row::Beats | Row::Trans)
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
                | Row::PatternPrev
                | Row::PatternNext
                | Row::PatternNew
                | Row::PatternCopy
                | Row::PatternDelete
                | Row::PatternZap
        )
    }

    fn label(self) -> &'static str {
        match self {
            Row::Bpm => "BPM:     ",
            Row::FxSpeed => "FxSpeed: ",
            Row::Beats => "Beats:   ",
            Row::Trans => "Trans:   ",
            Row::Instrument(_)
            | Row::ActionPlay
            | Row::ActionNewOpen
            | Row::ActionTuning
            | Row::ActionQuit
            | Row::ActionSave
            | Row::ActionExport
            | Row::PatternPrev
            | Row::PatternNext
            | Row::PatternNew
            | Row::PatternCopy
            | Row::PatternDelete
            | Row::PatternZap => "",
        }
    }

    fn range(self) -> (u16, u16) {
        match self {
            Row::Bpm => (1, 399),
            Row::FxSpeed => (1, 31),
            Row::Beats => (0, 255),
            Row::Trans
            | Row::Instrument(_)
            | Row::ActionPlay
            | Row::ActionNewOpen
            | Row::ActionTuning
            | Row::ActionQuit
            | Row::ActionSave
            | Row::ActionExport
            | Row::PatternPrev
            | Row::PatternNext
            | Row::PatternNew
            | Row::PatternCopy
            | Row::PatternDelete
            | Row::PatternZap => (0, 0),
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
        Row::Bpm | Row::FxSpeed | Row::Beats | Row::Trans => Col::Input,
        Row::Instrument(_)
        | Row::ActionPlay
        | Row::ActionNewOpen
        | Row::ActionTuning
        | Row::ActionQuit
        | Row::ActionSave
        | Row::ActionExport
        | Row::PatternPrev
        | Row::PatternNext
        | Row::PatternNew
        | Row::PatternCopy
        | Row::PatternDelete
        | Row::PatternZap => Col::Open,
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
    fx_speed_input: TextInputState,
    fx_speed_minus: ButtonState,
    fx_speed_plus: ButtonState,
    fx_speed_snapshot: String,
    editing: bool,
    instruments: [InstrumentEntry; NUM_INSTRUMENTS],
    pub playing: bool,
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
    pattern_prev_btn: ButtonState,
    pattern_next_btn: ButtonState,
    pattern_new_btn: ButtonState,
    pattern_copy_btn: ButtonState,
    pattern_delete_btn: ButtonState,
    pattern_zap_btn: ButtonState,
    pattern_idx: u8,
    pattern_count: usize,
}

impl ControlDeck {
    pub fn init() -> Self {
        let mut bpm_input = TextInputState::new();
        bpm_input.set_value("78");
        let mut fx_speed_input = TextInputState::new();
        fx_speed_input.set_value("6");
        let mut rows_input = TextInputState::new();
        rows_input.set_value("40");
        let mut trans_input = TextInputState::new();
        trans_input.set_value("0");
        Self {
            selected_row: Row::Bpm,
            selected_col: Col::Input,
            editing: false,
            bpm_input,
            bpm_snapshot: "78".to_string(),
            bpm_plus: ButtonState::new(),
            bpm_minus: ButtonState::new(),
            fx_speed_input,
            fx_speed_snapshot: "6".to_string(),
            fx_speed_plus: ButtonState::new(),
            fx_speed_minus: ButtonState::new(),
            rows_input,
            rows_snapshot: "40".to_string(),
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
                InstrumentEntry::new(&format!("instrument_{}", i))
            }),
            playing: false,
            action_tuning_btn: ButtonState::new(),
            action_new_open_btn: ButtonState::new(),
            action_play_btn: ButtonState::new(),
            action_quit_btn: ButtonState::new(),
            action_save_btn: ButtonState::new(),
            action_export_btn: ButtonState::new(),
            pattern_prev_btn: ButtonState::new(),
            pattern_next_btn: ButtonState::new(),
            pattern_new_btn: ButtonState::new(),
            pattern_copy_btn: ButtonState::new(),
            pattern_delete_btn: ButtonState::new(),
            pattern_zap_btn: ButtonState::new(),
            pattern_idx: 0,
            pattern_count: 1,
            save_feedback_until: None,
            export_feedback_until: None,
        }
    }

    pub fn get_names(&self) -> [String; NUM_INSTRUMENTS] {
        std::array::from_fn(|i| self.instruments[i].name_input.value::<String>())
    }

    pub fn set_names(&mut self, names: &[String; NUM_INSTRUMENTS]) {
        for (i, instrument) in self.instruments.iter_mut().enumerate() {
            instrument.name_input.set_value(names[i].clone());
            instrument.name_snapshot = names[i].clone();
        }
    }

    pub fn blur_all(&mut self) {
        if self.editing {
            self.confirm_editing();
        }
        self.bpm_input.focus.set(false);
        self.bpm_plus.focus.set(false);
        self.bpm_minus.focus.set(false);
        self.fx_speed_input.focus.set(false);
        self.fx_speed_plus.focus.set(false);
        self.fx_speed_minus.focus.set(false);
        self.rows_input.focus.set(false);
        self.rows_plus.focus.set(false);
        self.rows_minus.focus.set(false);
        self.trans_input.focus.set(false);
        self.trans_plus.focus.set(false);
        self.trans_minus.focus.set(false);
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
        self.pattern_prev_btn.focus.set(false);
        self.pattern_next_btn.focus.set(false);
        self.pattern_new_btn.focus.set(false);
        self.pattern_copy_btn.focus.set(false);
        self.pattern_delete_btn.focus.set(false);
        self.pattern_zap_btn.focus.set(false);
    }

    pub fn get_bpm(&self) -> u16 {
        let (min, max) = Row::Bpm.range();
        u16::from_str_radix(&self.bpm_input.value::<String>(), 16)
            .unwrap_or(min)
            .clamp(min, max)
    }

    pub fn set_bpm(&mut self, bpm: u16) {
        let (min, max) = Row::Bpm.range();
        let clamped = bpm.clamp(min, max);
        let s = format!("{:X}", clamped);
        self.bpm_input.set_value(s.clone());
        self.bpm_snapshot = s;
    }

    pub fn get_fx_speed(&self) -> u8 {
        let (min, max) = Row::FxSpeed.range();
        u16::from_str_radix(&self.fx_speed_input.value::<String>(), 16)
            .unwrap_or(min)
            .clamp(min, max) as u8
    }

    pub fn set_fx_speed(&mut self, speed: u8) {
        let (min, max) = Row::FxSpeed.range();
        let clamped = (speed as u16).clamp(min, max);
        let s = format!("{:X}", clamped);
        self.fx_speed_input.set_value(s.clone());
        self.fx_speed_snapshot = s;
    }

    pub fn get_beats(&self) -> u8 {
        u16::from_str_radix(&self.rows_input.value::<String>(), 16)
            .unwrap_or(PATTERN_BEATS as u16)
            .clamp(1, PATTERN_BEATS as u16) as u8
    }

    pub fn set_beats(&mut self, beats: u8) {
        let clamped = (beats as u16).clamp(1, PATTERN_BEATS as u16);
        let s = format!("{:X}", clamped);
        self.rows_input.set_value(s.clone());
        self.rows_snapshot = s;
    }

    pub fn mark_export_success(&mut self) {
        self.export_feedback_until = Some(Instant::now() + Duration::from_secs(3));
    }

    pub fn set_pattern_info(&mut self, pattern_idx: u8, pattern_count: usize, beats: u8) {
        if pattern_idx != self.pattern_idx {
            let s = format!("{:X}", beats);
            self.rows_input.set_value(s.clone());
            self.rows_snapshot = s;
        }
        self.pattern_idx = pattern_idx;
        self.pattern_count = pattern_count.max(1);
    }

    pub fn sync_beats_to_file(&self, file: &mut TrackerFile) {
        file.set_beats_for(self.pattern_idx, self.get_beats());
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
        self.fx_speed_input
            .focus
            .set(row == Row::FxSpeed && col == Col::Input && editing);
        self.fx_speed_plus
            .focus
            .set(row == Row::FxSpeed && col == Col::Plus);
        self.fx_speed_minus
            .focus
            .set(row == Row::FxSpeed && col == Col::Minus);
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
        self.pattern_prev_btn.focus.set(row == Row::PatternPrev);
        self.pattern_next_btn.focus.set(row == Row::PatternNext);
        self.pattern_new_btn.focus.set(row == Row::PatternNew);
        self.pattern_copy_btn.focus.set(row == Row::PatternCopy);
        self.pattern_delete_btn.focus.set(row == Row::PatternDelete);
        self.pattern_zap_btn.focus.set(row == Row::PatternZap);
    }

    fn current_input_mut(&mut self) -> &mut TextInputState {
        match self.selected_row {
            Row::Bpm => &mut self.bpm_input,
            Row::FxSpeed => &mut self.fx_speed_input,
            Row::Beats => &mut self.rows_input,
            Row::Trans => &mut self.trans_input,
            Row::Instrument(i) => &mut self.instruments[i].name_input,
            Row::ActionPlay
            | Row::ActionNewOpen
            | Row::ActionTuning
            | Row::ActionQuit
            | Row::ActionSave
            | Row::ActionExport
            | Row::PatternPrev
            | Row::PatternNext
            | Row::PatternNew
            | Row::PatternCopy
            | Row::PatternDelete
            | Row::PatternZap => {
                unreachable!("action rows have no text input")
            }
        }
    }

    fn take_snapshot(&mut self) {
        match self.selected_row {
            Row::Bpm => self.bpm_snapshot = self.bpm_input.value::<String>(),
            Row::FxSpeed => self.fx_speed_snapshot = self.fx_speed_input.value::<String>(),
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
            | Row::PatternPrev
            | Row::PatternNext
            | Row::PatternNew
            | Row::PatternCopy
            | Row::PatternDelete
            | Row::PatternZap => {}
        }
    }

    fn restore_snapshot(&mut self) {
        match self.selected_row {
            Row::Bpm => {
                let s = self.bpm_snapshot.clone();
                self.bpm_input.set_value(s);
            }
            Row::FxSpeed => {
                let s = self.fx_speed_snapshot.clone();
                self.fx_speed_input.set_value(s);
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
            | Row::PatternPrev
            | Row::PatternNext
            | Row::PatternNew
            | Row::PatternCopy
            | Row::PatternDelete
            | Row::PatternZap => {}
        }
        self.editing = false;
    }

    fn start_editing(&mut self) {
        if self.selected_row.is_action() {
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
                let clamped = u16::from_str_radix(&raw, 16)
                    .map(|v| v.clamp(min, max))
                    .unwrap_or(min);
                let s = format!("{:X}", clamped);
                self.bpm_input.set_value(s.clone());
                self.bpm_snapshot = s;
            }
            Row::FxSpeed => {
                let (min, max) = Row::FxSpeed.range();
                let raw = self.fx_speed_input.value::<String>();
                let clamped = u16::from_str_radix(&raw, 16)
                    .map(|v| v.clamp(min, max))
                    .unwrap_or(min);
                let s = format!("{:X}", clamped);
                self.fx_speed_input.set_value(s.clone());
                self.fx_speed_snapshot = s;
            }
            Row::Beats => {
                let (min, max) = Row::Beats.range();
                let raw = self.rows_input.value::<String>();
                let clamped = u16::from_str_radix(&raw, 16)
                    .map(|v| v.clamp(min, max))
                    .unwrap_or(min);
                let s = format!("{:X}", clamped);
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
            | Row::PatternPrev
            | Row::PatternNext
            | Row::PatternNew
            | Row::PatternCopy
            | Row::PatternDelete
            | Row::PatternZap => {}
        }
        self.editing = false;
    }

    fn get_setting_value(&self, row: Row) -> u16 {
        let (min, max) = row.range();
        let raw = match row {
            Row::Bpm => self.bpm_input.value::<String>(),
            Row::FxSpeed => self.fx_speed_input.value::<String>(),
            Row::Beats => self.rows_input.value::<String>(),
            _ => return min,
        };
        u16::from_str_radix(&raw, 16).unwrap_or(min).clamp(min, max)
    }

    fn set_setting_value(&mut self, row: Row, value: u16) {
        let s = format!("{:X}", value);
        match row {
            Row::Bpm => {
                self.bpm_input.set_value(s.clone());
                self.bpm_snapshot = s;
            }
            Row::FxSpeed => {
                self.fx_speed_input.set_value(s.clone());
                self.fx_speed_snapshot = s;
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
        let bpm_plus_area = self.bpm_plus.area;
        let bpm_minus_area = self.bpm_minus.area;
        let fx_speed_plus_area = self.fx_speed_plus.area;
        let fx_speed_minus_area = self.fx_speed_minus.area;
        let rows_plus_area = self.rows_plus.area;
        let rows_minus_area = self.rows_minus.area;
        let bpm_input_area = self.bpm_input.area;
        let fx_speed_input_area = self.fx_speed_input.area;
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
        if fx_speed_plus_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::FxSpeed;
            self.selected_col = Col::Plus;
            self.increment();
            self.update_focus_states();
            return true;
        }
        if fx_speed_minus_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::FxSpeed;
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
        if fx_speed_input_area.contains(pos) {
            if self.editing && self.selected_row == Row::FxSpeed && self.selected_col == Col::Input
            {
                return true;
            }
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::FxSpeed;
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

        let pattern_prev_area = self.pattern_prev_btn.area;
        let pattern_next_area = self.pattern_next_btn.area;
        let pattern_new_area = self.pattern_new_btn.area;
        let pattern_copy_area = self.pattern_copy_btn.area;
        let pattern_delete_area = self.pattern_delete_btn.area;
        let pattern_zap_area = self.pattern_zap_btn.area;

        if pattern_prev_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::PatternPrev;
            self.selected_col = Col::Open;
            actions.push(ComponentAction::PatternPrev);
            self.update_focus_states();
            return true;
        }
        if pattern_next_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::PatternNext;
            self.selected_col = Col::Open;
            actions.push(ComponentAction::PatternNext);
            self.update_focus_states();
            return true;
        }
        if pattern_new_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::PatternNew;
            self.selected_col = Col::Open;
            actions.push(ComponentAction::PatternNew);
            self.update_focus_states();
            return true;
        }
        if pattern_copy_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::PatternCopy;
            self.selected_col = Col::Open;
            actions.push(ComponentAction::PatternCopy);
            self.update_focus_states();
            return true;
        }
        if pattern_delete_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::PatternDelete;
            self.selected_col = Col::Open;
            actions.push(ComponentAction::OpenPatternDeleteConfirm);
            self.update_focus_states();
            return true;
        }
        if pattern_zap_area.contains(pos) {
            if self.editing {
                self.confirm_editing();
            }
            self.selected_row = Row::PatternZap;
            self.selected_col = Col::Open;
            actions.push(ComponentAction::PatternZap);
            self.update_focus_states();
            return true;
        }

        false
    }
}

impl Component for ControlDeck {
    fn update(&mut self, events: Vec<Event>, file: &mut TrackerFile) -> Vec<ComponentAction> {
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
                                if matches!(self.selected_row, Row::Bpm | Row::FxSpeed | Row::Beats)
                                    && !c.is_ascii_hexdigit() => {}
                            KeyCode::Char(c)
                                if self.selected_row == Row::Trans
                                    && !c.is_ascii_digit()
                                    && *c != '-' => {}
                            _ => {
                                let state = self.current_input_mut();
                                state.focus.set(true);
                                let _ = handle_events(state, true, event);
                            }
                        }
                    } else {
                        match code {
                            KeyCode::Up => {
                                let new_row = match self.selected_row {
                                    Row::Bpm => Row::Bpm,
                                    Row::FxSpeed => Row::Bpm,
                                    Row::Beats => Row::FxSpeed,
                                    Row::Trans => Row::Beats,
                                    Row::ActionPlay => Row::Trans,
                                    Row::Instrument(0) => Row::ActionPlay,
                                    Row::Instrument(i) => Row::Instrument(i - 1),
                                    Row::ActionTuning => Row::Instrument(NUM_INSTRUMENTS - 1),
                                    Row::ActionNewOpen => Row::ActionTuning,
                                    Row::ActionQuit => Row::ActionTuning,
                                    Row::ActionSave => Row::ActionNewOpen,
                                    Row::ActionExport => Row::ActionQuit,
                                    Row::PatternPrev => Row::ActionSave,
                                    Row::PatternNext => Row::ActionExport,
                                    Row::PatternNew => Row::PatternPrev,
                                    Row::PatternCopy => Row::PatternNew,
                                    Row::PatternDelete => Row::PatternCopy,
                                    Row::PatternZap => Row::PatternDelete,
                                };
                                self.selected_row = new_row;
                                self.selected_col = default_col(new_row);
                                self.update_focus_states();
                            }
                            KeyCode::Down => {
                                let new_row = match self.selected_row {
                                    Row::Bpm => Row::FxSpeed,
                                    Row::FxSpeed => Row::Beats,
                                    Row::Beats => Row::Trans,
                                    Row::Trans => Row::ActionPlay,
                                    Row::ActionPlay => Row::Instrument(0),
                                    Row::Instrument(i) if i < NUM_INSTRUMENTS - 1 => {
                                        Row::Instrument(i + 1)
                                    }
                                    Row::Instrument(_) => Row::ActionTuning,
                                    Row::ActionTuning => Row::ActionNewOpen,
                                    Row::ActionNewOpen => Row::ActionSave,
                                    Row::ActionQuit => Row::ActionExport,
                                    Row::ActionSave => Row::PatternPrev,
                                    Row::ActionExport => Row::PatternNext,
                                    Row::PatternPrev => Row::PatternNew,
                                    Row::PatternNext => Row::PatternNew,
                                    Row::PatternNew => Row::PatternCopy,
                                    Row::PatternCopy => Row::PatternDelete,
                                    Row::PatternDelete => Row::PatternZap,
                                    Row::PatternZap => Row::PatternZap,
                                };
                                self.selected_row = new_row;
                                self.selected_col = default_col(new_row);
                                self.update_focus_states();
                            }
                            KeyCode::Left => {
                                let row = self.selected_row;
                                match row {
                                    Row::ActionQuit => self.selected_row = Row::ActionNewOpen,
                                    Row::ActionExport => self.selected_row = Row::ActionSave,
                                    _ => self.selected_col = self.selected_col.prev(row),
                                }
                                self.update_focus_states();
                            }
                            KeyCode::Right => {
                                let row = self.selected_row;
                                match row {
                                    Row::ActionNewOpen => self.selected_row = Row::ActionQuit,
                                    Row::ActionSave => self.selected_row = Row::ActionExport,
                                    _ => self.selected_col = self.selected_col.next(row),
                                }
                                self.update_focus_states();
                            }
                            KeyCode::Enter => {
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
                                        Row::PatternPrev => {
                                            actions.push(ComponentAction::PatternPrev);
                                        }
                                        Row::PatternNext => {
                                            actions.push(ComponentAction::PatternNext);
                                        }
                                        Row::PatternNew => {
                                            actions.push(ComponentAction::PatternNew);
                                        }
                                        Row::PatternCopy => {
                                            actions.push(ComponentAction::PatternCopy);
                                        }
                                        Row::PatternDelete => {
                                            actions
                                                .push(ComponentAction::OpenPatternDeleteConfirm);
                                        }
                                        Row::PatternZap => {
                                            actions.push(ComponentAction::PatternZap);
                                        }
                                        _ => {}
                                    },
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
        self.sync_beats_to_file(file);
        actions
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, _file: &TrackerFile) {
        let bg = SCHEME.true_dark_color(SCHEME.black[3]);
        frame.render_widget(Block::new().style(Style::new().bg(bg)), area);

        let sel_row = self.selected_row;
        let sel_col = self.selected_col;
        let editing = self.editing;

        let label_w = Row::Bpm.label().len() as u16;
        let input_w = [Row::Bpm, Row::FxSpeed, Row::Beats]
            .iter()
            .map(|r| format!("{:X}", r.range().1).len() as u16)
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
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .split(settings_col);

        for (row_area, row) in [
            (setting_rows[0], Row::Bpm),
            (setting_rows[1], Row::FxSpeed),
            (setting_rows[2], Row::Beats),
            (setting_rows[3], Row::Trans),
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
                Row::FxSpeed => &mut self.fx_speed_input,
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
                Row::FxSpeed => (&mut self.fx_speed_plus, &mut self.fx_speed_minus),
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

        for vis in 0..4usize {
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
            x: right_inst_rows[4].x,
            y: right_inst_rows[4].y,
            width: TUNING_BTN_W.min(right_inst_rows[4].width),
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
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .split(actions_col);

        const NEW_OPEN_BTN_W: u16 = 10;
        const QUIT_BTN_W: u16 = 6;

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

        let new_open_w = NEW_OPEN_BTN_W.min(action_rows[0].width);
        let new_open_area = Rect {
            x: action_rows[0].x,
            y: action_rows[0].y,
            width: new_open_w,
            height: 1,
        };
        let quit_x = action_rows[0].x + new_open_w + 1;
        let quit_area = Rect {
            x: quit_x,
            y: action_rows[0].y,
            width: QUIT_BTN_W.min(action_rows[0].width.saturating_sub(new_open_w + 1)),
            height: 1,
        };
        let save_w = save_btn_w.min(action_rows[1].width);
        let save_area = Rect {
            x: action_rows[1].x,
            y: action_rows[1].y,
            width: save_w,
            height: 1,
        };
        let export_x = action_rows[1].x + save_w + 1;
        let export_area = Rect {
            x: export_x,
            y: action_rows[1].y,
            width: export_btn_w.min(action_rows[1].width.saturating_sub(save_w + 1)),
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

        let pattern_nav_row = action_rows[4];
        let pattern_prev_focused = sel_row == Row::PatternPrev;
        let pattern_next_focused = sel_row == Row::PatternNext;
        const PATTERN_NAV_BTN_W: u16 = 5;
        let [pattern_prev_area, pattern_next_area, pattern_label_area] = Layout::horizontal([
            Constraint::Length(PATTERN_NAV_BTN_W),
            Constraint::Length(PATTERN_NAV_BTN_W),
            Constraint::Fill(1),
        ])
        .areas(pattern_nav_row);

        self.pattern_prev_btn.focus.set(pattern_prev_focused);
        self.pattern_next_btn.focus.set(pattern_next_focused);
        frame.render_stateful_widget(
            Button::new(Line::from("[<--]").style(if pattern_prev_focused {
                btn_focus
            } else {
                btn_base
            }))
            .style(btn_base)
            .focus_style(btn_focus),
            pattern_prev_area,
            &mut self.pattern_prev_btn,
        );
        frame.render_stateful_widget(
            Button::new(Line::from("[-->]").style(if pattern_next_focused {
                btn_focus
            } else {
                btn_base
            }))
            .style(btn_base)
            .focus_style(btn_focus),
            pattern_next_area,
            &mut self.pattern_next_btn,
        );
        frame.render_widget(
            Paragraph::new(format!(" Pattern {:02}", self.pattern_idx)).style(default_style),
            pattern_label_area,
        );

        let pattern_mgmt_row = action_rows[5];
        let pattern_new_focused = sel_row == Row::PatternNew;
        let pattern_copy_focused = sel_row == Row::PatternCopy;
        let pattern_delete_focused = sel_row == Row::PatternDelete;
        let pattern_zap_focused = sel_row == Row::PatternZap;

        const PATTERN_NEW_W: u16 = 5;
        const PATTERN_COPY_W: u16 = 5;
        const PATTERN_DELETE_W: u16 = 5;
        const PATTERN_ZAP_W: u16 = 5;
        let [pattern_new_area, pattern_copy_area, pattern_delete_area, pattern_zap_area, _] =
            Layout::horizontal([
                Constraint::Length(PATTERN_NEW_W),
                Constraint::Length(PATTERN_COPY_W),
                Constraint::Length(PATTERN_DELETE_W),
                Constraint::Length(PATTERN_ZAP_W),
                Constraint::Fill(1),
            ])
            .areas(pattern_mgmt_row);

        self.pattern_new_btn.focus.set(pattern_new_focused);
        self.pattern_copy_btn.focus.set(pattern_copy_focused);
        self.pattern_delete_btn.focus.set(pattern_delete_focused);
        self.pattern_zap_btn.focus.set(pattern_zap_focused);

        frame.render_stateful_widget(
            Button::new(Line::from("[New]").style(if pattern_new_focused {
                btn_focus
            } else {
                btn_base
            }))
            .style(btn_base)
            .focus_style(btn_focus),
            pattern_new_area,
            &mut self.pattern_new_btn,
        );
        frame.render_stateful_widget(
            Button::new(Line::from("[Cpy]").style(if pattern_copy_focused {
                btn_focus
            } else {
                btn_base
            }))
            .style(btn_base)
            .focus_style(btn_focus),
            pattern_copy_area,
            &mut self.pattern_copy_btn,
        );
        frame.render_stateful_widget(
            Button::new(Line::from("[Del]").style(if pattern_delete_focused {
                btn_focus
            } else {
                btn_base
            }))
            .style(btn_base)
            .focus_style(btn_focus),
            pattern_delete_area,
            &mut self.pattern_delete_btn,
        );
        frame.render_stateful_widget(
            Button::new(Line::from("[Zap]").style(if pattern_zap_focused {
                btn_focus
            } else {
                btn_base
            }))
            .style(btn_base)
            .focus_style(btn_focus),
            pattern_zap_area,
            &mut self.pattern_zap_btn,
        );
    }
}
