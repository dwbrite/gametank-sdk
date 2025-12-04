pub mod pattern_editor;
mod midi;
pub mod lane;

use crossbeam_channel::{Receiver, Sender};
use indexmap::IndexMap;
use rat_widget::{list::selection::RowSelection, table::{selection::CellSelection, textdata::{Cell, Row}, Table, TableData, TableDataIter, TableState}};
use ratatui::{crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers}, layout::{Alignment, Constraint, Direction, Layout, Rect}, style::{Color, Modifier, Style, Stylize}, text::{Line, Span}, widgets::{Block, Borders, Paragraph, Widget}};

use crate::{helpers::SCHEME, main_menu::MainMenu, tracker::{midi::MidiNote, pattern_editor::PatternEditor}, Component, GlobalEvent};

pub struct Handler {
    pub event: Event,
    pub action: Box<dyn Fn()>
}

// tracker subcomponent
pub trait TSub: Component {
    fn active_handlers(&self) -> &Vec<Handler>;
    fn global_handlers(&self) -> &Vec<Handler>;
}

// pub enum Modes {
//     Playback,
//     EditPattern,
//     EditSequence,
// }

#[derive(Clone, Copy)]
pub enum TrackerCmd {
    Quit,
    FocusComponent(Option<usize>),
}

type Pattern = [[Beat; 64]; 9];

fn empty_pattern() -> Pattern {
    std::array::from_fn(|_| std::array::from_fn(|_| Beat::default()))
}

pub enum VoiceOpKind {
    Tremolo,
    Vibrato,
    Note,
    Wavetable,
}

pub enum VoiceOp {
    Tremolo(u8, u8), // volume
    Vibrato(u8, u8), // pitch
    Wavetable(u16), // set wavetable
    Phase(u16), // set phase
    Note(u8), // set note (freq)
    Volume(u8), // volume index (0..=16)
    SlideVol(u8, i16), // how many beats, delta
    StopVSlide,
    SlidePitch(u8, i16), // how many beats, delta
    StopPSlide,
}

pub struct VoiceBeat {
    // idk: IndexMap<>,
}

#[derive(Debug, Default, Clone)]
pub struct Beat {
    cmd_list: Vec<ChannelCmd>,
    sqc_list: Vec<SequencerCmd>
}


#[derive(Debug, Clone)]
pub enum SequencerCmd {
    Tempo(u8), // 0 - 256 in bpm. 60hz * 60s = 3600 / tempo = tick counter.
    Load(u8, u16), // load a wavetable from a pointer?
    Pattern(u8), // change to pattern #
    Beat(u8), // set next beat to beat #
    Advance, // continues to the next pattern in the sequence
    Stop, // stops the sequencer
}

pub enum ChannelFx {
    Tremolo(u8, u8),
    Vibrato(u8, u8),
    
}


#[derive(Debug, Clone)]
pub enum ChannelCmd {
    Tremolo(u8, u8), // volume
    Vibrato(u8, u8), // pitch
    Wavetable(u16), // set wavetable
    Phase(u16), // set phase
    Note(u8), // set note (freq)
    Volume(u8), // volume index (0..=16)
    SlideVol(u8, i16), // how many beats, delta
    StopVSlide,
    SlidePitch(u8, i16), // how many beats, delta
    StopPSlide,
}



pub struct TrackerData {
    beat: u8,
    pattern: u8,
    sequence: u8,

    sequences: [usize; 256], // a sequence is an array of pattern indices
    patterns: Vec<Pattern>,
}

pub struct Tracker {
    tx_main: Sender<GlobalEvent>,
    tr_tx: Sender<TrackerCmd>,
    tr_rx: Receiver<TrackerCmd>,

    selected_subcomponent: Option<usize>,
    subcomponents: Vec<Box<dyn TSub>>,
    handlers: Vec<Handler>,
}

pub fn tx_handler(tx: &Sender<TrackerCmd>, code: KeyCode, cmd: TrackerCmd) -> Handler {
    let txx = tx.clone();
    let cmd = cmd.clone();
    Handler { event: Event::Key(KeyEvent::new(code, KeyModifiers::NONE)), action: Box::new(move || {
        let _ = txx.send(cmd);
    })}
}

impl Tracker {
    pub fn init(tx_main: Sender<GlobalEvent>) -> Self {
        let (tr_tx, tr_rx) = crossbeam_channel::unbounded();

        let mut subcomponents: Vec<Box<dyn TSub>> = vec![
            Box::new(PatternEditor::init(tr_tx.clone())),
        ];

        let handlers = vec![
            tx_handler(&tr_tx, KeyCode::Char('q'), TrackerCmd::Quit),
        ];

        Tracker {
            tx_main,
            tr_tx,
            tr_rx,
            selected_subcomponent: Some(0),
            subcomponents,
            handlers,
        }
    }
}

impl Component for Tracker {
    fn update(&mut self, events: Vec<ratatui::crossterm::event::Event>) {
        for e in &events {
            let handlers = match self.selected_subcomponent {
                Some(selected) => self.subcomponents[selected].active_handlers(),
                None => &self.handlers,
            };

            for h in handlers {
                if h.event == *e {
                    (h.action)()
                }
            }

            for h in self.subcomponents.iter().map(|c| c.global_handlers()).flatten() {
                if h.event == *e {
                    (h.action)()
                }
            }
        }

        for component in &mut self.subcomponents {
            component.update(events.clone());
        }
        
        for cmd in self.tr_rx.try_iter() {
            match cmd {
                TrackerCmd::Quit => {
                    let menu = MainMenu::init(self.tx_main.clone());
                    let _ = self.tx_main.send(GlobalEvent::ChangeInterface(Box::new(menu)));
                },
                TrackerCmd::FocusComponent(c) => {
                    self.selected_subcomponent = c;
                }
            }
        }
    }

    fn render(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(8),
                Constraint::Percentage(100),
            ])
            .split(frame.area());

        let block1 = Block::new()
            .bg(SCHEME.true_dark_color(SCHEME.black[3]))
            .borders(Borders::TOP)
            .title(" Gametank GO! | ☆•° . * . ﾟTRACKER  ﾟ. * . °•☆ ")
            .title_alignment(Alignment::Center)
            .italic()
            .fg(SCHEME.orange[3]);

        let blk = Block::new()
            .bg(SCHEME.true_dark_color(SCHEME.black[0]));
 
        frame.render_widget(block1.clone(), layout[0]);
        frame.render_widget(blk.clone(), layout[1]);

        let ed = &mut self.subcomponents[0];
        ed.render(frame, layout[1]);
    }
}
