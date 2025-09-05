use crossbeam_channel::{Receiver, Sender};
use ratatui::{crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers}, layout::{Alignment, Constraint, Direction, Layout}, style::Stylize, widgets::{Block, Borders}};

use crate::{helpers::SCHEME, main_menu::MainMenu, Component, GlobalEvent};

pub struct Handler {
    pub event: Event,
    pub action: Box<dyn Fn()>
}

// tracker subcomponent
pub trait TSub: Component {
    fn global_handlers(&self) -> Vec<Handler>;
}

pub enum TrackerCmd {
    Quit,
}

pub struct Tracker {
    tx_main: Sender<GlobalEvent>,
    tr_tx: Sender<TrackerCmd>,
    tr_rx: Receiver<TrackerCmd>,
    subcomponents: Vec<Box<dyn TSub>>,
    handlers: Vec<Handler>,
}

impl Tracker {
    pub fn init(tx_main: Sender<GlobalEvent>) -> Self {
        let (tr_tx, tr_rx) = crossbeam_channel::unbounded();

        let mut subcomponents = Vec::new();

        let tx1 = tr_tx.clone();
        let tx2 = tr_tx.clone();
        let handlers = vec![
            Handler {
                event: Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)), 
                action: Box::new(move || {
                    let _ = tx1.send(TrackerCmd::Quit);
                })
            },
            Handler {
                event: Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)), 
                action: Box::new(move || {
                    let _ = tx2.send(TrackerCmd::Quit);
                })
            },
        ];

        Tracker {
            tx_main,
            tr_tx,
            tr_rx,
            subcomponents,
            handlers,
        }
    }
}

impl Component for Tracker {
    fn update(&mut self, events: Vec<ratatui::crossterm::event::Event>) {
        for e in events {
            // TODO: combine iterators
            for h in &self.handlers {
                if h.event == e {
                    (h.action)()
                }
            }

            for h in self.subcomponents.iter().map(|c| c.global_handlers()).flatten() {
                if h.event == e {
                    (h.action)()
                }
            }
        }
        
        for cmd in self.tr_rx.try_recv() {
            match cmd {
                TrackerCmd::Quit => {
                    let menu = MainMenu::init(self.tx_main.clone());
                    let _ = self.tx_main.send(GlobalEvent::ChangeInterface(Box::new(menu)));
                },
            }
        }
    }

    fn render(&mut self, frame: &mut ratatui::Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(5),
                Constraint::Percentage(100),
            ])
            .split(frame.area());
        
        let block1 = Block::new()
            .bg(SCHEME.black[1])
            .borders(Borders::TOP)
            .title(" Gametank GO! | ☆•° . * . ﾟTRACKER  ﾟ. * . °•☆ ")
            .title_alignment(Alignment::Center)
            .italic()
            .fg(SCHEME.orange[3]);

        let block2 = Block::new().style(SCHEME.reduced_white(0));        
        
        frame.render_widget(block1.clone(), layout[0]);
        frame.render_widget(block2, layout[1]);
    }
}
