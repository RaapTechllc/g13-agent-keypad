use std::path::Path;

use crate::simulate::{parse_cli, run_check_to, run_simulate_to, CliMode};

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn config_flag_requires_dry_run_mode() {
    let err = "--config is only valid with --check-config or --simulate";
    assert_eq!(parse_cli(&args(&["--config", "c.toml"])).unwrap_err(), err);
    assert_eq!(
        parse_cli(&args(&["--config", "c.toml", "--headless"])).unwrap_err(),
        err
    );
    assert_eq!(
        parse_cli(&args(&["--headless", "--config", "c.toml"])).unwrap_err(),
        err
    );
    assert_eq!(
        parse_cli(&args(&["--minimized", "--config", "c.toml"])).unwrap_err(),
        err
    );
    assert_eq!(
        parse_cli(&args(&["--updated", "--config", "c.toml"])).unwrap_err(),
        err
    );
    assert_eq!(parse_cli(&args(&["--config=c.toml"])).unwrap_err(), err);

    assert!(matches!(
        parse_cli(&args(&["--check-config", "--config", "c.toml"])).unwrap(),
        CliMode::CheckConfig { config: Some(_) }
    ));
    assert!(matches!(
        parse_cli(&args(&["--simulate", "M1,G1", "--config", "c.toml"])).unwrap(),
        CliMode::Simulate {
            config: Some(_),
            ..
        }
    ));
    assert!(matches!(
        parse_cli(&[]).unwrap(),
        CliMode::Live {
            headless: false,
            minimized: false,
            updated: false
        }
    ));
    assert!(matches!(
        parse_cli(&args(&["--headless"])).unwrap(),
        CliMode::Live {
            headless: true,
            minimized: false,
            updated: false
        }
    ));
}

#[test]
fn simulate_and_check_config_output() {
    let mut check = Vec::new();
    let code = run_check_to(Path::new("config.toml"), &mut check);
    let text = String::from_utf8(check).unwrap();
    assert_eq!(code, 0, "{text}");
    assert!(text.lines().any(|l| l.starts_with("[check] config: ")));
    assert!(text.contains("[check] M1 v1-build.toml: ok"), "{text}");
    assert!(text.contains("[check] ok: 3 profiles, 0 errors"), "{text}");

    let mut sim = Vec::new();
    let spec = "M1,G1,G2,G3,G4,BTN1,BTN2,M2,G1";
    let code = run_simulate_to(Path::new("config.toml"), spec, &mut sim);
    let text = String::from_utf8(sim).unwrap();
    assert_eq!(code, 0, "{text}");
    assert!(
        text.contains("[dry-run] M1 -> profile \"V1 build\""),
        "{text}"
    );
    assert!(text.contains(
        "[dry-run] G1 -> paste_text file=prompts/v1-build/01-spec-pr.md method=clipboard restore_clipboard=true chars="
    ), "{text}");
    assert!(
        text.contains("[dry-run] G2 -> paste_text file=prompts/v1-build/02-grill-me.md "),
        "{text}"
    );
    assert!(
        text.contains("[dry-run] BTN2 -> launch_or_focus process=Cursor.exe "),
        "{text}"
    );
    assert!(
        text.contains("[dry-run] G1 -> paste_text file=prompts/workflow-2/01-step.md "),
        "{text}"
    );
    assert!(
        text.contains("[dry-run] done: 9 events, 0 errors"),
        "{text}"
    );
    assert!(!text.to_lowercase().contains("paste your"), "{text}");
    assert!(!text.contains("REPLACE_ME:"), "{text}");

    let mut bad = Vec::new();
    assert_eq!(
        run_simulate_to(Path::new("config.toml"), "M1,NOPE", &mut bad),
        3
    );

    let dir = std::env::temp_dir().join(format!("g13-sim-fail-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("profiles")).unwrap();
    std::fs::write(
        dir.join("profiles/p.toml"),
        "[keys.G1]\ntype = \"paste_text\"\nfile = \"missing.md\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("config.toml"),
        "profiles_dir = \"profiles\"\nm1 = \"p.toml\"\n",
    )
    .unwrap();
    let mut failed = Vec::new();
    assert_eq!(run_check_to(&dir.join("config.toml"), &mut failed), 2);
    let failed = String::from_utf8(failed).unwrap();
    assert!(failed.contains("[check] FAILED: 1 error(s)"), "{failed}");
    let mut sim_fail = Vec::new();
    assert_eq!(
        run_simulate_to(&dir.join("config.toml"), "G1", &mut sim_fail),
        2
    );
    let _ = std::fs::remove_dir_all(&dir);
}
