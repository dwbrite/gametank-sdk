use std::path::PathBuf;

use rat_widget::button::{Button, ButtonState};
use ratatui::{
    Frame,
    crossterm::event::{
        Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind,
    },
    layout::{Position, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Clear, Paragraph},
};

use crate::scheme::SCHEME;
use crate::{action::ComponentAction, component::Component, file::TrackerFile};

const MODAL_W: u16 = 48;
const MODAL_H: u16 = 5;
const CANCEL_W: u16 = 8;
const EXPORT_W: u16 = 8;
const BTN_GAP: u16 = 4;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ModalState {
    Confirm,
    Error,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Btn {
    Cancel,
    Export,
}

pub struct ExportConfirmModal {
    pub visible: bool,
    state: ModalState,
    focused: Btn,
    path: PathBuf,
    cancel_button: ButtonState,
    export_button: ButtonState,
}

impl ExportConfirmModal {
    pub fn init() -> Self {
        Self {
            visible: false,
            state: ModalState::Confirm,
            focused: Btn::Cancel,
            path: PathBuf::new(),
            cancel_button: ButtonState::new(),
            export_button: ButtonState::new(),
        }
    }

    pub fn open(&mut self, path: PathBuf) {
        self.visible = true;
        self.state = ModalState::Confirm;
        self.focused = Btn::Cancel;
        self.path = path;
    }

    pub fn open_error(&mut self, path: PathBuf) {
        self.visible = true;
        self.state = ModalState::Error;
        self.focused = Btn::Cancel;
        self.path = path;
    }

    pub fn set_error(&mut self) {
        self.state = ModalState::Error;
        self.focused = Btn::Cancel;
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Component for ExportConfirmModal {
    fn update(&mut self, events: Vec<Event>, _file: &mut TrackerFile) -> Vec<ComponentAction> {
        let mut actions = Vec::new();
        for event in &events {
            match event {
                Event::Key(KeyEvent {
                    code: KeyCode::Esc,
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
                    if self.state == ModalState::Confirm {
                        self.focused = Btn::Cancel;
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Right,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if self.state == ModalState::Confirm {
                        self.focused = Btn::Export;
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    kind: KeyEventKind::Press,
                    ..
                }) => match (self.state, self.focused) {
                    (_, Btn::Cancel) => {
                        self.visible = false;
                    }
                    (ModalState::Confirm, Btn::Export) => {
                        actions.push(ComponentAction::ConfirmExport(self.path.clone()));
                    }
                    (ModalState::Error, Btn::Export) => {}
                },
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
                    } else if self.export_button.area.contains(pos)
                        && self.state == ModalState::Confirm
                    {
                        actions.push(ComponentAction::ConfirmExport(self.path.clone()));
                    }
                }
                _ => {}
            }
        }
        actions
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, _file: &TrackerFile) {
        let modal_x = area.x + area.width.saturating_sub(MODAL_W) / 2;
        let modal_y = area.y + area.height.saturating_sub(MODAL_H) / 2;
        let modal_area = Rect {
            x: modal_x,
            y: modal_y,
            width: MODAL_W.min(area.width),
            height: MODAL_H.min(area.height),
        };

        let bg = SCHEME.true_dark_color(SCHEME.black[2]);
        let default_style = Style::new().bg(bg).fg(SCHEME.white[2]);
        let focus_style = Style::new()
            .bg(SCHEME.orange[3])
            .fg(SCHEME.black[0])
            .add_modifier(Modifier::BOLD);
        let text_style = Style::new().bg(bg).fg(SCHEME.white[3]);
        let error_style = Style::new().bg(bg).fg(SCHEME.red[2]);

        frame.render_widget(Clear, modal_area);

        let block = Block::bordered()
            .border_style(Style::new().fg(SCHEME.orange[2]))
            .style(Style::new().bg(bg));
        let inner = block.inner(modal_area);
        frame.render_widget(block, modal_area);

        if inner.height == 0 {
            return;
        }

        let (msg, msg_style) = match self.state {
            ModalState::Confirm => (" Directory already exists. Overwrite?", text_style),
            ModalState::Error => (" Could not write directory.", error_style),
        };

        frame.render_widget(
            Paragraph::new(msg).style(msg_style),
            Rect {
                x: inner.x,
                y: inner.y,
                width: inner.width,
                height: 1,
            },
        );

        if inner.height < 3 {
            return;
        }

        let btn_row_y = inner.y + 2;

        let cancel_focused = self.focused == Btn::Cancel;
        let export_focused = self.focused == Btn::Export;

        self.cancel_button.focus.set(cancel_focused);
        self.export_button.focus.set(export_focused);

        match self.state {
            ModalState::Confirm => {
                let total_btn_w = CANCEL_W + BTN_GAP + EXPORT_W;
                let btn_x_start = inner.x + inner.width.saturating_sub(total_btn_w) / 2;

                let cancel_area = Rect {
                    x: btn_x_start,
                    y: btn_row_y,
                    width: CANCEL_W,
                    height: 1,
                };
                let export_area = Rect {
                    x: btn_x_start + CANCEL_W + BTN_GAP,
                    y: btn_row_y,
                    width: EXPORT_W,
                    height: 1,
                };

                frame.render_stateful_widget(
                    Button::new(Line::from("[Cancel]").style(if cancel_focused {
                        focus_style
                    } else {
                        default_style
                    }))
                    .style(default_style)
                    .focus_style(focus_style),
                    cancel_area,
                    &mut self.cancel_button,
                );
                frame.render_stateful_widget(
                    Button::new(Line::from("[Export]").style(if export_focused {
                        focus_style
                    } else {
                        default_style
                    }))
                    .style(default_style)
                    .focus_style(focus_style),
                    export_area,
                    &mut self.export_button,
                );
            }
            ModalState::Error => {
                let cancel_area = Rect {
                    x: inner.x + inner.width.saturating_sub(CANCEL_W) / 2,
                    y: btn_row_y,
                    width: CANCEL_W,
                    height: 1,
                };
                frame.render_stateful_widget(
                    Button::new(Line::from("[Cancel]").style(if cancel_focused {
                        focus_style
                    } else {
                        default_style
                    }))
                    .style(default_style)
                    .focus_style(focus_style),
                    cancel_area,
                    &mut self.cancel_button,
                );
            }
        }
    }
}
