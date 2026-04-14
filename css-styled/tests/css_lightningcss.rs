use css_styled::{StyledComponent, IntoCss, StyledComponentBase, css};

#[derive(StyledComponent, Clone)]
#[component(scope = "lg")]
#[component(class(inner = "lg-inner"))]
#[component(modifier(active))]
#[component(base_css)]
struct LgStyle {
    #[prop(var = "--lg-color")]
    pub color: String,
}

impl Default for LgStyle {
    fn default() -> Self {
        Self {
            color: "blue".into(),
        }
    }
}

impl StyledComponentBase for LgStyle {
    fn base_css() -> &'static str {
        css!(LgStyle, r#"
            .{SCOPE} {
                display: flex;
                color: var(--lg-color);
            }
            .{SCOPE}:hover .{INNER} {
                opacity: 0.8;
            }
            .{SCOPE}.{ACTIVE} {
                font-weight: bold;
            }
            @media (max-width: 768px) {
                .{SCOPE} {
                    flex-direction: column;
                }
            }
            @keyframes {SCOPE}-fade-in {
                from { opacity: 0; }
                to { opacity: 1; }
            }
        "#)
    }
}

#[test]
fn basic_selector() {
    let css = LgStyle::base_css();
    assert!(css.contains(".lg"), "got: {css}");
    assert!(css.contains("display: flex"), "got: {css}");
}

#[test]
fn hover_pseudo() {
    let css = LgStyle::base_css();
    assert!(css.contains(".lg:hover .lg-inner"), "got: {css}");
}

#[test]
fn modifier_compound() {
    let css = LgStyle::base_css();
    assert!(css.contains(".lg.active"), "got: {css}");
}

#[test]
fn media_query() {
    let css = LgStyle::base_css();
    assert!(css.contains("@media"), "got: {css}");
    assert!(css.contains("max-width"), "got: {css}");
    assert!(css.contains("flex-direction: column"), "got: {css}");
}

#[test]
fn keyframes() {
    let css = LgStyle::base_css();
    assert!(css.contains("@keyframes lg-fade-in"), "got: {css}");
    assert!(css.contains("opacity: 0"), "got: {css}");
    assert!(css.contains("opacity: 1"), "got: {css}");
}

#[test]
fn var_references() {
    let css = LgStyle::base_css();
    assert!(css.contains("var(--lg-color)"), "got: {css}");
}
