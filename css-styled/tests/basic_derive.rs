use css_styled::{StyledComponent, IntoCss, StyledComponentBase};

#[derive(StyledComponent, Clone)]
#[component(scope = "test-widget")]
#[component(class(inner = "test-widget-inner"))]
#[component(modifier(active, disabled))]
struct TestWidgetStyle {
    #[prop(css = "width")]
    pub size: String,

    #[prop(css = "background", on = inner)]
    pub bg: String,

    #[prop(css = "color", on = inner, pseudo = ":hover")]
    pub hover_color: String,
}

impl Default for TestWidgetStyle {
    fn default() -> Self {
        Self {
            size: "100px".into(),
            bg: "#fff".into(),
            hover_color: "red".into(),
        }
    }
}

#[test]
fn scope_constant() {
    assert_eq!(TestWidgetStyle::SCOPE, "test-widget");
}

#[test]
fn class_constant() {
    assert_eq!(TestWidgetStyle::INNER, "test-widget-inner");
}

#[test]
fn modifier_enum_exists() {
    let _ = TestWidgetModifier::Active;
    let _ = TestWidgetModifier::Disabled;
}

#[test]
fn class_with_no_modifiers() {
    assert_eq!(TestWidgetStyle::class(&[]), "test-widget");
}

#[test]
fn class_with_one_modifier() {
    assert_eq!(
        TestWidgetStyle::class(&[TestWidgetModifier::Active]),
        "test-widget active"
    );
}

#[test]
fn class_with_multiple_modifiers() {
    let result = TestWidgetStyle::class(&[TestWidgetModifier::Active, TestWidgetModifier::Disabled]);
    assert_eq!(result, "test-widget active disabled");
}

#[test]
fn to_css_output() {
    let style = TestWidgetStyle::default();
    let css = style.to_css();
    assert!(css.contains(".test-widget { width: 100px; }"), "got: {css}");
    assert!(css.contains(".test-widget .test-widget-inner { background: #fff; }"), "got: {css}");
    assert!(css.contains(".test-widget:hover .test-widget-inner { color: red; }"), "got: {css}");
}

#[test]
fn scope_method() {
    let style = TestWidgetStyle::default();
    assert_eq!(style.scope(), "test-widget");
}
