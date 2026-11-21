//! Barcode encoders: Code 128 (sets B and C), Code 39, EAN-13 and UPC-A.
//!
//! Every encoder returns a module pattern (`true` = dark) including the quiet
//! zones the standard asks for, so a renderer only has to scale it.
//!
//! Tables are from ISO/IEC 15417 (Code 128), ISO/IEC 16388 (Code 39) and
//! ISO/IEC 15420 (EAN/UPC). The tests check them against reference module
//! strings produced by an independent encoder (reportlab).

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use serde::{Deserialize, Serialize};

/// Supported symbologies. The serde names match what version 0.1 of the app
/// stored, so old settings files still load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Format {
    #[serde(rename = "code128")]
    Code128,
    #[serde(rename = "code39")]
    Code39,
    #[serde(rename = "ean13")]
    Ean13,
    #[serde(rename = "upca")]
    UpcA,
}

impl Format {
    pub const ALL: [Format; 4] = [Format::Code128, Format::Code39, Format::Ean13, Format::UpcA];

    pub fn label(self) -> &'static str {
        match self {
            Format::Code128 => "Code 128",
            Format::Code39 => "Code 39",
            Format::Ean13 => "EAN-13",
            Format::UpcA => "UPC-A",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Format::Code128 => "C128",
            Format::Code39 => "C39",
            Format::Ean13 => "EAN13",
            Format::UpcA => "UPCA",
        }
    }

    pub fn next(self) -> Format {
        match self {
            Format::Code128 => Format::Code39,
            Format::Code39 => Format::Ean13,
            Format::Ean13 => Format::UpcA,
            Format::UpcA => Format::Code128,
        }
    }

    pub fn prev(self) -> Format {
        match self {
            Format::Code128 => Format::UpcA,
            Format::Code39 => Format::Code128,
            Format::Ean13 => Format::Code39,
            Format::UpcA => Format::Ean13,
        }
    }

    /// Characters the format can carry, for help text.
    pub fn charset(self) -> &'static str {
        match self {
            Format::Code128 => "printable ASCII",
            Format::Code39 => "A-Z 0-9 space - . $ / + %",
            Format::Ean13 => "12 digits (+ check digit)",
            Format::UpcA => "11 digits (+ check digit)",
        }
    }
}

/// Why a text can't be encoded in a format. Shown to the user verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    Empty,
    /// A character the format can't represent.
    BadChar { format: Format, ch: char },
    /// Wrong number of digits for EAN-13 / UPC-A.
    BadLength { format: Format, got: usize },
    /// The last digit doesn't match the computed check digit.
    BadCheckDigit { format: Format, expected: u8, got: u8 },
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncodeError::Empty => write!(f, "Nothing to encode"),
            EncodeError::BadChar { format, ch } if ch.is_ascii_lowercase() && *format == Format::Code39 => {
                write!(f, "Code 39 has no lowercase ('{ch}'); use Code 128")
            }
            EncodeError::BadChar { format, ch } => {
                if ch.is_ascii_graphic() {
                    write!(f, "{} can't encode '{}'", format.label(), ch)
                } else {
                    write!(f, "{} can't encode U+{:04X}", format.label(), *ch as u32)
                }
            }
            EncodeError::BadLength { format: Format::Ean13, got } => {
                write!(f, "EAN-13 needs 12 or 13 digits, got {got}")
            }
            EncodeError::BadLength { format, got } => {
                write!(f, "{} needs 11 or 12 digits, got {}", format.label(), got)
            }
            EncodeError::BadCheckDigit { format, expected, got } => {
                write!(f, "{} check digit is {}, not {}", format.label(), expected, got)
            }
        }
    }
}

/// An encoded symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Barcode {
    /// Module pattern including quiet zones: `true` = dark bar.
    pub modules: Vec<bool>,
    /// Human-readable text (with the check digit for EAN/UPC).
    pub text: String,
    pub format: Format,
}

impl Barcode {
    /// Width in modules, quiet zones included.
    pub fn width(&self) -> usize { self.modules.len() }

    /// Dark runs as `(first_module, length)`. Drawing one rectangle per run
    /// instead of per module keeps the number of GAM calls small.
    pub fn dark_runs(&self) -> Vec<(usize, usize)> {
        let mut runs = Vec::new();
        let mut i = 0;
        while i < self.modules.len() {
            if self.modules[i] {
                let start = i;
                while i < self.modules.len() && self.modules[i] {
                    i += 1;
                }
                runs.push((start, i - start));
            } else {
                i += 1;
            }
        }
        runs
    }
}

/// Pick a format from the text alone: 13 digits → EAN-13, 12 digits → UPC-A,
/// Code 39's character set → Code 39, anything else → Code 128.
///
/// EAN/UPC are chosen by length only, so a mistyped check digit is reported
/// rather than silently encoded as some other symbology.
pub fn auto_detect(text: &str) -> Format {
    let all_digits = !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
    if all_digits && text.len() == 13 {
        Format::Ean13
    } else if all_digits && text.len() == 12 {
        Format::UpcA
    } else if !text.is_empty() && text.chars().all(|c| code39_index(c).is_some()) {
        Format::Code39
    } else {
        Format::Code128
    }
}

/// Validate without encoding. Uses the same predicates as [`encode`], so the
/// live "OK / invalid" indicator and Enter can never disagree.
pub fn check(text: &str, format: Format) -> Result<(), EncodeError> { encode(text, format).map(|_| ()) }

/// Encode `text` in `format`.
pub fn encode(text: &str, format: Format) -> Result<Barcode, EncodeError> {
    if text.is_empty() {
        return Err(EncodeError::Empty);
    }
    match format {
        Format::Code128 => encode_code128(text),
        Format::Code39 => encode_code39(text),
        Format::Ean13 => encode_ean13(text),
        Format::UpcA => encode_upca(text),
    }
}

/// Largest whole number of pixels per module, at most `preferred`, at which
/// `modules` fit in `avail_px`. `None` if it doesn't fit even at 1 px.
pub fn fit_module_px(modules: usize, avail_px: usize, preferred: usize) -> Option<usize> {
    if modules == 0 {
        return Some(preferred.max(1));
    }
    let max = avail_px / modules;
    if max == 0 {
        None
    } else {
        Some(max.min(preferred.max(1)))
    }
}

fn push_run(modules: &mut Vec<bool>, dark: bool, n: usize) { modules.extend(core::iter::repeat_n(dark, n)); }

// ─── Code 128 ───────────────────────────────────────────────────────────────

/// Code 128 symbol widths (bar, space, bar, space, bar, space) for values
/// 0..=105, ISO/IEC 15417 Table 1. Each row sums to 11 modules.
pub const CODE128_PATTERNS: [[u8; 6]; 106] = [
    [2, 1, 2, 2, 2, 2], [2, 2, 2, 1, 2, 2], [2, 2, 2, 2, 2, 1], [1, 2, 1, 2, 2, 3], // 0-3
    [1, 2, 1, 3, 2, 2], [1, 3, 1, 2, 2, 2], [1, 2, 2, 2, 1, 3], [1, 2, 2, 3, 1, 2], // 4-7
    [1, 3, 2, 2, 1, 2], [2, 2, 1, 2, 1, 3], [2, 2, 1, 3, 1, 2], [2, 3, 1, 2, 1, 2], // 8-11
    [1, 1, 2, 2, 3, 2], [1, 2, 2, 1, 3, 2], [1, 2, 2, 2, 3, 1], [1, 1, 3, 2, 2, 2], // 12-15
    [1, 2, 3, 1, 2, 2], [1, 2, 3, 2, 2, 1], [2, 2, 3, 2, 1, 1], [2, 2, 1, 1, 3, 2], // 16-19
    [2, 2, 1, 2, 3, 1], [2, 1, 3, 2, 1, 2], [2, 2, 3, 1, 1, 2], [3, 1, 2, 1, 3, 1], // 20-23
    [3, 1, 1, 2, 2, 2], [3, 2, 1, 1, 2, 2], [3, 2, 1, 2, 2, 1], [3, 1, 2, 2, 1, 2], // 24-27
    [3, 2, 2, 1, 1, 2], [3, 2, 2, 2, 1, 1], [2, 1, 2, 1, 2, 3], [2, 1, 2, 3, 2, 1], // 28-31
    [2, 3, 2, 1, 2, 1], [1, 1, 1, 3, 2, 3], [1, 3, 1, 1, 2, 3], [1, 3, 1, 3, 2, 1], // 32-35
    [1, 1, 2, 3, 1, 3], [1, 3, 2, 1, 1, 3], [1, 3, 2, 3, 1, 1], [2, 1, 1, 3, 1, 3], // 36-39
    [2, 3, 1, 1, 1, 3], [2, 3, 1, 3, 1, 1], [1, 1, 2, 1, 3, 3], [1, 1, 2, 3, 3, 1], // 40-43
    [1, 3, 2, 1, 3, 1], [1, 1, 3, 1, 2, 3], [1, 1, 3, 3, 2, 1], [1, 3, 3, 1, 2, 1], // 44-47
    [3, 1, 3, 1, 2, 1], [2, 1, 1, 3, 3, 1], [2, 3, 1, 1, 3, 1], [2, 1, 3, 1, 1, 3], // 48-51
    [2, 1, 3, 3, 1, 1], [2, 1, 3, 1, 3, 1], [3, 1, 1, 1, 2, 3], [3, 1, 1, 3, 2, 1], // 52-55
    [3, 3, 1, 1, 2, 1], [3, 1, 2, 1, 1, 3], [3, 1, 2, 3, 1, 1], [3, 3, 2, 1, 1, 1], // 56-59
    [3, 1, 4, 1, 1, 1], [2, 2, 1, 4, 1, 1], [4, 3, 1, 1, 1, 1], [1, 1, 1, 2, 2, 4], // 60-63
    [1, 1, 1, 4, 2, 2], [1, 2, 1, 1, 2, 4], [1, 2, 1, 4, 2, 1], [1, 4, 1, 1, 2, 2], // 64-67
    [1, 4, 1, 2, 2, 1], [1, 1, 2, 2, 1, 4], [1, 1, 2, 4, 1, 2], [1, 2, 2, 1, 1, 4], // 68-71
    [1, 2, 2, 4, 1, 1], [1, 4, 2, 1, 1, 2], [1, 4, 2, 2, 1, 1], [2, 4, 1, 2, 1, 1], // 72-75
    [2, 2, 1, 1, 1, 4], [4, 1, 3, 1, 1, 1], [2, 4, 1, 1, 1, 2], [1, 3, 4, 1, 1, 1], // 76-79
    [1, 1, 1, 2, 4, 2], [1, 2, 1, 1, 4, 2], [1, 2, 1, 2, 4, 1], [1, 1, 4, 2, 1, 2], // 80-83
    [1, 2, 4, 1, 1, 2], [1, 2, 4, 2, 1, 1], [4, 1, 1, 2, 1, 2], [4, 2, 1, 1, 1, 2], // 84-87
    [4, 2, 1, 2, 1, 1], [2, 1, 2, 1, 4, 1], [2, 1, 4, 1, 2, 1], [4, 1, 2, 1, 2, 1], // 88-91
    [1, 1, 1, 1, 4, 3], [1, 1, 1, 3, 4, 1], [1, 3, 1, 1, 4, 1], [1, 1, 4, 1, 1, 3], // 92-95
    [1, 1, 4, 3, 1, 1], [4, 1, 1, 1, 1, 3], [4, 1, 1, 3, 1, 1], [1, 1, 3, 1, 4, 1], // 96-99
    [1, 1, 4, 1, 3, 1], [3, 1, 1, 1, 4, 1], [4, 1, 1, 1, 3, 1], [2, 1, 1, 4, 1, 2], // 100-103
    [2, 1, 1, 2, 1, 4], [2, 1, 1, 2, 3, 2], // 104-105
];

/// The stop symbol: 2,3,3,1,1,1,2 (13 modules, ends on a bar).
pub const CODE128_STOP: [u8; 7] = [2, 3, 3, 1, 1, 1, 2];

pub const CODE128_CODE_C: u8 = 99;
pub const CODE128_CODE_B: u8 = 100;
pub const CODE128_START_B: u8 = 104;
pub const CODE128_START_C: u8 = 105;
/// Quiet zone on each side, in modules (ISO/IEC 15417 §4.4.1: at least 10X).
pub const CODE128_QUIET: usize = 10;

/// Characters Code 128 set B can carry. Set A (control characters) isn't
/// offered because the keyboard can't type them.
pub fn code128_char_ok(c: char) -> bool { (' '..='~').contains(&c) }

/// Symbol values (start code first, no checksum or stop) for `text`.
///
/// Uses set C for runs of at least four digits (two digits per symbol) and set
/// B otherwise. An odd digit run starting the text keeps its last digit for
/// set B.
pub fn code128_values(text: &str) -> Result<Vec<u8>, EncodeError> {
    if let Some(ch) = text.chars().find(|&c| !code128_char_ok(c)) {
        return Err(EncodeError::BadChar { format: Format::Code128, ch });
    }
    let b = text.as_bytes();
    let digit_run = |from: usize| b[from..].iter().take_while(|c| c.is_ascii_digit()).count();
    let mut values = Vec::with_capacity(b.len() + 4);
    let mut in_c = digit_run(0) >= 4;
    values.push(if in_c { CODE128_START_C } else { CODE128_START_B });
    let mut i = 0;
    while i < b.len() {
        if in_c {
            if i + 1 < b.len() && b[i].is_ascii_digit() && b[i + 1].is_ascii_digit() {
                values.push((b[i] - b'0') * 10 + (b[i + 1] - b'0'));
                i += 2;
            } else {
                values.push(CODE128_CODE_B);
                in_c = false;
            }
        } else if digit_run(i) >= 4 {
            values.push(CODE128_CODE_C);
            in_c = true;
        } else {
            values.push(b[i] - b' ');
            i += 1;
        }
    }
    Ok(values)
}

/// Mod-103 checksum: start value plus each value times its position.
pub fn code128_checksum(values: &[u8]) -> u8 {
    let sum: u32 = values
        .iter()
        .enumerate()
        .map(|(pos, &v)| v as u32 * (pos.max(1) as u32))
        .sum();
    (sum % 103) as u8
}

fn encode_code128(text: &str) -> Result<Barcode, EncodeError> {
    let mut values = code128_values(text)?;
    values.push(code128_checksum(&values));
    let mut modules = Vec::with_capacity(values.len() * 11 + 13 + 2 * CODE128_QUIET);
    push_run(&mut modules, false, CODE128_QUIET);
    for &v in &values {
        for (i, &w) in CODE128_PATTERNS[v as usize].iter().enumerate() {
            push_run(&mut modules, i % 2 == 0, w as usize);
        }
    }
    for (i, &w) in CODE128_STOP.iter().enumerate() {
        push_run(&mut modules, i % 2 == 0, w as usize);
    }
    push_run(&mut modules, false, CODE128_QUIET);
    Ok(Barcode { modules, text: String::from(text), format: Format::Code128 })
}

// ─── Code 39 ────────────────────────────────────────────────────────────────

const CODE39_CHARS: &[u8; 44] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%*";

/// Code 39 element widths: 0 = narrow, 1 = wide, for bar/space/bar/…/bar.
/// Each character has exactly three wide elements (hence the name).
pub const CODE39_PATTERNS: [[u8; 9]; 44] = [
    [0, 0, 0, 1, 1, 0, 1, 0, 0], // 0
    [1, 0, 0, 1, 0, 0, 0, 0, 1], // 1
    [0, 0, 1, 1, 0, 0, 0, 0, 1], // 2
    [1, 0, 1, 1, 0, 0, 0, 0, 0], // 3
    [0, 0, 0, 1, 1, 0, 0, 0, 1], // 4
    [1, 0, 0, 1, 1, 0, 0, 0, 0], // 5
    [0, 0, 1, 1, 1, 0, 0, 0, 0], // 6
    [0, 0, 0, 1, 0, 0, 1, 0, 1], // 7
    [1, 0, 0, 1, 0, 0, 1, 0, 0], // 8
    [0, 0, 1, 1, 0, 0, 1, 0, 0], // 9
    [1, 0, 0, 0, 0, 1, 0, 0, 1], // A
    [0, 0, 1, 0, 0, 1, 0, 0, 1], // B
    [1, 0, 1, 0, 0, 1, 0, 0, 0], // C
    [0, 0, 0, 0, 1, 1, 0, 0, 1], // D
    [1, 0, 0, 0, 1, 1, 0, 0, 0], // E
    [0, 0, 1, 0, 1, 1, 0, 0, 0], // F
    [0, 0, 0, 0, 0, 1, 1, 0, 1], // G
    [1, 0, 0, 0, 0, 1, 1, 0, 0], // H
    [0, 0, 1, 0, 0, 1, 1, 0, 0], // I
    [0, 0, 0, 0, 1, 1, 1, 0, 0], // J
    [1, 0, 0, 0, 0, 0, 0, 1, 1], // K
    [0, 0, 1, 0, 0, 0, 0, 1, 1], // L
    [1, 0, 1, 0, 0, 0, 0, 1, 0], // M
    [0, 0, 0, 0, 1, 0, 0, 1, 1], // N
    [1, 0, 0, 0, 1, 0, 0, 1, 0], // O
    [0, 0, 1, 0, 1, 0, 0, 1, 0], // P
    [0, 0, 0, 0, 0, 0, 1, 1, 1], // Q
    [1, 0, 0, 0, 0, 0, 1, 1, 0], // R
    [0, 0, 1, 0, 0, 0, 1, 1, 0], // S
    [0, 0, 0, 0, 1, 0, 1, 1, 0], // T
    [1, 1, 0, 0, 0, 0, 0, 0, 1], // U
    [0, 1, 1, 0, 0, 0, 0, 0, 1], // V
    [1, 1, 1, 0, 0, 0, 0, 0, 0], // W
    [0, 1, 0, 0, 1, 0, 0, 0, 1], // X
    [1, 1, 0, 0, 1, 0, 0, 0, 0], // Y
    [0, 1, 1, 0, 1, 0, 0, 0, 0], // Z
    [0, 1, 0, 0, 0, 0, 1, 0, 1], // -
    [1, 1, 0, 0, 0, 0, 1, 0, 0], // .
    [0, 1, 1, 0, 0, 0, 1, 0, 0], // space
    [0, 1, 0, 1, 0, 1, 0, 0, 0], // $
    [0, 1, 0, 1, 0, 0, 0, 1, 0], // /
    [0, 1, 0, 0, 0, 1, 0, 1, 0], // +
    [0, 0, 0, 1, 0, 1, 0, 1, 0], // %
    [0, 1, 0, 0, 1, 0, 1, 0, 0], // * (start/stop only)
];

const CODE39_STAR: usize = 43;
pub const CODE39_NARROW: usize = 1;
/// Wide:narrow ratio 3:1 (the spec allows 2:1 to 3:1; 3 scans most reliably).
pub const CODE39_WIDE: usize = 3;
pub const CODE39_QUIET: usize = 10;

/// Index into the Code 39 table for a data character (`*` is excluded: it is
/// the start/stop delimiter and can't appear in data).
pub fn code39_index(c: char) -> Option<usize> {
    if c == '*' || !c.is_ascii() {
        return None;
    }
    CODE39_CHARS.iter().position(|&b| b == c as u8)
}

fn code39_char(modules: &mut Vec<bool>, idx: usize) {
    for (i, &wide) in CODE39_PATTERNS[idx].iter().enumerate() {
        push_run(modules, i % 2 == 0, if wide == 1 { CODE39_WIDE } else { CODE39_NARROW });
    }
}

fn encode_code39(text: &str) -> Result<Barcode, EncodeError> {
    let mut idx = Vec::with_capacity(text.len());
    for ch in text.chars() {
        idx.push(code39_index(ch).ok_or(EncodeError::BadChar { format: Format::Code39, ch })?);
    }
    let mut modules = Vec::new();
    push_run(&mut modules, false, CODE39_QUIET);
    code39_char(&mut modules, CODE39_STAR);
    for i in idx {
        push_run(&mut modules, false, CODE39_NARROW); // inter-character gap
        code39_char(&mut modules, i);
    }
    push_run(&mut modules, false, CODE39_NARROW);
    code39_char(&mut modules, CODE39_STAR);
    push_run(&mut modules, false, CODE39_QUIET);
    Ok(Barcode { modules, text: String::from(text), format: Format::Code39 })
}

// ─── EAN-13 / UPC-A ─────────────────────────────────────────────────────────

/// L-code (odd parity) for digits 0-9 as 7-bit patterns, MSB first.
const EAN_L: [u8; 10] = [0b0001101, 0b0011001, 0b0010011, 0b0111101, 0b0100011, 0b0110001, 0b0101111, 0b0111011, 0b0110111, 0b0001011];
/// G-code (even parity): the L-code reversed and inverted.
const EAN_G: [u8; 10] = [0b0100111, 0b0110011, 0b0011011, 0b0100001, 0b0011101, 0b0111001, 0b0000101, 0b0010001, 0b0001001, 0b0010111];
/// R-code: the L-code inverted.
const EAN_R: [u8; 10] = [0b1110010, 0b1100110, 0b1101100, 0b1000010, 0b1011100, 0b1001110, 0b1010000, 0b1000100, 0b1001000, 0b1110100];
/// Which of the six left digits use G (bit 5 = first left digit), by first digit.
const EAN_PARITY: [u8; 10] = [0b000000, 0b001011, 0b001101, 0b001110, 0b010011, 0b011001, 0b011100, 0b010101, 0b010110, 0b011010];

/// EAN-13 quiet zones (ISO/IEC 15420): 11 modules left, 7 right.
pub const EAN13_QUIET_LEFT: usize = 11;
pub const EAN13_QUIET_RIGHT: usize = 7;
/// UPC-A quiet zones: 9 modules each side.
pub const UPCA_QUIET: usize = 9;

/// GS1 mod-10 check digit for a digit string without its check digit:
/// weight 3 on the digit next to the check digit, alternating with 1.
pub fn gs1_check_digit(digits: &[u8]) -> u8 {
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &d)| d as u32 * if i % 2 == 0 { 3 } else { 1 })
        .sum();
    ((10 - sum % 10) % 10) as u8
}

/// Parse digits for EAN/UPC: `n` digits get a check digit appended; `n + 1`
/// digits must already end in the correct one.
fn gs1_digits(text: &str, format: Format, n: usize) -> Result<Vec<u8>, EncodeError> {
    if let Some(ch) = text.chars().find(|c| !c.is_ascii_digit()) {
        return Err(EncodeError::BadChar { format, ch });
    }
    let mut d: Vec<u8> = text.bytes().map(|b| b - b'0').collect();
    if d.len() == n {
        d.push(gs1_check_digit(&d));
    } else if d.len() == n + 1 {
        let expected = gs1_check_digit(&d[..n]);
        if d[n] != expected {
            return Err(EncodeError::BadCheckDigit { format, expected, got: d[n] });
        }
    } else {
        return Err(EncodeError::BadLength { format, got: d.len() });
    }
    Ok(d)
}

fn push_bits7(modules: &mut Vec<bool>, bits: u8) {
    for k in (0..7).rev() {
        modules.push(bits >> k & 1 == 1);
    }
}

/// The 95 modules of an EAN-13 symbol (guards included, no quiet zones).
fn ean13_body(d: &[u8]) -> Vec<bool> {
    let mut m = Vec::with_capacity(95);
    m.extend_from_slice(&[true, false, true]);
    let parity = EAN_PARITY[d[0] as usize];
    for i in 0..6 {
        let digit = d[i + 1] as usize;
        let g = parity >> (5 - i) & 1 == 1;
        push_bits7(&mut m, if g { EAN_G[digit] } else { EAN_L[digit] });
    }
    m.extend_from_slice(&[false, true, false, true, false]);
    for &digit in &d[7..13] {
        push_bits7(&mut m, EAN_R[digit as usize]);
    }
    m.extend_from_slice(&[true, false, true]);
    m
}

fn digits_text(d: &[u8]) -> String { d.iter().map(|&x| (x + b'0') as char).collect() }

fn encode_ean13(text: &str) -> Result<Barcode, EncodeError> {
    let d = gs1_digits(text, Format::Ean13, 12)?;
    let mut modules = Vec::with_capacity(95 + EAN13_QUIET_LEFT + EAN13_QUIET_RIGHT);
    push_run(&mut modules, false, EAN13_QUIET_LEFT);
    modules.extend(ean13_body(&d));
    push_run(&mut modules, false, EAN13_QUIET_RIGHT);
    Ok(Barcode { modules, text: digits_text(&d), format: Format::Ean13 })
}

fn encode_upca(text: &str) -> Result<Barcode, EncodeError> {
    let d = gs1_digits(text, Format::UpcA, 11)?;
    // UPC-A is EAN-13 with a leading 0 (all-L left half).
    let mut ean = Vec::with_capacity(13);
    ean.push(0);
    ean.extend_from_slice(&d);
    let mut modules = Vec::with_capacity(95 + 2 * UPCA_QUIET);
    push_run(&mut modules, false, UPCA_QUIET);
    modules.extend(ean13_body(&ean));
    push_run(&mut modules, false, UPCA_QUIET);
    Ok(Barcode { modules, text: digits_text(&d), format: Format::UpcA })
}
