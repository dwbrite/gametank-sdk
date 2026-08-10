use std::path::{Path, PathBuf};

use rat_widget::button::{Button, ButtonState};
use ratatui::{
    Frame,
    crossterm::event::{
        Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind,
    },
    layout::{Margin, Position, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Clear, Scrollbar, ScrollbarOrientation, ScrollbarState, WidgetRef},
};
use ratatui_explorer::{FileExplorer, FileExplorerBuilder, Input as ExplorerInput, Theme};

use crate::{action::ComponentAction, component::Component, file::TrackerFile, scheme::SCHEME};

const CANCEL_BTN_W: u16 = 8;

pub struct FileBrowser {
    pub visible: bool,
    cancel_button: ButtonState,
    explorer_area: Rect,
    explorer_scroll_offset: usize,
    pub file_explorer: Option<FileExplorer>,
}

impl FileBrowser {
    pub fn new() -> Self {
        Self {
            visible: false,
            cancel_button: ButtonState::new(),
            explorer_area: Rect::default(),
            explorer_scroll_offset: 0,
            file_explorer: None,
        }
    }

    pub fn open_file_explorer(&mut self) {
        let theme = Theme::default()
            .with_block(
                Block::bordered()
                    .border_style(Style::new().fg(SCHEME.orange[2]))
                    .title_bottom(
                        Line::from(" <n> New file ").style(
                            Style::new()
                                .fg(SCHEME.white[3])
                                .add_modifier(Modifier::BOLD),
                        ),
                    ),
            )
            .with_style(Style::new().bg(SCHEME.true_dark_color(SCHEME.black[2])))
            .with_highlight_dir_style(
                Style::new()
                    .fg(SCHEME.yellow[2])
                    .add_modifier(Modifier::BOLD),
            )
            .with_dir_style(Style::new().fg(SCHEME.green[2]));

        let result = FileExplorerBuilder::default().theme(theme).build();

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

    fn find_available_filename(&self, dir: &Path) -> PathBuf {
        let base_name = dir.join("track.gtt");
        if !base_name.exists() {
            return base_name;
        }

        for n in 1..10000 {
            let candidate = dir.join(format!("track{}.gtt", n));
            if !candidate.exists() {
                return candidate;
            }
        }

        base_name
    }
}

impl Component for FileBrowser {
    fn update(&mut self, events: Vec<Event>, _file: &mut TrackerFile) -> Vec<ComponentAction> {
        let mut actions = Vec::new();

        if self.file_explorer.is_some() {
            let mut close = false;
            let mut open_path: Option<PathBuf> = None;
            let mut create_new: Option<PathBuf> = None;

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
                        code: KeyCode::Char('n' | 'N'),
                        kind: KeyEventKind::Press,
                        ..
                    }) => {
                        if let Some(ref explorer) = self.file_explorer {
                            let current_dir = if explorer.current().is_dir {
                                explorer.current().path.clone()
                            } else {
                                explorer
                                    .current()
                                    .path
                                    .parent()
                                    .map(|p| p.to_path_buf())
                                    .unwrap_or_else(|| PathBuf::from("."))
                            };
                            let new_file = self.find_available_filename(&current_dir);
                            create_new = Some(new_file);
                            close = true;
                            break;
                        }
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Enter | KeyCode::Right,
                        kind: KeyEventKind::Press,
                        ..
                    }) => {
                        let explorer = self.file_explorer.as_ref().unwrap();
                        if !explorer.current().is_dir {
                            open_path = Some(explorer.current().path.clone());
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
                        let pos = Position {
                            x: *column,
                            y: *row,
                        };
                        if self.cancel_button.area.contains(pos) {
                            close = true;
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if close {
                self.file_explorer = None;
                self.visible = false;
            }

            if let Some(path) = open_path {
                actions.push(ComponentAction::OpenFile(path));
                self.visible = false;
            } else if let Some(path) = create_new {
                actions.push(ComponentAction::CreateNewFile(path));
                self.visible = false;
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
            .title(Line::from(" Open File ").style(title_style))
            .border_style(Style::new().fg(SCHEME.orange[2]))
            .style(Style::new().bg(bg));

        let inner = block.inner(modal_area);
        frame.render_widget(block, modal_area);

        let cancel_area = Rect {
            x: modal_area.right().saturating_sub(CANCEL_BTN_W + 1),
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
        frame.render_stateful_widget(
            Button::new(Line::from("[Cancel]").style(default_style))
                .style(default_style)
                .focus_style(focus_style),
            cancel_area,
            &mut self.cancel_button,
        );

        if let Some(ref explorer) = self.file_explorer {
            self.explorer_area = inner;
            let buf = frame.buffer_mut();
            explorer.widget().render_ref(inner, buf);

            let total_items = explorer.files().len();
            let visible_h = inner.height.saturating_sub(2) as usize;

            if total_items > visible_h {
                let mut sb_state =
                    ScrollbarState::new(total_items).position(self.explorer_scroll_offset);
                frame.render_stateful_widget(
                    Scrollbar::new(ScrollbarOrientation::VerticalRight)
                        .begin_symbol(None)
                        .end_symbol(None),
                    inner.inner(Margin {
                        vertical: 1,
                        horizontal: 0,
                    }),
                    &mut sb_state,
                );
            }
        }
    }
}
