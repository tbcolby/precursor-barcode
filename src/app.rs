//! Screens, key handling and persistence. Encoding and the saved-code model
//! are in `barcode-core`; drawing is in `ui.rs`.

use barcode_core::model::{self, MAX_NAME_LEN, MAX_TEXT_LEN};
use barcode_core::{encode, Barcode, Codes, Format, IndexRecord, LegacyCode, Settings, TextField};
use precursor_appkit::keys::Key;
use precursor_appkit::layout::ListState;
use precursor_appkit::screen::Screen;
use precursor_appkit::store::Store;

/// PDDB dictionaries (same names as version 0.1, so existing data is found).
pub const SETTINGS_DICT: &str = "barcode.settings";
pub const CODES_DICT: &str = "barcode.codes";
const SETTINGS_KEY: &str = "config";
const CODES_KEY: &str = "index";

#[derive(PartialEq, Eq)]
pub enum Action {
    Stay,
    Leave,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Menu,
    Input,
    Display,
    SaveName,
    Saved,
    Settings,
    Help,
}

pub const MENU_ITEMS: [&str; 4] = ["New barcode", "Saved codes", "Settings", "Help"];
pub const SETTINGS_ROWS: usize = 4;

pub struct App {
    pub(crate) screen: Screen,
    settings_store: Store,
    codes_store: Store,

    pub(crate) view: View,
    pub(crate) menu: ListState,
    pub(crate) saved_list: ListState,
    pub(crate) settings_row: usize,
    /// Rows of the saved list that fit, updated by the last draw.
    pub(crate) visible_rows: usize,

    pub(crate) settings: Settings,
    settings_dirty: bool,
    pub(crate) codes: Codes,
    /// Saved codes couldn't be read; refuse to overwrite them.
    codes_read_only: bool,

    pub(crate) input: TextField,
    pub(crate) name: TextField,
    /// The code on the Display screen and the format it was encoded with.
    pub(crate) barcode: Option<Barcode>,
    /// Where Backspace from the Display screen returns to.
    display_return: View,
    /// Delete needs a second press of `d`.
    pub(crate) confirm_delete: bool,
    /// One-line message (errors, confirmations). Cleared on the next key.
    pub(crate) status: String,
}

impl App {
    pub fn new(screen: Screen, settings_store: Store, codes_store: Store) -> Self {
        let mut app = Self {
            screen,
            settings_store,
            codes_store,
            view: View::Menu,
            menu: ListState::default(),
            saved_list: ListState::default(),
            settings_row: 0,
            visible_rows: 1,
            settings: Settings::default(),
            settings_dirty: false,
            codes: Codes::new(),
            codes_read_only: false,
            input: TextField::new(MAX_TEXT_LEN),
            name: TextField::new(MAX_NAME_LEN),
            barcode: None,
            display_return: View::Input,
            confirm_delete: false,
            status: String::new(),
        };
        app.reload();
        app
    }

    pub fn screen(&self) -> &Screen { &self.screen }

    /// (Re)load from the PDDB. Called at start and whenever the set of open
    /// bases changes, so nothing from a just-locked basis stays on screen.
    pub fn reload(&mut self) {
        self.settings = match self.settings_store.load::<Settings>(SETTINGS_KEY) {
            Ok(Some(s)) => s.sanitized(),
            Ok(None) => Settings::default(),
            Err(e) => {
                log::warn!("settings unreadable ({:?}); using defaults", e.kind());
                Settings::default()
            }
        };
        self.settings_dirty = false;
        self.load_codes();
        // Anything on screen may have come from a basis that just closed.
        self.barcode = None;
        self.input.clear();
        self.name.clear();
        self.confirm_delete = false;
        self.saved_list = ListState::default();
        self.view = View::Menu;
    }

    fn load_codes(&mut self) {
        self.codes_read_only = false;
        self.codes = match self.codes_store.load::<IndexRecord>(CODES_KEY) {
            Ok(None) => Codes::new(),
            Ok(Some(IndexRecord::Current(c))) => match c.check_version() {
                Ok(()) => c,
                Err(v) => {
                    log::warn!("saved codes are version {}, newer than this app", v);
                    self.codes_read_only = true;
                    self.status = "Saved codes are from a newer app version".into();
                    Codes::new()
                }
            },
            Ok(Some(IndexRecord::Legacy(names))) => self.migrate_legacy(names),
            Err(e) => {
                log::error!("saved codes unreadable: {:?}", e.kind());
                self.codes_read_only = true;
                self.status = "Couldn't read saved codes".into();
                Codes::new()
            }
        };
    }

    /// Convert the 0.1 layout (an index of names plus one `code.<name>` key
    /// each) to a single record, then delete every old payload key, including
    /// orphans that 0.1's delete never removed.
    fn migrate_legacy(&mut self, names: Vec<String>) -> Codes {
        let entries = names
            .into_iter()
            .map(|n| {
                let payload = self.codes_store.load::<LegacyCode>(&model::legacy_code_key(&n)).ok().flatten();
                (n, payload)
            })
            .collect();
        let codes = model::migrate_legacy(entries);
        if let Err(e) = self.codes_store.save(CODES_KEY, &codes) {
            log::error!("migration save failed: {:?}", e.kind());
            self.codes_read_only = true;
            return codes;
        }
        for key in self.codes_store.keys().unwrap_or_default() {
            if model::is_legacy_code_key(&key) {
                if let Err(e) = self.codes_store.delete(&key) {
                    log::warn!("couldn't delete old key: {:?}", e.kind());
                }
            }
        }
        log::info!("migrated {} saved codes to the v1 layout", codes.len());
        codes
    }

    fn save_codes(&mut self) -> bool {
        if self.codes_read_only {
            self.status = "Saved codes are read-only (see above)".into();
            return false;
        }
        match self.codes_store.save(CODES_KEY, &self.codes) {
            Ok(()) => true,
            Err(e) => {
                log::error!("save failed: {:?}", e.kind());
                self.status = "Save failed".into();
                false
            }
        }
    }

    fn save_settings(&mut self) {
        if !self.settings_dirty {
            return;
        }
        match self.settings_store.save(SETTINGS_KEY, &self.settings) {
            Ok(()) => self.settings_dirty = false,
            Err(e) => log::error!("settings save failed: {:?}", e.kind()),
        }
    }

    /// Basis that holds (or would receive) the saved-codes record.
    pub(crate) fn codes_basis(&self) -> String {
        self.codes_store
            .basis_of(CODES_KEY)
            .ok()
            .flatten()
            .or_else(|| self.codes_store.target_basis())
            .unwrap_or_else(|| "?".into())
    }

    /// Format the Input screen will use for the current text.
    pub(crate) fn active_format(&self) -> Format {
        if self.settings.auto_format {
            barcode_core::auto_detect(self.input.as_str())
        } else {
            self.settings.format
        }
    }

    /// Canvas width available for bars.
    pub(crate) fn bar_area_width(&self) -> usize { self.screen.size().x.max(0) as usize }

    pub fn on_foreground(&mut self) {
        self.screen.refresh_size();
        self.draw();
    }

    /// Save only what changed. Saved codes are written when added or deleted.
    pub fn on_background(&mut self) { self.save_settings(); }

    pub fn draw(&mut self) { crate::ui::draw(self); }

    pub fn on_key(&mut self, key: Key) -> Action {
        self.status.clear();
        if !matches!(key, Key::Char('d') | Key::Char('D')) {
            self.confirm_delete = false;
        }
        match self.view {
            View::Menu => self.key_menu(key),
            View::Input => self.key_input(key),
            View::Display => self.key_display(key),
            View::SaveName => self.key_save_name(key),
            View::Saved => self.key_saved(key),
            View::Settings => self.key_settings(key),
            View::Help => {
                if key.is_back() || key == Key::Enter {
                    self.view = View::Menu;
                }
                Action::Stay
            }
        }
    }

    fn key_menu(&mut self, key: Key) -> Action {
        match key {
            Key::Up => self.menu.step_wrapping(-1, MENU_ITEMS.len(), MENU_ITEMS.len()),
            Key::Down => self.menu.step_wrapping(1, MENU_ITEMS.len(), MENU_ITEMS.len()),
            Key::Enter => match self.menu.selected {
                0 => self.start_input(),
                1 => {
                    self.saved_list.clamp(self.codes.len(), self.visible_rows);
                    self.view = View::Saved;
                }
                2 => {
                    self.settings_row = 0;
                    self.view = View::Settings;
                }
                _ => self.view = View::Help,
            },
            Key::Char('n') | Key::Char('N') => self.start_input(),
            k if k.is_back() => {
                self.save_settings();
                return Action::Leave;
            }
            _ => {}
        }
        Action::Stay
    }

    fn start_input(&mut self) {
        self.input.clear();
        self.view = View::Input;
    }

    fn key_input(&mut self, key: Key) -> Action {
        match key {
            Key::Enter => self.generate(),
            Key::Backspace => {
                if !self.input.pop() {
                    self.view = View::Menu;
                }
            }
            Key::F(4) => self.view = View::Menu,
            Key::F(1) => {
                // Cycle Auto → Code 128 → Code 39 → EAN-13 → UPC-A → Auto.
                if self.settings.auto_format {
                    self.settings.auto_format = false;
                    self.settings.format = Format::Code128;
                } else if self.settings.format == Format::UpcA {
                    self.settings.auto_format = true;
                } else {
                    self.settings.format = self.settings.format.next();
                }
                self.settings_dirty = true;
            }
            Key::Char(c) => {
                if !self.input.push(c) && self.input.len() >= self.input.max() {
                    self.status = format!("Limit is {} characters", self.input.max());
                }
            }
            _ => {}
        }
        Action::Stay
    }

    /// Encode the input and show it, or explain why not.
    fn generate(&mut self) {
        let format = self.active_format();
        match self.encode_fitting(self.input.as_str(), format) {
            Ok(b) => {
                self.barcode = Some(b);
                self.display_return = View::Input;
                self.view = View::Display;
            }
            Err(msg) => self.status = msg,
        }
    }

    /// Encode and check the result fits the screen at 1 px per module.
    pub(crate) fn encode_fitting(&self, text: &str, format: Format) -> Result<Barcode, String> {
        let b = encode(text, format).map_err(|e| e.to_string())?;
        let avail = self.bar_area_width();
        if barcode_core::fit_module_px(b.width(), avail, 1).is_none() {
            return Err(format!("Too long: {} modules, screen fits {}", b.width(), avail));
        }
        Ok(b)
    }

    fn key_display(&mut self, key: Key) -> Action {
        match key {
            Key::Up => self.adjust(Settings::taller),
            Key::Down => self.adjust(Settings::shorter),
            Key::Right => self.adjust(Settings::wider),
            Key::Left => self.adjust(Settings::narrower),
            Key::Char('s') | Key::Char('S') => {
                self.name.clear();
                self.view = View::SaveName;
            }
            Key::Char('n') | Key::Char('N') => self.start_input(),
            k if k.is_back() => {
                self.save_settings();
                self.view = self.display_return;
            }
            _ => {}
        }
        Action::Stay
    }

    fn adjust(&mut self, f: fn(&mut Settings)) {
        let before = self.settings.clone();
        f(&mut self.settings);
        if self.settings != before {
            self.settings_dirty = true;
        }
    }

    fn key_save_name(&mut self, key: Key) -> Action {
        match key {
            Key::Enter => {
                let Some(b) = self.barcode.as_ref() else {
                    self.view = View::Menu;
                    return Action::Stay;
                };
                // The symbol's text includes any appended check digit, so
                // reloading re-encodes exactly the same symbol.
                let (text, format) = (b.text.clone(), b.format);
                match self.codes.add(self.name.as_str(), &text, format) {
                    Ok(()) => {
                        if self.save_codes() {
                            self.status = format!("Saved \"{}\"", self.name.as_str().trim());
                        } else {
                            self.codes.remove(self.codes.len() - 1);
                        }
                        self.view = View::Display;
                    }
                    Err(e) => self.status = e.to_string(),
                }
            }
            Key::Backspace => {
                if !self.name.pop() {
                    self.view = View::Display;
                }
            }
            Key::F(4) => self.view = View::Display,
            Key::Char(c) => {
                self.name.push(c);
            }
            _ => {}
        }
        Action::Stay
    }

    fn key_saved(&mut self, key: Key) -> Action {
        let len = self.codes.len();
        match key {
            Key::Up => self.saved_list.step(-1, len, self.visible_rows),
            Key::Down => self.saved_list.step(1, len, self.visible_rows),
            Key::Enter => {
                if let Some(c) = self.codes.get(self.saved_list.selected) {
                    // Use the code's own format; don't touch the global setting.
                    let (text, format) = (c.text.clone(), c.format);
                    match self.encode_fitting(&text, format) {
                        Ok(b) => {
                            self.barcode = Some(b);
                            self.display_return = View::Saved;
                            self.view = View::Display;
                        }
                        Err(msg) => self.status = msg,
                    }
                }
            }
            Key::Char('d') | Key::Char('D') if len > 0 => {
                if !self.confirm_delete {
                    self.confirm_delete = true;
                    self.status = "Press d again to delete".into();
                } else {
                    self.confirm_delete = false;
                    let i = self.saved_list.selected;
                    if let Some(removed) = self.codes.remove(i) {
                        if self.save_codes() {
                            self.status = format!("Deleted \"{}\"", removed.name);
                        } else {
                            self.codes.codes.insert(i, removed);
                        }
                    }
                    self.saved_list.clamp(self.codes.len(), self.visible_rows);
                }
            }
            k if k.is_back() => self.view = View::Menu,
            _ => {}
        }
        Action::Stay
    }

    fn key_settings(&mut self, key: Key) -> Action {
        let forward = matches!(key, Key::Right | Key::Enter);
        match key {
            Key::Up => self.settings_row = self.settings_row.checked_sub(1).unwrap_or(SETTINGS_ROWS - 1),
            Key::Down => self.settings_row = (self.settings_row + 1) % SETTINGS_ROWS,
            Key::Left | Key::Right | Key::Enter => {
                match self.settings_row {
                    0 => {
                        self.settings.format =
                            if forward { self.settings.format.next() } else { self.settings.format.prev() }
                    }
                    1 => self.settings.auto_format = !self.settings.auto_format,
                    2 => {
                        if forward {
                            self.settings.wider()
                        } else {
                            self.settings.narrower()
                        }
                    }
                    _ => {
                        if forward {
                            self.settings.taller()
                        } else {
                            self.settings.shorter()
                        }
                    }
                }
                self.settings_dirty = true;
            }
            k if k.is_back() => {
                self.save_settings();
                self.view = View::Menu;
            }
            _ => {}
        }
        Action::Stay
    }
}
