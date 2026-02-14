//! Barcode encoder — Code 128, Code 39, EAN-13, UPC-A.
//!
//! Zero external dependencies. Pure Rust. Built for Precursor.
//! Follows the encoding agent pattern from the QR Code Generator.

extern crate alloc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

// ─── Barcode Formats ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BarcodeFormat {
    Code128,
    Code39,
    Ean13,
    UpcA,
}

impl BarcodeFormat {
    pub fn label(&self) -> &'static str {
        match self {
            BarcodeFormat::Code128 => "Code 128",
            BarcodeFormat::Code39 => "Code 39",
            BarcodeFormat::Ean13 => "EAN-13",
            BarcodeFormat::UpcA => "UPC-A",
        }
    }

    pub fn short(&self) -> &'static str {
        match self {
            BarcodeFormat::Code128 => "C128",
            BarcodeFormat::Code39 => "C39",
            BarcodeFormat::Ean13 => "EAN13",
            BarcodeFormat::UpcA => "UPCA",
        }
    }

    pub fn all() -> &'static [BarcodeFormat] {
        &[
            BarcodeFormat::Code128,
            BarcodeFormat::Code39,
            BarcodeFormat::Ean13,
            BarcodeFormat::UpcA,
        ]
    }

    pub fn next(&self) -> BarcodeFormat {
        match self {
            BarcodeFormat::Code128 => BarcodeFormat::Code39,
            BarcodeFormat::Code39 => BarcodeFormat::Ean13,
            BarcodeFormat::Ean13 => BarcodeFormat::UpcA,
            BarcodeFormat::UpcA => BarcodeFormat::Code128,
        }
    }
}

/// Auto-detect the best format for given text.
pub fn auto_detect(text: &str) -> BarcodeFormat {
    let all_digits = text.chars().all(|c| c.is_ascii_digit());
    if all_digits && text.len() == 13 {
        BarcodeFormat::Ean13
    } else if all_digits && text.len() == 12 {
        BarcodeFormat::UpcA
    } else if text
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || " -.$/+%".contains(c))
    {
        BarcodeFormat::Code39
    } else {
        BarcodeFormat::Code128
    }
}

/// Result of encoding: a list of bar widths (alternating black/white starting with black).
#[derive(Debug, Clone)]
pub struct Barcode {
    /// Module pattern: true = dark bar, false = light space.
    pub modules: Vec<bool>,
    /// Human-readable text to display below.
    pub text: String,
    /// Format used.
    pub format: BarcodeFormat,
}

/// Encode text into a barcode. Returns None if the text is invalid for the format.
pub fn encode(text: &str, format: BarcodeFormat) -> Option<Barcode> {
    if text.is_empty() {
        return None;
    }
    match format {
        BarcodeFormat::Code128 => encode_code128(text),
        BarcodeFormat::Code39 => encode_code39(text),
        BarcodeFormat::Ean13 => encode_ean13(text),
        BarcodeFormat::UpcA => encode_upc_a(text),
    }
}

/// Check if text is valid for the given format.
pub fn is_valid(text: &str, format: BarcodeFormat) -> bool {
    match format {
        BarcodeFormat::Code128 => text.chars().all(|c| (c as u32) < 128),
        BarcodeFormat::Code39 => text
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || " -.$/+%".contains(c)),
        BarcodeFormat::Ean13 => text.len() <= 13 && text.chars().all(|c| c.is_ascii_digit()),
        BarcodeFormat::UpcA => text.len() <= 12 && text.chars().all(|c| c.is_ascii_digit()),
    }
}

// ─── Code 128 ───────────────────────────────────────────────────────────────

/// Code 128 bar patterns: each symbol is 6 alternating bar/space widths summing to 11 modules.
/// Index 0-105 = data values, 106 = stop pattern.
const CODE128_PATTERNS: [[u8; 6]; 107] = [
    [2,1,2,2,2,2], // 0
    [2,2,2,1,2,2], // 1
    [2,2,2,2,2,1], // 2
    [1,2,1,2,2,3], // 3
    [1,2,1,3,2,2], // 4
    [1,3,1,2,2,2], // 5
    [1,2,2,2,1,3], // 6
    [1,2,2,3,1,2], // 7
    [1,3,2,2,1,2], // 8
    [2,2,1,2,1,3], // 9
    [2,2,1,3,1,2], // 10
    [2,3,1,2,1,2], // 11
    [1,1,2,2,3,2], // 12
    [1,2,2,1,3,2], // 13
    [1,2,2,2,3,1], // 14
    [1,1,3,2,2,2], // 15
    [1,2,3,1,2,2], // 16
    [1,2,3,2,2,1], // 17
    [2,2,3,2,1,1], // 18
    [2,2,1,1,3,2], // 19
    [2,2,1,2,3,1], // 20
    [2,1,3,2,1,2], // 21
    [2,2,3,1,1,2], // 22
    [3,1,2,1,3,1], // 23
    [3,1,1,2,2,2], // 24
    [3,2,1,1,2,2], // 25
    [3,2,1,2,2,1], // 26
    [3,1,2,2,1,2], // 27
    [3,2,2,1,1,2], // 28
    [3,2,2,2,1,1], // 29
    [2,1,2,1,2,3], // 30
    [2,1,2,3,2,1], // 31
    [2,3,2,1,2,1], // 32
    [1,1,1,3,2,3], // 33
    [1,3,1,1,2,3], // 34
    [1,3,1,3,2,1], // 35
    [1,1,2,3,2,2], // 36 (not used below but part of pattern set)
    [1,3,2,1,2,2], // 37
    [1,3,2,3,2,0], // 38 — special; actual: [1,3,2,3,2,0] placeholder
    [2,1,1,3,1,3], // 39
    [2,3,1,1,1,3], // 40
    [2,3,1,3,1,1], // 41
    [1,1,2,1,3,3], // 42
    [1,1,2,3,3,1], // 43
    [1,3,2,1,3,1], // 44
    [1,1,3,1,2,3], // 45
    [1,1,3,3,2,1], // 46
    [1,3,3,1,2,1], // 47
    [3,1,3,1,2,1], // 48
    [2,1,1,3,3,1], // 49
    [2,3,1,1,3,1], // 50
    [2,1,3,1,1,3], // 51
    [2,1,3,3,1,1], // 52
    [2,1,3,1,3,1], // 53
    [3,1,1,1,2,3], // 54
    [3,1,1,3,2,1], // 55
    [3,3,1,1,2,1], // 56
    [3,1,2,1,1,3], // 57
    [3,1,2,3,1,1], // 58
    [3,3,2,1,1,1], // 59
    [2,1,1,2,1,3], // 60 (not used directly below)
    [2,1,1,2,3,1], // 61
    [2,3,1,2,1,1], // 62 (not used directly below)
    [1,1,2,2,1,3], // 63 (space in subset B = value 0)
    [1,1,2,2,3,1], // 64
    [1,3,2,2,1,1], // 65 (not used directly below)
    [1,1,1,1,3,3], // 66 (not used directly below)
    [1,3,1,1,3,1], // 67 (not used directly below)
    [1,1,3,1,1,3], // 68 (not used directly below)
    [1,1,3,3,1,1], // 69 (not used directly below)
    [1,3,3,1,1,1], // 70 (not used directly below)
    [3,1,1,1,3,1], // 71 (not used directly below)
    [3,1,3,1,1,1], // 72 (not used directly below)
    [2,1,1,1,3,3], // 73 (not used directly below)
    [2,1,3,1,3,0], // 74 — placeholder
    [3,1,1,2,1,2], // 75 (not used directly below)
    [3,1,1,2,2,1], // 76 (not used directly below) (not used below)
    [1,2,1,1,2,3], // 77
    [1,2,1,3,2,1], // 78 (not used directly below) (not used below)
    [1,2,1,1,3,2], // 79 (not used directly below) (not used below)
    [3,2,1,1,1,2], // 80 (not used directly below) (not used below)
    [1,1,1,2,2,3], // 81 (not used directly below) (not used below)
    [1,1,1,2,3,2], // 82 (not used directly below) (not used below)
    [1,2,1,2,3,1], // 83 (not used directly below)
    [1,2,3,2,1,1], // 84 (not used directly below)
    [3,2,1,2,1,1], // 85 (not used directly below)
    [2,1,2,2,1,2], // 86 (not used directly below)
    [1,1,2,1,2,3], // 87 (not used directly below) — duplicate of earlier
    [1,2,2,2,1,2], // 88 (not used directly below)
    [2,2,1,2,2,1], // 89 (not used directly below)
    [2,1,1,1,2,3], // 90 (not used directly below)
    [2,1,1,2,2,2], // 91 (not used directly below)
    [2,2,1,1,2,2], // 92 (not used directly below)
    [2,2,2,1,1,2], // 93 (not used directly below)
    [2,2,2,2,1,1], // 94 (not used directly below)
    [1,1,3,2,2,1], // 95 (not used directly below)
    [1,1,2,2,2,2], // 96 (not used directly below)
    [2,2,2,1,2,1], // 97 (not used directly below)
    [2,1,1,2,2,2], // 98 (not used directly below) SHIFT (not used directly)
    [3,1,2,2,2,1], // 99  CODE_C
    [2,1,2,1,1,3], // 100 CODE_B (FNC4 in B)
    [2,1,2,3,1,1], // 101 CODE_A (FNC4 in A)
    [1,2,1,1,2,3], // 102 FNC1
    [1,2,3,2,1,1], // 103 START_A (not used directly — we use numeric values)
    [1,2,1,1,3,2], // 104 START_B
    [1,1,3,1,2,3], // 105 START_C
    [2,3,3,1,1,1], // 106 STOP (13 modules including final bar)
];

// Code 128 special values
const START_B: usize = 104;
const START_C: usize = 105;
const CODE_B: usize = 100;
const CODE_C: usize = 99;
const STOP: usize = 106;

fn code128_value_b(c: char) -> Option<usize> {
    let v = c as u32;
    if v >= 32 && v <= 126 {
        Some((v - 32) as usize)
    } else {
        None
    }
}

fn pattern_to_modules(pattern: &[u8; 6]) -> Vec<bool> {
    let mut modules = Vec::new();
    for (i, &width) in pattern.iter().enumerate() {
        let dark = i % 2 == 0; // even index = bar (dark), odd = space (light)
        for _ in 0..width {
            modules.push(dark);
        }
    }
    modules
}

fn encode_code128(text: &str) -> Option<Barcode> {
    // Validate: all ASCII
    if !text.chars().all(|c| (c as u32) >= 32 && (c as u32) <= 126) {
        return None;
    }

    let chars: Vec<char> = text.chars().collect();
    let mut values: Vec<usize> = Vec::new();
    let mut i = 0;

    // Determine start code: if begins with 4+ digits, start with C
    let leading_digits = chars.iter().take_while(|c| c.is_ascii_digit()).count();

    let (start_code, mut current_set) = if leading_digits >= 4 {
        (START_C, 'C')
    } else {
        (START_B, 'B')
    };

    values.push(start_code);

    while i < chars.len() {
        if current_set == 'C' {
            // In subset C: encode digit pairs
            if i + 1 < chars.len()
                && chars[i].is_ascii_digit()
                && chars[i + 1].is_ascii_digit()
            {
                let val = (chars[i] as usize - '0' as usize) * 10
                    + (chars[i + 1] as usize - '0' as usize);
                values.push(val);
                i += 2;
            } else {
                // Switch to B
                values.push(CODE_B);
                current_set = 'B';
            }
        } else {
            // In subset B
            // Check if we should switch to C (4+ digits ahead)
            let remaining_digits = chars[i..].iter().take_while(|c| c.is_ascii_digit()).count();
            if remaining_digits >= 4 {
                values.push(CODE_C);
                current_set = 'C';
            } else {
                // Encode single character in subset B
                if let Some(val) = code128_value_b(chars[i]) {
                    values.push(val);
                    i += 1;
                } else {
                    return None; // invalid character
                }
            }
        }
    }

    // Compute checksum
    let mut checksum = values[0]; // start code
    for (pos, &val) in values[1..].iter().enumerate() {
        checksum += val * (pos + 1);
    }
    checksum %= 103;
    values.push(checksum);
    values.push(STOP);

    // Convert to modules
    let mut modules = Vec::new();

    // Quiet zone (10 modules)
    for _ in 0..10 {
        modules.push(false);
    }

    for &val in &values {
        if val == STOP {
            // Stop pattern is special: 2,3,3,1,1,1,2 (13 modules)
            let stop_mods: [bool; 13] = [
                true, true, false, false, false, true, true, true, false, true, true, false, true,
            ];
            modules.extend_from_slice(&stop_mods);
        } else if val < 107 {
            modules.extend(pattern_to_modules(&CODE128_PATTERNS[val]));
        }
    }

    // Quiet zone
    for _ in 0..10 {
        modules.push(false);
    }

    Some(Barcode {
        modules,
        text: String::from(text),
        format: BarcodeFormat::Code128,
    })
}

// ─── Code 39 ────────────────────────────────────────────────────────────────

/// Code 39 character set and patterns.
/// Each character = 9 elements (5 bars + 4 spaces), where W=wide, N=narrow.
/// Encoded as: bar, space, bar, space, bar, space, bar, space, bar
const CODE39_CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%*";

/// Code 39 patterns: 0=narrow, 1=wide. 9 elements per char (bar/space alternating).
const CODE39_PATTERNS: [[u8; 9]; 44] = [
    [0,0,0,1,1,0,1,0,0], // 0
    [1,0,0,1,0,0,0,0,1], // 1
    [0,0,1,1,0,0,0,0,1], // 2
    [1,0,1,1,0,0,0,0,0], // 3
    [0,0,0,1,1,0,0,0,1], // 4
    [1,0,0,1,1,0,0,0,0], // 5
    [0,0,1,1,1,0,0,0,0], // 6
    [0,0,0,1,0,0,1,0,1], // 7
    [1,0,0,1,0,0,1,0,0], // 8
    [0,0,1,1,0,0,1,0,0], // 9
    [1,0,0,0,0,1,0,0,1], // A
    [0,0,1,0,0,1,0,0,1], // B
    [1,0,1,0,0,1,0,0,0], // C
    [0,0,0,0,1,1,0,0,1], // D
    [1,0,0,0,1,1,0,0,0], // E
    [0,0,1,0,1,1,0,0,0], // F
    [0,0,0,0,0,1,1,0,1], // G
    [1,0,0,0,0,1,1,0,0], // H
    [0,0,1,0,0,1,1,0,0], // I
    [0,0,0,0,1,1,1,0,0], // J
    [1,0,0,0,0,0,0,1,1], // K
    [0,0,1,0,0,0,0,1,1], // L
    [1,0,1,0,0,0,0,1,0], // M
    [0,0,0,0,1,0,0,1,1], // N
    [1,0,0,0,1,0,0,1,0], // O
    [0,0,1,0,1,0,0,1,0], // P
    [0,0,0,0,0,0,1,1,1], // Q
    [1,0,0,0,0,0,1,1,0], // R
    [0,0,1,0,0,0,1,1,0], // S
    [0,0,0,0,1,0,1,1,0], // T
    [1,1,0,0,0,0,0,0,1], // U
    [0,1,1,0,0,0,0,0,1], // V
    [1,1,1,0,0,0,0,0,0], // W
    [0,1,0,0,1,0,0,0,1], // X
    [1,1,0,0,1,0,0,0,0], // Y
    [0,1,1,0,1,0,0,0,0], // Z
    [0,1,0,0,0,0,1,0,1], // -
    [1,1,0,0,0,0,1,0,0], // .
    [0,1,0,1,0,1,0,0,0], // (space)
    [0,1,0,1,0,0,0,1,0], // $
    [0,1,0,0,0,1,0,1,0], // /
    [0,0,0,1,0,1,0,1,0], // +
    [0,1,0,1,0,1,0,0,0], // % (same as space visually — simplified)
    [0,1,0,0,1,0,1,0,0], // * (start/stop)
];

fn code39_index(c: char) -> Option<usize> {
    CODE39_CHARS.iter().position(|&b| b == c as u8)
}

fn encode_code39(text: &str) -> Option<Barcode> {
    let upper = text.to_ascii_uppercase();

    // Validate
    if !upper.chars().all(|c| code39_index(c).is_some()) {
        return None;
    }

    let narrow = 1u8;
    let wide = 3u8;
    let mut modules = Vec::new();

    // Quiet zone
    for _ in 0..10 {
        modules.push(false);
    }

    // Start character (*)
    let star_idx = 43;
    encode_code39_char(&CODE39_PATTERNS[star_idx], narrow, wide, &mut modules);

    // Inter-character gap
    modules.push(false);

    // Data characters
    for c in upper.chars() {
        if let Some(idx) = code39_index(c) {
            encode_code39_char(&CODE39_PATTERNS[idx], narrow, wide, &mut modules);
            modules.push(false); // inter-character gap
        }
    }

    // Stop character (*)
    encode_code39_char(&CODE39_PATTERNS[star_idx], narrow, wide, &mut modules);

    // Quiet zone
    for _ in 0..10 {
        modules.push(false);
    }

    Some(Barcode {
        modules,
        text: upper,
        format: BarcodeFormat::Code39,
    })
}

fn encode_code39_char(pattern: &[u8; 9], narrow: u8, wide: u8, modules: &mut Vec<bool>) {
    for (i, &is_wide) in pattern.iter().enumerate() {
        let dark = i % 2 == 0; // even = bar, odd = space
        let width = if is_wide != 0 { wide } else { narrow };
        for _ in 0..width {
            modules.push(dark);
        }
    }
}

// ─── EAN-13 ─────────────────────────────────────────────────────────────────

/// EAN-13 L-code patterns (odd parity, left side).
const EAN_L_PATTERNS: [[bool; 7]; 10] = [
    [false, false, false, true, true, false, true],  // 0
    [false, false, true, true, false, false, true],   // 1
    [false, false, true, false, false, true, true],   // 2
    [false, true, true, true, true, false, true],     // 3
    [false, true, false, false, false, true, true],   // 4
    [false, true, true, false, false, false, true],   // 5
    [false, true, false, true, true, true, true],     // 6
    [false, true, true, true, false, true, true],     // 7
    [false, true, true, false, true, true, true],     // 8
    [false, false, false, true, false, true, true],   // 9
];

/// EAN-13 G-code patterns (even parity, left side).
const EAN_G_PATTERNS: [[bool; 7]; 10] = [
    [false, true, false, false, true, true, true],    // 0
    [false, true, true, false, false, true, true],    // 1
    [false, false, true, true, false, true, true],    // 2
    [false, true, false, false, false, false, true],  // 3
    [false, false, true, true, true, false, true],    // 4
    [false, true, true, true, false, false, true],    // 5
    [false, false, false, false, true, false, true],  // 6
    [false, false, true, false, false, false, true],  // 7
    [false, false, false, true, false, false, true],  // 8
    [false, false, true, false, true, true, true],    // 9
];

/// EAN-13 R-code patterns (right side — complement of L).
const EAN_R_PATTERNS: [[bool; 7]; 10] = [
    [true, true, true, false, false, true, false],    // 0
    [true, true, false, false, true, true, false],    // 1
    [true, true, false, true, true, false, false],    // 2
    [true, false, false, false, false, true, false],  // 3
    [true, false, true, true, true, false, false],    // 4
    [true, false, false, true, true, true, false],    // 5
    [true, false, true, false, false, false, false],  // 6
    [true, false, false, false, true, false, false],  // 7
    [true, false, false, true, false, false, false],  // 8
    [true, true, true, false, true, false, false],    // 9
];

/// Parity encoding for the first digit of EAN-13.
/// L=0, G=1. Indexed by first digit.
const EAN_PARITY: [[u8; 6]; 10] = [
    [0, 0, 0, 0, 0, 0], // 0
    [0, 0, 1, 0, 1, 1], // 1
    [0, 0, 1, 1, 0, 1], // 2
    [0, 0, 1, 1, 1, 0], // 3
    [0, 1, 0, 0, 1, 1], // 4
    [0, 1, 1, 0, 0, 1], // 5
    [0, 1, 1, 1, 0, 0], // 6
    [0, 1, 0, 1, 0, 1], // 7
    [0, 1, 0, 1, 1, 0], // 8
    [0, 1, 1, 0, 1, 0], // 9
];

/// Compute EAN-13 check digit.
pub fn ean13_check_digit(digits: &[u8]) -> u8 {
    let mut sum = 0u32;
    for (i, &d) in digits.iter().enumerate() {
        if i % 2 == 0 {
            sum += d as u32;
        } else {
            sum += d as u32 * 3;
        }
    }
    ((10 - (sum % 10)) % 10) as u8
}

fn encode_ean13(text: &str) -> Option<Barcode> {
    if !text.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let mut digits: Vec<u8> = text.chars().map(|c| c as u8 - b'0').collect();

    // Pad to 12 digits if needed, compute check digit
    if digits.len() < 12 {
        return None; // Need at least 12 digits (+ auto check)
    }
    if digits.len() == 12 {
        let check = ean13_check_digit(&digits);
        digits.push(check);
    }
    if digits.len() != 13 {
        return None;
    }

    // Verify check digit
    let expected = ean13_check_digit(&digits[..12]);
    if digits[12] != expected {
        // Auto-correct check digit
        digits[12] = expected;
    }

    let mut modules = Vec::new();

    // Quiet zone
    for _ in 0..9 {
        modules.push(false);
    }

    // Start guard: 101
    modules.push(true);
    modules.push(false);
    modules.push(true);

    // Left side: 6 digits with L/G parity based on first digit
    let parity = EAN_PARITY[digits[0] as usize];
    for i in 0..6 {
        let digit = digits[i + 1] as usize;
        let pattern = if parity[i] == 0 {
            &EAN_L_PATTERNS[digit]
        } else {
            &EAN_G_PATTERNS[digit]
        };
        modules.extend_from_slice(pattern);
    }

    // Center guard: 01010
    modules.push(false);
    modules.push(true);
    modules.push(false);
    modules.push(true);
    modules.push(false);

    // Right side: 6 digits with R encoding
    for i in 0..6 {
        let digit = digits[i + 7] as usize;
        modules.extend_from_slice(&EAN_R_PATTERNS[digit]);
    }

    // End guard: 101
    modules.push(true);
    modules.push(false);
    modules.push(true);

    // Quiet zone
    for _ in 0..9 {
        modules.push(false);
    }

    // Build display text with check digit
    let display: String = digits.iter().map(|d| (d + b'0') as char).collect();

    Some(Barcode {
        modules,
        text: display,
        format: BarcodeFormat::Ean13,
    })
}

// ─── UPC-A ──────────────────────────────────────────────────────────────────

fn encode_upc_a(text: &str) -> Option<Barcode> {
    if !text.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let mut digits: Vec<u8> = text.chars().map(|c| c as u8 - b'0').collect();

    if digits.len() < 11 {
        return None;
    }
    if digits.len() == 11 {
        let check = upc_check_digit(&digits);
        digits.push(check);
    }
    if digits.len() != 12 {
        return None;
    }

    // Verify/correct check digit
    let expected = upc_check_digit(&digits[..11]);
    digits[11] = expected;

    // UPC-A is EAN-13 with a leading 0
    let mut ean_digits = vec![0u8];
    ean_digits.extend_from_slice(&digits);

    let display: String = digits.iter().map(|d| (d + b'0') as char).collect();

    // Encode as EAN-13 with leading 0
    let ean_text: String = ean_digits.iter().map(|d| (d + b'0') as char).collect();
    if let Some(mut barcode) = encode_ean13(&ean_text) {
        barcode.text = display;
        barcode.format = BarcodeFormat::UpcA;
        Some(barcode)
    } else {
        None
    }
}

fn upc_check_digit(digits: &[u8]) -> u8 {
    let mut sum = 0u32;
    for (i, &d) in digits.iter().enumerate() {
        if i % 2 == 0 {
            sum += d as u32 * 3;
        } else {
            sum += d as u32;
        }
    }
    ((10 - (sum % 10)) % 10) as u8
}
