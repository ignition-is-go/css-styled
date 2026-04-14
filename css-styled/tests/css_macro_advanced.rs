use css_styled::{css, IntoCss, StyledComponent, StyledComponentBase};

#[derive(StyledComponent, Clone)]
#[component(scope = "adv")]
#[component(class(panel = "adv-panel", label = "adv-label"))]
#[component(modifier(active))]
#[component(base_css)]
struct AdvStyle {
    #[prop(var = "--adv-width")]
    pub width: String,
}

impl Default for AdvStyle {
    fn default() -> Self {
        Self {
            width: "100px".into(),
        }
    }
}

impl StyledComponentBase for AdvStyle {
    fn base_css() -> &'static str {
        css!(AdvStyle, r#"
            .{SCOPE} {
                display: flex;
            }
            .{SCOPE}:hover .{PANEL} {
                width: var(--adv-width);
                padding-right: 8px;
            }
            .{LABEL} {
                display: none;
            }
            .{SCOPE}:hover .{LABEL} {
                display: inline;
            }
            .{PANEL} {
                overflow-y: auto;
                overflow-x: hidden;
            }
        "#)
    }
}

#[test]
fn hover_selector() {
    let css = AdvStyle::base_css();
    assert!(css.contains(".adv:hover .adv-panel"), "got: {css}");
    assert!(css.contains("var(--adv-width)"), "got: {css}");
}

#[test]
fn hover_label() {
    let css = AdvStyle::base_css();
    assert!(css.contains(".adv:hover .adv-label"), "got: {css}");
    assert!(css.contains("display: inline"), "got: {css}");
}

#[test]
fn overflow_properties() {
    let css = AdvStyle::base_css();
    assert!(css.contains("overflow-y: auto"), "got: {css}");
    assert!(css.contains("overflow-x: hidden"), "got: {css}");
}

#[derive(StyledComponent, Clone)]
#[component(scope = "tr")]
#[component(class(panel = "tr-panel"))]
#[component(base_css)]
struct TransitionStyle {
    #[prop(var = "--tr-width")]
    pub width: String,
}

impl Default for TransitionStyle {
    fn default() -> Self {
        Self {
            width: "200px".into(),
        }
    }
}

impl StyledComponentBase for TransitionStyle {
    fn base_css() -> &'static str {
        css!(TransitionStyle, r#"
            .{PANEL} {
                transition: width 0.15s ease, padding-right 0.15s ease;
            }
        "#)
    }
}

#[test]
fn transition_duration_and_commas() {
    let css = TransitionStyle::base_css();
    assert!(
        css.contains("transition: width 0.15s ease, padding-right 0.15s ease"),
        "got: {css}"
    );
}
