# Troubleshooting

## Symptom table

| What you see | What it usually means |
|---|---|
| `G13 not found` | WinUSB is not bound to USB ID 046D C21C. Follow [ZADIG.md](ZADIG.md). |
| `failed to claim USB interface 0` | Logitech Gaming Software, LCore, or another tool already has interface 0 open. |
| Presses show in the monitor and nothing is typed | The app is in Dry-run, or the target window is elevated. UIPI blocks `SendInput` with no error. |
| A multi-line prompt pasted into a terminal runs each line | A terminal treats each pasted line as input. That warning is only about terminals. |
| Memory Integrity will not turn on | Only relevant when Windows Security lists an incompatible driver. Leave Memory Integrity on. |

## LGS is holding the device

`failed to claim USB interface 0` means something else owns the interface. Quit Logitech
Gaming Software and LCore (check the tray and Task Manager), then start g13-driver again.
If LGS is set to start with Windows, exit it before this app. The same message appears
when another libusb tool already has the G13 open.

## Driver conflict or rollback

`G13 not found` means the keypad is not on WinUSB. A Windows Update or a replug can put
the HID driver back. Repeat the Zadig steps in [ZADIG.md](ZADIG.md). To go the other way,
use the revert section there (uninstall, delete the driver, replug). Logitech's removal
article covers leftover Gaming Software packages:
https://support.logi.com/hc/en-us/articles/19426387012119-Unable-to-turn-on-Memory-integrity-on-Windows-10-or-11

## Memory Integrity (Core Isolation)

Look at this only when Windows Security lists a driver that blocks Memory Integrity.
Microsoft describes the setting here:
https://support.microsoft.com/en-us/windows/core-isolation-e30ed737-17d8-42f3-a2a9-87521df09b78

Leave Memory Integrity on. If Windows Security names a Logitech driver, remove that
driver with Logitech's article above. Do not turn Memory Integrity off to make the
keypad work.

## Keys show in the monitor but nothing is typed

The monitor shows USB reports in both Dry-run and Active. Dry-run never injects.
Switch the tray (or the window) to **Active**.

If the app is Active and a normal window still gets nothing, the focused window may be
running elevated. User Interface Privilege Isolation drops `SendInput` from a
non-elevated process, and the call can fail silently. Run the target app as the same
user, without elevation. See
https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput

Pasting multi-line text into a terminal runs each line. Chat boxes do not: paste does
not press Enter, so a chat input never auto-submits.

## The September 20 test

On 2026-09-20 a different stock-HID tool read 0 input reports both with LGS running
and with it stopped, so check LGS first. That result was not from this WinUSB driver.
If the monitor here stays empty, confirm WinUSB is bound and that LGS is not holding
the device, then collect a log.

## Collecting a log

From the folder that contains `g13-driver.exe`:

```text
set RUST_LOG=debug
g13-driver.exe --headless
```

Copy the lines around the failure. `G13 not found` and `failed to claim USB interface 0`
are the two messages that distinguish a missing WinUSB binding from a busy interface.
