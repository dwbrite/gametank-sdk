use crate::sdk::{scr::SystemControl, video_dma::blitter::BlitterGuard};

#[derive(Copy, Clone)]
pub struct Ball {
    pub x: i8,
    pub y: i8,
    pub vx: i8,
    pub vy: i8,
    pub size: u8,
    pub color: u8,
}

impl Ball {
    pub fn do_ball_things(&mut self) {
        self.x += self.vx;
        self.y += self.vy;
        if self.x >= (128 - self.size) as i8 {
            self.vx = -1;
        }
        if self.x <= 33 {
            self.vx = 1;
        }
        if self.y >= (128 - self.size) as i8 {
            self.vy = -1;
        }
        if self.y <= 1 {
            self.vy = 1;
        }
    }

    pub fn draw(&self, sc: &mut SystemControl, blitter: &mut BlitterGuard) {
        blitter.draw_square(
            sc,
            self.x as u8,
            self.y as u8,
            self.size,
            self.size,
            !self.color,
        );
        blitter.wait_blit();
    }
}

pub fn init_balls() -> [Ball; 6] {
    let base_ball = Ball {
        x: 44,
        y: 19,
        size: 8,
        vx: 1,
        vy: 1,
        color: 0b010_11_100,
    };

    let mut balls = [base_ball; 6];
    for (n, ball) in balls.iter_mut().enumerate() {
        ball.size = (7 - n) as u8;
        ball.x -= n as i8;
        ball.y -= n as i8;
        ball.color += (n as u8) << 5;
    }
    balls
}
