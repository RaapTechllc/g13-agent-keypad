//! `--check-config` and `--simulate`. Decides the mode before USB, the tray, or the GUI.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::actions::key_label;
use crate::config::{parse_g13_key, ProfileSet};
use crate::dispatcher::Dispatcher;
use crate::injector::dry_run::DryRunInjector;
use crate::protocol::{G13Event, G13Key, MKey};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliMode {
    /// No config field: the live path resolves `config.toml` on its own.
    Live {
        headless: bool,
        minimized: bool,
        updated: bool,
    },
    CheckConfig {
        config: Option<PathBuf>,
    },
    Simulate {
        config: Option<PathBuf>,
        spec: String,
    },
}

const CONFIG_FLAG_ERROR: &str = "--config is only valid with --check-config or --simulate";

pub fn parse_cli(args: &[String]) -> Result<CliMode, String> {
    let mut headless = false;
    let mut minimized = false;
    let mut updated = false;
    let mut check = false;
    let mut simulate: Option<String> = None;
    let mut config: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        if let Some(rest) = arg.strip_prefix("--config=") {
            config = Some(PathBuf::from(rest));
        } else if let Some(rest) = arg.strip_prefix("--simulate=") {
            simulate = Some(rest.to_string());
        } else {
            match arg {
                "--headless" => headless = true,
                "--minimized" => minimized = true,
                "--updated" => updated = true,
                "--check-config" => check = true,
                "--simulate" => {
                    i += 1;
                    let spec = args
                        .get(i)
                        .ok_or_else(|| "--simulate needs a value".to_string())?;
                    simulate = Some(spec.clone());
                }
                "--config" => {
                    i += 1;
                    let path = args
                        .get(i)
                        .ok_or_else(|| "--config needs a path".to_string())?;
                    config = Some(PathBuf::from(path));
                }
                other => return Err(format!("unknown argument {other}")),
            }
        }
        i += 1;
    }
    if check && simulate.is_some() {
        return Err("--check-config and --simulate cannot be combined".into());
    }
    if config.is_some() && !check && simulate.is_none() {
        return Err(CONFIG_FLAG_ERROR.into());
    }
    if check {
        return Ok(CliMode::CheckConfig { config });
    }
    if let Some(spec) = simulate {
        return Ok(CliMode::Simulate { config, spec });
    }
    Ok(CliMode::Live {
        headless,
        minimized,
        updated,
    })
}

pub struct Stdout;

impl Write for Stdout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut out = io::stdout();
        let _ = out.write_all(buf);
        let _ = out.flush();
        #[cfg(windows)]
        if let Ok(mut con) = std::fs::OpenOptions::new().write(true).open("CONOUT$") {
            let _ = con.write_all(buf);
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        io::stdout().flush()
    }
}

pub fn run_check(path: &Path) -> i32 {
    run_check_to(path, &mut Stdout)
}

pub fn run_check_to(path: &Path, out: &mut dyn Write) -> i32 {
    let _ = writeln!(out, "[check] config: {}", path.display());
    let set = match ProfileSet::load(path) {
        Ok(set) => set,
        Err(e) => {
            let _ = writeln!(out, "[check] FAILED: 1 error(s)");
            let _ = writeln!(out, "{e:#}");
            return 2;
        }
    };
    let mut profiles = 0;
    let mut errors = 0;
    for slot in [MKey::M1, MKey::M2, MKey::M3] {
        let Some(file) = set.name(slot) else { continue };
        profiles += 1;
        match set.slot_errors().iter().find(|(k, _)| *k == slot) {
            Some((_, msg)) => {
                errors += 1;
                let _ = writeln!(out, "[check] {slot:?} {file}: ERROR {msg}");
            }
            None => {
                let _ = writeln!(out, "[check] {slot:?} {file}: ok");
            }
        }
    }
    if errors == 0 {
        let _ = writeln!(out, "[check] ok: {profiles} profiles, 0 errors");
        0
    } else {
        let _ = writeln!(out, "[check] FAILED: {errors} error(s)");
        2
    }
}

enum Tok {
    M(MKey),
    Key(G13Key),
}

fn parse_token(raw: &str) -> Result<(String, Tok), ()> {
    if let Some(key) = parse_g13_key(raw) {
        return Ok((key_label(key), Tok::Key(key)));
    }
    match raw.to_ascii_uppercase().as_str() {
        "M1" => Ok(("M1".into(), Tok::M(MKey::M1))),
        "M2" => Ok(("M2".into(), Tok::M(MKey::M2))),
        "M3" => Ok(("M3".into(), Tok::M(MKey::M3))),
        _ => Err(()),
    }
}

pub fn run_simulate(path: &Path, spec: &str) -> i32 {
    run_simulate_to(path, spec, &mut Stdout)
}

pub fn run_simulate_to(path: &Path, spec: &str, out: &mut dyn Write) -> i32 {
    let set = match ProfileSet::load(path) {
        Ok(set) => set,
        Err(e) => {
            let _ = writeln!(out, "[check] FAILED: 1 error(s)");
            let _ = writeln!(out, "{e:#}");
            return 2;
        }
    };
    if !set.slot_errors().is_empty() {
        let n = set.slot_errors().len();
        let _ = writeln!(out, "[check] FAILED: {n} error(s)");
        return 2;
    }
    let tokens: Vec<&str> = spec
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let mut parsed = Vec::with_capacity(tokens.len());
    for token in &tokens {
        match parse_token(token) {
            Ok(item) => parsed.push(item),
            Err(()) => {
                let _ = writeln!(out, "unknown token {token}");
                return 3;
            }
        }
    }
    let profiles = Arc::new(RwLock::new(set));
    let (injector, lines) = DryRunInjector::new();
    let mut dispatcher = Dispatcher::new(profiles.clone(), Box::new(injector));
    for (label, tok) in &parsed {
        match tok {
            Tok::M(m) => {
                let _ = dispatcher.handle(G13Event::MKeyDown(*m));
                let name = {
                    let set = profiles.read().unwrap();
                    set.active_profile()
                        .and_then(|p| p.meta_name())
                        .unwrap_or("(none)")
                        .to_string()
                };
                lines
                    .lock()
                    .unwrap()
                    .push(format!("[dry-run] {label} -> profile \"{name}\""));
            }
            Tok::Key(key) => {
                let _ = dispatcher.handle(G13Event::KeyDown(*key));
                let _ = dispatcher.handle(G13Event::KeyUp(*key));
            }
        }
    }
    let lines = lines.lock().unwrap();
    for line in lines.iter() {
        let _ = writeln!(out, "{line}");
    }
    let _ = writeln!(out, "[dry-run] done: {} events, 0 errors", tokens.len());
    0
}
