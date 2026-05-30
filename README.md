# Cursory

[한국어](README.ko.md)

> Confine the mouse cursor to a window, monitor, or custom rectangle on Windows.

![license](https://img.shields.io/badge/license-MIT-blue)
![platform](https://img.shields.io/badge/platform-Windows-lightgrey)

![Cursory demo](assets/demo.webp)

## Features

- **Three confine modes** — lock the cursor to an app window, a monitor, or a
  custom rectangle you draw on screen.
- **Global hotkey** — toggle confinement from anywhere; record, preview, and
  confirm a new combo in-app (default `Ctrl+Alt+L`).
- **System tray** — minimize to tray and restore on click; choose whether the
  close button exits the app or hides it to the tray.
- **Launch on startup** — optional, via the per-user registry Run key.
- **Single instance** — a second launch surfaces the running window instead of
  opening a duplicate.
- **Live updates** — re-applies automatically when the resolution or monitor
  layout changes.
- **Padding** — inset the confine area by a configurable margin.

## Install

Download the latest build from the
[Releases](https://github.com/devsepnine/cursory/releases) page:

- **`Cursory-x.y.z.msi`** — installer; adds a Start Menu entry (search "Cursory").
- **`Cursory-x.y.z.exe`** — portable; run it directly, no install required.

> The binary is unsigned, so Windows SmartScreen may warn on first run. Choose
> **More info → Run anyway**.

## Usage

1. Pick a **mode**: App window / Monitor / Custom rect.
2. Choose the target — a window from the list, a monitor, or draw a rectangle.
3. Press **ACTIVATE** (or the global hotkey) to confine the cursor; press again
   to release.

- **Hotkey** — click *Change* in Settings, press a combo, then *Confirm*.
- **Close button** — set it to send to tray or exit the app.
- **Launch on startup** / **Minimize on activate** — toggle in Settings.

Settings persist to `%APPDATA%\cursory\settings.conf`.

## Build from source

Requires the Rust toolchain (1.85+, edition 2024).

```powershell
cargo build --release
# -> target/release/cursory.exe
```

### Packaging an MSI

Requires [WiX Toolset v3.14](https://github.com/wixtoolset/wix3/releases) and
`cargo-wix` (the script installs `cargo-wix` automatically).

```powershell
pwsh scripts/release.ps1
# -> dist/Cursory-<version>.msi and dist/Cursory-<version>.exe
```

## License

[MIT](LICENSE) © HibiCanvas
