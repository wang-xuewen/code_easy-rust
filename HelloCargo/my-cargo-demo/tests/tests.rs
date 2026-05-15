use my_cargo_demo::Data;

#[test]
fn test_integration() {
    let data = Data::new("integration", 999);
    let json = data.to_json();
    assert!(json.contains("integration"));
    assert!(json.contains("999"));
}

#[test]
#[cfg(feature = "regex")]
fn test_regex_integration() {
    assert!(my_cargo_demo::validate_pattern(r"^\d+$", "123"));
    assert!(!my_cargo_demo::validate_pattern(r"^\d+$", "abc"));
}