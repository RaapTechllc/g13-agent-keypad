# Configuration reference

The GUI (Bindings / Profiles / Settings tabs) is the easy way to configure g13-driver and covers
everything most people need. This document is the reference for hand-editing the config files.

## Where config lives

Next to `g13-driver.exe`:

- `config.toml` — the **manifest**: global settings + which profile file each M-key uses.
- `profiles/*.toml` — one file per profile (a full binding set).

The app looks for `config.toml` **next to the exe first, then in the current directory**. Files are
**hot-reloaded** — save and changes apply immediately (no restart).

> A GUI **Save** rewrites the profile file and does **not** preserve comments or key order (known
> limitation). If you keep comments, edit by hand.

## The manifest (`config.toml`)

```toml
profiles_dir = "profiles"   # folder holding the profile files
m1 = "default.toml"         # profile loaded on M1
m2 = "game.toml"            # profile loaded on M2 (optional)
m3 = "media.toml"           # profile loaded on M3 (optional)

[autorepeat]                # optional; global auto-repeat timing
delay_ms = 400              # wait before repeating starts
interval_ms = 40            # gap between repeats (min 1)

[app]                       # managed by the GUI
start_active = false        # resume Active (true) or Dry-run (false) on launch
```

(A legacy `config.toml` that is itself a bare `[keys]` profile still works as a single M1 profile.)

## A profile file (`profiles/<name>.toml`)

```toml
[keys]
G1  = "ctrl+c"
G2  = "w"
BTN1 = "space"
STICK = "enter"

[repeat]        # optional: which bindings auto-repeat while held
G2 = true

[joystick]      # optional: map the stick to WASD
mode = "wasd"   # "wasd" is implemented; "mouse" parses but is not functional yet
deadzone = 30   # 0-127; distance from center before a direction fires
up = "w"
down = "s"
left = "a"
right = "d"
```

### Bindable inputs

`G1`–`G22`, `BTN1`, `BTN2` (the two thumb buttons), `STICK` (the joystick click). Names are
case-insensitive.

### Binding syntax

A binding is optional **modifiers** + one **key**: `modifier+modifier+key`. Modifiers: `ctrl`,
`shift`, `alt`, `win` (aliases `control`, `windows`, `super`). Examples: `a`, `ctrl+c`, `ctrl+shift+z`,
`win+d`. A modifier **alone** is allowed (e.g. `shift`). Bindings are **hold-means-hold** (held
while the G-key is held); multimedia keys are the tap-only exception.

### Valid key names

- **Letters:** `a`–`z`
- **Digits:** `0`–`9`
- **Function keys:** `f1`–`f24`
- **Modifiers:** `ctrl` (`control`), `shift`, `alt`, `win` (`windows`, `super`)
- **Editing / navigation:** `enter` (`return`), `esc` (`escape`), `space`, `tab`, `backspace`,
  `delete`, `insert`, `home`, `end`, `pageup`, `pagedown`, `up`, `down`, `left`, `right`
- **Locks / misc:** `capslock`, `printscreen`, `pause`, `numlock`, `scrolllock`
- **Media (tap-only):** `playpause`, `nexttrack` (`next`), `prevtrack` (`prev`), `mediastop`,
  `volup` (`volumeup`), `voldown` (`volumedown`), `mute`

An invalid key name is rejected (the GUI shows the binding as `bad`).

## Auto-repeat

Holding a bound key does **not** auto-repeat by default (a held key = one character). Tick **repeat**
on a binding (GUI) or add it to `[repeat]` to make it repeat while held, like a physical keyboard.
Timing is global in `[autorepeat]` (`delay_ms`, `interval_ms`). Joystick directions and modifier-only
bindings never repeat.

## Action bindings

A `[keys]` value is either a shortcut string (unchanged) or a table. Tables use the
planned form `[keys.G1]` with a `type` field. This repository is public: keep real
prompt text in your installed copy, not in git. The shipped files start with
`REPLACE_ME` so you can see which ones to replace.

```toml
[keys.G1]
type = "paste_text"
file = "prompts/v1-build/01-spec-pr.md"   # relative to the folder that holds config.toml
method = "clipboard"                       # or "type"
restore_clipboard = true

[keys.BTN2]
type = "launch_or_focus"
exe = '%LOCALAPPDATA%\Programs\cursor\Cursor.exe'
args = []
window_title = "Cursor"                    # optional substring
process_name = "Cursor.exe"                # optional; default is the exe file name
```

`paste_text` reads that file when the key goes down and pastes it into the focused
window. `file` must stay inside the config folder (no `..`, no drive, no symlink).
`method` is `clipboard` (default) or `type`. Clipboard mode saves the clipboard and
puts it back after the paste. `type` sends the characters with a Shift+Enter for each
line break.

`launch_or_focus` brings a matching window forward, or starts the `.exe` if none is
open. `exe` may use only `%LOCALAPPDATA%`, `%APPDATA%`, `%USERPROFILE%`,
`%ProgramFiles%`, and `%ProgramFiles(x86)%`, and the path after expansion must end in
`.exe`. `args` are literal strings. The program is not checked when the profile loads.

An action key fires once on key-down. It does not repeat, and key-up does nothing.
If a key is both a shortcut and an action, the action is the one that runs.

Paste never presses Enter by itself, so a chat input never auto-submits. Pasting
multi-line text into a terminal runs each line, because the terminal does. That
"never auto-submits" behavior holds only for chat inputs.

The Bindings tab shows an action as a read-only summary. Save keeps the action.

## Worked example

`profiles/game.toml` — WASD on the stick, jump on a thumb button, an auto-repeating attack:

```toml
[keys]
BTN1 = "space"     # jump
G1   = "e"         # interact
G2   = "r"         # reload / attack

[repeat]
G2 = true          # hold G2 to repeat "r"

[joystick]
mode = "wasd"
deadzone = 30
up = "w"
down = "s"
left = "a"
right = "d"
```
