pub use css_styled_derive::{StyledComponent, Theme, css};
pub use css_spec_data;

/// Trait for types that can produce scoped CSS.
pub trait IntoCss {
    /// Returns the full scoped CSS string (base + dynamic).
    fn to_css(&self) -> String;

    /// Returns the scope class name.
    fn scope(&self) -> &'static str;
}

/// Trait for theme types that produce `:root { ... }` CSS custom properties.
pub trait IntoThemeCss {
    fn to_theme_css(&self) -> String;
}

/// Trait for styled components that also have static/structural CSS.
pub trait StyledComponentBase: IntoCss {
    /// Returns the static CSS for this component.
    fn base_css() -> &'static str {
        ""
    }
}

/// Const-evaluable helper to check if a string slice is in a static array.
/// Used by the `css!` macro for compile-time var() reference validation.
pub const fn const_contains(haystack: &[&str], needle: &str) -> bool {
    let mut i = 0;
    while i < haystack.len() {
        if const_str_eq(haystack[i], needle) {
            return true;
        }
        i += 1;
    }
    false
}

const fn const_str_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}
