use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::file::default_tuning;

#[derive(Debug, Serialize, Deserialize)]
pub struct BindingsConfig {
    pub key_assignments: IndexMap<String, String>,
}

impl Default for BindingsConfig {
    fn default() -> Self {
        let mut key_assignments = IndexMap::new();
        for (note, keys) in default_tuning().key_assignments {
            for key in keys {
                key_assignments.insert(key, note.clone());
            }
        }
        Self { key_assignments }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GttConfig {
    pub schema_version: u32,
    pub bindings: BindingsConfig,
}

impl Default for GttConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            bindings: BindingsConfig::default(),
        }
    }
}

/// Convert config storage format (key_char → note_name) to tuning format (note_name → Vec<key_char>).
pub fn config_to_tuning_keys(config: &IndexMap<String, String>) -> IndexMap<String, Vec<String>> {
    let mut result: IndexMap<String, Vec<String>> = IndexMap::new();
    for (key, note) in config {
        result.entry(note.clone()).or_default().push(key.clone());
    }
    result
}

/// Convert tuning format (note_name → Vec<key_char>) to config storage format (key_char → note_name).
pub fn build_key_assignments(tuning: &IndexMap<String, Vec<String>>) -> IndexMap<String, String> {
    let mut result = IndexMap::new();
    for (note, keys) in tuning {
        for key in keys {
            result.insert(key.clone(), note.clone());
        }
    }
    result
}
