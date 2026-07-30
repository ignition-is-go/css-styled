//! Regression: placeholder collisions past ten distinct names.
//!
//! `css!` substitutes each distinct SCREAMING_CASE name via a placeholder, then
//! replaces placeholders with `{}` format slots by plain string search. When
//! placeholders were minted unpadded (`css-s-0` … `css-s-10`), `css-s-1` was a
//! *prefix* of `css-s-10`: `find` returned the same offset for both, the shorter
//! won the strict `pos < earliest` comparison, and it consumed only 7 of the 8
//! characters. The orphaned trailing digit stayed in the output and welded onto
//! the next emitted class, so `.scope-b` came out as `.scope-b0` — silently
//! dropping the rule for every name from the eleventh onward, and again at 21,
//! 31, and so on.
//!
//! It went unnoticed because no `css!` block in this suite, or in any consumer,
//! had previously exceeded ten distinct names — the failure was unreachable in
//! test. This file exists to make it reachable.
//!
//! Names are letters only: `is_uppercase_name` accepts `A-Z` and `_`, so an
//! identifier containing a digit is never treated as a name reference at all.

use css_styled::{css, StyledComponent, StyledComponentBase};

/// Eleven distinct names: SCOPE plus ten classes. Modifiers count too, which is
/// the easy way to cross the threshold without noticing.
#[derive(StyledComponent, Clone)]
#[component(scope = "many")]
#[component(class(
    alfa = "many-alfa",
    bravo = "many-bravo",
    charlie = "many-charlie",
    delta = "many-delta",
    echo = "many-echo",
    foxtrot = "many-foxtrot",
    golf = "many-golf",
    hotel = "many-hotel",
    india = "many-india",
    juliet = "many-juliet"
))]
#[component(base_css)]
struct ManyStyle {}

impl StyledComponentBase for ManyStyle {
    fn base_css() -> &'static str {
        css!(ManyStyle, {
            SCOPE { display: block; }
            ALFA { order: 1; }
            BRAVO { order: 2; }
            CHARLIE { order: 3; }
            DELTA { order: 4; }
            ECHO { order: 5; }
            FOXTROT { order: 6; }
            GOLF { order: 7; }
            HOTEL { order: 8; }
            INDIA { order: 9; }
            JULIET { order: 10; }
        })
    }
}

#[test]
fn names_past_the_tenth_still_resolve() {
    let css = ManyStyle::base_css();
    for expected in [
        ".many {",
        ".many-alfa",
        ".many-bravo",
        ".many-charlie",
        ".many-delta",
        ".many-echo",
        ".many-foxtrot",
        ".many-golf",
        ".many-hotel",
        ".many-india",
        // The eleventh distinct name — the first the collision used to eat.
        ".many-juliet",
    ] {
        assert!(css.contains(expected), "missing `{expected}` in:\n{css}");
    }
}

#[test]
fn no_class_is_emitted_with_an_orphaned_digit() {
    // Guards the *shape* of the failure rather than one victim: a partial
    // replacement always leaves a stray digit fused to another class name.
    // Catches the collision whichever index it lands on.
    let css = ManyStyle::base_css();
    let mut chars = css.chars().peekable();
    let mut current = String::new();
    let mut in_class = false;
    let mut offenders = Vec::new();
    while let Some(c) = chars.next() {
        if c == '.' {
            in_class = true;
            current.clear();
        } else if in_class && (c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            current.push(c);
        } else if in_class {
            if current.ends_with(|c: char| c.is_ascii_digit()) {
                offenders.push(current.clone());
            }
            in_class = false;
            current.clear();
        }
    }
    assert!(
        offenders.is_empty(),
        "classes ending in a digit (partial placeholder replacement): {offenders:?}\n{css}"
    );
}
