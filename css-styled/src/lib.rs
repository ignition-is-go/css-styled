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
