use css_styled::{StyledComponent, IntoCss, StyledComponentBase, css};

#[derive(StyledComponent, Clone)]
#[component(scope = "split-handle")]
#[component(class(bar = "split-handle-bar"))]
#[component(modifier(horizontal, vertical))]
#[component(base_css)]
pub struct SplitHandleStyle {
    #[prop(css = "width", on = bar)]
    pub thickness: String,

    #[prop(css = "width")]
    pub hover_target_thickness: String,

    #[prop(css = "background", on = bar)]
    pub color: String,

    #[prop(css = "background", on = bar, pseudo = ":hover")]
    pub hover_color: String,
}

impl Default for SplitHandleStyle {
    fn default() -> Self {
        Self {
            thickness: "4px".into(),
            hover_target_thickness: "8px".into(),
            color: "transparent".into(),
            hover_color: "#007acc".into(),
        }
    }
}

impl StyledComponentBase for SplitHandleStyle {
    fn base_css() -> &'static str {
        css!(SplitHandleStyle, {
            SCOPE {
                display: flex;
                align-items: center;
                flex-shrink: 0;
            }
            SCOPE.HORIZONTAL {
                cursor: col-resize;
            }
            SCOPE.HORIZONTAL BAR {
                height: 100%;
            }
            SCOPE.VERTICAL {
                cursor: row-resize;
            }
            SCOPE.VERTICAL BAR {
                width: 100%;
            }
            BAR {
                pointer-events: none;
            }
        })
    }
}

#[test]
fn full_css_output() {
    let style = SplitHandleStyle {
        thickness: "2px".into(),
        hover_target_thickness: "8px".into(),
        color: "#1a1a1a".into(),
        hover_color: "#333".into(),
    };

    let css = style.to_css();

    // Base rules
    assert!(css.contains(".split-handle {"), "missing scope rule in: {css}");
    assert!(css.contains("display: flex"), "missing display in: {css}");
    assert!(css.contains("align-items: center"), "missing align-items in: {css}");
    assert!(css.contains("flex-shrink: 0"), "missing flex-shrink in: {css}");

    assert!(css.contains(".split-handle.horizontal {"), "missing horizontal in: {css}");
    assert!(css.contains("cursor: col-resize"), "missing cursor in: {css}");

    assert!(css.contains(".split-handle.horizontal .split-handle-bar {"), "missing h bar in: {css}");
    assert!(css.contains("height: 100%"), "missing height in: {css}");

    assert!(css.contains(".split-handle.vertical {"), "missing vertical in: {css}");
    assert!(css.contains("cursor: row-resize"), "missing cursor v in: {css}");

    assert!(css.contains(".split-handle-bar {"), "missing bar rule in: {css}");
    assert!(css.contains("pointer-events: none"), "missing pointer-events in: {css}");

    // Dynamic rules
    assert!(css.contains(".split-handle { width: 8px; }"), "missing width in: {css}");
    assert!(css.contains(".split-handle .split-handle-bar {"), "missing bar selector in: {css}");
    assert!(css.contains("width: 2px"), "missing bar width in: {css}");
    assert!(css.contains("background: #1a1a1a"), "missing bar bg in: {css}");
    assert!(css.contains(".split-handle:hover .split-handle-bar { background: #333; }"), "missing hover in: {css}");
}

#[test]
fn class_method_with_modifier() {
    assert_eq!(
        SplitHandleStyle::class(&[SplitHandleModifier::Horizontal]),
        "split-handle horizontal"
    );
    assert_eq!(
        SplitHandleStyle::class(&[SplitHandleModifier::Vertical]),
        "split-handle vertical"
    );
}

#[test]
fn constants() {
    assert_eq!(SplitHandleStyle::SCOPE, "split-handle");
    assert_eq!(SplitHandleStyle::BAR, "split-handle-bar");
}
