pub mod main_menu;
pub mod helpers;
pub mod ui;
pub mod tracker;

use std::{thread::sleep, time::Duration};

use ratatui::{crossterm::event::Event, layout::Rect, DefaultTerminal, Frame};
use anyhow::{bail, Ok, Result};
use crossbeam_channel::unbounded;

use crate::{helpers::poll_events, main_menu::MainMenu};

pub trait Component {
    fn update(&mut self, events: Vec<Event>);
    fn render(&mut self, frame: &mut Frame, area: Rect);
}

pub enum GlobalEvent {
    ChangeInterface(Box<dyn Component>),
    Quit,
}

pub struct GtGo {
    terminal: DefaultTerminal,
    state: Box<dyn Component>,
    rx: crossbeam_channel::Receiver<GlobalEvent>
}

impl GtGo {
    fn run(&mut self) -> Result<()> {
        let _ = self.terminal.draw(|f| {
            let events = poll_events();
            self.state.update(events);
            self.state.render(f, f.area()); // unhandled error
        });

        for event in self.rx.try_iter() {
            match event {
                GlobalEvent::ChangeInterface(component) => self.state = component,
                GlobalEvent::Quit => bail!("Exit"),
            }
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}

fn run(terminal: DefaultTerminal) -> Result<()> {
    let (tx, rx) = crossbeam_channel::unbounded();

    let mut app = GtGo { 
        terminal, 
        state: Box::new(MainMenu::init(tx)),
        rx,
    };

    // Drain any pending terminal input (for example a newline from launching via a
    // shell) so the first update() call doesn't see stale key events.
    let _ = poll_events();
    
    loop {
        sleep(Duration::from_millis(16));
        app.run()?
    }
}
