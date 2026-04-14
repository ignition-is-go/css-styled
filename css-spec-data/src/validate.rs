use crate::ValidationResult;

/// The classified type of a CSS value.
#[derive(Debug, PartialEq)]
enum ValueType {
    Length,
    Percentage,
    Color,
    Number,
    Keyword,
    Function,
    Compound,
}

/// CSS global keywords that are valid for any property.
const GLOBAL_KEYWORDS: &[&str] = &["inherit", "initial", "unset", "revert", "revert-layer"];

/// CSS function prefixes that we pass through without further validation.
const CSS_FUNCTION_PREFIXES: &[&str] = &["var(", "calc(", "env(", "min(", "max(", "clamp("];

/// Absolute length units.
const LENGTH_UNITS: &[&str] = &[
    "px", "cm", "mm", "in", "pt", "pc", "Q", "cap", "ch", "em", "ex", "ic", "lh", "rem", "rlh",
    "dvb", "dvh", "dvi", "dvmax", "dvmin", "dvw", "lvb", "lvh", "lvi", "lvmax", "lvmin", "lvw",
    "svb", "svh", "svi", "svmax", "svmin", "svw", "vb", "vh", "vi", "vmax", "vmin", "vw",
    "cqb", "cqh", "cqi", "cqmax", "cqmin", "cqw",
    "fr", "deg", "grad", "rad", "turn", "s", "ms",
];

/// CSS named colors (all lowercase for case-insensitive comparison).
const NAMED_COLORS: &[&str] = &[
    "aliceblue", "antiquewhite", "aqua", "aquamarine", "azure", "beige", "bisque", "black",
    "blanchedalmond", "blue", "blueviolet", "brown", "burlywood", "cadetblue", "chartreuse",
    "chocolate", "coral", "cornflowerblue", "cornsilk", "crimson", "cyan", "darkblue",
    "darkcyan", "darkgoldenrod", "darkgray", "darkgreen", "darkgrey", "darkkhaki",
    "darkmagenta", "darkolivegreen", "darkorange", "darkorchid", "darkred", "darksalmon",
    "darkseagreen", "darkslateblue", "darkslategray", "darkslategrey", "darkturquoise",
    "darkviolet", "deeppink", "deepskyblue", "dimgray", "dimgrey", "dodgerblue", "firebrick",
    "floralwhite", "forestgreen", "fuchsia", "gainsboro", "ghostwhite", "gold", "goldenrod",
    "gray", "green", "greenyellow", "grey", "honeydew", "hotpink", "indianred", "indigo",
    "ivory", "khaki", "lavender", "lavenderblush", "lawngreen", "lemonchiffon", "lightblue",
    "lightcoral", "lightcyan", "lightgoldenrodyellow", "lightgray", "lightgreen", "lightgrey",
    "lightpink", "lightsalmon", "lightseagreen", "lightskyblue", "lightslategray",
    "lightslategrey", "lightsteelblue", "lightyellow", "lime", "limegreen", "linen", "magenta",
    "maroon", "mediumaquamarine", "mediumblue", "mediumorchid", "mediumpurple", "mediumseagreen",
    "mediumslateblue", "mediumspringgreen", "mediumturquoise", "mediumvioletred", "midnightblue",
    "mintcream", "mistyrose", "moccasin", "navajowhite", "navy", "oldlace", "olive",
    "olivedrab", "orange", "orangered", "orchid", "palegoldenrod", "palegreen", "paleturquoise",
    "palevioletred", "papayawhip", "peachpuff", "peru", "pink", "plum", "powderblue",
    "purple", "rebeccapurple", "red", "rosybrown", "royalblue", "saddlebrown", "salmon",
    "sandybrown", "seagreen", "seashell", "sienna", "silver", "skyblue", "slateblue",
    "slategray", "slategrey", "snow", "springgreen", "steelblue", "tan", "teal", "thistle",
    "tomato", "turquoise", "violet", "wheat", "white", "whitesmoke", "yellow", "yellowgreen",
    "transparent", "currentcolor",
];

/// Returns true if the value contains a space outside of parentheses (compound value).
fn has_top_level_space(value: &str) -> bool {
    let mut depth = 0usize;
    for ch in value.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ' ' if depth == 0 => return true,
            _ => {}
        }
    }
    false
}

/// Returns true if the value looks like a CSS color function.
fn is_color_function(value: &str) -> bool {
    let lower = value.to_lowercase();
    for prefix in &[
        "rgb(", "rgba(", "hsl(", "hsla(", "hwb(", "lab(", "lch(", "oklab(", "oklch(",
        "color(", "color-mix(", "device-cmyk(", "light-dark(",
    ] {
        if lower.starts_with(prefix) {
            return true;
        }
    }
    false
}

/// Returns true if the value is a CSS named color (case-insensitive).
fn is_named_color(value: &str) -> bool {
    let lower = value.to_lowercase();
    NAMED_COLORS.contains(&lower.as_str())
}

/// Returns true if the string is a valid CSS number (integer or decimal, optional leading sign).
fn is_number(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let s = s.strip_prefix('-').or_else(|| s.strip_prefix('+')).unwrap_or(s);
    if s.is_empty() {
        return false;
    }
    let mut has_dot = false;
    for ch in s.chars() {
        if ch == '.' {
            if has_dot {
                return false;
            }
            has_dot = true;
        } else if !ch.is_ascii_digit() {
            return false;
        }
    }
    true
}

/// Classify a CSS value string into a ValueType.
fn classify_value(value: &str) -> ValueType {
    // Compound: spaces outside parentheses
    if has_top_level_space(value) {
        return ValueType::Compound;
    }

    // Function: ends with ')'
    if value.ends_with(')') {
        if is_color_function(value) {
            return ValueType::Color;
        }
        return ValueType::Function;
    }

    // Hex color: starts with '#', rest are hex digits, length 4..=9
    if value.starts_with('#') {
        let rest = &value[1..];
        if !rest.is_empty() && rest.len() <= 8 && rest.chars().all(|c| c.is_ascii_hexdigit()) {
            return ValueType::Color;
        }
    }

    // Named color
    if is_named_color(value) {
        return ValueType::Color;
    }

    // Percentage: ends with '%'
    if let Some(num) = value.strip_suffix('%') {
        if is_number(num) {
            return ValueType::Percentage;
        }
    }

    // Bare zero is a valid length
    if value == "0" {
        return ValueType::Length;
    }

    // Length: ends with a known unit, prefix is a valid number
    for unit in LENGTH_UNITS {
        if let Some(num) = value.strip_suffix(unit) {
            if is_number(num) {
                return ValueType::Length;
            }
        }
    }

    // Number: plain numeric value
    if is_number(value) {
        return ValueType::Number;
    }

    // Default: keyword
    ValueType::Keyword
}

/// Returns true if the syntax string suggests the property accepts length or length-percentage values.
fn syntax_accepts_length(syntax: &str) -> bool {
    syntax.contains("<length") || syntax.contains("<line-width") || syntax.contains("<size")
}

/// Returns true if the syntax string suggests the property accepts percentage values.
fn syntax_accepts_percentage(syntax: &str) -> bool {
    syntax.contains("<percentage") || syntax.contains("<length-percentage")
}

/// Returns true if the syntax string suggests the property accepts color values.
fn syntax_accepts_color(syntax: &str) -> bool {
    syntax.contains("<color")
}

/// Returns true if the syntax string is exclusively color (no length tokens).
fn syntax_is_color_only(syntax: &str) -> bool {
    syntax_accepts_color(syntax) && !syntax_accepts_length(syntax)
}

/// Returns true if the syntax string is exclusively length/percentage (no color tokens).
fn syntax_is_length_only(syntax: &str) -> bool {
    syntax_accepts_length(syntax) && !syntax_accepts_color(syntax)
}

/// Validate a CSS value against a property name.
pub fn validate_value(property: &str, value: &str) -> ValidationResult {
    let trimmed = value.trim();

    // 1. Global keywords are always valid.
    if GLOBAL_KEYWORDS.contains(&trimmed) {
        return ValidationResult::Valid;
    }

    // 2. CSS pass-through functions are always valid.
    for prefix in CSS_FUNCTION_PREFIXES {
        if trimmed.starts_with(prefix) {
            return ValidationResult::Valid;
        }
    }

    // 3. Unknown properties: can't validate, return Valid.
    let spec = match crate::property(property) {
        Some(s) => s,
        None => return ValidationResult::Valid,
    };

    let syntax = spec.syntax;

    // Empty syntax means no data available.
    if syntax.is_empty() {
        return ValidationResult::Valid;
    }

    // 4. Classify the value.
    let vtype = classify_value(trimmed);

    // 5. Check for mismatches we can detect.
    match vtype {
        ValueType::Color => {
            // Color on a length-only property → Warn
            if syntax_is_length_only(syntax) {
                return ValidationResult::Warn(format!(
                    "color value '{}' is not expected for property '{}' (syntax: {})",
                    trimmed, property, syntax
                ));
            }
        }
        ValueType::Length => {
            // Length on a color-only property → Warn
            if syntax_is_color_only(syntax) {
                return ValidationResult::Warn(format!(
                    "length value '{}' is not expected for property '{}' (syntax: {})",
                    trimmed, property, syntax
                ));
            }
        }
        ValueType::Percentage => {
            // Percentage on a color-only property → Warn
            if syntax_is_color_only(syntax) && !syntax_accepts_percentage(syntax) {
                return ValidationResult::Warn(format!(
                    "percentage value '{}' is not expected for property '{}' (syntax: {})",
                    trimmed, property, syntax
                ));
            }
        }
        // Keywords, Numbers, Functions, Compound: too hard to validate precisely
        _ => {}
    }

    ValidationResult::Valid
}
