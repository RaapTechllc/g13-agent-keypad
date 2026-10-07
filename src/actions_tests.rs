use std::path::Path;
use std::time::Duration;

use anyhow::Result;

use crate::actions::{
    expand_exe, paste_via_clipboard, pick_window, plan_typed_input, prompt_is_inside, Action,
    ClipboardOps, Saved, TypeStep, WindowCand,
};
use crate::injector::{KeyCombo, KeyInjector, Modifier};

fn env(name: &str) -> Option<String> {
    match name {
        "LOCALAPPDATA" => Some(r"C:\Users\example\AppData\Local".into()),
        "APPDATA" => Some(r"C:\Users\example\AppData\Roaming".into()),
        "USERPROFILE" => Some(r"C:\Users\example".into()),
        "ProgramFiles" => Some(r"C:\Program Files".into()),
        "ProgramFiles(x86)" => Some(r"C:\Program Files (x86)".into()),
        _ => None,
    }
}

#[test]
fn launch_target_must_be_exe() {
    let cmd_ext = format!(".{}", ["cm", "d"].concat());
    let bad = [
        ".bat".to_string(),
        cmd_ext,
        ".ps1".to_string(),
        ".com".to_string(),
        ".vbs".to_string(),
        ".js".to_string(),
        ".lnk".to_string(),
        ".url".to_string(),
        String::new(),
    ];
    for ext in bad {
        let raw = format!(r"C:\Tools\app{ext}");
        let err = expand_exe(&raw, env).unwrap_err().to_string();
        assert!(err.contains("exe must be a .exe file"), "{raw}: {err}");
    }
    let ok = expand_exe(r"C:\Tools\Cursor.EXE", env).unwrap();
    assert!(ok.ends_with("Cursor.EXE"));
    let expanded = expand_exe(r"%LOCALAPPDATA%\Programs\cursor\Cursor.exe", env).unwrap();
    assert_eq!(
        expanded,
        r"C:\Users\example\AppData\Local\Programs\cursor\Cursor.exe"
    );
}

#[test]
fn env_allowlist_rejects_other_variables() {
    for raw in [
        "%WINDIR%\\notepad.exe",
        "%TEMP%\\a.exe",
        "%PATH%",
        "%ProgramFiles%\\..\\x.bat",
    ] {
        let err = expand_exe(raw, env).unwrap_err().to_string();
        assert!(
            err.contains("not allowed") || err.contains("exe must be a .exe file"),
            "{raw}: {err}"
        );
    }
    let err = expand_exe("%NOT_A_REAL_VAR%\\notepad.exe", env)
        .unwrap_err()
        .to_string();
    assert!(err.contains("%NOT_A_REAL_VAR%"), "{err}");
    let pf = expand_exe(r"%ProgramFiles(x86)%\cursor\Cursor.exe", env).unwrap();
    assert!(pf.starts_with(r"C:\Program Files (x86)\"));
    let unset = expand_exe("%LOCALAPPDATA%\\Cursor.exe", |_| None)
        .unwrap_err()
        .to_string();
    assert!(unset.contains("not set"), "{unset}");
    let rel = expand_exe(r"cursor\Cursor.exe", env)
        .unwrap_err()
        .to_string();
    assert!(rel.contains("absolute"), "{rel}");
}

#[test]
fn launch_args_stay_literal() {
    let raw: toml::Value = toml::from_str(
        r#"[keys.BTN2]
type = "launch_or_focus"
exe = 'C:\Tools\Cursor.exe'
args = ['%PATH%', '/c', 'do not expand']
"#,
    )
    .unwrap();
    let table = raw.get("keys").unwrap().get("BTN2").unwrap();
    let action = crate::actions::parse_action("BTN2", table).unwrap();
    match action {
        Action::LaunchOrFocus(l) => {
            assert_eq!(l.args, vec!["%PATH%", "/c", "do not expand"]);
            assert_eq!(l.exe, r"C:\Tools\Cursor.exe");
            assert_eq!(l.process_name, "Cursor.exe");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn plan_typed_input_surrogates_and_shift_enter() {
    let steps = plan_typed_input("a\r\n😀");
    assert_eq!(
        steps,
        vec![
            TypeStep::Unit(u16::from(b'a')),
            TypeStep::ShiftEnter,
            TypeStep::Unit(0xD83D),
            TypeStep::Unit(0xDE00),
        ]
    );
    assert!(plan_typed_input("\r").is_empty());
}

struct FakeClip {
    saved: Saved,
    ops: Vec<String>,
}

impl ClipboardOps for FakeClip {
    fn snapshot(&mut self) -> Result<Saved> {
        self.ops.push("snapshot".into());
        Ok(self.saved.clone())
    }
    fn set_text(&mut self, text: &str) -> Result<()> {
        self.ops.push(format!("set:{text}"));
        Ok(())
    }
    fn clear(&mut self) -> Result<()> {
        self.ops.push("clear".into());
        Ok(())
    }
}

struct OrderKeys {
    ops: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl KeyInjector for OrderKeys {
    fn press(&self, combo: &KeyCombo) -> Result<()> {
        let key = combo.key.clone().unwrap_or_default();
        let mods = combo.modifiers.len();
        self.ops.lock().unwrap().push(format!("press:{mods}:{key}"));
        Ok(())
    }
    fn key_down(&self, _: &str) -> Result<()> {
        Ok(())
    }
    fn key_up(&self, _: &str) -> Result<()> {
        Ok(())
    }
    fn combo_down(&self, _: &KeyCombo) -> Result<()> {
        Ok(())
    }
    fn combo_up(&self, _: &KeyCombo) -> Result<()> {
        Ok(())
    }
}

#[test]
fn clipboard_snapshot_restore_order() {
    let ops = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let keys = OrderKeys { ops: ops.clone() };
    let mut clip = FakeClip {
        saved: Saved::Text("canary".into()),
        ops: Vec::new(),
    };
    paste_via_clipboard(&mut clip, &keys, "PROMPT", true, Duration::ZERO).unwrap();
    assert_eq!(
        clip.ops,
        vec![
            "snapshot".to_string(),
            "set:PROMPT".into(),
            "set:canary".into()
        ]
    );
    assert_eq!(ops.lock().unwrap().as_slice(), ["press:1:v"]);
    assert_eq!(
        KeyCombo {
            modifiers: vec![Modifier::Ctrl],
            key: Some("v".into())
        }
        .key
        .as_deref(),
        Some("v")
    );

    let mut empty = FakeClip {
        saved: Saved::Empty,
        ops: Vec::new(),
    };
    paste_via_clipboard(&mut empty, &keys, "PROMPT", true, Duration::ZERO).unwrap();
    assert_eq!(empty.ops.last().map(String::as_str), Some("clear"));

    let mut nont = FakeClip {
        saved: Saved::NonText,
        ops: Vec::new(),
    };
    paste_via_clipboard(&mut nont, &keys, "PROMPT", true, Duration::ZERO).unwrap();
    assert_eq!(nont.ops, vec!["snapshot".to_string(), "set:PROMPT".into()]);
}

#[test]
fn canonical_prompt_path_must_stay_under_config_dir() {
    let root = std::env::temp_dir().join(format!("g13-canon-{}-{}", std::process::id(), line!()));
    let _ = std::fs::remove_dir_all(&root);
    let cfg = root.join("cfg");
    let cfg2 = root.join("cfg2");
    std::fs::create_dir_all(cfg.join("prompts")).unwrap();
    std::fs::create_dir_all(&cfg2).unwrap();
    std::fs::write(cfg.join("prompts/a.md"), "hi").unwrap();
    std::fs::write(cfg2.join("a.md"), "hi").unwrap();
    assert!(prompt_is_inside(&cfg, &cfg.join("prompts/a.md")));
    assert!(!prompt_is_inside(&cfg, &cfg2.join("a.md")));
    let root_c = cfg.canonicalize().unwrap();
    let sib_c = cfg2.join("a.md").canonicalize().unwrap();
    let root_s = root_c.to_string_lossy().to_string();
    let sib_s = sib_c.to_string_lossy().to_string();
    assert!(
        sib_s.starts_with(&root_s),
        "fixture should share a string prefix so a naive check would be wrong: {root_s} vs {sib_s}"
    );
    assert!(!sib_c.starts_with(&root_c));
    assert!(!prompt_is_inside(Path::new(&root_s), Path::new(&sib_s)));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn pick_window_z_order_process_and_title() {
    let cands = vec![
        WindowCand {
            pid: 1,
            visible: false,
            process_name: "Cursor.exe".into(),
            title: "Cursor".into(),
        },
        WindowCand {
            pid: 9,
            visible: true,
            process_name: "Cursor.exe".into(),
            title: "Mine".into(),
        },
        WindowCand {
            pid: 2,
            visible: true,
            process_name: "cursor.exe".into(),
            title: "Notepad".into(),
        },
        WindowCand {
            pid: 3,
            visible: true,
            process_name: "Cursor.exe".into(),
            title: "Cursor Settings".into(),
        },
    ];
    let hit = pick_window(&cands, 9, "Cursor.exe", Some("settings")).unwrap();
    assert_eq!(hit.pid, 3);
    let first = pick_window(&cands, 9, "CURSOR.EXE", None).unwrap();
    assert_eq!(first.pid, 2);
    assert!(pick_window(&cands, 9, "Nope.exe", None).is_none());
}
