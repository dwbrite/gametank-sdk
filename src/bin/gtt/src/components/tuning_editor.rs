use std::collections::HashMap;
use std::path::Path;

use rat_widget::button::{Button, ButtonState};
use rat_widget::text::HasScreenCursor;
use rat_widget::text_input::{TextInput, TextInputState, handle_events};
use ratatui::{
    Frame,
    crossterm::event::{
        Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind,
    },
    layout::{Constraint, Layout, Margin, Position, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{
        Block, Cell, Clear, Paragraph, Row as TableRow, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, WidgetRef,
    },
};
use ratatui_explorer::{FileExplorer, FileExplorerBuilder, Input as ExplorerInput};

use crate::{
    action::ComponentAction,
    component::Component,
    file::{Interval, TrackerFile, TuningData},
    scheme::SCHEME,
    util::hz_to_inc_q16,
};

const CANCEL_BTN_W: u16 = 8;
const SAVE_BTN_W: u16 = 6;
const OPEN_SCL_BTN_W: u16 = 11;
const BTN_GAP: u16 = 2;
const MAX_NAME_LEN: usize = 4;
const DEGREE_COL_W: u16 = 3;
const NAME_COL_W: u16 = 5;
const CENTS_COL_W: u16 = 7;
const FREQ_MIN: f64 = 7.0;
const FREQ_MAX: f64 = 4200.0;
const REPEAT_NUM_PIVOT_HZ: f64 = 510.0;
const REPEAT_NUM_PIVOT: i32 = 5;

#[derive(Clone, Copy, PartialEq, Eq)]
enum EntryCol {
    Name,
    Cents,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BaseCol {
    Input,
    Plus,
    Minus,
}

struct SclEntry {
    name_input: TextInputState,
    cents_input: TextInputState,
    name_snapshot: String,
    cents_snapshot: String,
}

impl SclEntry {
    fn new(name: &str, cents_val: f64) -> Self {
        let cents_str = format!("{}", cents_val.round() as i64);
        let mut name_input = TextInputState::new();
        name_input.set_value(name);
        let mut cents_input = TextInputState::new();
        cents_input.set_value(&cents_str);
        Self {
            name_input,
            cents_input,
            name_snapshot: name.to_string(),
            cents_snapshot: cents_str,
        }
    }
}

pub struct TuningEditor {
    pub visible: bool,
    base_col: BaseCol,
    base_editing: bool,
    base_focused: bool,
    base_freq: f64,
    base_freq_input: TextInputState,
    base_freq_snapshot: String,
    base_minus_btn: ButtonState,
    base_plus_btn: ButtonState,
    cancel_button: ButtonState,
    capturing_row: Option<usize>,
    cents_col_x: u16,
    editing: bool,
    entries: Vec<SclEntry>,
    entry_scroll: usize,
    explorer_area: Rect,
    explorer_scroll_offset: usize,
    file_explorer: Option<FileExplorer>,
    freq_panel_area: Rect,
    freq_rows: Vec<(String, f64, u16)>,
    freq_scroll_offset: usize,
    key_assignments: HashMap<String, Vec<char>>,
    list_h: u16,
    list_y: u16,
    name_col_x: u16,
    name_duplicate: bool,
    open_scl_btn: ButtonState,
    save_button: ButtonState,
    scl_error: Option<String>,
    selected_col: EntryCol,
    selected: usize,
}

impl TuningEditor {
    pub fn init() -> Self {
        let mut base_freq_input = TextInputState::new();
        base_freq_input.set_value("440");
        Self {
            base_col: BaseCol::Input,
            base_editing: false,
            base_focused: false,
            base_freq: 440.0,
            base_freq_input,
            base_freq_snapshot: "440".to_string(),
            base_minus_btn: ButtonState::new(),
            base_plus_btn: ButtonState::new(),
            cancel_button: ButtonState::new(),
            capturing_row: None,
            cents_col_x: 0,
            editing: false,
            entries: Vec::new(),
            entry_scroll: 0,
            explorer_area: Rect::default(),
            explorer_scroll_offset: 0,
            file_explorer: None,
            freq_panel_area: Rect::default(),
            freq_rows: Vec::new(),
            freq_scroll_offset: 0,
            key_assignments: HashMap::new(),
            list_h: 0,
            list_y: 0,
            name_col_x: 0,
            name_duplicate: false,
            open_scl_btn: ButtonState::new(),
            save_button: ButtonState::new(),
            scl_error: None,
            selected: 0,
            selected_col: EntryCol::Name,
            visible: false,
        }
    }

    pub fn open(&mut self, tuning: &TuningData) {
        self.base_col = BaseCol::Input;
        self.base_editing = false;
        self.base_focused = false;
        self.capturing_row = None;
        self.editing = false;
        self.entry_scroll = 0;
        self.explorer_scroll_offset = 0;
        self.file_explorer = None;
        self.name_duplicate = false;
        self.scl_error = None;
        self.selected = 0;
        self.selected_col = EntryCol::Name;
        self.visible = true;

        let freq_str = format!("{}", self.base_freq.round().clamp(1.0, 1999.0) as u32);
        self.base_freq_input.set_value(freq_str.clone());
        self.base_freq_snapshot = freq_str;

        self.key_assignments = tuning
            .key_assignments
            .iter()
            .map(|(note, keys)| {
                let chars: Vec<char> = keys.iter().filter_map(|s| s.chars().next()).collect();
                (note.clone(), chars)
            })
            .collect();

        if !tuning.scale.is_empty() {
            self.entries = tuning
                .scale
                .iter()
                .map(|d| SclEntry::new(&d.name, d.cents))
                .collect();
            self.recompute_freq_table();
        } else {
            self.entries = Vec::new();
            self.freq_rows = tuning
                .notes
                .iter()
                .map(|(name, &freq)| {
                    let osc = hz_to_inc_q16((freq * 65536.0).round() as u32);
                    (name.clone(), freq, osc)
                })
                .collect();
        }
        self.freq_scroll_offset = 0;
    }

    pub fn get_tuning_data(&self) -> TuningData {
        use indexmap::IndexMap;
        let notes: IndexMap<String, f64> = self
            .freq_rows
            .iter()
            .map(|(name, freq, _)| (name.clone(), *freq))
            .collect();
        let key_assignments: IndexMap<String, Vec<String>> = self
            .key_assignments
            .iter()
            .map(|(note, chars)| {
                let keys: Vec<String> = chars.iter().map(|c| c.to_string()).collect();
                (note.clone(), keys)
            })
            .collect();
        let scale: Vec<Interval> = self
            .entries
            .iter()
            .map(|e| {
                let cents = e.cents_input.value::<String>().parse::<i64>().unwrap_or(0) as f64;
                Interval {
                    name: e.name_input.value(),
                    cents,
                }
            })
            .collect();
        TuningData {
            notes,
            key_assignments,
            scale,
        }
    }

    fn check_name_duplicate(&self) -> bool {
        if self.entries.is_empty() {
            return false;
        }
        let current: String = self.entries[self.selected].name_input.value();
        self.entries
            .iter()
            .enumerate()
            .any(|(i, e)| i != self.selected && e.name_input.value::<String>() == current)
    }

    fn enforce_name_limit(&mut self) {
        let current: String = self.entries[self.selected].name_input.value();
        if current.chars().count() > MAX_NAME_LEN {
            let truncated: String = current.chars().take(MAX_NAME_LEN).collect();
            self.entries[self.selected].name_input.set_value(truncated);
        }
    }

    fn take_snapshot(&mut self) {
        match self.selected_col {
            EntryCol::Name => {
                let v: String = self.entries[self.selected].name_input.value();
                self.entries[self.selected].name_snapshot = v;
            }
            EntryCol::Cents => {
                let v: String = self.entries[self.selected].cents_input.value();
                self.entries[self.selected].cents_snapshot = v;
            }
        }
    }

    fn restore_snapshot(&mut self) {
        match self.selected_col {
            EntryCol::Name => {
                let s = self.entries[self.selected].name_snapshot.clone();
                self.entries[self.selected].name_input.set_value(s);
            }
            EntryCol::Cents => {
                let s = self.entries[self.selected].cents_snapshot.clone();
                self.entries[self.selected].cents_input.set_value(s);
            }
        }
        self.editing = false;
        self.name_duplicate = false;
    }

    fn start_editing(&mut self) {
        self.take_snapshot();
        self.editing = true;
    }

    fn confirm_editing(&mut self) {
        match self.selected_col {
            EntryCol::Name => {
                if self.name_duplicate {
                    return;
                }
                let v: String = self.entries[self.selected].name_input.value();
                self.entries[self.selected].name_snapshot = v;
            }
            EntryCol::Cents => {
                let raw: String = self.entries[self.selected].cents_input.value();
                match raw.trim().parse::<i64>() {
                    Ok(v) => {
                        let s = format!("{}", v);
                        self.entries[self.selected].cents_input.set_value(s.clone());
                        self.entries[self.selected].cents_snapshot = s;
                    }
                    Err(_) => {
                        let s = self.entries[self.selected].cents_snapshot.clone();
                        self.entries[self.selected].cents_input.set_value(s);
                    }
                }
            }
        }
        self.editing = false;
        self.name_duplicate = false;
        self.recompute_freq_table();
    }

    fn compute_entry_scroll(&self, visible_h: usize) -> usize {
        if visible_h >= self.entries.len() {
            0
        } else {
            self.selected
                .saturating_sub(visible_h / 2)
                .min(self.entries.len() - visible_h)
        }
    }

    fn base_freq_set(&mut self, val: u32) {
        let clamped = val.clamp(1, 1999);
        let s = format!("{}", clamped);
        self.base_freq_input.set_value(s.clone());
        self.base_freq_snapshot = s;
        self.base_freq = clamped as f64;
        self.recompute_freq_table();
    }

    fn base_freq_increment(&mut self) {
        let cur: u32 = self
            .base_freq_input
            .value::<String>()
            .parse()
            .unwrap_or(self.base_freq.round() as u32);
        self.base_freq_set(cur.saturating_add(1));
    }

    fn base_freq_decrement(&mut self) {
        let cur: u32 = self
            .base_freq_input
            .value::<String>()
            .parse()
            .unwrap_or(self.base_freq.round() as u32);
        self.base_freq_set(cur.saturating_sub(1));
    }

    fn base_freq_confirm(&mut self) {
        let raw: String = self.base_freq_input.value();
        let val: u32 = raw.trim().parse().unwrap_or(self.base_freq.round() as u32);
        self.base_editing = false;
        self.base_freq_set(val);
    }

    fn base_freq_restore(&mut self) {
        self.base_freq_input
            .set_value(self.base_freq_snapshot.clone());
        self.base_editing = false;
    }

    fn recompute_freq_table(&mut self) {
        if self.entries.is_empty() {
            self.freq_rows.clear();
            self.freq_scroll_offset = 0;
            return;
        }

        let n = self.entries.len();
        let names: Vec<String> = self.entries.iter().map(|e| e.name_input.value()).collect();
        let ratios: Vec<f64> = self
            .entries
            .iter()
            .map(|e| {
                let cents: f64 = e.cents_input.value::<String>().parse::<i64>().unwrap_or(0) as f64;
                2f64.powf(cents / 1200.0)
            })
            .collect();

        let period_ratio = ratios[n - 1];
        if period_ratio <= 1.0 || !period_ratio.is_finite() {
            self.freq_rows.clear();
            self.freq_scroll_offset = 0;
            return;
        }

        let log_period = period_ratio.ln();
        let start_unison = ((FREQ_MIN / self.base_freq).ln() / log_period).floor() as i32 - 1;
        let end_unison = ((FREQ_MAX / self.base_freq).ln() / log_period).ceil() as i32 + 1;

        let mut rows: Vec<(String, f64, usize)> = Vec::new();

        for unison in start_unison..=end_unison {
            for degree in 0..n {
                let mut freq = self.base_freq * period_ratio.powi(unison);
                if degree > 0 {
                    freq *= ratios[degree - 1];
                }
                if (FREQ_MIN..=FREQ_MAX).contains(&freq) {
                    let name = if degree == 0 {
                        names[n - 1].clone()
                    } else {
                        names[degree - 1].clone()
                    };
                    rows.push((name, freq, degree));
                }
            }
        }

        rows.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.2.cmp(&b.2))
        });

        let n_i32 = n as i32;
        let pivot = rows
            .iter()
            .position(|(_, freq, _)| *freq > REPEAT_NUM_PIVOT_HZ)
            .unwrap_or(rows.len()) as i32;
        self.freq_rows = rows
            .into_iter()
            .enumerate()
            .map(|(j, (name, freq, _))| {
                let repeat_num = REPEAT_NUM_PIVOT + (j as i32 - pivot).div_euclid(n_i32);
                let display_name = format!("{}{}", name, repeat_num);
                let osc_val = hz_to_inc_q16((freq * 65536.0).round() as u32);
                (display_name, freq, osc_val)
            })
            .collect();
        self.freq_scroll_offset = 0;
    }

    fn open_file_explorer(&mut self) {
        let bg = SCHEME.true_dark_color(SCHEME.black[2]);
        let theme = ratatui_explorer::Theme::default().with_block(
            Block::bordered()
                .title(" Select .scl file ")
                .border_style(Style::new().fg(SCHEME.orange[2]))
                .style(Style::new().bg(bg)),
        );
        let result = FileExplorerBuilder::default()
            .filter_map(|file| {
                if file.is_dir {
                    Some(file)
                } else if file.path.extension().is_some_and(|ext| ext == "scl") {
                    Some(file)
                } else {
                    None
                }
            })
            .theme(theme)
            .build();
        if let Ok(explorer) = result {
            self.file_explorer = Some(explorer);
            self.explorer_scroll_offset = 0;
        }
    }

    fn explorer_sync_offset(&mut self) {
        if let Some(ref explorer) = self.file_explorer {
            let selected = explorer.selected_idx();
            let visible_h = self.explorer_area.height.saturating_sub(2) as usize;
            if visible_h == 0 {
                return;
            }
            if selected < self.explorer_scroll_offset {
                self.explorer_scroll_offset = selected;
            } else if selected >= self.explorer_scroll_offset + visible_h {
                self.explorer_scroll_offset = selected + 1 - visible_h;
            }
        }
    }

    fn try_import_scl(&mut self, path: &Path) {
        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".to_string());

        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(_) => {
                self.scl_error = Some(format!("Failed to parse {}", filename));
                return;
            }
        };

        match tune::scala::Scl::import(file) {
            Err(_) => {
                self.scl_error = Some(format!("Failed to parse {}", filename));
            }
            Ok(scl) => {
                self.scl_error = None;
                self.entries.clear();
                let n = scl.num_items() as i32;
                for degree in 1..=n {
                    let cents = scl.relative_pitch_of(degree).as_cents();
                    let name = format!("{}", degree);
                    self.entries.push(SclEntry::new(&name, cents));
                }
                self.selected = 0;
                self.entry_scroll = 0;
                self.editing = false;
                self.name_duplicate = false;
                self.capturing_row = None;
                self.recompute_freq_table();
            }
        }
    }
}

impl Component for TuningEditor {
    fn update(&mut self, events: Vec<Event>, _file: &mut TrackerFile) -> Vec<ComponentAction> {
        let mut actions = Vec::new();

        if self.file_explorer.is_some() {
            let mut close = false;
            let mut import_path: Option<std::path::PathBuf> = None;

            for event in &events {
                match event {
                    Event::Key(KeyEvent {
                        code: KeyCode::Esc,
                        kind: KeyEventKind::Press,
                        ..
                    }) => {
                        close = true;
                        break;
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Enter | KeyCode::Right,
                        kind: KeyEventKind::Press,
                        ..
                    }) => {
                        let explorer = self.file_explorer.as_ref().unwrap();
                        if !explorer.current().is_dir {
                            import_path = Some(explorer.current().path.clone());
                            close = true;
                            break;
                        } else {
                            let _ = self.file_explorer.as_mut().unwrap().handle(event);
                            self.explorer_scroll_offset = 0;
                            self.explorer_sync_offset();
                        }
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Left | KeyCode::Backspace,
                        kind: KeyEventKind::Press,
                        ..
                    }) => {
                        let _ = self.file_explorer.as_mut().unwrap().handle(event);
                        self.explorer_scroll_offset = 0;
                        self.explorer_sync_offset();
                    }
                    Event::Key(_) => {
                        let _ = self.file_explorer.as_mut().unwrap().handle(event);
                        self.explorer_sync_offset();
                    }
                    Event::Mouse(MouseEvent {
                        kind: MouseEventKind::ScrollUp,
                        column,
                        row,
                        ..
                    }) => {
                        if self.explorer_area.contains(Position {
                            x: *column,
                            y: *row,
                        }) {
                            let _ = self
                                .file_explorer
                                .as_mut()
                                .unwrap()
                                .handle(ExplorerInput::Up);
                            self.explorer_sync_offset();
                        }
                    }
                    Event::Mouse(MouseEvent {
                        kind: MouseEventKind::ScrollDown,
                        column,
                        row,
                        ..
                    }) => {
                        if self.explorer_area.contains(Position {
                            x: *column,
                            y: *row,
                        }) {
                            let _ = self
                                .file_explorer
                                .as_mut()
                                .unwrap()
                                .handle(ExplorerInput::Down);
                            self.explorer_sync_offset();
                        }
                    }
                    Event::Mouse(MouseEvent {
                        kind: MouseEventKind::Down(MouseButton::Left),
                        column,
                        row,
                        ..
                    }) => {
                        let area = self.explorer_area;
                        let content_top = area.y + 1;
                        let content_bottom = area.y + area.height.saturating_sub(1);
                        let content_left = area.x + 1;
                        let content_right = area.x + area.width.saturating_sub(1);
                        if *row >= content_top
                            && *row < content_bottom
                            && *column >= content_left
                            && *column < content_right
                        {
                            let content_row = (*row - content_top) as usize;
                            let explorer = self.file_explorer.as_mut().unwrap();
                            let n = explorer.files().len();
                            if n == 0 {
                                continue;
                            }
                            let target = (self.explorer_scroll_offset + content_row).min(n - 1);
                            explorer.set_selected_idx(target);
                            if explorer.current().is_dir {
                                let _ = explorer.handle(ExplorerInput::Right);
                                self.explorer_scroll_offset = 0;
                            } else {
                                import_path = Some(explorer.current().path.clone());
                                close = true;
                                break;
                            }
                            self.explorer_sync_offset();
                        }
                    }
                    _ => {}
                }
            }

            if close {
                self.file_explorer = None;
            }
            if let Some(path) = import_path {
                self.try_import_scl(&path);
            }
            return vec![];
        }

        for event in &events {
            match event {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q') | KeyCode::Char('Q'),
                    kind: KeyEventKind::Press,
                    ..
                }) if !self.editing && !self.base_editing && self.capturing_row.is_none() => {
                    self.visible = false;
                }
                Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if let Some(_cap_row) = self.capturing_row {
                        match code {
                            KeyCode::Esc => {
                                let cap_row = self.capturing_row.take().unwrap();
                                if cap_row < self.freq_rows.len() {
                                    let note_name = &self.freq_rows[cap_row].0.clone();
                                    self.key_assignments.remove(note_name.as_str());
                                }
                            }
                            KeyCode::Char(':') => {
                                // Global hotkey, do nothing
                            }
                            KeyCode::Char(c) if !c.is_control() => {
                                let cap_row = self.capturing_row.take().unwrap();
                                if cap_row < self.freq_rows.len() {
                                    let note_name = self.freq_rows[cap_row].0.clone();
                                    // Remove this key from any other note's binding list
                                    for chars in self.key_assignments.values_mut() {
                                        chars.retain(|k| k != c);
                                    }
                                    self.key_assignments.retain(|_, v| !v.is_empty());
                                    // Add key to this note's binding list
                                    self.key_assignments.entry(note_name).or_default().push(*c);
                                }
                            }
                            _ => {}
                        }
                    } else if self.base_editing {
                        match code {
                            KeyCode::Esc => self.base_freq_restore(),
                            KeyCode::Enter => self.base_freq_confirm(),
                            _ => {
                                let pass = match code {
                                    KeyCode::Char(c) => c.is_ascii_digit(),
                                    _ => true,
                                };
                                if pass {
                                    let state = &mut self.base_freq_input;
                                    state.focus.set(true);
                                    let _ = handle_events(state, true, event);
                                    let v: String = state.value();
                                    if v.len() > 4 {
                                        state.set_value(v.chars().take(4).collect::<String>());
                                    }
                                }
                            }
                        }
                    } else if self.editing {
                        match code {
                            KeyCode::Esc => {
                                self.restore_snapshot();
                            }
                            KeyCode::Enter => {
                                self.confirm_editing();
                            }
                            _ => match self.selected_col {
                                EntryCol::Name => {
                                    let state = &mut self.entries[self.selected].name_input;
                                    state.focus.set(true);
                                    let _ = handle_events(state, true, event);
                                    self.enforce_name_limit();
                                    self.name_duplicate = self.check_name_duplicate();
                                }
                                EntryCol::Cents => {
                                    let pass = match code {
                                        KeyCode::Char(c) => c.is_ascii_digit() || *c == '-',
                                        _ => true,
                                    };
                                    if pass {
                                        let state = &mut self.entries[self.selected].cents_input;
                                        state.focus.set(true);
                                        let _ = handle_events(state, true, event);
                                    }
                                }
                            },
                        }
                    } else if self.base_focused {
                        match code {
                            KeyCode::Up => {
                                self.base_focused = false;
                            }
                            KeyCode::Left => {
                                self.base_col = match self.base_col {
                                    BaseCol::Plus => BaseCol::Input,
                                    BaseCol::Minus => BaseCol::Plus,
                                    BaseCol::Input => BaseCol::Input,
                                };
                            }
                            KeyCode::Right => {
                                self.base_col = match self.base_col {
                                    BaseCol::Input => BaseCol::Plus,
                                    BaseCol::Plus => BaseCol::Minus,
                                    BaseCol::Minus => BaseCol::Minus,
                                };
                            }
                            KeyCode::Enter => match self.base_col {
                                BaseCol::Input => {
                                    self.base_freq_snapshot = self.base_freq_input.value();
                                    self.base_editing = true;
                                }
                                BaseCol::Plus => self.base_freq_increment(),
                                BaseCol::Minus => self.base_freq_decrement(),
                            },
                            _ => {}
                        }
                    } else {
                        match code {
                            KeyCode::Up => {
                                if self.selected > 0 {
                                    self.selected -= 1;
                                }
                            }
                            KeyCode::Down => {
                                if self.selected + 1 < self.entries.len() {
                                    self.selected += 1;
                                } else if !self.entries.is_empty() {
                                    self.base_focused = true;
                                    self.base_col = BaseCol::Input;
                                }
                            }
                            KeyCode::Left => {
                                self.selected_col = EntryCol::Name;
                            }
                            KeyCode::Right => {
                                self.selected_col = EntryCol::Cents;
                            }
                            KeyCode::Enter if !self.entries.is_empty() => {
                                self.start_editing();
                            }
                            _ => {}
                        }
                    }
                }
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column,
                    row,
                    ..
                }) => {
                    let pos = Position {
                        x: *column,
                        y: *row,
                    };
                    if self.open_scl_btn.area.contains(pos) {
                        if self.editing {
                            self.confirm_editing();
                        }
                        if self.base_editing {
                            self.base_freq_confirm();
                        }
                        self.open_file_explorer();
                    } else if self.cancel_button.area.contains(pos) {
                        if self.editing {
                            self.restore_snapshot();
                        }
                        if self.base_editing {
                            self.base_freq_restore();
                        }
                        self.visible = false;
                    } else if self.save_button.area.contains(pos) {
                        if self.editing {
                            self.confirm_editing();
                        }
                        if self.base_editing {
                            self.base_freq_confirm();
                        }
                        actions.push(ComponentAction::TuningSaved(self.get_tuning_data()));
                        self.visible = false;
                    } else if self.base_plus_btn.area.contains(pos) {
                        if self.editing {
                            self.confirm_editing();
                        }
                        if self.base_editing {
                            self.base_freq_confirm();
                        }
                        self.base_focused = true;
                        self.base_col = BaseCol::Plus;
                        self.base_freq_increment();
                    } else if self.base_minus_btn.area.contains(pos) {
                        if self.editing {
                            self.confirm_editing();
                        }
                        if self.base_editing {
                            self.base_freq_confirm();
                        }
                        self.base_focused = true;
                        self.base_col = BaseCol::Minus;
                        self.base_freq_decrement();
                    } else if self.base_freq_input.area.contains(pos) {
                        if self.editing {
                            self.confirm_editing();
                        }
                        self.base_focused = true;
                        self.base_col = BaseCol::Input;
                        if !self.base_editing {
                            self.base_freq_snapshot = self.base_freq_input.value();
                            self.base_editing = true;
                        }
                    } else if self.list_h > 0
                        && *row >= self.list_y
                        && *row < self.list_y + self.list_h
                        && *column < self.freq_panel_area.x
                    {
                        let vis_idx = (*row - self.list_y) as usize;
                        let entry_idx = self.entry_scroll + vis_idx;
                        if entry_idx < self.entries.len() {
                            let in_name = *column >= self.name_col_x
                                && *column < self.name_col_x + NAME_COL_W;
                            let in_cents = *column >= self.cents_col_x
                                && *column < self.cents_col_x + CENTS_COL_W;
                            if in_name || in_cents {
                                if self.editing {
                                    self.confirm_editing();
                                }
                                if self.base_editing {
                                    self.base_freq_confirm();
                                }
                                self.base_focused = false;
                                self.capturing_row = None;
                                self.selected = entry_idx;
                                self.selected_col = if in_cents {
                                    EntryCol::Cents
                                } else {
                                    EntryCol::Name
                                };
                                self.start_editing();
                            }
                        }
                    } else if !self.freq_rows.is_empty()
                        && *column < self.freq_panel_area.right().saturating_sub(1)
                        && *column >= self.freq_panel_area.x
                        && *row > self.freq_panel_area.y
                        && *row < self.freq_panel_area.bottom()
                    {
                        let row_idx =
                            self.freq_scroll_offset + (*row - self.freq_panel_area.y - 1) as usize;
                        if row_idx < self.freq_rows.len() {
                            if self.editing {
                                self.confirm_editing();
                            }
                            if self.base_editing {
                                self.base_freq_confirm();
                            }
                            self.capturing_row = Some(row_idx);
                        }
                    } else {
                        let scrollbar_col = self.freq_panel_area.right().saturating_sub(1);
                        let top_arrow = self.freq_panel_area.y + 1;
                        let bottom_arrow = self.freq_panel_area.bottom().saturating_sub(2);
                        if *column == scrollbar_col {
                            if *row == top_arrow {
                                self.freq_scroll_offset = self.freq_scroll_offset.saturating_sub(1);
                            } else if *row == bottom_arrow {
                                let max = self.freq_rows.len().saturating_sub(1);
                                if self.freq_scroll_offset < max {
                                    self.freq_scroll_offset += 1;
                                }
                            }
                        }
                    }
                }
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::ScrollUp,
                    column,
                    row,
                    ..
                }) => {
                    if self.freq_panel_area.contains(Position {
                        x: *column,
                        y: *row,
                    }) {
                        self.freq_scroll_offset = self.freq_scroll_offset.saturating_sub(1);
                    }
                }
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::ScrollDown,
                    column,
                    row,
                    ..
                }) if self.freq_panel_area.contains(Position {
                    x: *column,
                    y: *row,
                }) =>
                {
                    let max = self.freq_rows.len().saturating_sub(1);
                    if self.freq_scroll_offset < max {
                        self.freq_scroll_offset += 1;
                    }
                }
                _ => {}
            }
        }
        actions
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, _file: &TrackerFile) {
        if area.height < 2 {
            return;
        }
        let modal_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: area.height.saturating_sub(1),
        };

        let bg = SCHEME.true_dark_color(SCHEME.black[2]);
        let title_style = Style::new()
            .fg(SCHEME.white[3])
            .add_modifier(Modifier::BOLD);

        frame.render_widget(Clear, modal_area);

        let block = Block::bordered()
            .title(Line::from(" Tuning editor ").style(title_style))
            .border_style(Style::new().fg(SCHEME.orange[2]))
            .style(Style::new().bg(bg));

        let inner = block.inner(modal_area);
        frame.render_widget(block, modal_area);

        let save_area = Rect {
            x: modal_area.right().saturating_sub(SAVE_BTN_W + 1),
            y: modal_area.y,
            width: SAVE_BTN_W,
            height: 1,
        };
        let cancel_area = Rect {
            x: save_area.x.saturating_sub(BTN_GAP + CANCEL_BTN_W),
            y: modal_area.y,
            width: CANCEL_BTN_W,
            height: 1,
        };

        let default_style = Style::new().bg(bg).fg(SCHEME.white[2]);
        let focus_style = Style::new()
            .bg(SCHEME.orange[3])
            .fg(SCHEME.black[0])
            .add_modifier(Modifier::BOLD);

        self.cancel_button.focus.set(false);
        self.save_button.focus.set(false);
        frame.render_stateful_widget(
            Button::new(Line::from("[Cancel]").style(default_style))
                .style(default_style)
                .focus_style(focus_style),
            cancel_area,
            &mut self.cancel_button,
        );
        frame.render_stateful_widget(
            Button::new(Line::from("[Save]").style(default_style))
                .style(default_style)
                .focus_style(focus_style),
            save_area,
            &mut self.save_button,
        );

        if inner.height == 0 {
            return;
        }

        let has_status = self.scl_error.is_some();
        let [
            _before_btn,
            btn_row,
            _gap1,
            base_freq_row,
            _gap2,
            status_row,
            content_area,
        ] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(if has_status { 1 } else { 0 }),
            Constraint::Fill(1),
        ])
        .areas(inner);

        let open_scl_area = Rect {
            x: btn_row.x,
            y: btn_row.y,
            width: OPEN_SCL_BTN_W.min(btn_row.width),
            height: 1,
        };
        self.open_scl_btn.focus.set(false);
        frame.render_stateful_widget(
            Button::new(Line::from("[Open .scl]").style(default_style))
                .style(default_style)
                .focus_style(focus_style),
            open_scl_area,
            &mut self.open_scl_btn,
        );

        self.render_base_freq(frame, base_freq_row, bg, default_style);

        if has_status {
            let error_style = Style::new()
                .bg(bg)
                .fg(SCHEME.red[2])
                .add_modifier(Modifier::BOLD);
            frame.render_widget(
                Paragraph::new(self.scl_error.as_deref().unwrap_or("")).style(error_style),
                status_row,
            );
        }

        if let Some(explorer) = &self.file_explorer {
            frame.render_widget(Clear, content_area);
            self.explorer_area = content_area;
            let buf = frame.buffer_mut();
            explorer.widget().render_ref(content_area, buf);
            return;
        }

        if content_area.height == 0 {
            return;
        }

        let [left_area, right_area] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(content_area);

        self.freq_panel_area = right_area;
        self.render_entries(frame, left_area, bg, default_style, focus_style);
        self.render_freq_table(frame, right_area, bg, default_style);
    }
}

impl TuningEditor {
    fn render_entries(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        bg: ratatui::style::Color,
        default_style: Style,
        _focus_style: Style,
    ) {
        if self.entries.is_empty() || area.height == 0 {
            return;
        }

        let visible_rows = area.height as usize;
        let offset = self.compute_entry_scroll(visible_rows);
        self.entry_scroll = offset;
        self.list_y = area.y;
        self.list_h = area.height;

        let name_x = area.x + DEGREE_COL_W + 1;
        let cents_x = name_x + NAME_COL_W + 1;
        self.name_col_x = name_x;
        self.cents_col_x = cents_x;

        let selected_style = Style::new()
            .bg(bg)
            .fg(SCHEME.orange[3])
            .add_modifier(Modifier::BOLD);
        let dup_style = Style::new()
            .bg(bg)
            .fg(SCHEME.red[2])
            .add_modifier(Modifier::BOLD);
        let dim_style = Style::new().bg(bg).fg(SCHEME.gray[1]);

        for vis in 0..visible_rows {
            let entry_idx = offset + vis;
            if entry_idx >= self.entries.len() {
                break;
            }

            let row_y = area.y + vis as u16;

            let degree_area = Rect {
                x: area.x,
                y: row_y,
                width: DEGREE_COL_W,
                height: 1,
            };
            let name_area = Rect {
                x: name_x,
                y: row_y,
                width: NAME_COL_W,
                height: 1,
            };
            let cents_area = Rect {
                x: cents_x,
                y: row_y,
                width: CENTS_COL_W.min(area.width.saturating_sub(cents_x - area.x)),
                height: 1,
            };

            let is_selected = entry_idx == self.selected;
            let name_focused = is_selected && self.selected_col == EntryCol::Name;
            let cents_focused = is_selected && self.selected_col == EntryCol::Cents;
            let name_editing = name_focused && self.editing;
            let cents_editing = cents_focused && self.editing;

            let degree_style = if is_selected {
                selected_style
            } else {
                dim_style
            };
            let name_style = if name_focused && self.editing && self.name_duplicate {
                dup_style
            } else if name_focused {
                selected_style
            } else {
                default_style
            };
            let cents_style = if cents_focused {
                selected_style
            } else {
                default_style
            };

            frame.render_widget(
                Paragraph::new(format!("{:>3}", entry_idx + 1)).style(degree_style),
                degree_area,
            );

            self.entries[entry_idx].name_input.focus.set(name_editing);
            self.entries[entry_idx].cents_input.focus.set(cents_editing);

            frame.render_stateful_widget(
                TextInput::new().style(name_style),
                name_area,
                &mut self.entries[entry_idx].name_input,
            );
            frame.render_stateful_widget(
                TextInput::new().style(cents_style),
                cents_area,
                &mut self.entries[entry_idx].cents_input,
            );

            if name_editing {
                if let Some((cx, cy)) = self.entries[entry_idx].name_input.screen_cursor() {
                    frame.set_cursor_position((cx, cy));
                }
            } else if cents_editing
                && let Some((cx, cy)) = self.entries[entry_idx].cents_input.screen_cursor()
            {
                frame.set_cursor_position((cx, cy));
            }
        }
    }

    fn render_freq_table(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        bg: ratatui::style::Color,
        default_style: Style,
    ) {
        if self.freq_rows.is_empty() || area.height == 0 || area.width == 0 {
            return;
        }

        let header_style = Style::new()
            .bg(bg)
            .fg(SCHEME.white[3])
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);

        let header = TableRow::new(vec![
            Cell::from("Note").style(header_style),
            Cell::from("Freq (Hz)").style(header_style),
            Cell::from("Oscillator value").style(header_style),
            Cell::from("Key assign").style(header_style),
        ]);

        let table_area = Rect {
            width: area.width.saturating_sub(1), // account for scrollbar
            ..area
        };

        let data_rows = table_area.height.saturating_sub(1) as usize;
        let offset = self
            .freq_scroll_offset
            .min(self.freq_rows.len().saturating_sub(1));

        let capture_style = Style::new()
            .bg(SCHEME.orange[3])
            .fg(SCHEME.black[0])
            .add_modifier(Modifier::BOLD);

        let visible_rows: Vec<TableRow> = self.freq_rows[offset..]
            .iter()
            .take(data_rows)
            .enumerate()
            .map(|(vis, (name, freq, osc))| {
                let abs_idx = offset + vis;
                let is_capturing = self.capturing_row == Some(abs_idx);
                let row_style = if is_capturing {
                    capture_style
                } else {
                    default_style
                };
                let key_cell = if is_capturing {
                    Cell::from("Press a key...").style(row_style)
                } else {
                    let key_str = self
                        .key_assignments
                        .get(name.as_str())
                        .map(|chars| {
                            chars
                                .iter()
                                .map(|c| c.to_string())
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                        .unwrap_or_default();
                    Cell::from(key_str).style(row_style)
                };
                TableRow::new(vec![
                    Cell::from(name.as_str()).style(row_style),
                    Cell::from(format!("{:.3}", freq)).style(row_style),
                    Cell::from(format!("{}", osc)).style(row_style),
                    key_cell,
                ])
            })
            .collect();

        let table = Table::new(
            visible_rows,
            [
                Constraint::Length(7),
                Constraint::Fill(1),
                Constraint::Length(17),
                Constraint::Length(11),
            ],
        )
        .header(header)
        .style(default_style);

        frame.render_widget(table, table_area);

        let mut sb_state = ScrollbarState::new(self.freq_rows.len()).position(offset);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut sb_state,
        );
    }

    fn render_base_freq(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        bg: ratatui::style::Color,
        default_style: Style,
    ) {
        if area.height == 0 || area.width < 14 {
            return;
        }

        const LABEL_W: u16 = 10;
        const INPUT_W: u16 = 4;
        const BTN_W: u16 = 3;

        let label_area = Rect {
            x: area.x,
            y: area.y,
            width: LABEL_W.min(area.width),
            height: 1,
        };
        let input_x = area.x + LABEL_W;
        let input_area = Rect {
            x: input_x,
            y: area.y,
            width: INPUT_W,
            height: 1,
        };
        let plus_x = input_x + INPUT_W;
        let plus_area = Rect {
            x: plus_x,
            y: area.y,
            width: BTN_W,
            height: 1,
        };
        let minus_x = plus_x + BTN_W;
        let minus_area = Rect {
            x: minus_x,
            y: area.y,
            width: BTN_W,
            height: 1,
        };

        let input_focused = self.base_focused && self.base_col == BaseCol::Input;
        let plus_focused = self.base_focused && self.base_col == BaseCol::Plus;
        let minus_focused = self.base_focused && self.base_col == BaseCol::Minus;

        let input_style = if input_focused {
            Style::new()
                .bg(bg)
                .fg(SCHEME.orange[3])
                .add_modifier(Modifier::BOLD)
        } else {
            default_style
        };
        let focus_style = Style::new()
            .bg(SCHEME.orange[3])
            .fg(SCHEME.black[0])
            .add_modifier(Modifier::BOLD);

        frame.render_widget(
            Paragraph::new("Base freq:").style(default_style),
            label_area,
        );

        self.base_freq_input.focus.set(self.base_editing);
        frame.render_stateful_widget(
            TextInput::new().style(input_style),
            input_area,
            &mut self.base_freq_input,
        );
        if self.base_editing
            && let Some((cx, cy)) = self.base_freq_input.screen_cursor()
        {
            frame.set_cursor_position((cx, cy));
        }

        self.base_plus_btn.focus.set(plus_focused);
        self.base_minus_btn.focus.set(minus_focused);
        frame.render_stateful_widget(
            Button::new(Line::from("[+]").style(default_style))
                .style(default_style)
                .focus_style(focus_style),
            plus_area,
            &mut self.base_plus_btn,
        );
        frame.render_stateful_widget(
            Button::new(Line::from("[-]").style(default_style))
                .style(default_style)
                .focus_style(focus_style),
            minus_area,
            &mut self.base_minus_btn,
        );
    }
}
