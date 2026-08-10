use rat_widget::button::{Button, ButtonState};
use ratatui::{
    Frame,
    buffer::Buffer,
    crossterm::event::{
        Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind,
    },
    layout::{Position, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Clear},
};

use crate::scheme::SCHEME;
use crate::{action::ComponentAction, component::Component, file::TrackerFile};

const CANCEL_BTN_W: u16 = 8;
const SAVE_BTN_W: u16 = 6;
const BTN_GAP: u16 = 2;
const Y_AXIS_W: u16 = 3;
const BAR_W: u16 = 1;
const NUM_BARS: usize = 256;

#[derive(Default, Clone, Copy)]
struct BarLayout {
    x: u16,
    y: u16,
    h: u16,
    offset: usize,
}

pub struct InstrumentEditor {
    pub visible: bool,
    pub instrument_idx: usize,
    instrument_name: String,
    cancel_button: ButtonState,
    save_button: ButtonState,
    values: [u8; NUM_BARS],
    selected: usize,
    painting: bool,
    bar_layout: BarLayout,
}

impl InstrumentEditor {
    pub fn init() -> Self {
        Self {
            visible: false,
            instrument_idx: 0,
            instrument_name: String::new(),
            cancel_button: ButtonState::new(),
            save_button: ButtonState::new(),
            values: [0u8; NUM_BARS],
            selected: 0,
            painting: false,
            bar_layout: BarLayout::default(),
        }
    }

    pub fn open(&mut self, idx: usize, name: &str, waveform: &[u8; NUM_BARS]) {
        self.visible = true;
        self.selected = 0;
        self.instrument_idx = idx;
        self.instrument_name = name.to_string();
        self.values = *waveform;
    }

    pub fn get_values(&self) -> [u8; NUM_BARS] {
        self.values
    }
}

impl Component for InstrumentEditor {
    fn update(&mut self, events: Vec<Event>, _file: &mut TrackerFile) -> Vec<ComponentAction> {
        let mut actions = Vec::new();
        for event in &events {
            match event {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q') | KeyCode::Char('Q'),
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    self.visible = false;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Left,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if self.selected > 0 {
                        self.selected -= 1;
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Right,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if self.selected < NUM_BARS - 1 {
                        self.selected += 1;
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    self.values[self.selected] = self.values[self.selected].saturating_add(1);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    self.values[self.selected] = self.values[self.selected].saturating_sub(1);
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
                    if self.cancel_button.area.contains(pos) {
                        self.visible = false;
                    } else if self.save_button.area.contains(pos) {
                        let waveform = self.get_values();
                        actions.push(ComponentAction::InstrumentSaved(
                            self.instrument_idx,
                            waveform,
                        ));
                        self.visible = false;
                    } else {
                        self.painting = true;
                        self.handle_bar_click(*column, *row);
                    }
                }
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::Drag(MouseButton::Left),
                    column,
                    row,
                    ..
                }) => {
                    if self.painting {
                        self.handle_bar_click(*column, *row);
                    }
                }
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::Up(MouseButton::Left),
                    ..
                }) => {
                    self.painting = false;
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

        let title_text = format!(
            " Editing {} (instrument {}) ",
            self.instrument_name,
            self.instrument_idx + 1
        );
        let block = Block::bordered()
            .title(Line::from(title_text).style(title_style))
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

        if inner.height > 0 && inner.width > Y_AXIS_W {
            let buf = frame.buffer_mut();
            self.render_bars(buf, inner, bg);
        }
    }
}

impl InstrumentEditor {
    fn handle_bar_click(&mut self, column: u16, row: u16) {
        let BarLayout { x, y, h, offset } = self.bar_layout;
        if h == 0 {
            return;
        }
        if column < x {
            return;
        }
        let vis_idx = ((column - x) / BAR_W) as usize;
        let bar_idx = offset + vis_idx;
        if bar_idx >= NUM_BARS {
            return;
        }
        if row < y || row >= y + h {
            return;
        }
        let rows_from_bottom = (y + h - 1 - row) as u32;
        let value = (rows_from_bottom * 255 / (h - 1).max(1) as u32).min(255) as u8;
        self.values[bar_idx] = value;
        self.selected = bar_idx;
    }

    fn render_bars(&mut self, buf: &mut Buffer, area: Rect, bg: Color) {
        let label_h: u16 = 2;
        let chart_h = area.height.saturating_sub(label_h);
        let chart_x = area.x + Y_AXIS_W;
        let chart_body_w = area.width.saturating_sub(Y_AXIS_W);
        let visible_count = (chart_body_w / BAR_W) as usize;

        if visible_count == 0 || chart_h == 0 {
            return;
        }

        let offset = if self.painting {
            self.bar_layout.offset
        } else if visible_count >= NUM_BARS {
            0
        } else {
            self.selected
                .saturating_sub(visible_count / 2)
                .min(NUM_BARS - visible_count)
        };

        self.bar_layout = BarLayout {
            x: chart_x,
            y: area.y,
            h: chart_h,
            offset,
        };

        let y_label_style = Style::new().bg(bg).fg(SCHEME.white[2]);

        buf.set_string(area.x, area.y, "FF", y_label_style);
        if chart_h > 2 {
            buf.set_string(area.x, area.y + chart_h / 2, "80", y_label_style);
        }
        buf.set_string(area.x, area.y + chart_h - 1, "00", y_label_style);

        for vis_idx in 0..visible_count {
            let bar_idx = offset + vis_idx;
            if bar_idx >= NUM_BARS {
                break;
            }

            let value = self.values[bar_idx];
            let is_selected = bar_idx == self.selected;

            let bar_h = (value as u32 * chart_h as u32 / 255) as u16;
            let bar_x = chart_x + vis_idx as u16 * BAR_W;
            let bar_bg = if is_selected {
                SCHEME.orange[2]
            } else {
                SCHEME.blue[1]
            };
            let bar_style = Style::new().bg(bar_bg).fg(SCHEME.white[3]);
            let empty_style = Style::new().bg(bg);
            let bar_top = area.y + chart_h.saturating_sub(bar_h);

            for row in area.y..area.y + chart_h {
                let s = if row >= bar_top {
                    bar_style
                } else {
                    empty_style
                };
                buf.set_string(bar_x, row, " ", s);
            }

            // Value nibbles stacked vertically inside the bar
            if bar_h >= 1 {
                buf.set_string(
                    bar_x,
                    bar_top,
                    format!("{:X}", value >> 4),
                    bar_style.add_modifier(Modifier::BOLD),
                );
            }
            if bar_h >= 2 {
                buf.set_string(
                    bar_x,
                    bar_top + 1,
                    format!("{:X}", value & 0xF),
                    bar_style.remove_modifier(Modifier::BOLD),
                );
            }

            // Index label nibbles stacked vertically below the bar
            let label_top = area.y + chart_h;
            if label_top + 1 < area.bottom() {
                let label_style = if is_selected {
                    Style::new().bg(bg).fg(SCHEME.orange[3])
                } else {
                    Style::new().bg(bg).fg(SCHEME.gray[1])
                };
                buf.set_string(
                    bar_x,
                    label_top,
                    format!("{:X}", bar_idx >> 4),
                    label_style.add_modifier(Modifier::BOLD),
                );
                buf.set_string(
                    bar_x,
                    label_top + 1,
                    format!("{:X}", bar_idx & 0xF),
                    label_style,
                );
            }
        }
    }
}
