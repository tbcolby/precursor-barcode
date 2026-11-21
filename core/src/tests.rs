//! Tests (compiled only under #[cfg(test)], see lib.rs). Reference module strings were produced with reportlab 4.4
//! (`reportlab.graphics.barcode.code128/code39/eanbc`), an independent encoder,
//! and cross-checked against the worked examples in the standards.

extern crate std;

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::encode::*;
use crate::model::*;

fn bits(b: &Barcode) -> String { b.modules.iter().map(|&m| if m { '1' } else { '0' }).collect() }

/// Module string without quiet zones.
fn body(b: &Barcode) -> String { bits(b).trim_matches('0').to_string() }

/// Decode a Code 128 module string back to values using the pattern table.
fn decode128(b: &Barcode) -> Vec<u8> {
    let s = body(b);
    let mut runs = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let j = bytes[i..].iter().take_while(|&&c| c == bytes[i]).count();
        runs.push(j as u8);
        i += j;
    }
    assert_eq!(&runs[runs.len() - 7..], &CODE128_STOP, "stop pattern");
    runs[..runs.len() - 7]
        .chunks(6)
        .map(|c| CODE128_PATTERNS.iter().position(|p| p == c).expect("unknown symbol") as u8)
        .collect()
}

// ─── Code 128 ───────────────────────────────────────────────────────────────

#[test]
fn code128_every_row_is_11_modules_3_bars_and_unique() {
    // Regression: 43 rows summed to 10 and two rows held a zero width.
    for (v, row) in CODE128_PATTERNS.iter().enumerate() {
        assert_eq!(row.iter().map(|&w| w as u32).sum::<u32>(), 11, "row {v}");
        assert!(row.iter().all(|&w| (1..=4).contains(&w)), "row {v}");
        // bars (even positions) are an even number of modules (parity rule)
        assert_eq!((row[0] + row[2] + row[4]) % 2, 0, "row {v} bar parity");
    }
    for (a, ra) in CODE128_PATTERNS.iter().enumerate() {
        for (b, rb) in CODE128_PATTERNS.iter().enumerate().skip(a + 1) {
            assert_ne!(ra, rb, "rows {a} and {b}");
        }
    }
}

#[test]
fn code128_special_symbols_match_iso_15417() {
    // Regression: START_B was 1,2,1,1,3,2 and START_C / CODE_B / CODE_C were wrong.
    assert_eq!(CODE128_PATTERNS[CODE128_START_B as usize], [2, 1, 1, 2, 1, 4]);
    assert_eq!(CODE128_PATTERNS[CODE128_START_C as usize], [2, 1, 1, 2, 3, 2]);
    assert_eq!(CODE128_PATTERNS[CODE128_CODE_B as usize], [1, 1, 4, 1, 3, 1]);
    assert_eq!(CODE128_PATTERNS[CODE128_CODE_C as usize], [1, 1, 3, 1, 4, 1]);
    assert_eq!(CODE128_PATTERNS[103], [2, 1, 1, 4, 1, 2]); // START_A
}

#[test]
fn code128_stop_pattern() {
    // Regression: stop was 2,3,3,1,2,1,1 (1100011101101).
    assert_eq!(CODE128_STOP, [2, 3, 3, 1, 1, 1, 2]);
    let b = encode("A", Format::Code128).unwrap();
    assert!(body(&b).ends_with("1100011101011"));
}

#[test]
fn code128_reference_vectors() {
    let cases = [
        ("Wikipedia", "11010010000111010001101000011010011000010010100001101001010011110010110010000100001001101000011010010010110000111100100101100011101011"),
        ("Hello, World!", "1101001000011000101000101100100001100101000011001010000100011110101011001110011011001100111010001101000111101010010011110110010100001000010011011001101100110010100001100011101011"),
        ("12345678", "1101001110010110011100100010110001110001011011000010100100011101101100011101011"),
        ("RI 476 394 652 CH", "110100100001100010111011000100010110110011001100100111011101101110110011101001101100110011001011100111001011001100100111011011001100110011101001101110010011001110010110110011001000100011011000101000100011110101100011101011"),
        ("a", "1101001000010010110000100100001101100011101011"),
    ];
    for (text, want) in cases {
        let b = encode(text, Format::Code128).unwrap();
        assert_eq!(body(&b), want, "{text}");
    }
}

#[test]
fn code128_quiet_zones() {
    let b = encode("X", Format::Code128).unwrap();
    assert!(b.modules[..CODE128_QUIET].iter().all(|&m| !m));
    assert!(b.modules[b.width() - CODE128_QUIET..].iter().all(|&m| !m));
    assert!(b.modules[CODE128_QUIET]);
    // quiet + start + data + check + stop + quiet
    assert_eq!(b.width(), 10 + 11 + 11 + 11 + 13 + 10);
}

#[test]
fn code128_every_printable_char_roundtrips() {
    for c in ' '..='~' {
        let s = c.to_string();
        let b = encode(&s, Format::Code128).unwrap();
        let v = decode128(&b);
        assert_eq!(v, vec![CODE128_START_B, c as u8 - b' ', code128_checksum(&v[..2])], "{c:?}");
    }
}

#[test]
fn code128_set_c_for_digit_runs() {
    assert_eq!(code128_values("1234").unwrap(), vec![CODE128_START_C, 12, 34]);
    // odd leading run: last digit goes to set B
    assert_eq!(code128_values("12345").unwrap(), vec![CODE128_START_C, 12, 34, CODE128_CODE_B, b'5' - b' ']);
    // short digit runs stay in B
    assert_eq!(code128_values("A12").unwrap(), vec![CODE128_START_B, 33, 17, 18]);
    // a long run in the middle switches to C and back
    assert_eq!(
        code128_values("A123456B").unwrap(),
        vec![CODE128_START_B, 33, CODE128_CODE_C, 12, 34, 56, CODE128_CODE_B, 34]
    );
}

#[test]
fn code128_checksum_worked_example() {
    // 104 + 48*1 + 42*2 + 42*3 + 17*4 + 18*5 + 19*6 + 35*7 = 879; 879 mod 103 = 55.
    let v = code128_values("PJJ123C").unwrap();
    assert_eq!(v, vec![104, 48, 42, 42, 17, 18, 19, 35]);
    assert_eq!(code128_checksum(&v), 55);
}

#[test]
fn code128_validation_is_shared() {
    // Regression: is_valid allowed 0-127 while the encoder allowed 32-126.
    for c in ['\u{7f}', '\t', 'é', '\0'] {
        let s = c.to_string();
        assert!(!code128_char_ok(c));
        assert!(matches!(check(&s, Format::Code128), Err(EncodeError::BadChar { .. })));
    }
}

// ─── Code 39 ────────────────────────────────────────────────────────────────

#[test]
fn code39_rows_have_three_wide_elements_and_are_unique() {
    for (i, row) in CODE39_PATTERNS.iter().enumerate() {
        assert_eq!(row.iter().filter(|&&w| w == 1).count(), 3, "row {i}");
    }
    for (a, ra) in CODE39_PATTERNS.iter().enumerate() {
        for (b, rb) in CODE39_PATTERNS.iter().enumerate().skip(a + 1) {
            assert_ne!(ra, rb, "rows {a} and {b}");
        }
    }
}

#[test]
fn code39_symbol_rows_fixed() {
    // Regression: space/$//+/% held each other's patterns.
    let row = |c| CODE39_PATTERNS[code39_index(c).unwrap()];
    assert_eq!(row(' '), [0, 1, 1, 0, 0, 0, 1, 0, 0]);
    assert_eq!(row('$'), [0, 1, 0, 1, 0, 1, 0, 0, 0]);
    assert_eq!(row('/'), [0, 1, 0, 1, 0, 0, 0, 1, 0]);
    assert_eq!(row('+'), [0, 1, 0, 0, 0, 1, 0, 1, 0]);
    assert_eq!(row('%'), [0, 0, 0, 1, 0, 1, 0, 1, 0]);
}

#[test]
fn code39_reference_vectors() {
    let cases = [
        ("HELLO WORLD", "100010111011101011101010001110101110101110001010101110101000111010111010100011101110101110100010100011101011101011100011101010101110101110100010111010101110001010111010100011101010111000101110100010111011101"),
        ("A/B", "1000101110111010111010100010111010001000101000101011101000101110100010111011101"),
    ];
    for (text, want) in cases {
        assert_eq!(body(&encode(text, Format::Code39).unwrap()), want, "{text}");
    }
}

#[test]
fn code39_rejects_lowercase_and_star() {
    assert_eq!(encode("abc", Format::Code39), Err(EncodeError::BadChar { format: Format::Code39, ch: 'a' }));
    assert!(encode("A*B", Format::Code39).is_err());
    assert!(encode("Aé", Format::Code39).is_err());
}

// ─── EAN-13 / UPC-A ─────────────────────────────────────────────────────────

#[test]
fn gs1_check_digits() {
    let d = |s: &str| s.bytes().map(|b| b - b'0').collect::<Vec<u8>>();
    assert_eq!(gs1_check_digit(&d("400638133393")), 1);
    assert_eq!(gs1_check_digit(&d("590123412345")), 7);
    assert_eq!(gs1_check_digit(&d("978030640615")), 7);
    assert_eq!(gs1_check_digit(&d("03600029145")), 2); // UPC-A
    assert_eq!(gs1_check_digit(&d("00000000000")), 0);
}

#[test]
fn ean13_reference_vectors() {
    let cases = [
        ("4006381333931", "10100011010100111010111101111010001001011001101010100001010000101000010111010010000101100110101"),
        ("5901234123457", "10100010110100111011001100100110111101001110101010110011011011001000010101110010011101000100101"),
        ("9780306406157", "10101110110001001010011101111010100111010111101010101110011100101010000110011010011101000100101"),
    ];
    for (text, want) in cases {
        let b = encode(text, Format::Ean13).unwrap();
        assert_eq!(&bits(&b)[EAN13_QUIET_LEFT..EAN13_QUIET_LEFT + 95], want, "{text}");
        assert_eq!(b.text, text);
        // 12 digits: check digit appended, same symbol
        assert_eq!(encode(&text[..12], Format::Ean13).unwrap(), b);
    }
}

#[test]
fn ean13_quiet_zones_are_11_and_7() {
    // Regression: both sides were 9.
    let b = encode("4006381333931", Format::Ean13).unwrap();
    assert_eq!(b.width(), 11 + 95 + 7);
    assert!(b.modules[..11].iter().all(|&m| !m) && b.modules[11]);
    assert!(b.modules[b.width() - 7..].iter().all(|&m| !m) && b.modules[b.width() - 8]);
}

#[test]
fn wrong_check_digit_is_rejected_not_replaced() {
    // Regression: a mistyped check digit was silently replaced.
    assert_eq!(
        encode("4006381333932", Format::Ean13),
        Err(EncodeError::BadCheckDigit { format: Format::Ean13, expected: 1, got: 2 })
    );
    assert_eq!(
        encode("036000291453", Format::UpcA),
        Err(EncodeError::BadCheckDigit { format: Format::UpcA, expected: 2, got: 3 })
    );
}

#[test]
fn upca_reference_vector() {
    let b = encode("036000291452", Format::UpcA).unwrap();
    assert_eq!(
        &bits(&b)[UPCA_QUIET..UPCA_QUIET + 95],
        "10100011010111101010111100011010001101000110101010110110011101001100110101110010011101101100101"
    );
    assert_eq!(b.width(), 9 + 95 + 9);
    assert_eq!(encode("03600029145", Format::UpcA).unwrap(), b);
}

#[test]
fn gs1_length_and_char_errors() {
    assert_eq!(encode("12345", Format::Ean13), Err(EncodeError::BadLength { format: Format::Ean13, got: 5 }));
    assert_eq!(encode("1234567890", Format::UpcA), Err(EncodeError::BadLength { format: Format::UpcA, got: 10 }));
    assert!(matches!(encode("12345678901a", Format::Ean13), Err(EncodeError::BadChar { ch: 'a', .. })));
    assert_eq!(encode("", Format::Ean13), Err(EncodeError::Empty));
}

// ─── Shared ─────────────────────────────────────────────────────────────────

#[test]
fn auto_detect_picks_by_content() {
    assert_eq!(auto_detect("4006381333931"), Format::Ean13);
    assert_eq!(auto_detect("036000291452"), Format::UpcA);
    assert_eq!(auto_detect("HELLO WORLD"), Format::Code39);
    assert_eq!(auto_detect("Hello"), Format::Code128);
    assert_eq!(auto_detect("12345"), Format::Code39);
    assert_eq!(auto_detect("A*B"), Format::Code128);
    assert_eq!(auto_detect(""), Format::Code128);
}

#[test]
fn error_messages_are_readable() {
    let e = encode("4006381333932", Format::Ean13).unwrap_err();
    assert_eq!(e.to_string(), "EAN-13 check digit is 1, not 2");
    assert_eq!(encode("ab", Format::Code39).unwrap_err().to_string(), "Code 39 has no lowercase ('a'); use Code 128");
    assert_eq!(encode("x\u{7f}", Format::Code128).unwrap_err().to_string(), "Code 128 can't encode U+007F");
}

#[test]
fn fit_to_width() {
    // Regression: wide codes were drawn from x=4 and clipped with no warning.
    assert_eq!(fit_module_px(113, 336, 2), Some(2));
    assert_eq!(fit_module_px(113, 336, 4), Some(2));
    assert_eq!(fit_module_px(200, 336, 3), Some(1));
    assert_eq!(fit_module_px(337, 336, 2), None);
    assert_eq!(fit_module_px(10, 336, 0), Some(1));
}

#[test]
fn dark_runs_cover_all_dark_modules() {
    let b = encode("Hello", Format::Code128).unwrap();
    let runs = b.dark_runs();
    let total: usize = runs.iter().map(|r| r.1).sum();
    assert_eq!(total, b.modules.iter().filter(|&&m| m).count());
    for (start, len) in runs {
        assert!(b.modules[start..start + len].iter().all(|&m| m));
        assert!(start == 0 || !b.modules[start - 1]);
        assert!(start + len == b.width() || !b.modules[start + len]);
    }
}

// ─── Model ──────────────────────────────────────────────────────────────────

#[test]
fn settings_load_legacy_record_and_clamp() {
    let old = r#"{"format":"code39","bar_width":9,"bar_height":5,"auto_format":false}"#;
    let s: Settings = serde_json::from_str(old).unwrap();
    assert_eq!(s.version, 0);
    let s = s.sanitized();
    assert_eq!((s.version, s.format, s.bar_width, s.bar_height, s.auto_format), (1, Format::Code39, 4, 40, false));
    let rt: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    assert_eq!(rt, s);
}

#[test]
fn settings_steps_stay_in_range() {
    let mut s = Settings::default();
    for _ in 0..10 {
        s.wider();
        s.taller();
    }
    assert_eq!((s.bar_width, s.bar_height), (MAX_BAR_WIDTH, MAX_BAR_HEIGHT));
    for _ in 0..30 {
        s.narrower();
        s.shorter();
    }
    assert_eq!((s.bar_width, s.bar_height), (MIN_BAR_WIDTH, MIN_BAR_HEIGHT));
}

#[test]
fn duplicate_names_are_rejected() {
    // Regression: saving "wifi" twice made both entries load the second payload.
    let mut c = Codes::new();
    c.add("wifi", "one", Format::Code128).unwrap();
    assert_eq!(c.add("WiFi ", "two", Format::Code128), Err(SaveError::Duplicate));
    assert_eq!(c.add("  ", "x", Format::Code128), Err(SaveError::EmptyName));
    assert_eq!(c.len(), 1);
    assert_eq!(c.get(0).unwrap().text, "one");
}

#[test]
fn remove_drops_payload_from_record() {
    // Regression: delete rewrote the index but left the payload stored.
    let mut c = Codes::new();
    c.add("a", "SECRET-A", Format::Code128).unwrap();
    c.add("b", "B", Format::Code39).unwrap();
    assert_eq!(c.remove(0).unwrap().name, "a");
    assert!(c.remove(5).is_none());
    let json = serde_json::to_string(&c).unwrap();
    assert!(!json.contains("SECRET-A"));
}

#[test]
fn saved_code_debug_redacts_payload() {
    let mut c = Codes::new();
    c.add("pw", "hunter2", Format::Code128).unwrap();
    let dbg = std::format!("{c:?}");
    assert!(!dbg.contains("hunter2"));
    assert!(dbg.contains("pw"));
}

#[test]
fn index_record_reads_both_layouts() {
    let legacy: IndexRecord = serde_json::from_str(r#"["wifi","door"]"#).unwrap();
    assert_eq!(legacy, IndexRecord::Legacy(vec!["wifi".into(), "door".into()]));
    let mut c = Codes::new();
    c.add("x", "1", Format::Ean13).unwrap();
    let cur: IndexRecord = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
    assert_eq!(cur, IndexRecord::Current(c));
}

#[test]
fn legacy_migration() {
    let p: LegacyCode = serde_json::from_str(r#"{"text":"ABC","format":"code39"}"#).unwrap();
    let codes = migrate_legacy(vec![
        ("a".into(), Some(p.clone())),
        ("gone".into(), None),
        ("a".into(), Some(p)),
    ]);
    assert_eq!(codes.len(), 1);
    assert_eq!(codes.get(0).unwrap().format, Format::Code39);
    assert_eq!(legacy_code_key("a"), "code.a");
    assert!(is_legacy_code_key("code.orphan") && !is_legacy_code_key("index"));
}

#[test]
fn newer_version_is_refused() {
    let c = Codes { version: 7, codes: Vec::new() };
    assert_eq!(c.check_version(), Err(7));
    assert_eq!(Codes::new().check_version(), Ok(()));
}

#[test]
fn text_field_limits() {
    let mut f = TextField::new(3);
    assert!(f.push('a') && f.push(' ') && f.push('~'));
    assert!(!f.push('b'));
    assert!(!f.push('\u{2190}'));
    assert_eq!(f.as_str(), "a ~");
    assert!(f.pop());
    f.set("xyz\u{8}w");
    assert_eq!(f.as_str(), "xyz");
    f.clear();
    assert!(!f.pop());
}

#[test]
fn list_label_truncates_preview() {
    let mut c = Codes::new();
    c.add("n", "0123456789ABCDEFG", Format::Code128).unwrap();
    assert_eq!(c.label(0), "n [C128] 0123456789ABCD…");
}
