# Precursor Barcode Generator

**Black bars on white. The display was born for this.**

```
"A barcode is a machine-readable representation of data
in a visual, scannable format. It is the oldest and most
universal language that bridges the physical and digital.
A label printer costs money. A 336-pixel display is free."
```

---

## What This Is

Precursor Barcode Generator creates standard 1D barcodes on the Precursor's monochrome display. Type text or numbers, select a format, and a scannable barcode appears. Point any barcode scanner — handheld, phone app, checkout terminal — at the screen. Done.

Four barcode standards. From-scratch encoders. Zero external dependencies.

---

## Why This Project

Barcodes are everywhere. Product shelves, shipping labels, library books, boarding passes, inventory warehouses. They are the most widely deployed machine-readable data format on the planet — over 6 billion scans per day.

Every one of those barcodes was printed on paper or plastic. What if you didn't need a printer?

The Precursor display is 336 pixels wide, 1-bit monochrome — black or white, nothing between. A barcode is black bars on a white background. The display doesn't need to *simulate* a barcode. It *is* a barcode. Each dark pixel is as dark as any printed bar. Each light pixel is as light as any printed gap. The contrast ratio is effectively infinite for scanner optics.

Type an ISBN. Generate a barcode. Hold it under the library's scanner. No paper, no ink, no printer. Just photons.

---

## Why Precursor

**1-bit display = perfect barcode contrast.** No grayscale bleeding, no LCD color fringing, no anti-aliasing softening edges. Every bar boundary is pixel-perfect.

**336px width is generous.** An EAN-13 barcode is 95 modules. At 3px per module = 285px. Fits with quiet zones. Code 128 is variable width — practical encoding of 15-20 characters at 2px/module.

**Physical keyboard for fast entry.** Numeric codes (EAN, UPC) use the top row. Text codes (Code 128, Code 39) use the full keyboard. No on-screen keyboard stealing screen space from the barcode.

**PDDB for encrypted storage.** Serial numbers, tracking codes, credential barcodes — saved and encrypted at rest.

---

## How It Works

### Supported Formats

| Format | Characters | Use Case |
|--------|-----------|----------|
| **Code 128** | Full ASCII (0-127) | General purpose, shipping, logistics |
| **Code 39** | A-Z, 0-9, space, -.$/+% | Military, automotive, ID badges |
| **EAN-13** | 13 digits | International product codes |
| **UPC-A** | 12 digits | US/Canada product codes |

### Features

- **Auto-detect format** — digits → EAN/UPC, uppercase → Code 39, mixed → Code 128
- **Auto checksum** — EAN-13, UPC-A, and Code 128 checksums computed automatically
- **Code 128 subset optimization** — auto-switches between B (text) and C (digit pairs)
- **Adjustable bar width** — 1-4px per module
- **Adjustable bar height** — 80-300px
- **Human-readable text** below barcode
- **Save/load** to encrypted PDDB
- **Format override** via F-keys

### Keyboard Controls

#### Main Menu
| Key | Action |
|-----|--------|
| Up/Down | Navigate |
| Enter | Select |
| N | New barcode |
| Q | Quit |

#### Input
| Key | Action |
|-----|--------|
| Type | Enter text/numbers |
| Enter | Generate barcode |
| F1 | Force Code 128 |
| F2 | Force Code 39 |
| F3 | Force EAN-13 |
| F4 | Force UPC-A |
| Q (empty) | Back |

#### Display
| Key | Action |
|-----|--------|
| S | Save barcode |
| N | New barcode |
| Up/Down | Adjust bar height |
| Left/Right | Adjust bar width |
| Q | Back |

#### Saved Codes
| Key | Action |
|-----|--------|
| Enter | Load and display |
| D | Delete selected |
| Q | Back |

---

## Screenshots

*Captured via headless Renode emulation on macOS ARM64.*

*(Screenshots pending Renode testing)*

---

## Technical Architecture

```
apps/barcode/
├── Cargo.toml           # Dependencies: xous, gam, pddb, serde
└── src/
    ├── main.rs          # Entry point, event loop, GAM registration
    ├── app.rs           # State machine, input handling, settings
    ├── barcode_encode.rs # Complete barcode encoder: Code 128/39, EAN-13, UPC-A
    ├── ui.rs            # Screen rendering for all states
    └── storage.rs       # PDDB persistence
```

### Design Decisions

**Four formats, one encoder module.** All barcode encoding lives in `barcode_encode.rs` with a unified `encode(text, format) -> Barcode` API. Each format has its own encoding function but shares the output format: a `Vec<bool>` of dark/light modules.

**Code 128 subset auto-switching.** The encoder automatically uses Subset C (digit pairs → 2 digits per symbol) for runs of 4+ consecutive digits, and Subset B (printable ASCII) for everything else. This produces optimal-length barcodes without user intervention.

**Rendering as rectangles.** Each dark module is a filled rectangle `bar_width` pixels wide and `bar_height` pixels tall. Light modules are simply gaps (the white background). This is simpler and faster than the QR code's 2D grid — just a 1D array of bars.

**Auto-detect via input analysis.** If auto-detect is on: 13 digits → EAN-13, 12 digits → UPC-A, all uppercase/digits/symbols → Code 39, anything else → Code 128. Users can override with F-keys.

### PDDB Storage Layout

| Dictionary | Key | Contents |
|-----------|-----|----------|
| `barcode.settings` | `config` | `{ "format": "code128", "bar_width": 2, "bar_height": 200, "auto_format": true }` |
| `barcode.codes` | `index` | JSON array of saved barcode names |
| `barcode.codes` | `code.{name}` | `{ "text": "...", "format": "code128" }` |

### Dependencies

```toml
[dependencies]
xous = "0.9.69"
xous-ipc = "0.10.9"
gam = { path = "../../services/gam" }
pddb = { path = "../../services/pddb" }
ticktimer-server = { package = "xous-api-ticktimer", version = "0.9.68" }
log-server = { package = "xous-api-log", version = "0.1.68" }
xous-names = { package = "xous-api-names", version = "0.9.70" }
num-derive = { version = "0.4.2", default-features = false }
num-traits = { version = "0.2.14", default-features = false }
serde = { version = "1.0", default-features = false, features = ["derive", "alloc"] }
serde_json = { version = "1.0", default-features = false, features = ["alloc"] }
log = "0.4.14"
```

---

## Building

Precursor Barcode Generator is a Xous app. It builds as part of the [xous-core](https://github.com/betrusted-io/xous-core) workspace.

### Integration

1. Copy `src/` and `Cargo.toml` to `xous-core/apps/barcode/`

2. Add to workspace `Cargo.toml`:
   ```toml
   "apps/barcode",
   ```

3. Add to `apps/manifest.json`:
   ```json
   "barcode": {
       "context_name": "Barcode",
       "menu_name": {
           "appmenu.barcode": {
               "en": "Barcode",
               "en-tts": "Barcode"
           }
       }
   }
   ```

4. Build for Renode emulator:
   ```bash
   cargo xtask renode-image barcode
   ```

5. Build for hardware:
   ```bash
   cargo xtask app-image barcode
   ```

---

## Technical Notes

- All barcode encoding is pure Rust with zero external dependencies
- Code 128 checksum: weighted modular sum mod 103
- EAN-13 check digit: alternating weight 1/3 mod 10
- UPC-A encoded as EAN-13 with leading zero
- Code 39 uses narrow/wide (1:3) bar ratio
- Auto-detect picks optimal format from input content
- Bar width and height adjustable in real-time on display screen

---

## Development

This app was developed using the methodology described in [xous-dev-toolkit](https://github.com/tbcolby/xous-dev-toolkit) — an LLM-assisted approach to Precursor app development on macOS ARM64.

App #2 in the 32-app Precursor ecosystem campaign. Leveraged the `encoding.md` specialist agent created during App #1 (QR Code Generator).

---

## Author

Made by Tyler Colby — [Colby's Data Movers, LLC](https://colbysdatamovers.com)

Contact: [tyler@colbysdatamovers.com](mailto:tyler@colbysdatamovers.com) | [GitHub Issues](https://github.com/tbcolby/precursor-barcode/issues)

---

## License

Licensed under the Apache License, Version 2.0.

See [LICENSE](LICENSE) for the full text.
