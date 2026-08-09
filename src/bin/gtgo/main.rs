pub mod main_menu;
pub mod helpers;
pub mod ui;

use std::{process::Command, thread::sleep, time::Duration};

use ratatui::{crossterm::event::Event, layout::Rect, DefaultTerminal, Frame};
use anyhow::{bail, Result};

use crate::{helpers::poll_events, main_menu::MainMenu};

pub trait Component {
    fn update(&mut self, events: Vec<Event>);
    fn render(&mut self, frame: &mut Frame, area: Rect);
}

pub enum GlobalEvent {
    ChangeInterface(Box<dyn Component>),
    LaunchTracker,
    Quit,
}

pub struct GtGo {
    terminal: DefaultTerminal,
    state: Box<dyn Component>,
    rx: crossbeam_channel::Receiver<GlobalEvent>
}

impl GtGo {
    fn draw(&mut self) {
        let _ = self.terminal.draw(|f| {
            let events = poll_events();
            self.state.update(events);
            self.state.render(f, f.area());
        });
    }
}

fn launch_gtt() {
    let sibling = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("gtt")))
        .filter(|p| p.exists());

    if let Some(path) = sibling {
        let _ = Command::new(path).status();
    } else if std::env::var_os("CARGO").is_some() {
        let _ = Command::new("cargo")
            .args(["run", "--bin", "gtt"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status();
    } else {
        let _ = Command::new("gtt").status();
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
        app.draw();

        for event in app.rx.try_iter() {
            match event {
                GlobalEvent::ChangeInterface(component) => app.state = component,
                GlobalEvent::LaunchTracker => {
                    ratatui::restore();
                    launch_gtt();
                    app.terminal = ratatui::init();
                    let _ = poll_events();
                }
                GlobalEvent::Quit => bail!("Exit"),
            }
        }
    }
}
