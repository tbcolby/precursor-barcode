# BARCODE GENERATOR
## Design Specification

---

## Problem Statement

You have an ISBN, a tracking number, a product code, an inventory label, or a string of text that needs to become a scannable barcode. Your Precursor has a razor-sharp 1-bit display — vertical black bars on white background is literally what it was born to render.

Type it. Encode it. Scan it with any barcode reader. No printer needed.

---

## Precursor Fit: Ideal

| Constraint | Advantage |
|-----------|-----------|
| 1-bit monochrome | Barcodes ARE black bars on white — native rendering |
| 336px wide | Code 128 at 2px/module: up to ~130 characters. Generous. |
| 536px tall | Bars can be 200+ pixels tall — scannable from distance |
| Physical keyboard | Direct text/number entry |
| PDDB encrypted | Saved barcodes (serial numbers, credentials) encrypted at rest |

---

## Features

### Must Have (P0)
- **Code 128** — Full ASCII, auto-switching between subsets A/B/C
- **Code 39** — Uppercase A-Z, 0-9, space, and 7 special chars
- **EAN-13** — 13-digit product codes (European Article Number)
- **UPC-A** — 12-digit product codes (Universal Product Code)
- Auto-select optimal encoding for Code 128 (subset switching)
- Human-readable text below barcode
- Adjustable bar width (1-4px per module)

### Should Have (P1)
- Save/load barcodes to PDDB
- Auto-detect format (pure digits → EAN/UPC, else Code 128)
- Checksum computation (auto for EAN-13, UPC-A, Code 128)
- Adjustable bar height

### Could Have (P2)
- TCP import (port 7881)
- Code 128 subset override
- Interleaved 2-of-5

### Won't Have (v1)
- 2D barcodes (that's what QR app is for)
- Barcode scanning/reading
- PDF417

---

## Screen Flows

```
                    ┌──────────┐
                    │ MainMenu │
                    └─────┬────┘
                          │
              ┌───────────┼───────────┐
              ▼           ▼           ▼
        ┌──────────┐ ┌────────┐ ┌──────────┐
        │  Input   │ │  Load  │ │ Settings │
        │  (type)  │ │  List  │ │          │
        └─────┬────┘ └───┬────┘ └──────────┘
              │           │
              ▼           ▼
        ┌──────────┐ ┌────────┐
        │ Display  │ │Display │
        │ Barcode  │ │Barcode │
        └──────────┘ └────────┘
```

---

## Keyboard Mapping (Ecosystem Standard)

### MainMenu
| Key | Action |
|-----|--------|
| Up/Down | Navigate |
| Enter | Select |
| N | New barcode |
| Q | Quit |
| H | Help |

### Input Mode
| Key | Action |
|-----|--------|
| A-Z, 0-9, symbols | Type |
| Backspace | Delete |
| Enter | Generate |
| F1 | Format: Code 128 |
| F2 | Format: Code 39 |
| F3 | Format: EAN-13 |
| F4 | Format: UPC-A |
| Q (empty input) | Back |

### Display Mode
| Key | Action |
|-----|--------|
| S | Save |
| N | New |
| Up/Down | Adjust bar height |
| Left/Right | Adjust bar width |
| Q | Back |

---

## PDDB Schema

### Dictionary: `barcode.codes`
| Key | Format | Description |
|-----|--------|-------------|
| `index` | JSON array | Ordered list of saved code names |
| `code.{name}` | JSON | `{ "text": "...", "format": "code128", "created": "..." }` |

### Dictionary: `barcode.settings`
| Key | Format | Description |
|-----|--------|-------------|
| `config` | JSON | `{ "bar_width": 2, "bar_height": 200, "default_format": "auto" }` |

---

## Barcode Encoding

### Code 128
- 3 subsets: A (control+upper), B (printable ASCII), C (digit pairs)
- Start code selects initial subset; shift/switch codes change mid-stream
- Auto-optimization: use C for runs of 4+ digits, B for text
- Checksum: weighted modular sum mod 103
- Encoding: each symbol = 11 modules (6 bars), except stop = 13

### Code 39
- Self-clocking, no checksum required (optional mod 43)
- Each character = 5 bars + 4 spaces = 9 modules (3 wide, 6 narrow)
- Inter-character gap
- Start/stop: asterisk (*)

### EAN-13
- 13 digits: 1 number system + 12 data (last is check digit)
- Left guard (101) + 6 left digits + center guard (01010) + 6 right digits + right guard (101)
- Left digits use L/G parity encoding based on first digit
- Check digit: alternating weight 1/3 mod 10

### UPC-A
- 12 digits: EAN-13 with leading 0
- Same structure, essentially a subset of EAN-13

---

## Rendering

### Bar Layout
- Bars span full content width centered, with quiet zones
- Bar height: configurable (100-300px default 200px)
- Centered vertically in content area
- Human-readable text below bars in Monospace font
- Format label and character count in Small font

### Module Sizing
- Code 128: each symbol = 11 modules, bar_width = px per module
- At 2px/module, 100-char Code 128 ≈ (100+3)*11*2 + guards ≈ 2,266px → exceeds 336px
- Practical limit at 2px: ~13 chars. At 1px: ~28 chars
- EAN-13: fixed 95 modules × bar_width → at 3px = 285px (fits perfectly)

### UX Type
`UxType::Chat` — standard layout. Bars drawn as filled rectangles.

---

## Complexity Estimate

| Metric | Estimate |
|--------|----------|
| Total LOC | ~1,200-1,500 |
| Modules | 5 (main, app, barcode_encode, ui, storage) |
| Threading | None |
| PDDB | 2 dictionaries |
| Key challenge | Code 128 subset auto-switching |
