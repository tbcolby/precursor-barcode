//! Application state machine and input handling for the Barcode Generator.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use crate::barcode_encode::{self, Barcode, BarcodeFormat};
use crate::storage::Storage;

// Standard key codes (ecosystem standard)
const KEY_UP: char = '\u{2191}';
const KEY_DOWN: char = '\u{2193}';
const KEY_LEFT: char = '\u{2190}';
const KEY_RIGHT: char = '\u{2192}';
const KEY_ENTER: char = '\r';
const KEY_BACKSPACE: char = '\u{0008}';

const MAX_TEXT_LEN: usize = 80;

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    MainMenu,
    Input,
    Display,
    SavePrompt,
    SaveNameEntry,
    LoadList,
    Settings,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuItem {
    NewBarcode,
    SavedCodes,
    Settings,
    Help,
}

impl MenuItem {
    pub fn label(&self) -> &'static str {
        match self {
            MenuItem::NewBarcode => "New Barcode",
            MenuItem::SavedCodes => "Saved Codes",
            MenuItem::Settings => "Settings",
            MenuItem::Help => "Help",
        }
    }

    pub fn all() -> &'static [MenuItem] {
        &[
            MenuItem::NewBarcode,
            MenuItem::SavedCodes,
            MenuItem::Settings,
            MenuItem::Help,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct BarcodeSettings {
    pub bar_width: u8,   // 1-4 px per module
    pub bar_height: u16, // 80-300 px
    pub format: BarcodeFormat,
    pub auto_format: bool,
}

impl Default for BarcodeSettings {
    fn default() -> Self {
        Self {
            bar_width: 2,
            bar_height: 200,
            format: BarcodeFormat::Code128,
            auto_format: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SavedBarcode {
    pub name: String,
    pub text: String,
    pub format: BarcodeFormat,
}

pub struct BarcodeApp {
    pub state: AppState,
    pub menu_index: usize,
    pub input_text: String,
    pub barcode: Option<Barcode>,
    pub barcode_text: String,
    pub settings: BarcodeSettings,
    pub saved_codes: Vec<SavedBarcode>,
    pub load_index: usize,
    pub save_name: String,
    pub settings_index: usize,
    pub needs_redraw: bool,
    storage: Option<Storage>,
}

impl BarcodeApp {
    pub fn new() -> Self {
        Self {
            state: AppState::MainMenu,
            menu_index: 0,
            input_text: String::new(),
            barcode: None,
            barcode_text: String::new(),
            settings: BarcodeSettings::default(),
            saved_codes: Vec::new(),
            load_index: 0,
            save_name: String::new(),
            settings_index: 0,
            needs_redraw: true,
            storage: None,
        }
    }

    pub fn init_storage(&mut self) {
        match Storage::new() {
            Ok(mut s) => {
                if let Some(settings) = s.load_settings() {
                    self.settings = settings;
                }
                self.saved_codes = s.load_codes();
                self.storage = Some(s);
            }
            Err(e) => log::warn!("Failed to init storage: {:?}", e),
        }
    }

    pub fn save_settings(&mut self) {
        if let Some(ref mut s) = self.storage {
            s.save_settings(&self.settings);
        }
    }

    pub fn save_state(&mut self) {
        self.save_settings();
    }

    pub fn active_format(&self) -> BarcodeFormat {
        if self.settings.auto_format && !self.input_text.is_empty() {
            barcode_encode::auto_detect(&self.input_text)
        } else {
            self.settings.format
        }
    }

    /// Returns false if app should quit.
    pub fn handle_key(&mut self, key: char) -> bool {
        self.needs_redraw = true;
        match self.state {
            AppState::MainMenu => self.handle_menu_key(key),
            AppState::Input => self.handle_input_key(key),
            AppState::Display => self.handle_display_key(key),
            AppState::SavePrompt => self.handle_save_prompt_key(key),
            AppState::SaveNameEntry => self.handle_save_name_key(key),
            AppState::LoadList => self.handle_load_key(key),
            AppState::Settings => self.handle_settings_key(key),
            AppState::Help => self.handle_help_key(key),
        }
    }

    fn handle_menu_key(&mut self, key: char) -> bool {
        let items = MenuItem::all();
        match key {
            KEY_UP => {
                if self.menu_index > 0 {
                    self.menu_index -= 1;
                }
            }
            KEY_DOWN => {
                if self.menu_index < items.len() - 1 {
                    self.menu_index += 1;
                }
            }
            KEY_ENTER => match items[self.menu_index] {
                MenuItem::NewBarcode => {
                    self.input_text.clear();
                    self.state = AppState::Input;
                }
                MenuItem::SavedCodes => {
                    self.load_index = 0;
                    self.state = AppState::LoadList;
                }
                MenuItem::Settings => {
                    self.settings_index = 0;
                    self.state = AppState::Settings;
                }
                MenuItem::Help => self.state = AppState::Help,
            },
            'n' | 'N' => {
                self.input_text.clear();
                self.state = AppState::Input;
            }
            'q' | 'Q' => return false,
            _ => self.needs_redraw = false,
        }
        true
    }

    fn handle_input_key(&mut self, key: char) -> bool {
        match key {
            KEY_ENTER => {
                if !self.input_text.is_empty() {
                    self.generate_barcode();
                }
            }
            KEY_BACKSPACE => {
                self.input_text.pop();
            }
            'q' | 'Q' if self.input_text.is_empty() => {
                self.state = AppState::MainMenu;
            }
            // F1-F4: Set format
            '\u{F001}' => {
                self.settings.auto_format = false;
                self.settings.format = BarcodeFormat::Code128;
            }
            '\u{F002}' => {
                self.settings.auto_format = false;
                self.settings.format = BarcodeFormat::Code39;
            }
            '\u{F003}' => {
                self.settings.auto_format = false;
                self.settings.format = BarcodeFormat::Ean13;
            }
            '\u{F004}' => {
                self.settings.auto_format = false;
                self.settings.format = BarcodeFormat::UpcA;
            }
            _ => {
                if key.is_ascii_graphic() || key == ' ' {
                    if self.input_text.len() < MAX_TEXT_LEN {
                        self.input_text.push(key);
                    }
                } else {
                    self.needs_redraw = false;
                }
            }
        }
        true
    }

    fn generate_barcode(&mut self) {
        let format = self.active_format();
        match barcode_encode::encode(&self.input_text, format) {
            Some(barcode) => {
                self.barcode_text = self.input_text.clone();
                self.barcode = Some(barcode);
                self.state = AppState::Display;
            }
            None => {
                log::warn!("Failed to encode barcode: invalid input for {:?}", format);
            }
        }
    }

    fn handle_display_key(&mut self, key: char) -> bool {
        match key {
            'q' | 'Q' => self.state = AppState::MainMenu,
            'n' | 'N' => {
                self.input_text.clear();
                self.state = AppState::Input;
            }
            's' | 'S' => {
                self.save_name.clear();
                self.state = AppState::SavePrompt;
            }
            KEY_UP => {
                if self.settings.bar_height < 300 {
                    self.settings.bar_height += 20;
                    self.save_settings();
                }
            }
            KEY_DOWN => {
                if self.settings.bar_height > 80 {
                    self.settings.bar_height -= 20;
                    self.save_settings();
                }
            }
            KEY_RIGHT => {
                if self.settings.bar_width < 4 {
                    self.settings.bar_width += 1;
                    self.save_settings();
                }
            }
            KEY_LEFT => {
                if self.settings.bar_width > 1 {
                    self.settings.bar_width -= 1;
                    self.save_settings();
                }
            }
            _ => self.needs_redraw = false,
        }
        true
    }

    fn handle_save_prompt_key(&mut self, key: char) -> bool {
        match key {
            'y' | 'Y' | KEY_ENTER => {
                self.save_name.clear();
                self.state = AppState::SaveNameEntry;
            }
            'n' | 'N' | 'q' | 'Q' => self.state = AppState::Display,
            _ => self.needs_redraw = false,
        }
        true
    }

    fn handle_save_name_key(&mut self, key: char) -> bool {
        match key {
            KEY_ENTER => {
                if !self.save_name.is_empty() {
                    let code = SavedBarcode {
                        name: self.save_name.clone(),
                        text: self.barcode_text.clone(),
                        format: self.barcode.as_ref().map(|b| b.format).unwrap_or(BarcodeFormat::Code128),
                    };
                    self.saved_codes.push(code);
                    if let Some(ref mut s) = self.storage {
                        s.save_codes(&self.saved_codes);
                    }
                    self.state = AppState::Display;
                }
            }
            KEY_BACKSPACE => {
                self.save_name.pop();
            }
            'q' | 'Q' if self.save_name.is_empty() => self.state = AppState::Display,
            _ => {
                if key.is_ascii_graphic() || key == ' ' {
                    if self.save_name.len() < 30 {
                        self.save_name.push(key);
                    }
                } else {
                    self.needs_redraw = false;
                }
            }
        }
        true
    }

    fn handle_load_key(&mut self, key: char) -> bool {
        match key {
            KEY_UP => {
                if self.load_index > 0 {
                    self.load_index -= 1;
                }
            }
            KEY_DOWN => {
                if !self.saved_codes.is_empty() && self.load_index < self.saved_codes.len() - 1 {
                    self.load_index += 1;
                }
            }
            KEY_ENTER => {
                if !self.saved_codes.is_empty() {
                    let code = &self.saved_codes[self.load_index];
                    self.input_text = code.text.clone();
                    self.settings.format = code.format;
                    self.settings.auto_format = false;
                    self.generate_barcode();
                }
            }
            'd' | 'D' => {
                if !self.saved_codes.is_empty() {
                    self.saved_codes.remove(self.load_index);
                    if let Some(ref mut s) = self.storage {
                        s.save_codes(&self.saved_codes);
                    }
                    if self.load_index > 0 && self.load_index >= self.saved_codes.len() {
                        self.load_index = self.saved_codes.len().saturating_sub(1);
                    }
                }
            }
            'q' | 'Q' => self.state = AppState::MainMenu,
            _ => self.needs_redraw = false,
        }
        true
    }

    fn handle_settings_key(&mut self, key: char) -> bool {
        // 4 settings: format, auto-detect, bar width, bar height
        match key {
            KEY_UP => {
                if self.settings_index > 0 {
                    self.settings_index -= 1;
                }
            }
            KEY_DOWN => {
                if self.settings_index < 3 {
                    self.settings_index += 1;
                }
            }
            KEY_LEFT | KEY_RIGHT | KEY_ENTER => {
                match self.settings_index {
                    0 => {
                        self.settings.format = self.settings.format.next();
                    }
                    1 => {
                        self.settings.auto_format = !self.settings.auto_format;
                    }
                    2 => {
                        if key == KEY_RIGHT || key == KEY_ENTER {
                            self.settings.bar_width = (self.settings.bar_width % 4) + 1;
                        } else {
                            self.settings.bar_width =
                                if self.settings.bar_width <= 1 { 4 } else { self.settings.bar_width - 1 };
                        }
                    }
                    3 => {
                        if key == KEY_RIGHT || key == KEY_ENTER {
                            self.settings.bar_height =
                                (self.settings.bar_height + 20).min(300);
                        } else {
                            self.settings.bar_height =
                                self.settings.bar_height.saturating_sub(20).max(80);
                        }
                    }
                    _ => {}
                }
                self.save_settings();
            }
            'q' | 'Q' => self.state = AppState::MainMenu,
            _ => self.needs_redraw = false,
        }
        true
    }

    fn handle_help_key(&mut self, key: char) -> bool {
        match key {
            'q' | 'Q' | KEY_ENTER | KEY_BACKSPACE => self.state = AppState::MainMenu,
            _ => self.needs_redraw = false,
        }
        true
    }
}
