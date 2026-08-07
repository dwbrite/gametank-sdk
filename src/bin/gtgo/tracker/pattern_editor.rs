use crossbeam_channel::{Receiver, Sender};
use rat_widget::table::{selection::RowSelection, textdata::{Cell, Row}, Table, TableData, TableState};
use ratatui::{crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers}, layout::{Constraint, Direction, Layout, Rect}, style::{Modifier, Style, Stylize}, text::{Line, Span}, widgets::Widget};

use crate::{helpers::SCHEME, tracker::{empty_pattern, lane::{Lane, LaneKind}, midi::MidiNote, Beat, ChannelCmd, Handler, Pattern, TSub, TrackerCmd, TrackerData}, Component};

#[derive(Clone, Copy)]
pub enum PatternEvent {
    Up,
    Down,
    Left,
    Right,
    Quit,
    Enter,
    SmallIncrement,
    SmallDecrement
}

pub struct PatternEditor {
    pub sel_x: u8,
    pub sel_y: u8,

    pub scroll: i8,
    lanes: Vec<Lane>,
    tracker_data: TrackerData,
    active_handlers: Vec<Handler>,
    global_handlers: Vec<Handler>,
    cx_rx: Receiver<PatternEvent>,
    #[allow(dead_code)]
    cx_tx: Sender<PatternEvent>,
    par_tx: Sender<TrackerCmd>,
}


pub fn tx_handler(tx: &Sender<PatternEvent>, code: KeyCode, cmd: PatternEvent) -> Handler {
    let txx = tx.clone();
    let cmd = cmd.clone();
    Handler { event: Event::Key(KeyEvent::new(code, KeyModifiers::NONE)), action: Box::new(move || {
        let _ = txx.send(cmd);
    })}
}

impl PatternEditor {
    pub fn init(parent_tx: Sender<TrackerCmd>) -> Self {
        let (cx_tx, cx_rx) = crossbeam_channel::unbounded();

        let handlers = vec![
            tx_handler(&cx_tx, KeyCode::Esc, PatternEvent::Quit),
            tx_handler(&cx_tx, KeyCode::Char('q'), PatternEvent::Quit),
            tx_handler(&cx_tx, KeyCode::Up, PatternEvent::Up),
            tx_handler(&cx_tx, KeyCode::Down, PatternEvent::Down),
            tx_handler(&cx_tx, KeyCode::Left, PatternEvent::Left),
            tx_handler(&cx_tx, KeyCode::Right, PatternEvent::Right),
            tx_handler(&cx_tx, KeyCode::Char('j'), PatternEvent::SmallIncrement),
            tx_handler(&cx_tx, KeyCode::Char('k'), PatternEvent::SmallDecrement),
        ];

        Self {
            scroll: -8,
            lanes: vec![
                Lane::beat(),
                Lane::seq(),
                Lane::note(0), Lane::vol(0), Lane::fx(0),
                Lane::note(1), Lane::vol(1), Lane::fx(1),
                Lane::note(2), Lane::vol(2), Lane::fx(2),
                Lane::note(3), Lane::vol(3), Lane::fx(3),
                Lane::note(4), Lane::vol(4), Lane::fx(4),
                Lane::note(5), Lane::vol(5), Lane::fx(5),
                Lane::note(6), Lane::vol(6), Lane::fx(6),
                Lane::note(7), Lane::vol(7), Lane::fx(7),
            ],
            tracker_data: TrackerData {
                beat: 0,
                pattern: 0,
                sequence: 0,
                sequences: [0; 256],
                patterns: vec![empty_pattern()],
            },
            sel_x: 2,
            sel_y: 2,
            active_handlers: handlers,
            cx_rx,
            cx_tx,
            par_tx: parent_tx,
            global_handlers: vec![], // mostly for mouse events ig
        }
    }

    pub fn current_pattern(&self) -> &Pattern {
        &self.tracker_data.patterns[self.tracker_data.pattern as usize]
    }

    pub fn current_pattern_mut(&mut self) -> &mut Pattern {
        &mut self.tracker_data.patterns[self.tracker_data.pattern as usize]
    }

    fn get_channel_beat(ch: Option<usize>, beat: u8, pattern: &Pattern) -> &Beat {
        match ch {
            Some(n) => &pattern[n+1][beat as usize],
            None => &pattern[0][beat as usize],
        }
    }

    pub fn get_selected_beat(&mut self) -> Option<&mut Beat> {
        // TODO: this is gonna confuse the SHIT out of people
        let beat_idx = self.sel_y as usize;
        let lane = &self.lanes[self.sel_x as usize];
        let ch_idx = match lane.kind {
            LaneKind::Beat => None,
            LaneKind::Seq => Some(0),
            _ => lane.ch,
        }?;

        Some(&mut self.current_pattern_mut()[ch_idx][beat_idx])
    }

    pub fn get_cell(&self, row: usize, column: usize) -> CellDisplay {
        let lane = &self.lanes[column];
        let pattern = self.current_pattern();

        // wrapping add i8->u8 can essentially subtraction
        let y = (row as u8).wrapping_add(self.scroll as u8);
        let ym64 = y % 64;

        match lane.kind {
            LaneKind::Beat => {
                CellDisplay::BeatNum(ym64)
            },
            LaneKind::Seq => {
                let beat = Self::get_channel_beat(lane.ch, ym64, pattern);
                let ct = beat.sqc_list.len();
                CellDisplay::SeqCmds(ct)
            },
            LaneKind::Note => {
                let beat = Self::get_channel_beat(lane.ch, ym64, pattern);
                let note = beat.cmd_list.iter().find_map(|c| match c {
                    ChannelCmd::Note(num) => Some(MidiNote::from(*num)),
                    _ => None,
                }).unwrap_or(MidiNote::None);
                CellDisplay::Note(note)
            },
            LaneKind::Vol => {
                let beat = Self::get_channel_beat(lane.ch, ym64, pattern);
                let vol = beat.cmd_list.iter().find_map(|c| match c {
                        ChannelCmd::Volume(v) => Some(*v),
                        _ => None,
                    });
                CellDisplay::Vol(vol)
            }
            LaneKind::Fx => {
                let beat = Self::get_channel_beat(lane.ch, ym64, pattern);
                let n = beat.cmd_list.iter().filter(|c| 
                    !matches!(c, ChannelCmd::Note(_) | ChannelCmd::Volume(_)))
                    .count()
                    .min(0xF) as u8;
                CellDisplay::Fx(n)
            }
        }
    }
}

impl <'a> TableData<'a> for &mut PatternEditor {
    fn rows(&self) -> usize {
        64
    }

    fn widths(&self) -> Vec<Constraint> {
        self.lanes.iter().map(|lane| Constraint::Length(lane.width)).collect()
    }


    fn header(&self) -> Option<rat_widget::table::textdata::Row<'a>> {
        let c = [
            SCHEME.red[3],
            SCHEME.orange[3],
            SCHEME.yellow[3],
            SCHEME.green[3],
            SCHEME.deepblue[3],
            SCHEME.blue[3],
            SCHEME.purple[3],
            SCHEME.magenta[3],
        ];

        let mut cells = vec![];

        for lane in &self.lanes {
            let cell = Cell::new(match lane.kind {
                LaneKind::Beat => Span::from(lane.title.clone()),
                LaneKind::Seq => Span::from(lane.title.clone()),
                LaneKind::Note => Span::from(lane.title.clone()).fg(c[lane.ch.unwrap()]).italic(),
                LaneKind::Vol => Span::from(lane.title.clone()).fg(c[lane.ch.unwrap()]),
                LaneKind::Fx => Span::from(lane.title.clone()).fg(c[lane.ch.unwrap()]),
            });
            cells.push(cell);
        }

        Some(Row::new(cells))
    }

    fn render_cell(
        &self,
        _ctx: &rat_widget::table::TableContext,
        column: usize,
        row: usize,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        let lane = &self.lanes[column].clone();
        let offset = row as i8 + self.scroll;

        let row_even = offset % 2 == 0;
        let is_active = (0..64).contains(&offset);
        let row_selected = row == (self.sel_y as i8 - self.scroll) as usize;
        let col_selected = column == self.sel_x as usize;

        let cell = self.get_cell(row, column);
        
        let style = if row_selected {
            if col_selected {
                CellStyle::SelectedCell
            } else {
                CellStyle::SelectedRow
            }
        } else if row_even {
            CellStyle::EvenRow
        } else {
            CellStyle::OddRow
        };

        let spans = cell.spans(lane, style, is_active);

        let line = Line::from(spans);
        line.render(area, buf);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellStyle {
    EvenRow,
    OddRow,
    SelectedRow,
    SelectedCell,
    Bar,
}

pub enum CellDisplay {
    BeatNum(u8), // cell number & is_active
    SeqCmds(usize), // 0 is ---, n is [n]
    Note(MidiNote),
    Vol(Option<u8>), // 0..=16 (no change is -)
    Fx(u8), // fx count, 0 is ---, n is [n]
}

impl CellDisplay {
    fn text(&self) -> String {
        match self {
            CellDisplay::BeatNum(beat) => format!("   {:02X}", beat),
            CellDisplay::SeqCmds(n) => match n {
                0 => "---".to_string(),
                n => format!("[{:1x}]", n),
            },
            CellDisplay::Note(midi_note) => midi_note.to_string(),
            CellDisplay::Vol(maybe_set) => match maybe_set {
                Some(v) => format!("{:1x}", v),
                None => "-".to_string(),
            },
            CellDisplay::Fx(n) => match n {
                0 => "---".to_string(),
                n => format!("[{:1x}]", n),
            }
        }
    }

    fn style(&self, cell_style: CellStyle, active_pattern: bool) -> Style {
        let black = SCHEME.true_dark_color(SCHEME.black[0]);
        let mut style = SCHEME.style(black);

        let (fg, modifiers)  = match self {
            CellDisplay::BeatNum(_) => (SCHEME.deepblue[2], Modifier::ITALIC),
            CellDisplay::SeqCmds(v) => (match v {
                0 => SCHEME.reduced_text_color(SCHEME.white[1]),
                _ => SCHEME.reduced_text_color(SCHEME.white[1]),
            }, Modifier::empty()),
            CellDisplay::Note(midi_note) => (match midi_note {
                MidiNote::None => SCHEME.gray[1],
                _ => SCHEME.orange[1],
            }, Modifier::empty()),
            CellDisplay::Vol(v) => (match v {
                None => SCHEME.gray[0],
                Some(_) => SCHEME.magenta[0],
            }, Modifier::empty()),
            CellDisplay::Fx(n) => (match n {
                0 => SCHEME.gray[0],
                _ => SCHEME.yellow[1],
            }, Modifier::empty()),
        };

        style = style.fg(fg).add_modifier(modifiers);

        // SCHEME.true_dark_color(SCHEME.black[0]);
        let (row_bg, add_modifiers) = match cell_style {
            CellStyle::EvenRow => (SCHEME.true_dark_color(SCHEME.black[3]), Modifier::empty()),
            CellStyle::OddRow => (SCHEME.true_dark_color(SCHEME.black[0]), Modifier::empty()),
            CellStyle::SelectedRow => (SCHEME.true_dark_color(SCHEME.blue[0]), Modifier::empty()),
            CellStyle::SelectedCell => {
                style = style.fg(SCHEME.deepblue[1]);
                (SCHEME.true_dark_color(SCHEME.blue[3]), Modifier::SLOW_BLINK | Modifier::REVERSED)
            },
            CellStyle::Bar => todo!(),
        };

        let style = style.bg(row_bg).add_modifier(add_modifiers);

        if active_pattern {
            style
        } else {
            style.fg(SCHEME.true_dark_color(SCHEME.white[2]))
        }
    }

    fn spans(&'_ self, lane: &Lane, style: CellStyle, is_active: bool) -> Vec<Span<'_>> {
        let (left_pad, right_pad) = lane.padding;

        let mut pre  = Span::from(" ".repeat(left_pad as usize));
        let mut post = Span::from(" ".repeat(right_pad as usize));

        if style == CellStyle::SelectedCell {
            pre = pre.style(self.style(CellStyle::SelectedRow, is_active));
            post = post.style(self.style(CellStyle::SelectedRow, is_active));
        } else {
            pre = pre.style(self.style(style, is_active));
            post = post.style(self.style(style, is_active));
        }

        let val = Span::from(self.text()).style(self.style(style, is_active));

        vec![pre, val, post]
    }

}

impl Component for PatternEditor {
    fn update(&mut self, _events: Vec<Event>) {
        let (lane_kind, ch) = {
            let lane = &self.lanes[self.sel_x as usize];
            let kind = lane.kind;
            let ch = lane.ch;
            (kind, ch)
        };
        let sel_beat = self.sel_y as usize;

        while let Ok(event) = self.cx_rx.try_recv() {
            match event {
                PatternEvent::Up => self.sel_y -= 1,
                PatternEvent::Down => self.sel_y += 1,
                PatternEvent::Left => self.sel_x -= 1,
                PatternEvent::Right => self.sel_x += 1,
                PatternEvent::Enter => todo!(),
                PatternEvent::Quit => { let _ = self.par_tx.send(TrackerCmd::FocusComponent(None)); },
                PatternEvent::SmallIncrement => {
                    if let Some(channel) = ch {
                        let beat = &mut self.current_pattern_mut()[channel+1][sel_beat];
                        match lane_kind {
                            LaneKind::Note => {
                                let found = beat.cmd_list.iter_mut().rev().find_map(|c| match c {
                                    ChannelCmd::Note(n) => {
                                        *n = n.saturating_add(1).min(127); Some(())
                                    }
                                    _ => None,
                                });
                                if found.is_none() {
                                    beat.cmd_list.push(ChannelCmd::Note(MidiNote::C4 as u8));
                                }
                            }
                            LaneKind::Vol => todo!(),
                            _ => { println!("wrong col!") }
                        }
                    }
                }
                PatternEvent::SmallDecrement => todo!(),
            }
        }
    }

    fn render(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        let table_width = self.lanes.iter().map(|l| l.width).sum();
        let lower_layouts = Layout::default().constraints([
            Constraint::Fill(1),
            // TODO: use widths and sum them from
            Constraint::Length(table_width),
            Constraint::Fill(1),
        ]).direction(Direction::Horizontal).split(area);

        let widths = self.widths();

        let table = Table::default()
            .data(self)
            .style(SCHEME.true_dark_black(0).fg(SCHEME.white[0]))
            .widths(widths);

        let mut ts = TableState::<RowSelection>::default();        
        frame.render_stateful_widget(table, lower_layouts[1], &mut ts);
    }
}

impl TSub for PatternEditor {
    fn active_handlers(&self) -> &Vec<Handler> {
        &self.active_handlers
    }
    
    fn global_handlers(&self) -> &Vec<Handler> {
        &self.global_handlers
    }
}