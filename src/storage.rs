//! PDDB storage for Barcode Generator.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use crate::app::{BarcodeSettings, SavedBarcode};
use crate::barcode_encode::BarcodeFormat;

const DICT_SETTINGS: &str = "barcode.settings";
const DICT_CODES: &str = "barcode.codes";
const KEY_CONFIG: &str = "config";
const KEY_INDEX: &str = "index";

pub struct Storage {
    pddb: pddb::Pddb,
}

impl Storage {
    pub fn new() -> Result<Self, ()> {
        let pddb = pddb::Pddb::new();
        pddb.is_mounted_blocking();
        Ok(Self { pddb })
    }

    pub fn load_settings(&mut self) -> Option<BarcodeSettings> {
        let mut key = self.pddb.get(DICT_SETTINGS, KEY_CONFIG, None, false, false, None, None::<fn()>).ok()?;
        let mut buf = Vec::new();
        use std::io::Read;
        key.read_to_end(&mut buf).ok()?;
        let json: serde_json::Value = serde_json::from_slice(&buf).ok()?;

        let format = match json.get("format").and_then(|v| v.as_str()) {
            Some("code39") => BarcodeFormat::Code39,
            Some("ean13") => BarcodeFormat::Ean13,
            Some("upca") => BarcodeFormat::UpcA,
            _ => BarcodeFormat::Code128,
        };
        let bar_width = json.get("bar_width").and_then(|v| v.as_u64()).unwrap_or(2) as u8;
        let bar_height = json.get("bar_height").and_then(|v| v.as_u64()).unwrap_or(200) as u16;
        let auto_format = json.get("auto_format").and_then(|v| v.as_bool()).unwrap_or(true);

        Some(BarcodeSettings { format, bar_width, bar_height, auto_format })
    }

    pub fn save_settings(&mut self, settings: &BarcodeSettings) {
        let fmt_str = match settings.format {
            BarcodeFormat::Code128 => "code128",
            BarcodeFormat::Code39 => "code39",
            BarcodeFormat::Ean13 => "ean13",
            BarcodeFormat::UpcA => "upca",
        };
        let json = serde_json::json!({
            "format": fmt_str,
            "bar_width": settings.bar_width,
            "bar_height": settings.bar_height,
            "auto_format": settings.auto_format,
        });
        let data = serde_json::to_vec(&json).unwrap_or_default();

        if let Ok(mut key) = self.pddb.get(DICT_SETTINGS, KEY_CONFIG, None, true, true, Some(data.len()), None::<fn()>) {
            use std::io::{Seek, Write};
            key.seek(std::io::SeekFrom::Start(0)).ok();
            key.write_all(&data).ok();
            key.set_len(data.len() as u64).ok();
        }
        self.pddb.sync().ok();
    }

    pub fn load_codes(&mut self) -> Vec<SavedBarcode> {
        let mut codes = Vec::new();

        let names: Vec<String> = match self.pddb.get(DICT_CODES, KEY_INDEX, None, false, false, None, None::<fn()>) {
            Ok(mut key) => {
                let mut buf = Vec::new();
                use std::io::Read;
                if key.read_to_end(&mut buf).is_ok() {
                    serde_json::from_slice(&buf).unwrap_or_default()
                } else {
                    Vec::new()
                }
            }
            Err(_) => Vec::new(),
        };

        for name in &names {
            let key_name = alloc::format!("code.{}", name);
            if let Ok(mut key) = self.pddb.get(DICT_CODES, &key_name, None, false, false, None, None::<fn()>) {
                let mut buf = Vec::new();
                use std::io::Read;
                if key.read_to_end(&mut buf).is_ok() {
                    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&buf) {
                        let text = json.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let format = match json.get("format").and_then(|v| v.as_str()) {
                            Some("code39") => BarcodeFormat::Code39,
                            Some("ean13") => BarcodeFormat::Ean13,
                            Some("upca") => BarcodeFormat::UpcA,
                            _ => BarcodeFormat::Code128,
                        };
                        codes.push(SavedBarcode { name: name.clone(), text: String::from(text), format });
                    }
                }
            }
        }

        codes
    }

    pub fn save_codes(&mut self, codes: &[SavedBarcode]) {
        let names: Vec<&str> = codes.iter().map(|c| c.name.as_str()).collect();
        let index_data = serde_json::to_vec(&names).unwrap_or_default();

        if let Ok(mut key) = self.pddb.get(DICT_CODES, KEY_INDEX, None, true, true, Some(index_data.len()), None::<fn()>) {
            use std::io::{Seek, Write};
            key.seek(std::io::SeekFrom::Start(0)).ok();
            key.write_all(&index_data).ok();
            key.set_len(index_data.len() as u64).ok();
        }

        for code in codes {
            let key_name = alloc::format!("code.{}", code.name);
            let fmt_str = match code.format {
                BarcodeFormat::Code128 => "code128",
                BarcodeFormat::Code39 => "code39",
                BarcodeFormat::Ean13 => "ean13",
                BarcodeFormat::UpcA => "upca",
            };
            let json = serde_json::json!({
                "text": code.text,
                "format": fmt_str,
            });
            let data = serde_json::to_vec(&json).unwrap_or_default();

            if let Ok(mut key) = self.pddb.get(DICT_CODES, &key_name, None, true, true, Some(data.len()), None::<fn()>) {
                use std::io::{Seek, Write};
                key.seek(std::io::SeekFrom::Start(0)).ok();
                key.write_all(&data).ok();
                key.set_len(data.len() as u64).ok();
            }
        }

        self.pddb.sync().ok();
    }
}
