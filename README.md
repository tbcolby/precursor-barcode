# Precursor Barcode Generator

Shows standard 1D barcodes on the Precursor's 1-bit display so a scanner can read them off the screen. Type text or digits, and the app encodes it as Code 128, Code 39, EAN-13 or UPC-A.

Status: builds at the toolkit's xous-core anchor, clippy-clean, 33 host tests pass. **Not yet verified on Renode or hardware**: `capture.py` is written but the Renode capture hasn't been run, and no barcode has been scanned off a real device yet.

## Features

| Format | Accepts | Notes |
|---|---|---|
| Code 128 | Printable ASCII (space to `~`) | Set B for text, set C (two digits per symbol) for runs of 4+ digits. Mod-103 checksum. No set A, so no control characters. |
| Code 39 | `A-Z 0-9 space - . $ / + %` | 3:1 wide:narrow. No lowercase (use Code 128). No optional mod-43 check character. |
| EAN-13 | 12 digits, or 13 with a correct check digit | Quiet zones 11 / 7 modules. |
| UPC-A | 11 digits, or 12 with a correct check digit | Quiet zones 9 / 9 modules. |

- **Auto-detect** (on by default): 13 digits → EAN-13, 12 digits → UPC-A, Code 39 characters only → Code 39, anything else → Code 128. F1 on the input screen cycles a fixed format instead.
- **Check digits** are appended when you leave them off. A wrong check digit is **reported, not corrected**, so a typo can't become a valid barcode for a different product.
- **Fit to screen**: bars are drawn at your preferred width (1–4 px per module) and narrowed automatically if the code is too wide. A code that doesn't fit even at 1 px is refused with the module count; the input screen shows whether the current text fits as you type. At 1 px the screen holds about 25 Code 128 symbols or about 20 Code 39 characters.
- **Saved codes** with unique names, stored in the PDDB. Delete asks for confirmation (`d` twice) and removes the payload.
- Human-readable text under the bars (with the check digit for EAN/UPC).

The encoders are in `core/` (`barcode-core`, no Xous dependencies). Their tests compare full module patterns against reference output from an independent encoder (reportlab) for Code 128, Code 39, EAN-13 and UPC-A, and check the Code 128 and Code 39 tables for the structural rules of their standards.

## Keys

Backspace or F4 leaves every screen. The `∴` key opens the system menu and never reaches the app.

| Screen | Key | Action |
|---|---|---|
| Menu | ↑ ↓, Enter | Choose New barcode / Saved codes / Settings / Help |
| | `n` | New barcode |
| | Backspace, F4 | Leave the app (it stays running in the background) |
| Input | type | Text (printable ASCII, up to 80 characters) |
| | Enter | Make the barcode, or show why it can't be made |
| | F1 | Cycle format: Auto → Code 128 → Code 39 → EAN-13 → UPC-A → Auto |
| | Backspace | Delete a character; on an empty field, back to the menu |
| | F4 | Back to the menu |
| Barcode | ↑ ↓ | Taller / shorter bars (40–300 px, limited by the screen) |
| | ← → | Narrower / wider modules (1–4 px) |
| | `s` | Save under a name |
| | `n` | New barcode |
| | Backspace, F4 | Back |
| Save | type, Enter | Name (up to 24 characters), save |
| | Backspace / F4 | Delete a character (empty: cancel) / cancel |
| Saved codes | ↑ ↓, Enter | Show the selected code (in its own format) |
| | `d` `d` | Delete the selected code |
| | Backspace, F4 | Back |
| Settings | ↑ ↓, ← → / Enter | Choose a row, change it |
| | Backspace, F4 | Back (saves changed settings) |

## Storage

| PDDB dictionary | Key | Contents |
|---|---|---|
| `barcode.settings` | `config` | `{"version":1,"bar_width":2,"bar_height":160,"format":"code128","auto_format":true}` |
| `barcode.codes` | `index` | `{"version":1,"codes":[{"name":…,"text":…,"format":…}]}` |

- Records are replaced with delete-then-write in the basis that already holds them. New records go to the most recently unlocked basis; the save screen shows which basis the saved codes live in.
- Settings are written only when they changed, when you leave the Settings or barcode screen or the app loses focus. Saved codes are written when you add or delete one.
- When a PDDB basis is locked or unlocked the app reloads everything and returns to the menu, so codes from a locked basis don't stay on screen.
- A record that can't be read, or that a newer version wrote, is never overwritten; the app shows a message instead.
- **Upgrading from 0.1**: 0.1 stored one `code.<name>` key per code and its delete never removed them. On first load the app moves those codes into the single `index` record and deletes every `code.*` key, including ones 0.1 failed to delete.

## Limitations and security notes

- A barcode on screen can be read by any camera or scanner in view. Don't display codes for secrets in public.
- The PDDB encrypts saved codes at rest. That is the extent of the protection; the app adds none of its own.
- Not implemented: Code 128 set A (control characters), Code 39 full-ASCII mode and check character, EAN-8, UPC-E, add-on codes, GS1-128 (FNC1), importing codes over the network.
- Screen geometry (bar placement, text sizes) hasn't been checked on a device yet.
- Renode capture not yet run.

## Building

The app builds inside a [xous-core](https://github.com/betrusted-io/xous-core) tree. With [xous-dev-toolkit](https://github.com/tbcolby/xous-dev-toolkit):

```bash
bin/integrate --xous-root ~/xous-core --grade ~/Dev/precursor/precursor-barcode   # build, clippy, tests, lint
cd ~/xous-core && cargo xtask renode-image barcode                                  # emulator image
```

`bin/integrate` copies the app to `apps/barcode`, merges `manifest-fragment.json` (key `barcode`) into `apps/manifest.json`, and adds the workspace member. Host tests alone: `cd core && cargo test`.

## Layout

```
Cargo.toml               app crate: gam, precursor-appkit, barcode-core
core/src/encode.rs       Code 128 / Code 39 / EAN-13 / UPC-A encoders
core/src/model.rs        settings, saved codes, 0.1 migration, text field
core/src/tests.rs        reference vectors and regression tests
src/main.rs              GAM registration and message loop
src/app.rs               screens, keys, persistence
src/ui.rs                drawing
capture.py               Renode capture sequence (not yet run)
```

## Author

Tyler Colby, [Colby's Data Movers, LLC](https://colbysdatamovers.com). Licensed under the Apache License, Version 2.0 (see [LICENSE](LICENSE)).
