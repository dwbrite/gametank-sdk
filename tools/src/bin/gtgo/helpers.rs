use std::time::Duration;

use rat_theme::Scheme;
use ratatui::{crossterm::event::{self, Event}, layout::{Constraint, Direction, Layout, Rect}};

pub fn centered_rect(pct_x: u16, pct_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - pct_y) / 2),
            Constraint::Percentage(pct_y),
            Constraint::Percentage((100 - pct_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - pct_x) / 2),
            Constraint::Percentage(pct_x),
            Constraint::Percentage((100 - pct_x) / 2),
        ])
        .split(vertical[1])[1]
}



pub fn poll_events() -> Vec<Event> {
    let mut events = vec![];
    while let Ok(true) = event::poll(Duration::from_millis(0)) {
        if let Ok(e) = event::read() {
            events.push(e)
        }
    }

    events
}

pub const SCHEME: rat_theme::Scheme = rat_theme::scheme::MONEKAI;