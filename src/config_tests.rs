use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::actions::Action;
use crate::config::{Profile, ProfileSet};
use crate::profiles;
use crate::protocol::{G13Key, MKey};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("g13-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("profiles")).unwrap();
    dir
}

fn write_manifest(dir: &Path, m1: &str) {
    std::fs::write(
        dir.join("config.toml"),
        format!("profiles_dir = \"profiles\"\nm1 = \"{m1}\"\n"),
    )
    .unwrap();
}

fn slot_error(dir: &Path, body: &str) -> String {
    std::fs::write(dir.join("profiles/p.toml"), body).unwrap();
    write_manifest(dir, "p.toml");
    let set = ProfileSet::load(&dir.join("config.toml")).expect("load must not panic");
    assert!(set.active_profile().is_none(), "a bad slot is emptied");
    set.slot_errors()
        .first()
        .map(|(_, m)| m.clone())
        .unwrap_or_default()
}

#[test]
fn parses_paste_and_launch_alongside_combos() {
    let cfg: crate::config::RawConfig = toml::from_str(
        r#"
[keys]
G22 = "ctrl+c"

[keys.G1]
type = "paste_text"
file = "prompts/a.md"
method = "type"
restore_clipboard = false

[keys.BTN2]
type = "launch_or_focus"
exe = 'C:\Tools\Cursor.exe'
args = ["--new"]
window_title = "Cursor"
"#,
    )
    .unwrap();
    let profile = Profile::from_raw(cfg).unwrap();
    assert_eq!(profile.get_binding(G13Key::G22), Some("ctrl+c"));
    match profile.action(G13Key::G1).unwrap() {
        Action::PasteText(p) => {
            assert_eq!(p.file, "prompts/a.md");
            assert_eq!(p.method, crate::actions::PasteMethod::Type);
            assert!(!p.restore_clipboard);
        }
        other => panic!("{other:?}"),
    }
    match profile.action(G13Key::Btn2).unwrap() {
        Action::LaunchOrFocus(l) => {
            assert_eq!(l.exe, r"C:\Tools\Cursor.exe");
            assert_eq!(l.args, vec!["--new"]);
            assert_eq!(l.window_title.as_deref(), Some("Cursor"));
            assert_eq!(l.process_name, "Cursor.exe");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn missing_prompt_file_is_slot_error() {
    let dir = scratch("missing-prompt");
    std::fs::write(
        dir.join("profiles/v1.toml"),
        "[meta]\nname = \"V1 build\"\n\n[keys.G1]\ntype = \"paste_text\"\nfile = \"prompts/v1-build/01-spec-pr.md\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("config.toml"),
        "profiles_dir = \"profiles\"\nm1 = \"v1.toml\"\n",
    )
    .unwrap();
    let set = ProfileSet::load(&dir.join("config.toml"))
        .expect("missing file is a slot error, not a panic");
    assert!(set.active_profile().is_none());
    let msg = &set
        .slot_errors()
        .iter()
        .find(|(k, _)| *k == MKey::M1)
        .unwrap()
        .1;
    assert!(msg.contains("v1.toml"), "{msg}");
    assert!(msg.contains("G1"), "{msg}");
    assert!(msg.contains("prompts/v1-build/01-spec-pr.md"), "{msg}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bad_action_config_errors() {
    let cases = [
        (
            "[keys.G1]\ntype = \"nope\"\nfile = \"a.md\"\n",
            "unknown action type",
        ),
        ("[keys.G1]\ntype = \"paste_text\"\n", "missing field"),
        (
            "[keys.G1]\ntype = \"paste_text\"\nfile = \"a.md\"\nmethod = \"ftp\"\n",
            "bad method",
        ),
        (
            "[keys.G1]\ntype = \"paste_text\"\nfile = \"C:/secret.md\"\n",
            "path must stay inside",
        ),
        (
            "[keys.G1]\ntype = \"paste_text\"\nfile = \"../secret.md\"\n",
            "path must stay inside",
        ),
        (
            "[keys.BTN1]\ntype = \"launch_or_focus\"\nexe = '%WINDIR%\\\\notepad.exe'\n",
            "not allowed",
        ),
    ];
    for (body, needle) in cases {
        let dir = scratch("bad-cfg");
        let msg = slot_error(&dir, body);
        assert!(msg.contains(needle), "{body} -> {msg}");
        assert!(msg.contains("p.toml"), "{msg}");
        assert!(msg.contains("G1") || msg.contains("BTN1"), "{msg}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    let dir = scratch("big-prompt");
    std::fs::create_dir_all(dir.join("prompts")).unwrap();
    std::fs::write(dir.join("prompts/big.md"), "x".repeat(65537)).unwrap();
    let msg = slot_error(
        &dir,
        "[keys.G1]\ntype = \"paste_text\"\nfile = \"prompts/big.md\"\n",
    );
    assert!(msg.contains("64 KiB"), "{msg}");
    let _ = std::fs::remove_dir_all(&dir);

    let dir = scratch("utf8");
    std::fs::create_dir_all(dir.join("prompts")).unwrap();
    std::fs::write(dir.join("prompts/bad.md"), [0xff, 0xfe]).unwrap();
    let msg = slot_error(
        &dir,
        "[keys.G1]\ntype = \"paste_text\"\nfile = \"prompts/bad.md\"\n",
    );
    assert!(msg.contains("UTF-8"), "{msg}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn symlinked_prompt_rejected() {
    let dir = scratch("symlink");
    std::fs::create_dir_all(dir.join("prompts")).unwrap();
    let target = dir.join("real.md");
    let link = dir.join("prompts/link.md");
    std::fs::write(&target, "REPLACE_ME\n").unwrap();
    match std::os::windows::fs::symlink_file(&target, &link) {
        Err(err)
            if err.kind() == std::io::ErrorKind::PermissionDenied
                || matches!(err.raw_os_error(), Some(1314 | 5)) =>
        {
            println!("skipped: no symlink privilege");
            let _ = std::fs::remove_dir_all(&dir);
            return;
        }
        Err(err) => panic!("symlink_file failed: {err}"),
        Ok(()) => {}
    }
    let msg = slot_error(
        &dir,
        "[keys.G1]\ntype = \"paste_text\"\nfile = \"prompts/link.md\"\n",
    );
    assert!(
        msg.contains("prompt file must be a regular file inside the config folder"),
        "{msg}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn upstream_profiles_and_catalog_load_identically() {
    let basic = Profile::load(&PathBuf::from("profiles/basic.toml")).unwrap();
    assert_eq!(basic.get_binding(G13Key::G1), Some("ctrl+c"));
    assert!(!basic.has_actions());
    let media = Profile::load(&PathBuf::from("profiles/media.toml")).unwrap();
    assert_eq!(media.get_binding(G13Key::G1), Some("space"));
    assert!(!media.has_actions());
    let gaming = crate::catalog::parse_profile(
        &std::fs::read_to_string("catalog/gaming.toml").unwrap(),
        "gaming.toml",
    )
    .unwrap();
    assert_eq!(gaming.get_binding(G13Key::G1), Some("1"));
    let coding = crate::catalog::parse_profile(
        &std::fs::read_to_string("catalog/coding.toml").unwrap(),
        "coding.toml",
    )
    .unwrap();
    assert_eq!(coding.get_binding(G13Key::G1), Some("ctrl+c"));
    let d3 = crate::catalog::parse_profile(
        &std::fs::read_to_string("catalog/diablo-iii.toml").unwrap(),
        "diablo-iii.toml",
    )
    .unwrap();
    assert_eq!(d3.get_binding(G13Key::G3), Some("q"));
    assert!(!gaming.has_actions() && !coding.has_actions() && !d3.has_actions());
}

#[test]
fn to_toml_save_and_duplicate_preserve_actions() {
    let dir = scratch("roundtrip");
    std::fs::create_dir_all(dir.join("prompts")).unwrap();
    std::fs::write(dir.join("prompts/a.md"), "REPLACE_ME\n").unwrap();
    let body = "\
[meta]\nname = \"Mix\"\n\n[keys]\nG22 = \"a\"\n\n[labels]\nG22 = \"A\"\n\n\
[keys.G1]\ntype = \"paste_text\"\nfile = \"prompts/a.md\"\n\n\
[keys.BTN2]\ntype = \"launch_or_focus\"\nexe = 'C:/Tools/Cursor.exe'\n";
    std::fs::write(dir.join("profiles/mix.toml"), body).unwrap();
    let mut profile = Profile::load(&dir.join("profiles/mix.toml")).unwrap();
    let mut bindings = profile.bindings().clone();
    bindings.insert(G13Key::G1, "ctrl+c".into());
    profile.set_bindings(bindings);
    assert!(profile.action(G13Key::G1).is_some());
    assert!(profile.get_binding(G13Key::G1).is_none());
    let text = profile.to_toml().unwrap();
    assert!(text.contains("[keys.G1]"), "{text}");
    assert!(text.contains("type = 'paste_text'"), "{text}");
    assert!(text.contains("[keys.BTN2]"), "{text}");
    assert!(
        text.contains("G22 = \"a\"") || text.contains("G22 = 'a'"),
        "{text}"
    );
    std::fs::write(dir.join("profiles/mix.toml"), &text).unwrap();

    std::fs::write(
        dir.join("config.toml"),
        "profiles_dir = \"profiles\"\nm1 = \"mix.toml\"\n",
    )
    .unwrap();
    let mut set = ProfileSet::load(&dir.join("config.toml")).unwrap();
    assert!(set.slot_errors().is_empty(), "{:?}", set.slot_errors());
    set.save_active_bindings(
        HashMap::from([(G13Key::G22, "b".into())]),
        HashMap::new(),
        HashMap::from([(G13Key::G22, "Bee".into())]),
        None,
        HashMap::new(),
        HashMap::new(),
    )
    .unwrap();
    let saved = Profile::load(&dir.join("profiles/mix.toml")).unwrap();
    assert!(saved.action(G13Key::G1).is_some());
    assert!(saved.action(G13Key::Btn2).is_some());
    assert_eq!(saved.get_binding(G13Key::G22), Some("b"));
    assert_eq!(saved.label(G13Key::G22), Some("Bee"));

    let copy =
        profiles::duplicate(dir.join("profiles").as_path(), "mix.toml", "Copy of Mix").unwrap();
    let dup = Profile::load(&dir.join("profiles").join(&copy)).unwrap();
    assert_eq!(dup.meta_name(), Some("Copy of Mix"));
    assert!(matches!(dup.action(G13Key::G1), Some(Action::PasteText(_))));
    assert!(matches!(
        dup.action(G13Key::Btn2),
        Some(Action::LaunchOrFocus(_))
    ));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn shipped_templates_load() {
    let mut set = ProfileSet::load(Path::new("config.toml")).unwrap();
    assert!(set.slot_errors().is_empty(), "{:?}", set.slot_errors());
    set.set_active(MKey::M1);
    {
        let m1 = set.active_profile().unwrap();
        assert_eq!(m1.meta_name(), Some("V1 build"));
        for (key, file) in [
            (G13Key::G1, "prompts/v1-build/01-spec-pr.md"),
            (G13Key::G2, "prompts/v1-build/02-grill-me.md"),
            (G13Key::G3, "prompts/v1-build/03-verify.md"),
            (G13Key::G4, "prompts/v1-build/04-ship.md"),
        ] {
            match m1.action(key).unwrap() {
                Action::PasteText(p) => assert_eq!(p.file, file),
                other => panic!("{other:?}"),
            }
        }
    }

    set.set_active(MKey::M2);
    {
        let m2 = set.active_profile().unwrap();
        assert_eq!(m2.meta_name(), Some("Workflow 2"));
        assert_steps(m2, "prompts/workflow-2");
    }
    set.set_active(MKey::M3);
    {
        let m3 = set.active_profile().unwrap();
        assert_eq!(m3.meta_name(), Some("Workflow 3"));
        assert_steps(m3, "prompts/workflow-3");
    }

    for slot in [MKey::M1, MKey::M2, MKey::M3] {
        set.set_active(slot);
        let profile = set.active_profile().unwrap();
        assert_launch(profile, G13Key::Btn1, "REPLACE_ME.exe");
        assert_launch(profile, G13Key::Btn2, "Cursor.exe");
        for action in profile.actions().values() {
            if let Action::PasteText(p) = action {
                let text = std::fs::read_to_string(Path::new(&p.file)).unwrap();
                let first = text.lines().next().unwrap_or("");
                assert!(first.starts_with("REPLACE_ME"), "{}: {first}", p.file);
            }
        }
    }
    let grill = std::fs::read_to_string("prompts/v1-build/02-grill-me.md").unwrap();
    assert!(grill.contains("REPLACE_ME: paste your grill-me prompt here."));
}

fn assert_steps(profile: &Profile, dir: &str) {
    for (key, name) in [
        (G13Key::G1, "01-step.md"),
        (G13Key::G2, "02-step.md"),
        (G13Key::G3, "03-step.md"),
    ] {
        match profile.action(key).unwrap() {
            Action::PasteText(p) => assert_eq!(p.file, format!("{dir}/{name}")),
            other => panic!("{other:?}"),
        }
    }
}

fn assert_launch(profile: &Profile, key: G13Key, process: &str) {
    match profile.action(key).unwrap() {
        Action::LaunchOrFocus(l) => assert_eq!(l.process_name, process),
        other => panic!("{other:?}"),
    }
}
