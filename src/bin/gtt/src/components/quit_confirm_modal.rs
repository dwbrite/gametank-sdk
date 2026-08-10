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

const MODAL_W: u16 = 44;
const MODAL_H: u16 = 5;
const CANCEL_W: u16 = 8;
const NO_W: u16 = 4;
const YES_W: u16 = 5;
const BTN_GAP: u16 = 4;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Btn {
    Cancel,
    No,
    Yes,
}

pub struct QuitConfirmModal {
    pub visible: bool,
    focused: Btn,
    cancel_button: ButtonState,
    no_button: ButtonState,
    yes_button: ButtonState,
}

impl QuitConfirmModal {
    pub fn init() -> Self {
        Self {
            visible: false,
            focused: Btn::Cancel,
            cancel_button: ButtonState::new(),
            no_button: ButtonState::new(),
            yes_button: ButtonState::new(),
        }
    }

    pub fn open(&mut self) {
        self.visible = true;
        self.focused = Btn::Cancel;
    }
}

impl Component for QuitConfirmModal {
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
                    self.focused = match self.focused {
                        Btn::Cancel => Btn::Cancel,
                        Btn::No => Btn::Cancel,
                        Btn::Yes => Btn::No,
                    };
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Right,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    self.focused = match self.focused {
                        Btn::Cancel => Btn::No,
                        Btn::No => Btn::Yes,
                        Btn::Yes => Btn::Yes,
                    };
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    kind: KeyEventKind::Press,
                    ..
                }) => match self.focused {
                    Btn::Cancel => {
                        self.visible = false;
                    }
                    Btn::No => {
                        actions.push(ComponentAction::Quit);
                        self.visible = false;
                    }
                    Btn::Yes => {
                        actions.push(ComponentAction::SaveAndQuit);
                        self.visible = false;
                    }
                },
                Event::Key(KeyEvent {
                    code: KeyCode::Char('n') | KeyCode::Char('N'),
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    actions.push(ComponentAction::Quit);
                    self.visible = false;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('y') | KeyCode::Char('Y'),
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    actions.push(ComponentAction::SaveAndQuit);
                    self.visible = false;
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
                    } else if self.no_button.area.contains(pos) {
                        actions.push(ComponentAction::Quit);
                        self.visible = false;
                    } else if self.yes_button.area.contains(pos) {
                        actions.push(ComponentAction::SaveAndQuit);
                        self.visible = false;
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

        frame.render_widget(Clear, modal_area);

        let block = Block::bordered()
            .border_style(Style::new().fg(SCHEME.orange[2]))
            .style(Style::new().bg(bg));
        let inner = block.inner(modal_area);
        frame.render_widget(block, modal_area);

        if inner.height == 0 {
            return;
        }

        frame.render_widget(
            Paragraph::new(" Save changes before exiting?").style(text_style),
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
        let total_btn_w = CANCEL_W + BTN_GAP + NO_W + BTN_GAP + YES_W;
        let btn_x_start = inner.x + inner.width.saturating_sub(total_btn_w) / 2;

        let cancel_area = Rect {
            x: btn_x_start,
            y: btn_row_y,
            width: CANCEL_W,
            height: 1,
        };
        let no_area = Rect {
            x: btn_x_start + CANCEL_W + BTN_GAP,
            y: btn_row_y,
            width: NO_W,
            height: 1,
        };
        let yes_area = Rect {
            x: btn_x_start + CANCEL_W + BTN_GAP + NO_W + BTN_GAP,
            y: btn_row_y,
            width: YES_W,
            height: 1,
        };

        let cancel_focused = self.focused == Btn::Cancel;
        let no_focused = self.focused == Btn::No;
        let yes_focused = self.focused == Btn::Yes;

        self.cancel_button.focus.set(cancel_focused);
        self.no_button.focus.set(no_focused);
        self.yes_button.focus.set(yes_focused);

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
            Button::new(Line::from("[No]").style(if no_focused {
                focus_style
            } else {
                default_style
            }))
            .style(default_style)
            .focus_style(focus_style),
            no_area,
            &mut self.no_button,
        );
        frame.render_stateful_widget(
            Button::new(Line::from("[Yes]").style(if yes_focused {
                focus_style
            } else {
                default_style
            }))
            .style(default_style)
            .focus_style(focus_style),
            yes_area,
            &mut self.yes_button,
        );
    }
}
