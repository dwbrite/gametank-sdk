

#[derive(Debug, Clone, Copy)]
pub enum LaneKind {
    Beat,
    Seq,
    Note,
    Vol,
    Fx,
}


#[derive(Clone)]
pub struct Lane {
    pub title: String,
    pub padding: (u8, u8), // padding left/right for non-title cells
    pub width: u16,
    pub kind: LaneKind,
    pub ch: Option<usize>
}

impl Lane {
    pub fn beat() -> Self {
        Self {
            title: " BEAT".to_string(),
            padding: (0, 2),
            width: 7,
            kind: LaneKind::Beat,
            ch: None            
        }
    }

    pub fn seq() -> Self {
        Self {
            title: "SEQ".to_string(),
            padding: (0, 0),
            width: 3,
            kind: LaneKind::Seq,
            ch: None
        }
    }

    pub fn note(ch: u8) -> Self {
        Self {
            title: format!("  ch{ch}"),
            padding: (2, 0),
            width: 5,
            kind: LaneKind::Note,
            ch: Some(ch as usize)
        }
    }

    pub fn vol(ch: u8) -> Self {
        Self {
            title: " v ".to_string(),
            padding: (1, 1),
            width: 3,
            kind: LaneKind::Vol,
            ch: Some(ch as usize)
        }
    }

    pub fn fx(ch: u8) -> Self {
        Self {
            title: ":↗↘ ".to_string(),
            padding: (0, 1),
            width: 4,
            kind: LaneKind::Fx,
            ch: Some(ch as usize)
        }
    }
}