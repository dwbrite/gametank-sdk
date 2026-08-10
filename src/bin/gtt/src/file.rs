use std::{io, path::Path};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::tracker::{Beat, Pattern, empty_pattern};

pub const NUM_INSTRUMENTS: usize = 11;

#[derive(Debug, Serialize, Deserialize)]
pub struct InstrumentData {
    pub name: String,
    pub waveform: Vec<u8>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Interval {
    pub name: String,
    pub cents: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningData {
    pub notes: IndexMap<String, f64>,
    #[serde(skip)]
    pub key_assignments: IndexMap<String, Vec<String>>,
    #[serde(default)]
    pub scale: Vec<Interval>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrackerFile {
    pub schema_version: u8,
    pub instruments: [InstrumentData; NUM_INSTRUMENTS],
    #[serde(default = "default_tuning")]
    pub tuning: TuningData,
    #[serde(
        serialize_with = "serialize_patterns",
        deserialize_with = "deserialize_patterns"
    )]
    pub patterns: Vec<Pattern>,
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u8,
}

fn serialize_patterns<S>(patterns: &Vec<Pattern>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(patterns.len()))?;
    for pattern in patterns {
        let as_vecs: Vec<Vec<Beat>> = pattern.iter().map(|row| row.to_vec()).collect();
        seq.serialize_element(&as_vecs)?;
    }
    seq.end()
}

fn deserialize_patterns<'de, D>(deserializer: D) -> Result<Vec<Pattern>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let vecs: Vec<Vec<Vec<Beat>>> = Vec::deserialize(deserializer)?;
    let mut patterns = Vec::new();
    for pattern_vec in vecs {
        if pattern_vec.len() != 9 {
            return Err(D::Error::custom(format!(
                "Expected 9 channels, got {}",
                pattern_vec.len()
            )));
        }
        let mut pattern: Pattern = empty_pattern();
        for (ch_idx, channel) in pattern_vec.iter().enumerate() {
            if channel.len() != 64 {
                return Err(D::Error::custom(format!(
                    "Expected 64 beats in channel {}, got {}",
                    ch_idx,
                    channel.len()
                )));
            }
            for (beat_idx, beat) in channel.iter().enumerate() {
                pattern[ch_idx][beat_idx] = beat.clone();
            }
        }
        patterns.push(pattern);
    }
    Ok(patterns)
}

pub fn default_tuning() -> TuningData {
    let degree_names = [
        "A", "A♯", "B", "C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯",
    ];
    let degree_cents: [f64; 12] = [
        0.0, 100.0, 200.0, 300.0, 400.0, 500.0, 600.0, 700.0, 800.0, 900.0, 1000.0, 1100.0,
    ];

    const BASE_FREQ: f64 = 440.0;
    const PERIOD_RATIO: f64 = 2.0;
    const FREQ_MIN: f64 = 7.0;
    const FREQ_MAX: f64 = 4200.0;
    const PIVOT_HZ: f64 = 510.0;
    const PIVOT_NUM: i32 = 5;
    const N: usize = 12;

    let ln_ratio = PERIOD_RATIO.ln();
    let start_unison = ((FREQ_MIN / BASE_FREQ).ln() / ln_ratio).floor() as i32 - 1;
    let end_unison = ((FREQ_MAX / BASE_FREQ).ln() / ln_ratio).ceil() as i32 + 1;

    let mut rows: Vec<(&str, f64, usize)> = Vec::new();
    for unison in start_unison..=end_unison {
        for degree in 0..N {
            let freq =
                BASE_FREQ * PERIOD_RATIO.powi(unison) * 2.0_f64.powf(degree_cents[degree] / 1200.0);
            if (FREQ_MIN..=FREQ_MAX).contains(&freq) {
                rows.push((degree_names[degree], freq, degree));
            }
        }
    }
    rows.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.2.cmp(&b.2))
    });

    let pivot = rows
        .iter()
        .position(|(_, freq, _)| *freq > PIVOT_HZ)
        .unwrap_or(rows.len()) as i32;

    let mut notes = IndexMap::new();
    for (j, (name, freq, _)) in rows.iter().enumerate() {
        let repeat_num = PIVOT_NUM + (j as i32 - pivot).div_euclid(N as i32);
        notes.insert(format!("{}{}", name, repeat_num), *freq);
    }

    let key_map: &[(&str, char)] = &[
        // Ideally capture key code to make the layout agnostic, but the
        // terminal doesn't pass that infomation through to the application
        // layer. All these are ANSI US QWERTY but reassignable.
        ("C5", 'q'),
        ("C♯5", '2'),
        ("D5", 'w'),
        ("D♯5", '3'),
        ("E5", 'e'),
        ("F5", 'r'),
        ("F♯5", '5'),
        ("G5", 't'),
        ("G♯5", '6'),
        ("A5", 'y'),
        ("A♯5", '7'),
        ("B5", 'u'),
        ("C6", 'i'),
        ("C♯6", '9'),
        ("D6", 'o'),
        ("D♯6", '0'),
        ("E6", 'p'),
        ("F6", '['),
        ("F♯6", '='),
        ("G6", ']'),
        // Shift keys
        ("C6", 'Q'),
        ("C♯6", '@'),
        ("D6", 'W'),
        ("D♯6", '#'),
        ("E6", 'E'),
        ("F6", 'R'),
        ("F♯6", '%'),
        ("G6", 'T'),
        ("G♯6", '^'),
        ("A6", 'Y'),
        ("A♯6", '&'),
        ("B6", 'U'),
        ("C7", 'I'),
        ("C♯7", '('),
        ("D7", 'O'),
        ("D♯7", ')'),
        ("E7", 'P'),
        ("F7", '{'),
        ("F♯7", '+'),
        ("G7", '}'),
    ];
    let mut key_assignments: IndexMap<String, Vec<String>> = IndexMap::new();
    for (name, ch) in key_map.iter() {
        key_assignments
            .entry(name.to_string())
            .or_default()
            .push(ch.to_string());
    }

    let mut scale: Vec<Interval> = (1..N)
        .map(|i| Interval {
            name: degree_names[i].to_string(),
            cents: degree_cents[i],
        })
        .collect();
    scale.push(Interval {
        name: degree_names[0].to_string(),
        cents: 1200.0,
    });

    TuningData {
        notes,
        key_assignments,
        scale,
    }
}

fn default_sample_rate() -> u8 {
    0xD0
}

impl TrackerFile {
    pub fn empty() -> Self {
        let mut square = vec![0xFFu8; 256];
        for b in square[128..].iter_mut() {
            *b = 0x00;
        }
        Self {
            schema_version: 0,
            instruments: std::array::from_fn(|i| InstrumentData {
                name: format!("instrument_{}", i + 1),
                waveform: square.clone(),
            }),
            tuning: default_tuning(),
            patterns: vec![empty_pattern()],
            sample_rate: 0xD0,
        }
    }

    pub fn current_pattern(&self, pattern_idx: u8) -> &Pattern {
        let idx = pattern_idx as usize;
        if idx >= self.patterns.len() {
            &self.patterns[0]
        } else {
            &self.patterns[idx]
        }
    }

    pub fn current_pattern_mut(&mut self, pattern_idx: u8) -> &mut Pattern {
        let idx = pattern_idx as usize;
        while idx >= self.patterns.len() {
            self.patterns.push(empty_pattern());
        }
        &mut self.patterns[idx]
    }

    pub fn instrument_names(&self) -> [String; NUM_INSTRUMENTS] {
        std::array::from_fn(|i| self.instruments[i].name.clone())
    }

    pub fn instrument_waveform(&self, idx: usize) -> [u8; 256] {
        let mut arr = [0u8; 256];
        let src = &self.instruments[idx].waveform;
        let len = src.len().min(256);
        arr[..len].copy_from_slice(&src[..len]);
        arr
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 0 {
            return Err(format!(
                "unsupported schema version {} (expected 0)",
                self.schema_version
            ));
        }
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let bytes = std::fs::read(path)?;
        let file: TrackerFile = rmp_serde::from_slice(&bytes)?;
        file.validate()
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        Ok(file)
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let bytes = rmp_serde::to_vec_named(self).map_err(io::Error::other)?;
        std::fs::write(path, bytes)
    }
}
