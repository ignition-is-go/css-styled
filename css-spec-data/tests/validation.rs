use css_spec_data::{validate_value, ValidationResult};

#[test]
fn length_values_valid() {
    assert_eq!(validate_value("width", "10px"), ValidationResult::Valid);
    assert_eq!(validate_value("width", "2em"), ValidationResult::Valid);
    assert_eq!(validate_value("width", "100vh"), ValidationResult::Valid);
    assert_eq!(validate_value("width", "0"), ValidationResult::Valid);
    assert_eq!(validate_value("height", "50%"), ValidationResult::Valid);
}

#[test]
fn color_values_valid() {
    assert_eq!(validate_value("color", "#fff"), ValidationResult::Valid);
    assert_eq!(validate_value("color", "#007acc"), ValidationResult::Valid);
    assert_eq!(validate_value("background", "transparent"), ValidationResult::Valid);
    assert_eq!(validate_value("color", "currentColor"), ValidationResult::Valid);
    assert_eq!(validate_value("color", "red"), ValidationResult::Valid);
    assert_eq!(validate_value("background-color", "rgb(0, 0, 0)"), ValidationResult::Valid);
    assert_eq!(validate_value("color", "hsl(120, 100%, 50%)"), ValidationResult::Valid);
}

#[test]
fn keyword_values_valid() {
    assert_eq!(validate_value("width", "auto"), ValidationResult::Valid);
    assert_eq!(validate_value("display", "flex"), ValidationResult::Valid);
    assert_eq!(validate_value("display", "none"), ValidationResult::Valid);
    assert_eq!(validate_value("pointer-events", "none"), ValidationResult::Valid);
}

#[test]
fn css_functions_pass_through() {
    assert_eq!(validate_value("width", "var(--custom)"), ValidationResult::Valid);
    assert_eq!(validate_value("width", "calc(100% - 20px)"), ValidationResult::Valid);
    assert_eq!(validate_value("width", "env(safe-area-inset-top)"), ValidationResult::Valid);
}

#[test]
fn global_keywords_always_valid() {
    assert_eq!(validate_value("width", "inherit"), ValidationResult::Valid);
    assert_eq!(validate_value("color", "initial"), ValidationResult::Valid);
    assert_eq!(validate_value("display", "unset"), ValidationResult::Valid);
    assert_eq!(validate_value("width", "revert"), ValidationResult::Valid);
}

#[test]
fn wrong_type_warns() {
    match validate_value("width", "#ff0000") {
        ValidationResult::Warn(_) => {}
        other => panic!("expected Warn, got {:?}", other),
    }
}

#[test]
fn shorthand_values_accepted() {
    assert_eq!(validate_value("transition", "background 0.1s ease"), ValidationResult::Valid);
    assert_eq!(validate_value("border", "1px solid #333"), ValidationResult::Valid);
}

#[test]
fn unknown_property_returns_valid() {
    assert_eq!(validate_value("not-a-property", "anything"), ValidationResult::Valid);
}
