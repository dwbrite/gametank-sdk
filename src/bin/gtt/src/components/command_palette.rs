use rat_widget::text::HasScreenCursor;
use rat_widget::text_input::{TextInput, TextInputState, handle_events};
use ratatui::{
    Frame,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind},
    layout::Rect,
    style::Style,
    text::Span,
    widgets::{Block, Clear},
};

use crate::{
    action::ComponentAction, component::Component, file::TrackerFile, scheme::SCHEME,
    tracker::PATTERN_TABLE_WIDTH,
};

pub struct CommandPalette {
    pub visible: bool,
    input: TextInputState,
}

impl CommandPalette {
    pub fn init() -> Self {
        Self {
            visible: false,
            input: TextInputState::new_focused(),
        }
    }

    fn execute(&mut self) -> Vec<ComponentAction> {
        let mut actions = Vec::new();
        match self.input.value::<String>().trim() {
            "q" | "quit" => actions.push(ComponentAction::Quit),
            "w" | "write" => actions.push(ComponentAction::SaveFile),
            "wq" | "write-quit" => actions.push(ComponentAction::SaveAndQuit),
            "instrument" => actions.push(ComponentAction::OpenInstrumentEditor(0)),
            _ => {}
        }
        self.input.set_value("");
        self.visible = false;
        actions
    }
}

impl Component for CommandPalette {
    fn update(&mut self, events: Vec<Event>, _file: &mut TrackerFile) -> Vec<ComponentAction> {
        let mut actions = Vec::new();
        for event in &events {
            if let Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                ..
            }) = event
            {
                match code {
                    KeyCode::Esc => {
                        self.input.set_value("");
                        self.visible = false;
                        continue;
                    }
                    KeyCode::Enter => {
                        actions.extend(self.execute());
                        continue;
                    }
                    _ => {}
                }
            }
            handle_events(&mut self.input, true, event);
        }
        actions
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, _file: &TrackerFile) {
        let bg = SCHEME.true_dark_color(SCHEME.gray[1]);
        let style = Style::new().bg(bg).fg(SCHEME.white[3]);

        frame.render_widget(Clear, area);
        frame.render_widget(Block::new().style(style), area);

        let input_width = PATTERN_TABLE_WIDTH.min(area.width);
        let input_x = area.x + (area.width.saturating_sub(input_width)) / 2;
        let input_area = Rect {
            x: input_x + 1,
            y: area.y,
            width: input_width,
            height: 1,
        };

        frame.render_widget(
            Span::from(":"),
            Rect {
                x: input_x,
                ..input_area
            },
        );
        let widget = TextInput::new().style(style);
        frame.render_stateful_widget(widget, input_area, &mut self.input);

        if let Some((cx, cy)) = self.input.screen_cursor() {
            frame.set_cursor_position((cx, cy));
        }
    }
}
