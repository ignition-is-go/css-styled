use css_spec_data;

#[test]
fn has_width_property() {
    let prop = css_spec_data::property("width");
    assert!(prop.is_some(), "width should be a known CSS property");
    let prop = prop.unwrap();
    assert_eq!(prop.name, "width");
    assert!(prop.syntax.contains("length"), "width syntax should mention length");
}

#[test]
fn has_background_property() {
    let prop = css_spec_data::property("background");
    assert!(prop.is_some());
}

#[test]
fn unknown_property_returns_none() {
    assert!(css_spec_data::property("not-a-real-property").is_none());
}

#[test]
fn has_pseudo_classes() {
    assert!(css_spec_data::is_pseudo_class(":hover"));
    assert!(css_spec_data::is_pseudo_class(":focus"));
    assert!(!css_spec_data::is_pseudo_class(":not-real"));
}

#[test]
fn has_pseudo_elements() {
    assert!(css_spec_data::is_pseudo_element("::before"));
    assert!(css_spec_data::is_pseudo_element("::after"));
    assert!(!css_spec_data::is_pseudo_element("::not-real"));
}
