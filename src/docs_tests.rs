fn text(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn headings(path: &str, expected: &[&str]) {
    let body = text(path);
    for heading in expected {
        assert!(body.contains(heading), "{path} missing {heading}");
    }
}

#[test]
fn docs_headings_and_terminal_warning() {
    headings(
        "docs/ZADIG.md",
        &[
            "## Before you start",
            "## Install WinUSB with Zadig (VID 046D, PID C21C)",
            "## Revert to the original driver",
            "## Why LGS and AutoHotkey stop seeing the G13",
            "## Sources",
        ],
    );
    headings(
        "docs/TROUBLESHOOTING.md",
        &[
            "## Symptom table",
            "## LGS is holding the device",
            "## Driver conflict or rollback",
            "## Memory Integrity (Core Isolation)",
            "## Keys show in the monitor but nothing is typed",
            "## The September 20 test",
            "## Collecting a log",
        ],
    );
    let config = text("docs/configuration.md");
    let trouble = text("docs/TROUBLESHOOTING.md");
    assert!(config.contains("runs each line"), "configuration.md");
    assert!(trouble.contains("runs each line"), "TROUBLESHOOTING.md");
    assert!(config.contains("## Action bindings"));
    assert!(text("README.md").contains("## Fork notice and attribution"));
    assert!(text("README.md").contains("## Agent workflow keys"));
    assert!(text("docs/zadig-setup.md").contains("ZADIG.md"));
}
