/// A CSS property specification entry.
#[derive(Debug, Clone)]
pub struct PropertySpec {
    pub name: &'static str,
    pub syntax: &'static str,
    pub initial: &'static str,
    pub inherited: bool,
}

/// Result of validating a CSS value against a property.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    Valid,
    Warn(String),
    Invalid(String),
}

mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}

/// Look up a CSS property by name.
pub fn property(name: &str) -> Option<&'static PropertySpec> {
    generated::PROPERTIES.iter().find(|p| p.name == name)
}

/// Validate a CSS value for a given property.
/// (Stub — full implementation is Task 3)
pub fn validate_value(_property: &str, _value: &str) -> ValidationResult {
    ValidationResult::Valid
}

/// Check if a name is a known CSS pseudo-class (e.g. ":hover").
pub fn is_pseudo_class(name: &str) -> bool {
    let base = name.split('(').next().unwrap_or(name);
    generated::PSEUDO_CLASSES.contains(&base)
}

/// Check if a name is a known CSS pseudo-element (e.g. "::before").
pub fn is_pseudo_element(name: &str) -> bool {
    let base = name.split('(').next().unwrap_or(name);
    generated::PSEUDO_ELEMENTS.contains(&base)
}

/// Return all known CSS property names (for fuzzy matching).
pub fn all_property_names() -> &'static [&'static str] {
    generated::PROPERTY_NAMES
}

/// Return all known CSS pseudo-class names (for fuzzy matching).
pub fn all_pseudo_classes() -> &'static [&'static str] {
    generated::PSEUDO_CLASSES
}

/// Return all known CSS pseudo-element names (for fuzzy matching).
pub fn all_pseudo_elements() -> &'static [&'static str] {
    generated::PSEUDO_ELEMENTS
}
