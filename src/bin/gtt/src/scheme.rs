use ratatui::style::{Color, Style};

#[derive(Debug, Clone, Copy)]
pub struct Scheme {
    pub white: [Color; 4],
    pub black: [Color; 4],
    pub gray: [Color; 4],
    pub red: [Color; 4],
    pub orange: [Color; 4],
    pub yellow: [Color; 4],
    pub green: [Color; 4],
    pub blue: [Color; 4],
    pub deepblue: [Color; 4],
    pub purple: [Color; 4],
    pub magenta: [Color; 4],
}

impl Scheme {
    pub const fn linear4(c0: u32, c1: u32) -> [Color; 4] {
        const fn i1(a: u8, b: u8) -> u8 {
            if a < b {
                a + (b - a) / 3
            } else {
                a - (a - b) / 3
            }
        }
        const fn i2(a: u8, b: u8) -> u8 {
            if a < b {
                b - (b - a) / 3
            } else {
                b + (a - b) / 3
            }
        }
        let r0 = (c0 >> 16) as u8;
        let g0 = (c0 >> 8) as u8;
        let b0 = c0 as u8;
        let r3 = (c1 >> 16) as u8;
        let g3 = (c1 >> 8) as u8;
        let b3 = c1 as u8;
        [
            Color::Rgb(r0, g0, b0),
            Color::Rgb(i1(r0, r3), i1(g0, g3), i1(b0, b3)),
            Color::Rgb(i2(r0, r3), i2(g0, g3), i2(b0, b3)),
            Color::Rgb(r3, g3, b3),
        ]
    }

    pub fn true_dark_color(&self, color: Color) -> Color {
        if let Color::Rgb(r, g, b) = color {
            Color::Rgb(
                (r as u16 * 63 / 255) as u8,
                (g as u16 * 63 / 255) as u8,
                (b as u16 * 63 / 255) as u8,
            )
        } else {
            color
        }
    }

    pub fn true_dark_black(&self, n: usize) -> Style {
        let dark = self.true_dark_color(self.black[n]);
        Style::new().bg(dark).fg(self.text_color(dark))
    }

    pub fn style(&self, color: Color) -> Style {
        Style::new().bg(color).fg(self.text_color(color))
    }

    fn rate_text_color(&self, color: Color) -> Option<bool> {
        match color {
            Color::Rgb(r, g, b) => {
                let grey = r as f32 * 0.3 + g as f32 * 0.59 + b as f32 * 0.11;
                Some(grey < 105.0)
            }
            Color::Reset => None,
            _ => Some(true),
        }
    }

    pub fn text_color(&self, color: Color) -> Color {
        match self.rate_text_color(color) {
            None => Color::Reset,
            Some(true) => self.white[3],
            Some(false) => self.black[0],
        }
    }

    pub fn reduced_text_color(&self, color: Color) -> Color {
        match self.rate_text_color(color) {
            None => Color::Reset,
            Some(true) => self.white[0],
            Some(false) => self.black[3],
        }
    }
}

pub const SCHEME: Scheme = Scheme {
    white: Scheme::linear4(0xb0b2a8, 0xf5f4f1),
    black: Scheme::linear4(0x272822, 0x464741),
    gray: Scheme::linear4(0x4d4e48, 0x64655f),
    red: Scheme::linear4(0x804c10, 0xfd971f),
    orange: Scheme::linear4(0x584180, 0xae81ff),
    yellow: Scheme::linear4(0x80643d, 0xf4bf75),
    green: Scheme::linear4(0x628043, 0x96c367),
    blue: Scheme::linear4(0x2f668c, 0x51afef),
    deepblue: Scheme::linear4(0x5e748c, 0x81a1c1),
    purple: Scheme::linear4(0x764980, 0xb26fc1),
    magenta: Scheme::linear4(0x80133a, 0xf92672),
};
