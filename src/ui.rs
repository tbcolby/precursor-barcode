//! UI rendering for the Barcode Generator.

use crate::app::{AppState, BarcodeApp, MenuItem};
use crate::barcode_encode;

use gam::*;

const SCREEN_WIDTH: isize = 336;
const HEADER_HEIGHT: isize = 30;
const FOOTER_HEIGHT: isize = 46;
const CONTENT_TOP: isize = HEADER_HEIGHT;
const CONTENT_BOTTOM: isize = 536 - FOOTER_HEIGHT;
const CONTENT_HEIGHT: isize = CONTENT_BOTTOM - CONTENT_TOP;

const REGULAR_HEIGHT: isize = 15;
const LINE_GAP: isize = 4;
const LINE_HEIGHT: isize = REGULAR_HEIGHT + LINE_GAP;

pub fn draw(app: &BarcodeApp, gam: &Gam, canvas: graphics_server::Gid) {
    // Clear screen
    let clear = graphics_server::Rectangle::new_coords_with_style(
        0, 0, SCREEN_WIDTH, 536,
        graphics_server::DrawStyle::new(
            graphics_server::PixelColor::Light,
            graphics_server::PixelColor::Light,
            0,
        ),
    );
    gam.draw_rectangle(canvas, clear).ok();

    match app.state {
        AppState::MainMenu => draw_main_menu(app, gam, canvas),
        AppState::Input => draw_input(app, gam, canvas),
        AppState::Display => draw_display(app, gam, canvas),
        AppState::SavePrompt => draw_save_prompt(app, gam, canvas),
        AppState::SaveNameEntry => draw_save_name(app, gam, canvas),
        AppState::LoadList => draw_load_list(app, gam, canvas),
        AppState::Settings => draw_settings(app, gam, canvas),
        AppState::Help => draw_help(app, gam, canvas),
    }

    gam.redraw().ok();
}

fn draw_header(gam: &Gam, canvas: graphics_server::Gid, title: &str) {
    let bg = graphics_server::Rectangle::new_coords_with_style(
        0, 0, SCREEN_WIDTH, HEADER_HEIGHT,
        graphics_server::DrawStyle::new(
            graphics_server::PixelColor::Dark,
            graphics_server::PixelColor::Dark,
            0,
        ),
    );
    gam.draw_rectangle(canvas, bg).ok();

    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(8, 2, SCREEN_WIDTH - 8, HEADER_HEIGHT - 2)),
    );
    tv.style = GlyphStyle::Bold;
    tv.invert = true;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "{}", title).ok();
    gam.post_textview(&mut tv).ok();
}

fn draw_footer(gam: &Gam, canvas: graphics_server::Gid, labels: &[&str; 4]) {
    let y = CONTENT_BOTTOM;
    let zone_width = SCREEN_WIDTH / 4;

    let sep = graphics_server::Line::new_with_style(
        Point::new(0, y), Point::new(SCREEN_WIDTH, y),
        graphics_server::DrawStyle::new(
            graphics_server::PixelColor::Dark,
            graphics_server::PixelColor::Dark,
            1,
        ),
    );
    gam.draw_line(canvas, sep).ok();

    for (i, label) in labels.iter().enumerate() {
        if label.is_empty() { continue; }
        let x = (i as isize) * zone_width;
        let mut tv = TextView::new(
            canvas,
            TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                x + 2, y + 4, x + zone_width - 2, y + FOOTER_HEIGHT - 4,
            )),
        );
        tv.style = GlyphStyle::Small;
        tv.draw_border = false;
        tv.margin = Point::new(0, 0);
        write!(tv, "F{}: {}", i + 1, label).ok();
        gam.post_textview(&mut tv).ok();
    }
}

fn draw_main_menu(app: &BarcodeApp, gam: &Gam, canvas: graphics_server::Gid) {
    draw_header(gam, canvas, "Barcode Generator");

    let items = MenuItem::all();
    for (i, item) in items.iter().enumerate() {
        let y = CONTENT_TOP + 20 + (i as isize) * (LINE_HEIGHT + 8);
        let selected = i == app.menu_index;

        if selected {
            let hl = graphics_server::Rectangle::new_coords_with_style(
                8, y - 2, SCREEN_WIDTH - 8, y + LINE_HEIGHT + 2,
                graphics_server::DrawStyle::new(
                    graphics_server::PixelColor::Dark,
                    graphics_server::PixelColor::Dark,
                    0,
                ),
            );
            gam.draw_rectangle(canvas, hl).ok();
        }

        let mut tv = TextView::new(
            canvas,
            TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                16, y, SCREEN_WIDTH - 16, y + LINE_HEIGHT,
            )),
        );
        tv.style = GlyphStyle::Regular;
        tv.invert = selected;
        tv.draw_border = false;
        tv.margin = Point::new(0, 0);
        write!(tv, "{}", item.label()).ok();
        gam.post_textview(&mut tv).ok();
    }

    if !app.saved_codes.is_empty() {
        let y = CONTENT_TOP + 20 + (LINE_HEIGHT + 8) * 4 + 20;
        let mut tv = TextView::new(
            canvas,
            TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(16, y, SCREEN_WIDTH - 16, y + LINE_HEIGHT)),
        );
        tv.style = GlyphStyle::Small;
        tv.draw_border = false;
        tv.margin = Point::new(0, 0);
        write!(tv, "{} saved barcodes", app.saved_codes.len()).ok();
        gam.post_textview(&mut tv).ok();
    }

    draw_footer(gam, canvas, &["", "", "", ""]);
}

fn draw_input(app: &BarcodeApp, gam: &Gam, canvas: graphics_server::Gid) {
    draw_header(gam, canvas, "Enter Text");

    // Instructions
    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            8, CONTENT_TOP + 8, SCREEN_WIDTH - 8, CONTENT_TOP + 8 + LINE_HEIGHT * 2,
        )),
    );
    tv.style = GlyphStyle::Small;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "Type text, press Enter to generate barcode").ok();
    gam.post_textview(&mut tv).ok();

    // Input box
    let input_top = CONTENT_TOP + 40;
    let input_bottom = CONTENT_BOTTOM - 100;

    let border = graphics_server::Rectangle::new_coords_with_style(
        8, input_top, SCREEN_WIDTH - 8, input_bottom,
        graphics_server::DrawStyle {
            fill_color: Some(graphics_server::PixelColor::Light),
            stroke_color: Some(graphics_server::PixelColor::Dark),
            stroke_width: 1,
        },
    );
    gam.draw_rectangle(canvas, border).ok();

    let display_text = if app.input_text.is_empty() { "(empty)" } else { &app.input_text };
    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            16, input_top + 8, SCREEN_WIDTH - 16, input_bottom - 8,
        )),
    );
    tv.style = GlyphStyle::Monospace;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "{}", display_text).ok();
    gam.post_textview(&mut tv).ok();

    // Status line
    let y_status = input_bottom + 8;
    let format = app.active_format();
    let valid = if app.input_text.is_empty() {
        true
    } else {
        barcode_encode::is_valid(&app.input_text, format)
    };

    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            8, y_status, SCREEN_WIDTH - 8, y_status + LINE_HEIGHT * 3,
        )),
    );
    tv.style = GlyphStyle::Small;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    if app.input_text.is_empty() {
        write!(
            tv,
            "Format: {} | Auto: {}\n{}px wide, {}px tall",
            format.label(),
            if app.settings.auto_format { "On" } else { "Off" },
            app.settings.bar_width,
            app.settings.bar_height,
        ).ok();
    } else {
        write!(
            tv,
            "{}ch | {} | {}\n{}",
            app.input_text.len(),
            format.label(),
            if valid { "OK" } else { "INVALID" },
            if !valid { "Input not valid for this format" } else { "" },
        ).ok();
    }
    gam.post_textview(&mut tv).ok();

    draw_footer(gam, canvas, &["C128", "C39", "EAN13", "UPC-A"]);
}

fn draw_display(app: &BarcodeApp, gam: &Gam, canvas: graphics_server::Gid) {
    if let Some(ref barcode) = app.barcode {
        let bar_w = app.settings.bar_width as isize;
        let bar_h = app.settings.bar_height as isize;
        let total_w = barcode.modules.len() as isize * bar_w;

        // Center barcode
        let x_offset = (SCREEN_WIDTH - total_w).max(0) / 2;
        let y_offset = (CONTENT_HEIGHT - bar_h - 40).max(0) / 2 + CONTENT_TOP;

        // If barcode is too wide, just start from left edge with small margin
        let x_start = if total_w > SCREEN_WIDTH - 8 { 4 } else { x_offset };

        // Draw bars
        for (i, &dark) in barcode.modules.iter().enumerate() {
            if dark {
                let x = x_start + (i as isize) * bar_w;
                if x + bar_w > SCREEN_WIDTH {
                    break; // clip to screen
                }
                let rect = graphics_server::Rectangle::new_coords_with_style(
                    x, y_offset, x + bar_w, y_offset + bar_h,
                    graphics_server::DrawStyle::new(
                        graphics_server::PixelColor::Dark,
                        graphics_server::PixelColor::Dark,
                        0,
                    ),
                );
                gam.draw_rectangle(canvas, rect).ok();
            }
        }

        // Human-readable text below bars
        let text_y = y_offset + bar_h + 8;
        if text_y + LINE_HEIGHT < CONTENT_BOTTOM {
            let mut tv = TextView::new(
                canvas,
                TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                    8, text_y, SCREEN_WIDTH - 8, text_y + LINE_HEIGHT,
                )),
            );
            tv.style = GlyphStyle::Monospace;
            tv.draw_border = false;
            tv.margin = Point::new(0, 0);
            write!(tv, "{}", barcode.text).ok();
            gam.post_textview(&mut tv).ok();
        }

        // Status line
        let status_y = text_y + LINE_HEIGHT + 4;
        if status_y + LINE_HEIGHT < 536 {
            let mut tv = TextView::new(
                canvas,
                TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                    4, status_y, SCREEN_WIDTH - 4, status_y + LINE_HEIGHT,
                )),
            );
            tv.style = GlyphStyle::Small;
            tv.draw_border = false;
            tv.margin = Point::new(0, 0);
            write!(
                tv,
                "{} {}w {}h  S:save N:new Q:back",
                barcode.format.short(),
                bar_w,
                bar_h,
            ).ok();
            gam.post_textview(&mut tv).ok();
        }
    }
}

fn draw_save_prompt(app: &BarcodeApp, gam: &Gam, canvas: graphics_server::Gid) {
    draw_display(app, gam, canvas);

    let dialog_y = 200isize;
    let dialog_h = 80isize;
    let bg = graphics_server::Rectangle::new_coords_with_style(
        30, dialog_y, SCREEN_WIDTH - 30, dialog_y + dialog_h,
        graphics_server::DrawStyle {
            fill_color: Some(graphics_server::PixelColor::Light),
            stroke_color: Some(graphics_server::PixelColor::Dark),
            stroke_width: 2,
        },
    );
    gam.draw_rectangle(canvas, bg).ok();

    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            44, dialog_y + 12, SCREEN_WIDTH - 44, dialog_y + dialog_h - 12,
        )),
    );
    tv.style = GlyphStyle::Regular;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "Save this barcode?\n\nY: Yes  N: No").ok();
    gam.post_textview(&mut tv).ok();
}

fn draw_save_name(app: &BarcodeApp, gam: &Gam, canvas: graphics_server::Gid) {
    draw_header(gam, canvas, "Save Barcode");

    let y = CONTENT_TOP + 30;
    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(16, y, SCREEN_WIDTH - 16, y + LINE_HEIGHT)),
    );
    tv.style = GlyphStyle::Regular;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "Enter a name:").ok();
    gam.post_textview(&mut tv).ok();

    let box_y = y + LINE_HEIGHT + 16;
    let border = graphics_server::Rectangle::new_coords_with_style(
        16, box_y, SCREEN_WIDTH - 16, box_y + LINE_HEIGHT + 16,
        graphics_server::DrawStyle {
            fill_color: Some(graphics_server::PixelColor::Light),
            stroke_color: Some(graphics_server::PixelColor::Dark),
            stroke_width: 1,
        },
    );
    gam.draw_rectangle(canvas, border).ok();

    let display = if app.save_name.is_empty() { "(type a name)" } else { &app.save_name };
    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            24, box_y + 4, SCREEN_WIDTH - 24, box_y + LINE_HEIGHT + 12,
        )),
    );
    tv.style = GlyphStyle::Monospace;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "{}", display).ok();
    gam.post_textview(&mut tv).ok();

    let instr_y = box_y + LINE_HEIGHT + 30;
    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(16, instr_y, SCREEN_WIDTH - 16, instr_y + LINE_HEIGHT)),
    );
    tv.style = GlyphStyle::Small;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "Enter: save | Q: cancel").ok();
    gam.post_textview(&mut tv).ok();

    draw_footer(gam, canvas, &["", "", "", ""]);
}

fn draw_load_list(app: &BarcodeApp, gam: &Gam, canvas: graphics_server::Gid) {
    draw_header(gam, canvas, "Saved Barcodes");

    if app.saved_codes.is_empty() {
        let mut tv = TextView::new(
            canvas,
            TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                16, CONTENT_TOP + 30, SCREEN_WIDTH - 16, CONTENT_TOP + 30 + LINE_HEIGHT * 2,
            )),
        );
        tv.style = GlyphStyle::Regular;
        tv.draw_border = false;
        tv.margin = Point::new(0, 0);
        write!(tv, "No saved barcodes.\n\nPress Q to go back.").ok();
        gam.post_textview(&mut tv).ok();
    } else {
        let max_visible = ((CONTENT_HEIGHT - 20) / (LINE_HEIGHT + 6)) as usize;
        let scroll_offset = if app.load_index >= max_visible {
            app.load_index - max_visible + 1
        } else {
            0
        };

        for (vi, i) in (scroll_offset..app.saved_codes.len()).take(max_visible).enumerate() {
            let code = &app.saved_codes[i];
            let y = CONTENT_TOP + 12 + (vi as isize) * (LINE_HEIGHT + 6);
            let selected = i == app.load_index;

            if selected {
                let hl = graphics_server::Rectangle::new_coords_with_style(
                    4, y - 2, SCREEN_WIDTH - 4, y + LINE_HEIGHT + 2,
                    graphics_server::DrawStyle::new(
                        graphics_server::PixelColor::Dark,
                        graphics_server::PixelColor::Dark,
                        0,
                    ),
                );
                gam.draw_rectangle(canvas, hl).ok();
            }

            let mut tv = TextView::new(
                canvas,
                TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                    12, y, SCREEN_WIDTH - 12, y + LINE_HEIGHT,
                )),
            );
            tv.style = GlyphStyle::Regular;
            tv.invert = selected;
            tv.draw_border = false;
            tv.margin = Point::new(0, 0);

            let preview: String = if code.text.len() > 16 {
                let mut s: String = code.text.chars().take(13).collect();
                s.push_str("...");
                s
            } else {
                code.text.clone()
            };
            write!(tv, "{} [{}] {}", code.name, code.format.short(), preview).ok();
            gam.post_textview(&mut tv).ok();
        }
    }

    draw_footer(gam, canvas, &["", "", "", ""]);
}

fn draw_settings(app: &BarcodeApp, gam: &Gam, canvas: graphics_server::Gid) {
    draw_header(gam, canvas, "Settings");

    let items: [(&str, &str); 4] = [
        ("Format", app.settings.format.label()),
        ("Auto-Detect", if app.settings.auto_format { "On" } else { "Off" }),
        ("Bar Width", match app.settings.bar_width {
            1 => "1px", 2 => "2px", 3 => "3px", 4 => "4px", _ => "2px",
        }),
        ("Bar Height", match app.settings.bar_height {
            80 => "80px", 100 => "100px", 120 => "120px", 140 => "140px",
            160 => "160px", 180 => "180px", 200 => "200px", 220 => "220px",
            240 => "240px", 260 => "260px", 280 => "280px", 300 => "300px",
            _ => "200px",
        }),
    ];

    for (i, (label, value)) in items.iter().enumerate() {
        let y = CONTENT_TOP + 20 + (i as isize) * (LINE_HEIGHT + 12);
        let selected = i == app.settings_index;

        if selected {
            let hl = graphics_server::Rectangle::new_coords_with_style(
                8, y - 4, SCREEN_WIDTH - 8, y + LINE_HEIGHT + 4,
                graphics_server::DrawStyle::new(
                    graphics_server::PixelColor::Dark,
                    graphics_server::PixelColor::Dark,
                    0,
                ),
            );
            gam.draw_rectangle(canvas, hl).ok();
        }

        let mut tv = TextView::new(
            canvas,
            TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(16, y, 180, y + LINE_HEIGHT)),
        );
        tv.style = GlyphStyle::Regular;
        tv.invert = selected;
        tv.draw_border = false;
        tv.margin = Point::new(0, 0);
        write!(tv, "{}", label).ok();
        gam.post_textview(&mut tv).ok();

        let mut tv = TextView::new(
            canvas,
            TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(190, y, SCREEN_WIDTH - 16, y + LINE_HEIGHT)),
        );
        tv.style = GlyphStyle::Bold;
        tv.invert = selected;
        tv.draw_border = false;
        tv.margin = Point::new(0, 0);
        write!(tv, "<  {}  >", value).ok();
        gam.post_textview(&mut tv).ok();
    }

    let y = CONTENT_TOP + 20 + (items.len() as isize) * (LINE_HEIGHT + 12) + 16;
    let mut tv = TextView::new(
        canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(16, y, SCREEN_WIDTH - 16, y + LINE_HEIGHT * 2)),
    );
    tv.style = GlyphStyle::Small;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "Up/Down: select | Left/Right: change\nQ: back").ok();
    gam.post_textview(&mut tv).ok();

    draw_footer(gam, canvas, &["", "", "", ""]);
}

fn draw_help(_app: &BarcodeApp, gam: &Gam, canvas: graphics_server::Gid) {
    draw_header(gam, canvas, "Help");

    let help_text = [
        "Barcode Generator v0.1",
        "",
        "FORMATS",
        "  Code 128: Full ASCII",
        "  Code 39: A-Z, 0-9, symbols",
        "  EAN-13: 12-13 digit products",
        "  UPC-A: 11-12 digit products",
        "",
        "INPUT",
        "  Type text, Enter to generate",
        "  F1: Code 128  F2: Code 39",
        "  F3: EAN-13    F4: UPC-A",
        "",
        "DISPLAY",
        "  S: Save  N: New  Q: Back",
        "  Up/Down: Bar height",
        "  Left/Right: Bar width",
        "",
        "SAVED CODES",
        "  Enter: Load  D: Delete",
        "",
        "Auto-detect picks format",
        "from your input text.",
    ];

    for (i, line) in help_text.iter().enumerate() {
        let y = CONTENT_TOP + 4 + (i as isize) * (REGULAR_HEIGHT + 2);
        if y + REGULAR_HEIGHT > CONTENT_BOTTOM { break; }

        let mut tv = TextView::new(
            canvas,
            TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(8, y, SCREEN_WIDTH - 8, y + REGULAR_HEIGHT)),
        );
        tv.style = if line.starts_with("Barcode") || line.starts_with("FORMATS")
            || line.starts_with("INPUT") || line.starts_with("DISPLAY")
            || line.starts_with("SAVED") || line.starts_with("Auto") {
            GlyphStyle::Bold
        } else {
            GlyphStyle::Small
        };
        tv.draw_border = false;
        tv.margin = Point::new(0, 0);
        write!(tv, "{}", line).ok();
        gam.post_textview(&mut tv).ok();
    }

    draw_footer(gam, canvas, &["", "", "", ""]);
}
