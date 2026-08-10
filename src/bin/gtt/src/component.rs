use crate::{action::ComponentAction, file::TrackerFile};
use ratatui::{Frame, crossterm::event::Event, layout::Rect};

pub trait Component {
    fn update(&mut self, events: Vec<Event>, file: &mut TrackerFile) -> Vec<ComponentAction>;
    fn render(&mut self, frame: &mut Frame, area: Rect, file: &TrackerFile);
}
