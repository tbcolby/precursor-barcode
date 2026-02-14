# Barcode Generator — Agent Instructions

## App Identity
- **Package**: `barcode`
- **SERVER_NAME**: `_Barcode Generator_`
- **APP_NAME**: `Barcode`
- **manifest context_name**: `Barcode`
- **TCP Port**: 7881 (import, not yet implemented)

## Architecture
- **UX Type**: `UxType::Chat` with raw keyboard input
- **State Machine**: 8 states (MainMenu, Input, Display, SavePrompt, SaveNameEntry, LoadList, Settings, Help)
- **Threading**: None
- **PDDB**: 2 dictionaries (`barcode.settings`, `barcode.codes`)

## Barcode Encoder (`barcode_encode.rs`)
Zero Xous dependencies. Standalone encoder.

### Formats:
- **Code 128**: Full ASCII. Auto-switches Subset B (text) / C (digit pairs). Weighted checksum mod 103. Each symbol = 11 modules.
- **Code 39**: A-Z, 0-9, 7 special chars. Narrow/wide (1:3 ratio). Self-clocking with start/stop asterisks.
- **EAN-13**: 13 digits. L/G/R parity encoding. Check digit auto-computed.
- **UPC-A**: 12 digits. Encoded as EAN-13 with leading 0.

### Output format:
`Vec<bool>` — module-level dark/light pattern. Rendering just iterates and draws dark rectangles.

## Patterns Reused from App #1
- Header/footer drawing helpers (identical)
- JSON PDDB storage with index+items pattern (identical)
- State machine with `needs_redraw` flag (identical)
- Menu navigation with highlight bar (identical)
- Standard key constants (identical)
- Focus-aware save (identical)

## Patterns Evolved
- **Format auto-detection**: Input analysis → format selection. New pattern for encoding apps.
- **Settings with 4 items**: Extended settings screen with more options than QR app.
- **1D bar rendering**: Simple left-to-right rectangle iteration vs QR's 2D grid.

## Build
```bash
cargo build -p barcode --target riscv32imac-unknown-xous-elf
cargo xtask renode-image barcode
```
