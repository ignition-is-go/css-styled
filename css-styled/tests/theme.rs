use css_styled::{Theme, StyledComponent, IntoCss, StyledComponentBase, IntoThemeCss, css};

#[derive(Theme, Clone)]
pub struct AppTheme {
    #[var("--primary")]
    pub primary: String,
    #[var("--surface")]
    pub surface: String,
}

impl Default for AppTheme {
    fn default() -> Self {
        Self {
            primary: "#007acc".into(),
            surface: "#1a1a1a".into(),
        }
    }
}

#[derive(StyledComponent, Clone)]
#[component(scope = "card")]
#[component(theme = AppTheme)]
#[component(base_css)]
struct CardStyle {
    #[prop(var = "--card-padding")]
    pub padding: String,
}

impl Default for CardStyle {
    fn default() -> Self {
        Self { padding: "16px".into() }
    }
}

impl StyledComponentBase for CardStyle {
    fn base_css() -> &'static str {
        css!(CardStyle, r#"
            .{SCOPE} {
                background: var(--surface);
                color: var(--primary);
                padding: var(--card-padding);
            }
        "#)
    }
}

#[test]
fn theme_to_css() {
    let theme = AppTheme::default();
    let css = theme.to_theme_css();
    assert!(css.contains(":root {"), "got: {css}");
    assert!(css.contains("--primary: #007acc"), "got: {css}");
    assert!(css.contains("--surface: #1a1a1a"), "got: {css}");
}

#[test]
fn theme_consts() {
    assert_eq!(AppTheme::VAR_PRIMARY, "--primary");
    assert_eq!(AppTheme::VAR_SURFACE, "--surface");
    assert_eq!(AppTheme::ALL_VARS, &["--primary", "--surface"]);
}

#[test]
fn component_references_theme_vars() {
    let style = CardStyle::default();
    let css = style.to_css();
    // Base CSS should contain var references
    assert!(css.contains("var(--surface)"), "got: {css}");
    assert!(css.contains("var(--primary)"), "got: {css}");
    assert!(css.contains("var(--card-padding)"), "got: {css}");
    // Dynamic CSS should set the component's own var
    assert!(css.contains("--card-padding: 16px"), "got: {css}");
}

#[test]
fn theme_vars_forwarded() {
    // CardStyle should have THEME_VARS from AppTheme
    assert!(CardStyle::THEME_VARS.contains(&"--primary"));
    assert!(CardStyle::THEME_VARS.contains(&"--surface"));
}

// --- Tests for `default = theme.field_name` and `default = "literal"` ---

#[derive(StyledComponent, Clone)]
#[component(scope = "panel")]
#[component(theme = AppTheme)]
struct PanelStyle {
    #[prop(css = "background", default = theme.surface)]
    pub background: String,

    #[prop(css = "color", default = theme.primary)]
    pub color: String,

    #[prop(css = "border-radius", default = "4px")]
    pub radius: String,

    #[prop(var = "--panel-gap", default = "8px")]
    pub gap: String,
}

#[test]
fn theme_default_generates_var_ref() {
    let style = PanelStyle::default();
    assert_eq!(style.background, "var(--surface)");
    assert_eq!(style.color, "var(--primary)");
}

#[test]
fn literal_default_uses_value() {
    let style = PanelStyle::default();
    assert_eq!(style.radius, "4px");
    assert_eq!(style.gap, "8px");
}

#[test]
fn theme_default_can_be_overridden() {
    let style = PanelStyle {
        background: "red".into(),
        ..Default::default()
    };
    assert_eq!(style.background, "red");
    assert_eq!(style.color, "var(--primary)");
}
