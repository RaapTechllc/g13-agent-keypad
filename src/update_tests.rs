use crate::update::{next_status, UpdateStatus, UPDATE_CHECK_ENABLED};

#[test]
#[allow(clippy::assertions_on_constants)]
fn update_check_disabled() {
    assert!(!UPDATE_CHECK_ENABLED);
    let json = r#"{
        "tag_name": "v9.9.9",
        "assets": [
            {"name": "g13-driver-v9.9.9-windows-x64.zip", "browser_download_url": "https://example.invalid/a.zip"},
            {"name": "g13-driver-v9.9.9-windows-x64.zip.sha256", "browser_download_url": "https://example.invalid/a.zip.sha256"}
        ]
    }"#;
    for manual in [false, true] {
        let mut calls = 0;
        let mut fetch = || {
            calls += 1;
            Ok(json.to_string())
        };
        let status = next_status(UPDATE_CHECK_ENABLED, &mut fetch, "0.2.1", manual);
        assert_eq!(calls, 0);
        assert!(matches!(status, UpdateStatus::Idle));
        assert!(!matches!(status, UpdateStatus::Available(_)));
    }
    let mut calls = 0;
    let mut fetch = || {
        calls += 1;
        Ok(json.to_string())
    };
    let status = next_status(true, &mut fetch, "0.2.1", false);
    assert_eq!(calls, 1);
    assert!(matches!(status, UpdateStatus::Available(_)));
}
