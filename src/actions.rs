//! Prompt-file and launch-or-focus actions. Parsing, path rules, and the
//! clipboard/typing plans live here; Win32 calls stay in the injector.

use std::path::{Component, Path};
use std::time::Duration;

use anyhow::{bail, Context, Result};

use crate::injector::{KeyCombo, KeyInjector, Modifier};
use crate::protocol::G13Key;

pub const MAX_PROMPT_BYTES: usize = 64 * 1024;

const EXE_ENV: &[&str] = &[
    "LOCALAPPDATA",
    "APPDATA",
    "USERPROFILE",
    "ProgramFiles",
    "ProgramFiles(x86)",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteMethod {
    Clipboard,
    Type,
}

impl PasteMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Clipboard => "clipboard",
            Self::Type => "type",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasteText {
    pub file: String,
    pub method: PasteMethod,
    pub restore_clipboard: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchOrFocus {
    /// Config text, before environment expansion.
    pub exe: String,
    pub args: Vec<String>,
    pub window_title: Option<String>,
    pub process_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    PasteText(PasteText),
    LaunchOrFocus(LaunchOrFocus),
}

impl Action {
    pub fn summary(&self) -> String {
        match self {
            Self::PasteText(p) => format!("paste_text {}", p.file),
            Self::LaunchOrFocus(l) => format!("launch_or_focus {}", l.process_name),
        }
    }

    pub fn toml_block(&self, key: &str) -> String {
        match self {
            Self::PasteText(p) => format!(
                "\n[keys.{key}]\ntype = 'paste_text'\nfile = {}\nmethod = '{}'\nrestore_clipboard = {}\n",
                toml_lit(&p.file),
                p.method.as_str(),
                p.restore_clipboard
            ),
            Self::LaunchOrFocus(l) => {
                let mut s = format!(
                    "\n[keys.{key}]\ntype = 'launch_or_focus'\nexe = {}\n",
                    toml_lit(&l.exe)
                );
                if !l.args.is_empty() {
                    let args = l.args.iter().map(|a| toml_lit(a)).collect::<Vec<_>>().join(", ");
                    s.push_str(&format!("args = [{args}]\n"));
                }
                if let Some(title) = &l.window_title {
                    s.push_str(&format!("window_title = {}\n", toml_lit(title)));
                }
                s.push_str(&format!("process_name = {}\n", toml_lit(&l.process_name)));
                s
            }
        }
    }
}

/// What the dispatcher hands to `KeyInjector::paste_text`. `text` is never logged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasteRequest {
    pub key_name: String,
    pub file_label: String,
    pub method: PasteMethod,
    pub restore_clipboard: bool,
    pub text: String,
}

/// What the dispatcher hands to `KeyInjector::launch_or_focus`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchTarget {
    pub key_name: String,
    pub exe: String,
    pub args: Vec<String>,
    pub window_title: Option<String>,
    pub process_name: String,
}

impl LaunchTarget {
    pub fn from_action(key_name: String, launch: &LaunchOrFocus) -> Self {
        Self {
            key_name,
            exe: launch.exe.clone(),
            args: launch.args.clone(),
            window_title: launch.window_title.clone(),
            process_name: launch.process_name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Saved {
    Empty,
    Text(String),
    NonText,
}

pub trait ClipboardOps {
    fn snapshot(&mut self) -> Result<Saved>;
    fn set_text(&mut self, text: &str) -> Result<()>;
    fn clear(&mut self) -> Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeStep {
    Unit(u16),
    ShiftEnter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowCand {
    pub pid: u32,
    pub visible: bool,
    pub process_name: String,
    pub title: String,
}

pub fn key_label(key: G13Key) -> String {
    match key {
        G13Key::Btn1 => "BTN1".to_string(),
        G13Key::Btn2 => "BTN2".to_string(),
        G13Key::Stick => "STICK".to_string(),
        other => format!("{other:?}"),
    }
}

pub fn parse_action(key: &str, value: &toml::Value) -> Result<Action> {
    let table = value
        .as_table()
        .with_context(|| format!("key {key} field type: binding must be a string or a table"))?;
    let kind = req_str(table, key, "type")?;
    match kind {
        "paste_text" => Ok(Action::PasteText(parse_paste(key, table)?)),
        "launch_or_focus" => Ok(Action::LaunchOrFocus(parse_launch(key, table)?)),
        other => bail!("key {key} field type: unknown action type {other:?}"),
    }
}

fn parse_paste(key: &str, table: &toml::Table) -> Result<PasteText> {
    let file = req_str(table, key, "file")?.to_string();
    check_prompt_syntax(&file).map_err(|e| anyhow::anyhow!("key {key} field file: {e}"))?;
    let method = match table.get("method") {
        None => PasteMethod::Clipboard,
        Some(toml::Value::String(s)) if s == "clipboard" => PasteMethod::Clipboard,
        Some(toml::Value::String(s)) if s == "type" => PasteMethod::Type,
        _ => bail!("key {key} field method: bad method"),
    };
    let restore_clipboard = match table.get("restore_clipboard") {
        None => true,
        Some(toml::Value::Boolean(b)) => *b,
        _ => bail!("key {key} field restore_clipboard: expected a bool"),
    };
    Ok(PasteText {
        file,
        method,
        restore_clipboard,
    })
}

fn parse_launch(key: &str, table: &toml::Table) -> Result<LaunchOrFocus> {
    let exe = req_str(table, key, "exe")?.to_string();
    let expanded = expand_exe(&exe, |n| std::env::var(n).ok())
        .map_err(|e| anyhow::anyhow!("key {key} field exe: {e}"))?;
    let args = match table.get("args") {
        None => Vec::new(),
        Some(toml::Value::Array(items)) => items
            .iter()
            .map(|v| match v {
                toml::Value::String(s) => Ok(s.clone()),
                _ => bail!("key {key} field args: expected strings"),
            })
            .collect::<Result<Vec<_>>>()?,
        _ => bail!("key {key} field args: expected an array"),
    };
    let window_title = match table.get("window_title") {
        None => None,
        Some(toml::Value::String(s)) => Some(s.clone()),
        _ => bail!("key {key} field window_title: expected a string"),
    };
    let process_name = match table.get("process_name") {
        None => file_name(&expanded),
        Some(toml::Value::String(s)) if !s.is_empty() => s.clone(),
        _ => bail!("key {key} field process_name: expected a string"),
    };
    Ok(LaunchOrFocus {
        exe,
        args,
        window_title,
        process_name,
    })
}

fn req_str<'a>(table: &'a toml::Table, key: &str, field: &str) -> Result<&'a str> {
    match table.get(field) {
        Some(toml::Value::String(s)) if !s.is_empty() => Ok(s),
        Some(toml::Value::String(_)) => bail!("key {key} field {field}: missing field"),
        Some(_) => bail!("key {key} field {field}: expected a string"),
        None => bail!("key {key} field {field}: missing field"),
    }
}

/// Relative prompt path: no `..`, no root, no drive. Does not touch the disk.
pub fn check_prompt_syntax(file: &str) -> Result<()> {
    if file.trim().is_empty() {
        bail!("missing field");
    }
    if looks_rooted(file)
        || Path::new(file).components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        bail!("path must stay inside the config folder");
    }
    Ok(())
}

fn looks_rooted(file: &str) -> bool {
    let b = file.as_bytes();
    if b.first() == Some(&b'/') || b.first() == Some(&b'\\') {
        return true;
    }
    if b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic() {
        return true;
    }
    Path::new(file).is_absolute()
}

/// True when `candidate` canonicalizes to a path inside `config_dir`.
/// Component-wise, so a sibling such as `cfg2` is not inside `cfg`.
pub fn prompt_is_inside(config_dir: &Path, candidate: &Path) -> bool {
    let Ok(root) = config_dir.canonicalize() else {
        return false;
    };
    let Ok(cand) = candidate.canonicalize() else {
        return false;
    };
    cand.starts_with(root)
}

fn is_reparse(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    meta.file_type().is_symlink() || meta.file_attributes() & 0x400 != 0
}

/// Load-time and press-time gate. The returned text is for the injector only.
pub fn read_prompt(config_dir: &Path, file: &str, key: &str, profile: &str) -> Result<String> {
    check_prompt_syntax(file)
        .map_err(|e| anyhow::anyhow!("{profile}: key {key}: field file: {e}"))?;
    let joined = config_dir.join(file);
    let meta = match std::fs::symlink_metadata(&joined) {
        Ok(m) => m,
        Err(_) => bail!("{profile}: key {key}: prompt file \"{file}\" not found"),
    };
    if is_reparse(&meta) || !meta.is_file() || !prompt_is_inside(config_dir, &joined) {
        bail!("{profile}: key {key}: prompt file must be a regular file inside the config folder");
    }
    let bytes = std::fs::read(&joined)
        .with_context(|| format!("{profile}: key {key}: prompt file \"{file}\" not found"))?;
    if bytes.len() > MAX_PROMPT_BYTES {
        bail!("{profile}: key {key}: prompt file \"{file}\" is larger than 64 KiB");
    }
    String::from_utf8(bytes).map_err(|_| {
        anyhow::anyhow!("{profile}: key {key}: prompt file \"{file}\" is not valid UTF-8")
    })
}

/// Expand the allowlisted variables only. `get` is `std::env::var` in production.
pub fn expand_exe(raw: &str, mut get: impl FnMut(&str) -> Option<String>) -> Result<String> {
    let mut out = String::new();
    let mut rest = raw;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('%') else {
            bail!("exe has an unclosed %")
        };
        let name = &after[..end];
        if !EXE_ENV.contains(&name) {
            bail!("exe env var %{name}% is not allowed");
        }
        let value = get(name)
            .filter(|s| !s.is_empty())
            .with_context(|| format!("exe env var %{name}% is not set"))?;
        out.push_str(&value);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    if !Path::new(&out).is_absolute() {
        bail!("exe must be an absolute path");
    }
    let ext = Path::new(&out)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !ext.eq_ignore_ascii_case("exe") {
        bail!("exe must be a .exe file");
    }
    Ok(out)
}

pub fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

/// Snapshot, paste, settle, restore. Non-text clipboard data is left in place.
pub fn paste_via_clipboard(
    clip: &mut dyn ClipboardOps,
    keys: &dyn KeyInjector,
    text: &str,
    restore: bool,
    settle: Duration,
) -> Result<()> {
    let saved = clip.snapshot()?;
    clip.set_text(text)?;
    let combo = KeyCombo {
        modifiers: vec![Modifier::Ctrl],
        key: Some("v".to_string()),
    };
    keys.press(&combo)?;
    std::thread::sleep(settle);
    if !restore {
        return Ok(());
    }
    match saved {
        Saved::Text(prev) => clip.set_text(&prev)?,
        Saved::Empty => clip.clear()?,
        Saved::NonText => {
            log::info!("clipboard held non-text data; left unchanged after paste");
        }
    }
    Ok(())
}

/// UTF-16 code units, including surrogate pairs. `\n` is Shift+Enter. `\r` is dropped.
pub fn plan_typed_input(text: &str) -> Vec<TypeStep> {
    let mut out = Vec::new();
    for ch in text.chars() {
        if ch == '\n' {
            out.push(TypeStep::ShiftEnter);
            continue;
        }
        if ch == '\r' {
            continue;
        }
        let mut buf = [0u16; 2];
        for unit in ch.encode_utf16(&mut buf) {
            out.push(TypeStep::Unit(*unit));
        }
    }
    out
}

/// First visible window in Z-order whose process file name matches, title ANDed, not `own_pid`.
pub fn pick_window<'a>(
    cands: &'a [WindowCand],
    own_pid: u32,
    process_name: &str,
    title: Option<&str>,
) -> Option<&'a WindowCand> {
    cands.iter().find(|c| {
        c.visible
            && c.pid != own_pid
            && c.process_name.eq_ignore_ascii_case(process_name)
            && match title {
                Some(t) => c.title.to_lowercase().contains(&t.to_lowercase()),
                None => true,
            }
    })
}

fn toml_lit(s: &str) -> String {
    if !s.contains('\'') && !s.contains('\n') && !s.contains('\r') {
        format!("'{s}'")
    } else {
        format!(
            "\"{}\"",
            s.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
        )
    }
}
