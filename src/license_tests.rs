#[test]
fn license_notice_and_cargo_field() {
    use sha2::Digest;
    let mut license = std::fs::read("LICENSE").expect("LICENSE");
    // A Windows checkout may expand LF to CRLF. The committed file is LF.
    license.retain(|b| *b != b'\r');
    let mut hasher = sha2::Sha256::new();
    hasher.update(&license);
    let digest = hasher.finalize();
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex,
        "3972dc9744f6499f0f9b2dbf76696f2ae7ad8af9b23dde66d6af86c9dfb36986"
    );
    let notice = std::fs::read_to_string("NOTICE").expect("NOTICE");
    assert!(notice.contains("cavefish-dev/g13-driver"));
    assert!(notice.contains("GPL-3.0-or-later"));
    assert!(notice.contains("Modified by RaapTech LLC"));
    assert!(notice.contains("2026"));
    assert!(notice.contains("5(a)"));
    let cargo = std::fs::read_to_string("Cargo.toml").unwrap();
    assert!(cargo.contains("license = \"GPL-3.0-or-later\""));
}
