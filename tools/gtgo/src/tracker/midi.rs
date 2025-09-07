#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MidiNote {
    CNeg1, CsNeg1, DNeg1, DsNeg1, ENeg1, FNeg1, FsNeg1, GNeg1, GsNeg1, ANeg1, AsNeg1, BNeg1, // 0..11
    C0, Cs0, D0, Ds0, E0, F0, Fs0, G0, Gs0, A0, As0, B0,                                     // 12..23
    C1, Cs1, D1, Ds1, E1, F1, Fs1, G1, Gs1, A1, As1, B1,                                     // 24..35
    C2, Cs2, D2, Ds2, E2, F2, Fs2, G2, Gs2, A2, As2, B2,                                     // 36..47
    C3, Cs3, D3, Ds3, E3, F3, Fs3, G3, Gs3, A3, As3, B3,                                     // 48..59
    C4, Cs4, D4, Ds4, E4, F4, Fs4, G4, Gs4, A4, As4, B4,                                     // 60..71
    C5, Cs5, D5, Ds5, E5, F5, Fs5, G5, Gs5, A5, As5, B5,                                     // 72..83
    C6, Cs6, D6, Ds6, E6, F6, Fs6, G6, Gs6, A6, As6, B6,                                     // 84..95
    C7, Cs7, D7, Ds7, E7, F7, Fs7, G7, Gs7, A7, As7, B7,                                     // 96..107
    C8, Cs8, D8, Ds8, E8, F8, Fs8, G8, Gs8, A8, As8, B8,                                     // 108..119
    C9, Cs9, D9, Ds9, E9, F9, Fs9, G9,                                                       // 120..127
    None
}

impl From<u8> for MidiNote {
    fn from(value: u8) -> Self {
        if value <= 127 {
            unsafe { std::mem::transmute(value) }
        } else {
            MidiNote::None
        }
    }
}

impl MidiNote {
    pub fn to_string(&self) -> String {
        if *self == Self::None {
            return String::from("---")
        }

        let v = *self as u8;
        let octave = (v / 12) as i8 - 1;
        let (c0, c1) = match v%12 {
            0  => ('C','-'),
            1  => ('C','♯'),
            2  => ('D','-'),
            3  => ('D','♯'),
            4  => ('E','-'),
            5  => ('F','-'),
            6  => ('F','♯'),
            7  => ('G','-'),
            8  => ('G','♯'),
            9  => ('A','-'),
            10 => ('A','♯'),
            11 => ('B','-'),
            _ => ('?', '?'),
        };

        let o = match octave {
            -1 => "-".to_string(),
            n => format!("{:1}", n)
        };

        let mut s = String::new();
        s.push(c0);
        s.push(c1);
        s.push_str(&o);
        
        s
    }
}
