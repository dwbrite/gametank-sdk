use crossbeam_channel::Sender;
use ratatui::{crossterm::event::Event, layout::Rect, style::{Color, Stylize}, symbols::border, widgets::{Block, Widget}, Frame};

use crate::{helpers::SCHEME, tracker::Tracker, ui::quickmenu::{qi, QuickMenu}, Component, GlobalEvent};

#[allow(dead_code)]
pub struct MainMenu {
    has_podman: bool,
    quit: bool,
    qm: QuickMenu,
    tx: Sender<GlobalEvent>
}

impl MainMenu {
    pub fn init(tx_main: Sender<GlobalEvent>) -> Self {
        // TODO: if has podman
        let has_podman = false;

        let txx = tx_main.clone();

        let qm = QuickMenu::init(" Program Select ".to_string(), vec![
            qi("_Emulator", true, || { todo!() }),
            qi("_Tracker", true, move || {
                let tracker = Tracker::init(txx.clone());
                let _ = txx.send(GlobalEvent::ChangeInterface(Box::new(tracker))); 
            }),
            qi("_Build", has_podman, || { println!("ur mom") }),
            qi("ROM _Flasher", true, || { todo!() }),
        ]);

        Self {
            has_podman,
            quit: false,
            qm,
            tx: tx_main,
        }
    }
}


impl Component for MainMenu {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .border_set(border::ROUNDED)
            .title("─ GameTank GO! ")
            .title_style(SCHEME.style(Color::Rgb(36, 36, 36)).italic().bold());
        block.render(frame.area(), frame.buffer_mut());
        self.qm.render(frame, area);
    }
    
    fn update(&mut self, events: Vec<Event>) {
        self.qm.update(events);

        if !self.qm.is_active() {
            let _ = self.tx.send(GlobalEvent::Quit);
        }
    }
}
