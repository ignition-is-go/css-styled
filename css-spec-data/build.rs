use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

// ── Serde structs for @webref/css JSON format ─────────────────────────────────

#[derive(Debug, Deserialize)]
struct SpecFile {
    #[serde(default)]
    properties: Vec<PropertyEntry>,
    #[serde(default)]
    selectors: Vec<SelectorEntry>,
}

#[derive(Debug, Deserialize)]
struct PropertyEntry {
    name: String,
    #[serde(default)]
    value: String,
    #[serde(rename = "newValues", default)]
    new_values: String,
    #[serde(default)]
    initial: InitialValue,
    #[serde(default)]
    inherited: InheritedValue,
}

#[derive(Debug, Deserialize, Default)]
#[serde(untagged)]
enum InitialValue {
    #[default]
    Missing,
    Str(String),
    Arr(Vec<String>),
}

impl InitialValue {
    fn as_str(&self) -> &str {
        match self {
            InitialValue::Str(s) => s.as_str(),
            InitialValue::Arr(v) => v.first().map(|s| s.as_str()).unwrap_or(""),
            InitialValue::Missing => "",
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(untagged)]
enum InheritedValue {
    #[default]
    Missing,
    Bool(bool),
    Str(String),
}

impl InheritedValue {
    fn as_bool(&self) -> bool {
        match self {
            InheritedValue::Bool(b) => *b,
            InheritedValue::Str(s) => s.eq_ignore_ascii_case("yes"),
            InheritedValue::Missing => false,
        }
    }
}

#[derive(Debug, Deserialize)]
struct SelectorEntry {
    name: String,
    #[serde(rename = "type", default)]
    kind: String,
}

// ── Spec file list ────────────────────────────────────────────────────────────

const SPEC_FILES: &[&str] = &[
    "CSS",
    "css-align",
    "css-animations",
    "css-backgrounds",
    "css-borders",
    "css-box",
    "css-break",
    "css-cascade",
    "css-color",
    "css-contain",
    "css-content",
    "css-display",
    "css-flexbox",
    "css-fonts",
    "css-grid",
    "css-images",
    "css-inline",
    "css-lists",
    "css-logical",
    "css-masking",
    "css-multicol",
    "css-overflow",
    "css-overscroll",
    "css-page",
    "css-position",
    "css-shapes",
    "css-sizing",
    "css-speech",
    "css-tables",
    "css-text",
    "css-text-decor",
    "css-transforms",
    "css-transitions",
    "css-ui",
    "css-values",
    "css-variables",
    "css-view-transitions",
    "css-will-change",
    "css-writing-modes",
    "filter-effects",
    "compositing",
    "motion",
    "scroll-animations",
    // selectors spec for pseudo-classes/elements
    "selectors",
];

const BASE_URL: &str =
    "https://raw.githubusercontent.com/w3c/webref/curated/ed/css";

// ── Cache helpers ─────────────────────────────────────────────────────────────

fn data_dir() -> PathBuf {
    // data/ lives alongside build.rs (CARGO_MANIFEST_DIR)
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    Path::new(&manifest).join("data")
}

fn fetch_or_cache(spec: &str) -> Option<String> {
    let cache_path = data_dir().join(format!("{spec}.json"));

    // Try cache first
    if cache_path.exists() {
        if let Ok(text) = fs::read_to_string(&cache_path) {
            if !text.trim().is_empty() {
                return Some(text);
            }
        }
    }

    // Fetch from web
    let url = format!("{BASE_URL}/{spec}.json");
    eprintln!("cargo:warning=Fetching {url}");
    let resp = match ureq::get(&url).call() {
        Ok(r) => r,
        Err(ureq::Error::Status(404, _)) => {
            eprintln!("cargo:warning=  404 for {spec}, skipping");
            return None;
        }
        Err(e) => {
            eprintln!("cargo:warning=  Error fetching {spec}: {e}");
            return None;
        }
    };

    let text = resp.into_string().unwrap_or_default();
    if text.trim().is_empty() {
        return None;
    }

    // Write cache (best-effort)
    let _ = fs::create_dir_all(cache_path.parent().unwrap());
    if let Ok(mut f) = fs::File::create(&cache_path) {
        let _ = f.write_all(text.as_bytes());
    }

    Some(text)
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    // Re-run only when data cache changes or build.rs itself changes
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=data/");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("generated.rs");

    // Collected properties (name → PropertyEntry, keeping first definition wins)
    let mut properties: BTreeMap<String, (String, String, bool)> = BTreeMap::new();
    // pseudo-classes and pseudo-elements from selectors spec
    let mut pseudo_classes: Vec<String> = Vec::new();
    let mut pseudo_elements: Vec<String> = Vec::new();

    for spec in SPEC_FILES {
        let text = match fetch_or_cache(spec) {
            Some(t) => t,
            None => continue,
        };

        let parsed: SpecFile = match serde_json::from_str(&text) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("cargo:warning=  Parse error in {spec}: {e}");
                continue;
            }
        };

        // Collect properties
        for prop in parsed.properties {
            // Skip custom properties and grammar references
            if prop.name.starts_with("--") || prop.name.starts_with('<') {
                continue;
            }
            let syntax = if !prop.value.is_empty() {
                prop.value.clone()
            } else {
                prop.new_values.clone()
            };
            let initial = prop.initial.as_str().to_string();
            let inherited = prop.inherited.as_bool();

            properties
                .entry(prop.name.clone())
                .or_insert((syntax, initial, inherited));
        }

        // Collect selectors
        for sel in parsed.selectors {
            let name = sel.name.trim().to_string();
            if name.is_empty() {
                continue;
            }
            // Strip parenthesized arguments for matching: ":not()" → ":not"
            let base = strip_parens(&name);
            match sel.kind.as_str() {
                "pseudo-class" => {
                    let key = if base.starts_with(':') {
                        base.to_string()
                    } else {
                        format!(":{base}")
                    };
                    if !pseudo_classes.contains(&key) {
                        pseudo_classes.push(key);
                    }
                }
                "pseudo-element" => {
                    let key = if base.starts_with("::") {
                        base.to_string()
                    } else if base.starts_with(':') {
                        format!(":{base}")
                    } else {
                        format!("::{base}")
                    };
                    if !pseudo_elements.contains(&key) {
                        pseudo_elements.push(key);
                    }
                }
                _ => {}
            }
        }
    }

    // ── Fallback pseudo-classes / elements ────────────────────────────────────
    let fallback_pseudo_classes = [
        ":active",
        ":any-link",
        ":checked",
        ":default",
        ":defined",
        ":disabled",
        ":empty",
        ":enabled",
        ":first-child",
        ":first-of-type",
        ":focus",
        ":focus-visible",
        ":focus-within",
        ":fullscreen",
        ":has",
        ":hover",
        ":in-range",
        ":indeterminate",
        ":invalid",
        ":is",
        ":lang",
        ":last-child",
        ":last-of-type",
        ":link",
        ":local-link",
        ":not",
        ":nth-child",
        ":nth-col",
        ":nth-last-child",
        ":nth-last-col",
        ":nth-last-of-type",
        ":nth-of-type",
        ":only-child",
        ":only-of-type",
        ":optional",
        ":out-of-range",
        ":placeholder-shown",
        ":read-only",
        ":read-write",
        ":required",
        ":root",
        ":scope",
        ":target",
        ":valid",
        ":visited",
        ":where",
    ];
    for &pc in &fallback_pseudo_classes {
        if !pseudo_classes.contains(&pc.to_string()) {
            pseudo_classes.push(pc.to_string());
        }
    }

    let fallback_pseudo_elements = [
        "::after",
        "::backdrop",
        "::before",
        "::cue",
        "::cue-region",
        "::first-letter",
        "::first-line",
        "::grammar-error",
        "::marker",
        "::part",
        "::placeholder",
        "::selection",
        "::slotted",
        "::spelling-error",
        "::target-text",
        "::view-transition",
        "::view-transition-group",
        "::view-transition-image-pair",
        "::view-transition-new",
        "::view-transition-old",
    ];
    for &pe in &fallback_pseudo_elements {
        if !pseudo_elements.contains(&pe.to_string()) {
            pseudo_elements.push(pe.to_string());
        }
    }

    pseudo_classes.sort();
    pseudo_elements.sort();

    // ── Write generated.rs ────────────────────────────────────────────────────
    let mut out = String::new();
    out.push_str("use crate::PropertySpec;\n\n");

    // PROPERTIES array
    out.push_str("pub static PROPERTIES: &[PropertySpec] = &[\n");
    for (name, (syntax, initial, inherited)) in &properties {
        let syntax_escaped = escape_str(syntax);
        let initial_escaped = escape_str(initial);
        out.push_str(&format!(
            "    PropertySpec {{ name: {name:?}, syntax: {syntax_escaped:?}, initial: {initial_escaped:?}, inherited: {inherited} }},\n"
        ));
    }
    out.push_str("];\n\n");

    // PROPERTY_NAMES array
    out.push_str("pub static PROPERTY_NAMES: &[&str] = &[\n");
    for name in properties.keys() {
        out.push_str(&format!("    {name:?},\n"));
    }
    out.push_str("];\n\n");

    // PSEUDO_CLASSES array
    out.push_str("pub static PSEUDO_CLASSES: &[&str] = &[\n");
    for pc in &pseudo_classes {
        out.push_str(&format!("    {pc:?},\n"));
    }
    out.push_str("];\n\n");

    // PSEUDO_ELEMENTS array
    out.push_str("pub static PSEUDO_ELEMENTS: &[&str] = &[\n");
    for pe in &pseudo_elements {
        out.push_str(&format!("    {pe:?},\n"));
    }
    out.push_str("];\n");

    fs::write(&out_path, out).expect("failed to write generated.rs");
    eprintln!("cargo:warning=Generated {} properties, {} pseudo-classes, {} pseudo-elements",
        properties.len(), pseudo_classes.len(), pseudo_elements.len());
}

fn strip_parens(s: &str) -> &str {
    if let Some(idx) = s.find('(') {
        &s[..idx]
    } else {
        s
    }
}

/// Escape a string for use as a Rust string literal value (not the delimiters).
/// We just use Rust's Debug formatting which handles escaping correctly, then
/// unwrap the outer quotes since we wrap in {:?} format ourselves.
fn escape_str(s: &str) -> &str {
    // We'll use {:?} formatting in the format! call, which handles escaping.
    // This function is a no-op; the caller uses {:?} on the value directly.
    s
}
