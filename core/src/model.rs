//! Settings and saved codes: what the app persists, with validation and the
//! migration from the 0.1 storage layout.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use serde::{Deserialize, Serialize};

use crate::encode::Format;

/// Bump when the stored format changes.
pub const STATE_VERSION: u32 = 1;

pub const MIN_BAR_WIDTH: u8 = 1;
pub const MAX_BAR_WIDTH: u8 = 4;
pub const MIN_BAR_HEIGHT: u16 = 40;
pub const MAX_BAR_HEIGHT: u16 = 300;
pub const BAR_HEIGHT_STEP: u16 = 20;
/// Longest text the input field accepts.
pub const MAX_TEXT_LEN: usize = 80;
/// Longest name for a saved code.
pub const MAX_NAME_LEN: usize = 24;

fn default_width() -> u8 { 2 }
fn default_height() -> u16 { 160 }
fn default_true() -> bool { true }
fn default_format() -> Format { Format::Code128 }

/// User settings. Field names match the 0.1 layout so old records load;
/// a missing `version` means 0.1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub version: u32,
    /// Preferred pixels per module. The renderer uses less if the code won't fit.
    #[serde(default = "default_width")]
    pub bar_width: u8,
    #[serde(default = "default_height")]
    pub bar_height: u16,
    /// Format used when auto-detect is off.
    #[serde(default = "default_format")]
    pub format: Format,
    #[serde(default = "default_true")]
    pub auto_format: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: STATE_VERSION,
            bar_width: default_width(),
            bar_height: default_height(),
            format: default_format(),
            auto_format: true,
        }
    }
}

impl Settings {
    /// Clamp values from storage into range (a corrupt or hand-edited record
    /// must not produce zero-width bars).
    pub fn sanitized(mut self) -> Self {
        self.version = STATE_VERSION;
        self.bar_width = self.bar_width.clamp(MIN_BAR_WIDTH, MAX_BAR_WIDTH);
        self.bar_height = self.bar_height.clamp(MIN_BAR_HEIGHT, MAX_BAR_HEIGHT);
        self
    }

    pub fn wider(&mut self) { self.bar_width = (self.bar_width + 1).min(MAX_BAR_WIDTH); }

    pub fn narrower(&mut self) { self.bar_width = self.bar_width.saturating_sub(1).max(MIN_BAR_WIDTH); }

    pub fn taller(&mut self) { self.bar_height = (self.bar_height + BAR_HEIGHT_STEP).min(MAX_BAR_HEIGHT); }

    pub fn shorter(&mut self) {
        self.bar_height = self.bar_height.saturating_sub(BAR_HEIGHT_STEP).max(MIN_BAR_HEIGHT);
    }
}

/// One saved code. `Debug` hides the payload: saved codes may hold
/// credentials or serial numbers, and nothing should print them to a log.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedCode {
    pub name: String,
    pub text: String,
    pub format: Format,
}

impl fmt::Debug for SavedCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SavedCode")
            .field("name", &self.name)
            .field("text", &"<redacted>")
            .field("format", &self.format)
            .finish()
    }
}

/// Why a code couldn't be saved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveError {
    EmptyName,
    /// Another saved code already has this name (case-insensitive).
    Duplicate,
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::EmptyName => write!(f, "Name can't be empty"),
            SaveError::Duplicate => write!(f, "A code with that name exists"),
        }
    }
}

/// All saved codes, stored as one record so a delete really removes the
/// payload (the record is rewritten without it).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Codes {
    pub version: u32,
    pub codes: Vec<SavedCode>,
}

impl Codes {
    pub fn new() -> Self { Self { version: STATE_VERSION, codes: Vec::new() } }

    pub fn len(&self) -> usize { self.codes.len() }

    pub fn is_empty(&self) -> bool { self.codes.is_empty() }

    pub fn get(&self, i: usize) -> Option<&SavedCode> { self.codes.get(i) }

    pub fn contains_name(&self, name: &str) -> bool {
        self.codes.iter().any(|c| c.name.eq_ignore_ascii_case(name.trim()))
    }

    /// Add a code. Names are trimmed and must be unique.
    pub fn add(&mut self, name: &str, text: &str, format: Format) -> Result<(), SaveError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(SaveError::EmptyName);
        }
        if self.contains_name(name) {
            return Err(SaveError::Duplicate);
        }
        self.codes.push(SavedCode { name: String::from(name), text: String::from(text), format });
        Ok(())
    }

    /// Remove and return a code.
    pub fn remove(&mut self, i: usize) -> Option<SavedCode> {
        if i < self.codes.len() {
            Some(self.codes.remove(i))
        } else {
            None
        }
    }

    /// One list line per code: name, format and a short preview.
    pub fn label(&self, i: usize) -> String {
        let c = &self.codes[i];
        let mut preview: String = c.text.chars().take(14).collect();
        if c.text.chars().count() > 14 {
            preview.push('…');
        }
        alloc::format!("{} [{}] {}", c.name, c.format.short(), preview)
    }

    /// Refuse records written by a newer app.
    pub fn check_version(&self) -> Result<(), u32> {
        if self.version > STATE_VERSION {
            Err(self.version)
        } else {
            Ok(())
        }
    }
}

/// What the `index` key may hold: the current single record, or the 0.1
/// layout's list of names (each payload then lives in its own `code.<name>` key).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum IndexRecord {
    Current(Codes),
    Legacy(Vec<String>),
}

/// A 0.1 `code.<name>` record.
#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct LegacyCode {
    pub text: String,
    #[serde(default = "default_format")]
    pub format: Format,
}

/// PDDB key of a 0.1 payload record.
pub fn legacy_code_key(name: &str) -> String { alloc::format!("code.{name}") }

/// True for every key the 0.1 layout used for payloads, including orphans
/// left behind by its broken delete.
pub fn is_legacy_code_key(key: &str) -> bool { key.starts_with("code.") }

/// Build the current record from legacy names and their loaded payloads.
/// Names whose payload is missing are dropped; duplicate names keep the first.
pub fn migrate_legacy(entries: Vec<(String, Option<LegacyCode>)>) -> Codes {
    let mut codes = Codes::new();
    for (name, payload) in entries {
        if let Some(p) = payload {
            let _ = codes.add(&name, &p.text, p.format);
        }
    }
    codes
}

/// A one-line text field with a length limit and a character filter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextField {
    text: String,
    max: usize,
}

impl TextField {
    pub fn new(max: usize) -> Self { Self { text: String::new(), max } }

    pub fn as_str(&self) -> &str { &self.text }

    pub fn is_empty(&self) -> bool { self.text.is_empty() }

    pub fn len(&self) -> usize { self.text.chars().count() }

    pub fn max(&self) -> usize { self.max }

    pub fn clear(&mut self) { self.text.clear(); }

    pub fn set(&mut self, s: &str) {
        self.text.clear();
        for c in s.chars() {
            self.push(c);
        }
    }

    /// Append a printable ASCII character. Returns false if it was refused.
    pub fn push(&mut self, c: char) -> bool {
        if (c.is_ascii_graphic() || c == ' ') && self.len() < self.max {
            self.text.push(c);
            true
        } else {
            false
        }
    }

    /// Delete the last character. Returns false if the field was empty.
    pub fn pop(&mut self) -> bool { self.text.pop().is_some() }
}
