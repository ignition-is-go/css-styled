use css_styled::{StyledComponent, IntoCss, StyledComponentBase, css};

#[derive(StyledComponent, Clone)]
#[component(scope = "widget")]
#[component(class(inner = "widget-inner"))]
#[component(modifier(active))]
#[component(base_css)]
struct WidgetStyle {
    #[prop(css = "color")]
    pub text_color: String,
}

impl Default for WidgetStyle {
    fn default() -> Self {
        Self { text_color: "black".into() }
    }
}

impl StyledComponentBase for WidgetStyle {
    fn base_css() -> &'static str {
        css!(WidgetStyle, r#"
            .{SCOPE} {
                display: flex;
                align-items: center;
            }
            .{SCOPE}.{ACTIVE} .{INNER} {
                font-weight: bold;
            }
            .{INNER} {
                padding: 8px;
            }
        "#)
    }
}

#[test]
fn base_css_contains_scope_rule() {
    let css = WidgetStyle::base_css();
    assert!(css.contains(".widget"), "got: {css}");
    assert!(css.contains("display: flex"), "got: {css}");
    assert!(css.contains("align-items: center"), "got: {css}");
}

#[test]
fn base_css_contains_modifier_rule() {
    let css = WidgetStyle::base_css();
    assert!(css.contains(".widget.active .widget-inner"), "got: {css}");
    assert!(css.contains("font-weight: bold"), "got: {css}");
}

#[test]
fn base_css_contains_child_rule() {
    let css = WidgetStyle::base_css();
    assert!(css.contains(".widget-inner"), "got: {css}");
    assert!(css.contains("padding: 8px"), "got: {css}");
}

#[test]
fn to_css_includes_base_and_dynamic() {
    let style = WidgetStyle::default();
    let css = style.to_css();
    assert!(css.contains(".widget"), "got: {css}");
    assert!(css.contains("display: flex"), "got: {css}");
    assert!(css.contains(".widget { color: black; }"), "got: {css}");
}
