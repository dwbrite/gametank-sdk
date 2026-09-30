use std::path::PathBuf;

use ratatui::{
    Frame,
    crossterm::event::{
        Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind,
    },
    layout::{Position, Rect},
    style::{Color, Style},
    widgets::{Block, Clear, WidgetRef},
};
use ratatui_explorer::{FileExplorer, FileExplorerBuilder, Input as ExplorerInput, Theme};

/// Result of feeding one input event to an open [`FilePicker`].
pub enum PickerOutcome {
    /// Keep browsing.
    None,
    /// The user cancelled (Esc). The picker has already closed itself.
    Cancelled,
    /// The user picked a file. The picker has already closed itself.
    Selected(PathBuf),
}

/// Reusable file-selection popup backed by `ratatui_explorer`, shared by any
/// modal that needs to import a file (instrument waveforms, tuning `.scl`
/// files, etc).
#[derive(Default)]
pub struct FilePicker {
    explorer: Option<FileExplorer>,
    area: Rect,
    scroll_offset: usize,
}

impl FilePicker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        self.explorer.is_some()
    }

    pub fn open(&mut self, title: &str, border_color: Color, bg: Color, extension: &'static str) {
        let theme = Theme::default().with_block(
            Block::bordered()
                .title(title.to_string())
                .border_style(Style::new().fg(border_color))
                .style(Style::new().bg(bg)),
        );
        let result = FileExplorerBuilder::default()
            .filter_map(move |file| {
                if file.is_dir || file.path.extension().is_some_and(|ext| ext == extension) {
                    Some(file)
                } else {
                    None
                }
            })
            .theme(theme)
            .build();
        if let Ok(explorer) = result {
            self.explorer = Some(explorer);
            self.scroll_offset = 0;
        }
    }

    pub fn close(&mut self) {
        self.explorer = None;
    }

    fn sync_offset(&mut self) {
        if let Some(ref explorer) = self.explorer {
            let selected = explorer.selected_idx();
            let visible_h = self.area.height.saturating_sub(2) as usize;
            if visible_h == 0 {
                return;
            }
            if selected < self.scroll_offset {
                self.scroll_offset = selected;
            } else if selected >= self.scroll_offset + visible_h {
                self.scroll_offset = selected + 1 - visible_h;
            }
        }
    }

    /// Feed one input event to the picker. Only meaningful while `is_active()`.
    pub fn handle_event(&mut self, event: &Event) -> PickerOutcome {
        let Some(explorer) = self.explorer.as_mut() else {
            return PickerOutcome::None;
        };
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Esc,
                kind: KeyEventKind::Press,
                ..
            }) => {
                self.close();
                return PickerOutcome::Cancelled;
            }
            Event::Key(KeyEvent {
                code: KeyCode::Enter | KeyCode::Right,
                kind: KeyEventKind::Press,
                ..
            }) => {
                if !explorer.current().is_dir {
                    let path = explorer.current().path.clone();
                    self.close();
                    return PickerOutcome::Selected(path);
                } else {
                    let _ = explorer.handle(event);
                    self.scroll_offset = 0;
                    self.sync_offset();
                }
            }
            Event::Key(KeyEvent {
                code: KeyCode::Left | KeyCode::Backspace,
                kind: KeyEventKind::Press,
                ..
            }) => {
                let _ = explorer.handle(event);
                self.scroll_offset = 0;
                self.sync_offset();
            }
            Event::Key(_) => {
                let _ = explorer.handle(event);
                self.sync_offset();
            }
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column,
                row,
                ..
            }) => {
                if self.area.contains(Position {
                    x: *column,
                    y: *row,
                }) {
                    let _ = explorer.handle(ExplorerInput::Up);
                    self.sync_offset();
                }
            }
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column,
                row,
                ..
            }) => {
                if self.area.contains(Position {
                    x: *column,
                    y: *row,
                }) {
                    let _ = explorer.handle(ExplorerInput::Down);
                    self.sync_offset();
                }
            }
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column,
                row,
                ..
            }) => {
                let area = self.area;
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
                    let n = explorer.files().len();
                    if n == 0 {
                        return PickerOutcome::None;
                    }
                    let target = (self.scroll_offset + content_row).min(n - 1);
                    explorer.set_selected_idx(target);
                    if explorer.current().is_dir {
                        let _ = explorer.handle(ExplorerInput::Right);
                        self.scroll_offset = 0;
                    } else {
                        let path = explorer.current().path.clone();
                        self.close();
                        return PickerOutcome::Selected(path);
                    }
                    self.sync_offset();
                }
            }
            _ => {}
        }
        PickerOutcome::None
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(explorer) = &self.explorer {
            frame.render_widget(Clear, area);
            self.area = area;
            let buf = frame.buffer_mut();
            explorer.widget().render_ref(area, buf);
        }
    }
}
