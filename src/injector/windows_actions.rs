//! Clipboard paste, Unicode typing, and launch-or-focus. Windows only.

use std::time::Duration;

use anyhow::{bail, Result};
use windows_sys::Win32::Foundation::{CloseHandle, GlobalFree, HANDLE, HWND, LPARAM};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, CountClipboardFormats, EmptyClipboard, GetClipboardData, OpenClipboard,
    SetClipboardData,
};
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows_sys::Win32::System::Ole::CF_UNICODETEXT;
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    SetForegroundWindow, ShowWindow, SW_RESTORE,
};

use super::windows::WindowsInjector;
use super::{KeyCombo, KeyInjector};
use crate::actions::{
    expand_exe, paste_via_clipboard, pick_window, plan_typed_input, ClipboardOps, LaunchTarget,
    PasteMethod, PasteRequest, Saved, TypeStep, WindowCand,
};

static PASTE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn spawn_paste(req: PasteRequest) {
    std::thread::spawn(move || {
        let _guard = PASTE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = do_paste(&req) {
            log::warn!("paste failed: {e:#}");
        }
    });
}

fn do_paste(req: &PasteRequest) -> Result<()> {
    match req.method {
        PasteMethod::Clipboard => {
            let keys = WinKeys;
            paste_via_clipboard(
                &mut WinClipboard,
                &keys,
                &req.text,
                req.restore_clipboard,
                Duration::from_millis(400),
            )?;
        }
        PasteMethod::Type => send_typed(&plan_typed_input(&req.text))?,
    }
    Ok(())
}

struct WinKeys;

impl KeyInjector for WinKeys {
    fn press(&self, combo: &KeyCombo) -> Result<()> {
        WindowsInjector::new().press(combo)
    }
    fn key_down(&self, key: &str) -> Result<()> {
        WindowsInjector::new().key_down(key)
    }
    fn key_up(&self, key: &str) -> Result<()> {
        WindowsInjector::new().key_up(key)
    }
    fn combo_down(&self, combo: &KeyCombo) -> Result<()> {
        WindowsInjector::new().combo_down(combo)
    }
    fn combo_up(&self, combo: &KeyCombo) -> Result<()> {
        WindowsInjector::new().combo_up(combo)
    }
}

struct WinClipboard;

struct ClipGuard;
impl ClipGuard {
    fn open() -> Result<Self> {
        if unsafe { OpenClipboard(std::ptr::null_mut()) } == 0 {
            bail!("OpenClipboard failed");
        }
        Ok(Self)
    }
}
impl Drop for ClipGuard {
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}

impl ClipboardOps for WinClipboard {
    fn snapshot(&mut self) -> Result<Saved> {
        let _guard = ClipGuard::open()?;
        let handle = unsafe { GetClipboardData(CF_UNICODETEXT as u32) };
        if handle.is_null() {
            return Ok(if unsafe { CountClipboardFormats() } == 0 {
                Saved::Empty
            } else {
                Saved::NonText
            });
        }
        let ptr = unsafe { GlobalLock(handle) } as *const u16;
        if ptr.is_null() {
            return Ok(Saved::Empty);
        }
        let mut units = Vec::new();
        unsafe {
            let mut i = 0;
            while *ptr.add(i) != 0 {
                units.push(*ptr.add(i));
                i += 1;
                if i > 1_000_000 {
                    break;
                }
            }
            GlobalUnlock(handle);
        }
        Ok(Saved::Text(String::from_utf16_lossy(&units)))
    }

    fn set_text(&mut self, text: &str) -> Result<()> {
        let _guard = ClipGuard::open()?;
        unsafe {
            EmptyClipboard();
        }
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes = wide.len() * std::mem::size_of::<u16>();
        let mem = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) };
        if mem.is_null() {
            bail!("GlobalAlloc failed");
        }
        let ptr = unsafe { GlobalLock(mem) } as *mut u16;
        if ptr.is_null() {
            unsafe {
                GlobalFree(mem);
            }
            bail!("GlobalLock failed");
        }
        unsafe {
            std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
            GlobalUnlock(mem);
            if SetClipboardData(CF_UNICODETEXT as u32, mem).is_null() {
                GlobalFree(mem);
                bail!("SetClipboardData failed");
            }
        }
        Ok(())
    }

    fn clear(&mut self) -> Result<()> {
        let _guard = ClipGuard::open()?;
        unsafe {
            EmptyClipboard();
        }
        Ok(())
    }
}

fn unicode_input(scan: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: 0,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn vk_input(vk: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send_typed(steps: &[TypeStep]) -> Result<()> {
    let mut inputs = Vec::new();
    for step in steps {
        match step {
            TypeStep::Unit(unit) => {
                inputs.push(unicode_input(*unit, KEYEVENTF_UNICODE));
                inputs.push(unicode_input(*unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
            }
            TypeStep::ShiftEnter => {
                inputs.push(vk_input(0x10, 0));
                inputs.push(vk_input(0x0D, 0));
                inputs.push(vk_input(0x0D, KEYEVENTF_KEYUP));
                inputs.push(vk_input(0x10, KEYEVENTF_KEYUP));
            }
        }
    }
    if inputs.is_empty() {
        return Ok(());
    }
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    if sent == 0 {
        log::warn!("SendInput returned 0 for typed paste");
    }
    Ok(())
}

struct Listed {
    hwnd: HWND,
    cand: WindowCand,
}

#[allow(clippy::zombie_processes)]
pub fn launch_or_focus(target: &LaunchTarget) -> Result<()> {
    let exe = expand_exe(&target.exe, |name| std::env::var(name).ok())?;
    let own = std::process::id();
    let listed = list_windows();
    let cands: Vec<WindowCand> = listed.iter().map(|w| w.cand.clone()).collect();
    if let Some(picked) = pick_window(
        &cands,
        own,
        &target.process_name,
        target.window_title.as_deref(),
    ) {
        let hwnd = listed
            .iter()
            .find(|w| w.cand == *picked)
            .map(|w| w.hwnd)
            .unwrap_or(std::ptr::null_mut());
        if !hwnd.is_null() {
            unsafe {
                if IsIconic(hwnd) != 0 {
                    ShowWindow(hwnd, SW_RESTORE);
                }
                if SetForegroundWindow(hwnd) == 0 {
                    log::warn!("SetForegroundWindow returned 0");
                }
            }
            return Ok(());
        }
    }
    match std::process::Command::new(&exe)
        .args(&target.args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => {
            drop(child);
            Ok(())
        }
        Err(e) => {
            log::warn!("could not start {exe}: {e}");
            Err(e.into())
        }
    }
}

fn list_windows() -> Vec<Listed> {
    let mut out = Vec::new();
    unsafe {
        EnumWindows(Some(enum_proc), &mut out as *mut Vec<Listed> as LPARAM);
    }
    out
}

unsafe extern "system" fn enum_proc(hwnd: HWND, lp: LPARAM) -> i32 {
    let out = &mut *(lp as *mut Vec<Listed>);
    let mut title = [0u16; 512];
    let n = GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32);
    let title = String::from_utf16_lossy(&title[..n.max(0) as usize]);
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, &mut pid);
    out.push(Listed {
        hwnd,
        cand: WindowCand {
            pid,
            visible: IsWindowVisible(hwnd) != 0,
            process_name: process_file_name(pid),
            title,
        },
    });
    1
}

fn process_file_name(pid: u32) -> String {
    if pid == 0 {
        return String::new();
    }
    unsafe {
        let handle: HANDLE = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return String::new();
        }
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, Default::default(), buf.as_mut_ptr(), &mut len);
        CloseHandle(handle);
        if ok == 0 {
            return String::new();
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        std::path::Path::new(&path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string()
    }
}
