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

/// Look up a CSS property by name.
pub fn property(_name: &str) -> Option<&'static PropertySpec> {
    todo!()
}

/// Validate a CSS value for a given property.
pub fn validate_value(_property: &str, _value: &str) -> ValidationResult {
    todo!()
}

/// Check if a name is a known CSS pseudo-class (e.g. ":hover").
pub fn is_pseudo_class(_name: &str) -> bool {
    todo!()
}

/// Check if a name is a known CSS pseudo-element (e.g. "::before").
pub fn is_pseudo_element(_name: &str) -> bool {
    todo!()
}

/// Return all known CSS property names (for fuzzy matching).
pub fn all_property_names() -> &'static [&'static str] {
    todo!()
}

/// Return all known CSS pseudo-class names (for fuzzy matching).
pub fn all_pseudo_classes() -> &'static [&'static str] {
    todo!()
}

/// Return all known CSS pseudo-element names (for fuzzy matching).
pub fn all_pseudo_elements() -> &'static [&'static str] {
    todo!()
}
