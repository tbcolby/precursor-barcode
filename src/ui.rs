//! Drawing for every screen. Sizes come from the canvas, never constants.

use gam::menu::{Point, Rectangle};
use gam::{GamObjectList, GamObjectType, GlyphStyle};
use precursor_appkit::layout::ListState;
use precursor_appkit::screen::MARGIN;

use crate::app::{App, View, MENU_ITEMS};

pub fn draw(app: &mut App) {
    app.screen.clear();
    match app.view {
        View::Menu => draw_menu(app),
        View::Input => draw_input(app),
        View::Display => draw_display(app),
        View::SaveName => draw_save_name(app),
        View::Saved => draw_saved(app),
        View::Settings => draw_settings(app),
        View::Help => draw_help(app),
    }
    app.screen.flush();
}

/// Draw the status line (if any) just above `bottom`; returns the new bottom.
fn status_line(app: &App, bottom: isize) -> isize {
    if app.status.is_empty() {
        return bottom;
    }
    let h = app.screen.line_height(GlyphStyle::Small);
    app.screen.text(bottom - h, GlyphStyle::Small, &app.status);
    bottom - h
}

fn draw_menu(app: &mut App) {
    let top = app.screen.header("Barcode Generator");
    let bottom = app.screen.footer("↑↓ move · ⏎ select · n new · ⌫ exit");
    let bottom = status_line(app, bottom);
    let info_h = app.screen.line_height(GlyphStyle::Small);
    app.screen.list(top, bottom - info_h, &MENU_ITEMS, &app.menu);
    let n = app.codes.len();
    let info = match n {
        0 => String::from("No saved codes"),
        1 => String::from("1 saved code"),
        _ => format!("{n} saved codes"),
    };
    app.screen.text(bottom - info_h, GlyphStyle::Small, &info);
}

fn draw_input(app: &mut App) {
    let s = &app.screen;
    let w = s.size().x;
    let mut y = s.header("New barcode");
    let bottom = s.footer("⏎ make · F1 format · ⌫ delete · F4 back");
    let bottom = status_line(app, bottom);

    let format = app.active_format();
    let mode = if app.settings.auto_format { "auto" } else { "fixed, F1 to change" };
    y += s.text(y, GlyphStyle::Small, &format!("Format: {} ({})", format.label(), mode));

    // input box: up to four lines of monospace text
    let line_h = s.line_height(GlyphStyle::Monospace);
    let box_bottom = (y + 4 * line_h + 8).min(bottom - 3 * line_h);
    s.frame(Point::new(MARGIN - 2, y), Point::new(w - MARGIN + 1, box_bottom), 1);
    let shown = if app.input.is_empty() { "(type text)" } else { app.input.as_str() };
    s.text_in(Point::new(MARGIN + 2, y + 4), Point::new(w - MARGIN - 2, box_bottom - 2), GlyphStyle::Monospace, shown);
    y = box_bottom + 6;

    y += s.text(y, GlyphStyle::Small, &format!("{}/{} characters", app.input.len(), app.input.max()));
    if app.input.is_empty() {
        s.text(y, GlyphStyle::Small, &format!("{} carries {}", format.label(), format.charset()));
    } else {
        let avail = app.bar_area_width();
        let msg = match app.encode_fitting(app.input.as_str(), format) {
            Ok(b) => {
                let px = barcode_core::fit_module_px(b.width(), avail, app.settings.bar_width as usize).unwrap_or(1);
                format!("OK: {} of {} modules, {} px each", b.width(), avail / px.max(1), px)
            }
            Err(e) => e,
        };
        s.text_in(Point::new(MARGIN, y), Point::new(w - MARGIN, bottom), GlyphStyle::Small, &msg);
    }
}

fn draw_display(app: &mut App) {
    let s = &app.screen;
    let size = s.size();
    let Some(b) = app.barcode.as_ref() else {
        s.text(4, GlyphStyle::Regular, "No barcode");
        return;
    };
    let bottom = s.footer("↑↓ height · ←→ width · s save · n new · ⌫ back");
    let bottom = status_line(app, bottom);
    let text_h = s.line_height(GlyphStyle::Monospace);
    let info_h = s.line_height(GlyphStyle::Small);

    let avail = size.x.max(0) as usize;
    let px = barcode_core::fit_module_px(b.width(), avail, app.settings.bar_width as usize).unwrap_or(1);
    let total = (b.width() * px) as isize;
    let max_h = (bottom - 8 - text_h - info_h - 8).max(20);
    let bar_h = (app.settings.bar_height as isize).min(max_h);
    let x0 = ((size.x - total) / 2).max(0);
    let y0 = ((bottom - bar_h - text_h - info_h) / 2).max(4);

    // One rectangle per dark run, sent in batches of 32.
    let gid = s.gid();
    let mut list = GamObjectList::new(gid);
    for (start, len) in b.dark_runs() {
        let x = x0 + (start * px) as isize;
        let rect = Rectangle::new_with_style(
            Point::new(x, y0),
            Point::new(x + (len * px) as isize - 1, y0 + bar_h - 1),
            gam::menu::DrawStyle::new(gam::PixelColor::Dark, gam::PixelColor::Dark, 0),
        );
        if let Err(obj) = list.push(GamObjectType::Rect(rect)) {
            s.gam().draw_list(list).ok();
            list = GamObjectList::new(gid);
            list.push(obj).ok();
        }
    }
    s.gam().draw_list(list).ok();

    let ty = y0 + bar_h + 4;
    s.text_centered(Point::new(0, ty), Point::new(size.x - 1, ty + text_h), GlyphStyle::Monospace, &b.text);
    let narrowed = if px < app.settings.bar_width as usize { " (narrowed to fit)" } else { "" };
    let info = format!("{} · {} px{} · {} tall", b.format.label(), px, narrowed, bar_h);
    s.text(ty + text_h, GlyphStyle::Small, &info);
}

fn draw_save_name(app: &mut App) {
    let s = &app.screen;
    let w = s.size().x;
    let mut y = s.header("Save barcode");
    let bottom = s.footer("⏎ save · ⌫ delete · F4 cancel");
    status_line(app, bottom);
    y += s.text(y, GlyphStyle::Regular, "Name:");
    let line_h = s.line_height(GlyphStyle::Monospace);
    s.frame(Point::new(MARGIN - 2, y), Point::new(w - MARGIN + 1, y + line_h + 6), 1);
    let shown = if app.name.is_empty() { "(type a name)" } else { app.name.as_str() };
    s.text_in(Point::new(MARGIN + 2, y + 3), Point::new(w - MARGIN - 2, y + line_h + 4), GlyphStyle::Monospace, shown);
    y += line_h + 12;
    y += s.text(y, GlyphStyle::Small, &format!("Stored in PDDB basis: {}", app.codes_basis()));
    s.text_in(
        Point::new(MARGIN, y),
        Point::new(w - MARGIN, bottom),
        GlyphStyle::Small,
        "Anyone who can see the screen can scan a displayed code.",
    );
}

fn draw_saved(app: &mut App) {
    let title = format!("Saved codes ({})", app.codes.len());
    let top = app.screen.header(&title);
    let bottom = app.screen.footer("⏎ show · d delete · ⌫ back");
    let bottom = status_line(app, bottom);
    if app.codes.is_empty() {
        app.screen.text(top + 8, GlyphStyle::Regular, "No saved codes.");
        return;
    }
    let labels: Vec<String> = (0..app.codes.len()).map(|i| app.codes.label(i)).collect();
    app.visible_rows = app.screen.list(top, bottom, &labels, &app.saved_list);
}

fn draw_settings(app: &mut App) {
    let top = app.screen.header("Settings");
    let bottom = app.screen.footer("↑↓ select · ←→ change · ⌫ back");
    let st = &app.settings;
    let rows = [
        format!("Default format: {}", st.format.label()),
        format!("Auto-detect: {}", if st.auto_format { "on" } else { "off" }),
        format!("Bar width: {} px", st.bar_width),
        format!("Bar height: {} px", st.bar_height),
    ];
    let state = ListState { selected: app.settings_row, top: 0 };
    app.screen.list(top, bottom, &rows, &state);
}

const HELP: &[&str] = &[
    "Formats",
    "  Code 128: printable ASCII",
    "  Code 39: A-Z 0-9 space - . $ / + %",
    "  EAN-13: 12 digits (+check)",
    "  UPC-A: 11 digits (+check)",
    "Auto-detect: 13 digits EAN-13,",
    "  12 digits UPC-A, Code 39 set,",
    "  else Code 128. A wrong check",
    "  digit is reported, not fixed.",
    "Input: ⏎ make, F1 format,",
    "  ⌫ delete / back, F4 back",
    "Display: ↑↓ height, ←→ width,",
    "  s save, n new, ⌫ back",
    "Saved: ⏎ show, d d delete",
    "Codes too wide for the screen",
    "  are narrowed, or refused.",
];

fn draw_help(app: &mut App) {
    let s = &app.screen;
    let mut y = s.header("Help");
    let bottom = s.footer("⌫ back");
    for line in HELP {
        let h = s.line_height(GlyphStyle::Small);
        if y + h > bottom {
            break;
        }
        y += s.text(y, GlyphStyle::Small, line);
    }
}
