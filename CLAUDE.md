# Barcode Generator: agent notes

- Package / manifest key: `barcode` → `gam::APP_NAME_BARCODE`. Server name `_Barcode Generator_`.
- `UxType::Framebuffer` via `precursor_appkit::app::register`. No pump or timers: nothing runs in the background.
- Pure logic in `core/` (`barcode-core`): encoders (`encode.rs`), settings/saved codes/0.1 migration (`model.rs`), tests with reference vectors from reportlab (`tests.rs`). Any encoder change needs a reference-vector test.
- Storage through `precursor_appkit::store::Store`: `barcode.settings/config`, `barcode.codes/index` (one record for all codes so delete removes the payload). Reload on basis change.
- Every screen exits on Backspace or F4. Never log keystrokes or saved payloads (`SavedCode`'s `Debug` redacts the text).
- Gate: `xous-dev-toolkit/bin/integrate --xous-root <tree> --grade <this repo>`.
- `DESIGN.md` and `AGENTS.md` are the 0.1 design notes, kept for history; README is authoritative.
