use std::{collections::HashMap, rc::Rc};

use ratatui::{crossterm::event::{Event, KeyCode, KeyEvent}, layout::Rect, style::{Color, Modifier, Style, Stylize}, symbols::border, text::{Line, Span, Text}, widgets::{Block, BorderType, List, ListState, Padding}, Frame};

use crate::{helpers::SCHEME, Component};

pub struct QmItem {
    label: &'static str,
    enabled: bool,
    active: Rc<Box<dyn Fn()>>,
}

pub fn qi<F>(label: &'static str, enabled: bool, active: F) -> QmItem
where F: Fn() + 'static, {
    QmItem { label, enabled, active: Rc::new(Box::new(active)) }
}


#[derive(Clone)]
struct QuickMenuItem { label: String, enabled: bool, hotkey_idx: usize, active: Rc<Box<dyn Fn() -> ()>> }

impl<'a> Into<Text<'a>> for QuickMenuItem {
    fn into(self) -> Text<'a> {
        let label = self.label;
        let idx = self.hotkey_idx.min(label.len().saturating_sub(1));

        let before = &label[..idx];
        let hot    = &label[idx..=idx];  // just one char
        let after  = &label[idx+1..];

        let spans = vec![
            Span::from(before.to_string()),
            Span::from(hot.to_string()).underlined(),
            Span::from(after.to_string()),
        ];

        let line = Text::from(Line::from(spans));

        if !self.enabled {
            line.style(Style::new().fg(SCHEME.gray[2]))
        } else {
            line
        }
    }
}


#[derive(Debug, Clone, Copy)]
enum Input {
    Selection(usize),
    Quit,
    Enter,
    Up,
    Down,
}

pub struct QuickMenu {
    title: String,
    selection: usize,
    list_items: Vec<QuickMenuItem>,
    bound_keys: HashMap<KeyCode, Input>,
    is_active: bool,
    width: u16,
    height: u16,
}

impl QuickMenu {
    pub fn init(title: String, items: Vec<QmItem>) -> Self {
        let mut bound_keys = HashMap::new();
        bound_keys.insert(KeyCode::Esc, Input::Quit);
        bound_keys.insert(KeyCode::Char('q'), Input::Quit);
        bound_keys.insert(KeyCode::Up, Input::Up);
        bound_keys.insert(KeyCode::Down, Input::Down);
        bound_keys.insert(KeyCode::Enter, Input::Enter);

        let mut list_items = vec![];

        let height = items.len() + 4; // top/bot border, pad

        for QmItem{ label: s, enabled: en, active} in items {
            let idx = s.as_bytes().iter().enumerate()
                .find_map(|(i, &b)| (b == b'_' && (i == 0 || s.as_bytes()[i - 1] != b'\\')).then_some(i))
                .unwrap();
            let hotkey = s[idx + 1..].chars().next();
            let without = format!("{}{}", &s[..idx], &s[idx + 1..]);

            list_items.push(QuickMenuItem { label: without, enabled: en, hotkey_idx: idx, active });
            if let Some(keychar) = hotkey {
                bound_keys.insert(KeyCode::Char(keychar.to_ascii_lowercase()), Input::Selection(list_items.len() - 1));
            }
        }

        Self {
            selection: 0,
            list_items,
            bound_keys,
            is_active: true,
            width: 24,
            height: height as u16,
            title,
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }


    pub fn set_active(&mut self, active: bool) {
        self.is_active = active
    }

    fn move_sel(&mut self, dir: i32) {
        let len = self.list_items.len() as i32;
        let mut i = self.selection as i32;
        for _ in 0..len { // at most one full wrap
            i = (i + dir).rem_euclid(len);
            if self.list_items[i as usize].enabled { break; }
        }
        self.selection = i as usize;
    }
    
    fn select(&self) {
        let x = self.list_items.get(self.selection).unwrap();
        (x.active)();
    }
}


impl Component for QuickMenu {
    fn render(&mut self, frame: &mut Frame, _: Rect) {

        let style = SCHEME.style(Color::Rgb(36, 36, 36));

        let select_border = Block::bordered()
            .title(self.title.clone())
            .title_style(style.gray().bold().not_italic().fg(SCHEME.orange[1]))
            .style(style.fg(SCHEME.orange[1]))
            .padding(Padding::new(1,0,1,1))
            .border_set(border::ROUNDED)
            .border_type(BorderType::Thick);

        let area = frame.area();
        let x = ((area.x + area.width) / 2) - self.width / 2;
        let y = ((area.y + area.height) / 2) - self.height / 2;
        let new_area = Rect::new(x, y, self.width, self.height);

        let list = List::new(self.list_items.clone())
        .highlight_symbol("» ")
        .highlight_style(style.add_modifier(Modifier::SLOW_BLINK).bold())
        .style(style.italic().not_bold())
        .block(select_border);

        let mut state = ListState::default().with_selected(Some(self.selection)); 

        frame.render_stateful_widget(list, new_area, &mut state);
    }

    fn update(&mut self, events: Vec<Event>) {
        for e in events {
            match e {
                Event::Key(KeyEvent { code, .. }) if self.bound_keys.contains_key(&code) => {
                    match self.bound_keys.get(&code).unwrap() {
                        Input::Selection(n) => {
                            if !self.list_items.get(*n).unwrap().enabled {
                                continue; // handle other events
                            }

                            if self.selection == *n {
                                self.select();
                            } else {
                                self.selection = *n
                            }
                        },
                        Input::Quit => self.set_active(false),
                        Input::Enter => {
                            self.select();
                        },
                        Input::Up => {
                            self.move_sel(-1);
                        },
                        Input::Down => {
                            self.move_sel(1);
                        },
                    }
                }
                _ => {}
            }
        }
    }
}

