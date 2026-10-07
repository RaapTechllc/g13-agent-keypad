use crate::catalog::parse_profile;

#[test]
fn catalog_rejects_action_profiles() {
    let body = "[keys.G1]\ntype = \"paste_text\"\nfile = \"prompts/a.md\"\n";
    let err = parse_profile(body, "local.toml").unwrap_err().to_string();
    assert!(err.contains("local-only"), "{err}");
    let combo = parse_profile("[keys]\nG1 = \"ctrl+c\"\n", "ok.toml");
    assert!(combo.is_ok());
}
