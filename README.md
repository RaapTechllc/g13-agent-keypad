# g13-driver

[![CI](https://github.com/RaapTechllc/g13-agent-keypad/actions/workflows/ci.yml/badge.svg)](https://github.com/RaapTechllc/g13-agent-keypad/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/RaapTechllc/g13-agent-keypad)](https://github.com/RaapTechllc/g13-agent-keypad/releases/latest)
[![License: GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-blue)](LICENSE)

An open-source replacement driver for the **Logitech G13** gaming keypad on Windows. Logitech's
official software is abandonware; this brings the G13 back to life with a small, modern app.

## Fork notice and attribution

This repository is a fork of [cavefish-dev/g13-driver](https://github.com/cavefish-dev/g13-driver),
licensed under GPL-3.0-or-later. Modifications are by RaapTech LLC (see [NOTICE](NOTICE) and
[LICENSE](LICENSE)). The in-app update check is off in this build. New builds come from this
fork's Releases page.

This repo is public. The files under `prompts/` are placeholders. Keep real prompt text in
your installed copy, not in git.

## Agent workflow keys

M1 loads **V1 build**: G1–G4 paste `prompts/v1-build/` (spec, grill-me, verify, ship).
M2 and M3 load **Workflow 2** and **Workflow 3** (G1–G3 paste `prompts/workflow-2/` and
`prompts/workflow-3/`). On every profile, BTN1 is Grok Bot and BTN2 is Cursor.

BTN1 ships as `%LOCALAPPDATA%\Programs\REPLACE_ME\REPLACE_ME.exe` because that path is not
verified. For a browser app, set `exe` to the browser and put the URL in `args`. BTN2 uses
the per-user Cursor install; a system install is `%ProgramFiles%\cursor\Cursor.exe`.

Before plugging in the keypad:

```text
g13-driver.exe --check-config
g13-driver.exe --simulate "M1,G1,G2,G3,G4,BTN1,BTN2,M2,G1"
```

`--check-config` and `--simulate` do not open USB or inject keys. Paste goes to the focused
window and does not press Enter, so a chat box does not submit. See
[docs/configuration.md](docs/configuration.md) and [docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md).

## What it does

- Remap the **G-keys** (G1–G22), the two **thumb buttons**, and the **joystick click** to any key
  or keyboard shortcut (e.g. `ctrl+c`, `alt+tab`).
- Map the **joystick** to WASD (hold-to-move).
- **Profiles** on the M1/M2/M3 keys — switch whole binding sets on the fly.
- Runs quietly in the **system tray**; optional **auto-start at login**.
- Configure everything in a simple GUI — no config files required.

## Requirements

- Windows 10 or 11
- A Logitech G13 keypad
- About 5 minutes for a one-time driver setup

## Quick start

### 1. Download

Grab the latest `g13-driver-vX.Y.Z-windows-x64.zip` from this fork's
[**Releases**](https://github.com/RaapTechllc/g13-agent-keypad/releases/latest) page and **extract the
whole folder**. Keep `g13-driver.exe`, `config.toml`, `profiles/`, and `prompts/` together.

### 2. One-time driver setup (Zadig)

Windows needs to let this app talk to the G13 over USB. You do this once with a free tool called
**Zadig**, which installs the generic **WinUSB** driver on the G13. It takes a minute and is
reversible.

Follow the step-by-step guide: **[docs/zadig-setup.md](docs/zadig-setup.md)**.

> This does not delete Logitech's software — it just points the G13 at WinUSB so g13-driver can read
> it. You can switch back anytime.

### 3. Run it

Double-click **`g13-driver.exe`**.

Because the app isn't code-signed yet, Windows **SmartScreen** may show *"Windows protected your
PC"*. Click **More info → Run anyway**. This is expected for a small open-source app; the code is
public in this repo and signing is on the roadmap.

### 4. Turn it on

The app starts in **Dry-run** mode — it shows what you press but injects nothing (safe for testing).
When you're ready, switch to **Active** (top-right toggle, or the tray menu) and your bindings take
effect.

## Using it

- **Tray app:** closing or minimizing the window **hides it to the tray** — the driver keeps
  running. Use **Quit** in the tray menu to actually exit.
- **Status icon:** green = Active, grey = Dry-run, **red = the G13 isn't connected** (run Zadig / check
  the cable). It auto-reconnects when you plug the G13 back in.
- **Auto-start at login:** enable it in **Settings** (or the tray menu) so the driver is ready every
  time you log in.

## Configuring

Everything is done in the GUI — you never have to touch a file:

- **Bindings tab:** click a key row, type the key or shortcut (e.g. `ctrl+c`), tick **repeat** to make
  it auto-repeat while held, then **Save**.
- **Profiles tab:** M1/M2/M3 each load a profile; press the M-key (or click the slot) to switch.
- **Joystick / auto-repeat:** joystick→WASD and per-binding auto-repeat (the **repeat** checkbox) are
  set in the GUI; global repeat timing (`delay_ms`/`interval_ms`) lives in `config.toml` (see the reference).

Power users can hand-edit the TOML config files next to the exe — see
**[docs/configuration.md](docs/configuration.md)** for the full reference.

## Updating

Update checks are off in this build. Download a newer zip from this fork's
[Releases](https://github.com/RaapTechllc/g13-agent-keypad/releases/latest) page and replace your
files (keep your edited `config.toml`, `profiles/`, and `prompts/`).

## Troubleshooting

| Problem | Fix |
|---|---|
| Tray icon is **red** / "not connected" | Run the [Zadig setup](docs/zadig-setup.md); check the USB cable. |
| Keys do nothing | You're in **Dry-run** — switch to **Active**. |
| "Windows protected your PC" | SmartScreen on an unsigned app — **More info → Run anyway**. |
| A binding won't save | The key name is invalid — see [configuration.md](docs/configuration.md) for valid names. |

## Building from source

Windows, with the **GNU** Rust toolchain (not MSVC) and a MinGW-w64 `gcc` (for `rusb`'s bundled
libusb):

```sh
rustup default stable-x86_64-pc-windows-gnu
# A MinGW-w64 gcc must be on PATH (e.g. MSYS2's mingw64 or Strawberry Perl) — see CONTRIBUTING.md.
cargo test
cargo build --release   # -> target/release/g13-driver.exe
```

See **[CONTRIBUTING.md](CONTRIBUTING.md)** for the full developer setup and workflow.

## Roadmap

Roughly where things are headed (direction, not promises):

- **Done:** key/thumb/stick remapping, joystick→WASD, M-key profiles, GUI monitor + bindings editor,
  hold-means-hold + media keys, auto-repeat, tray background app, CI + GitHub releases.
- **Next:** more agent-workflow profiles. In-app auto-update stays off in this fork.
- **Later:** macros + shell commands, the G13 LCD (160×43), RGB backlight, Linux support, and a
  standalone GUI configurator.

## Contributing & license

Contributions welcome — see [CONTRIBUTING.md](CONTRIBUTING.md). Licensed under
**GPL-3.0-or-later** (see [LICENSE](LICENSE)); contributions are accepted under the same license.

Thanks to the broader community whose G13 reverse-engineering made an open driver possible.
