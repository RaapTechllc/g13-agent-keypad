# WinUSB for the Logitech G13

This is the one-time driver swap that lets g13-driver read the keypad. It is reversible.
Steps that restore the original driver are marked **verify on your machine**.

## Before you start

- You need an Administrator account. Zadig and Device Manager both raise a UAC prompt.
- Plug the G13 into a USB port and leave it there for the swap.
- Download Zadig from https://zadig.akeo.ie/ (no installer). The project page is
  https://github.com/pbatard/libwdi/wiki/Zadig.
- Close Logitech Gaming Software and G HUB before you replace the driver.

## Install WinUSB with Zadig (VID 046D, PID C21C)

1. Run **zadig.exe** as Administrator and accept the UAC prompt.
2. In the menu bar, open **Options → List All Devices**.
3. In the device list, pick the G13 whose USB ID is **046D** **C21C**. If several rows
   appear, pick Interface 0.
4. The driver box on the right must say **WinUSB** (not libusbK or libusb0).
5. Click **Replace Driver** and wait until Zadig reports success.

g13-driver can then open the keypad. Repeat the swap if Windows Update or a replug
puts the original HID driver back.

## Revert to the original driver

**Verify on your machine.** These steps are the usual rollback; confirm them on the PC
you used for the swap.

1. Open Device Manager.
2. Find the G13 (often under Universal Serial Bus devices, named WinUSB or Logitech G13).
3. Right-click → **Uninstall device**.
4. Check **Delete the driver software for this device** (Attempt to remove the driver).
5. Unplug the G13, wait a few seconds, and plug it back in so Windows binds the inbox HID driver.

If the device still comes up as WinUSB, remove the driver package from an elevated
command prompt (the `oemNN.inf` name is the one Device Manager or `pnputil /enum-drivers`
shows for that package):

```text
pnputil /delete-driver <oemNN.inf> /uninstall
```

Then replug the keypad. **Verify on your machine** after a Windows Update as well:
updates sometimes restore WinUSB or the Logitech package, and a replug is part of
checking which driver bound.

## Why LGS and AutoHotkey stop seeing the G13

The swap replaces the HID function driver with WinUSB. Logitech Gaming Software and
G HUB talk to the HID interface, so they no longer see the keypad. Raw Input tools
do not either. An AutoHotkey script that reads usage page `0xFF00` is in that group:
Windows is no longer exposing the G13 as a HID device, so the script gets no reports.
That is the trade for this app reading the endpoint directly. Revert the driver when
you want LGS or that script to see the keypad again.

## Sources

- https://github.com/pbatard/libwdi/wiki/Zadig
- https://zadig.akeo.ie/
- https://support.microsoft.com/en-us/windows/core-isolation-e30ed737-17d8-42f3-a2a9-87521df09b78
- https://support.logi.com/hc/en-us/articles/19426387012119-Unable-to-turn-on-Memory-integrity-on-Windows-10-or-11
- https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput
- https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setforegroundwindow
