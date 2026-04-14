use proc_macro::TokenStream;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

mod parse_attrs;
mod fuzzy;
mod derive_styled;
mod derive_theme;
mod css_macro;

/// Registry of CSS variable names declared by each struct.
/// Populated by StyledComponent and Theme derives, read by the css! macro.
/// Both run in the same compiler process, so this shared state works.
static VAR_REGISTRY: Mutex<Option<HashMap<String, HashSet<String>>>> = Mutex::new(None);

/// Registry of which theme each component uses.
static THEME_REGISTRY: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

/// Registry of theme field names (for validating `default = theme.field_name`).
static THEME_FIELDS: Mutex<Option<HashMap<String, HashSet<String>>>> = Mutex::new(None);

/// Register CSS variable names for a struct (called by derives).
fn register_vars(struct_name: &str, vars: impl IntoIterator<Item = String>) {
    let mut guard = VAR_REGISTRY.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    let entry = map.entry(struct_name.to_string()).or_default();
    for var in vars {
        entry.insert(var);
    }
}

/// Register theme vars as available to a component (called by StyledComponent derive).
fn register_theme_vars(struct_name: &str, theme_name: &str) {
    // Record the theme association
    {
        let mut guard = THEME_REGISTRY.lock().unwrap();
        let map = guard.get_or_insert_with(HashMap::new);
        map.insert(struct_name.to_string(), theme_name.to_string());
    }
    // Copy theme vars to the component's entry
    let mut guard = VAR_REGISTRY.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    let theme_vars: HashSet<String> = map.get(theme_name).cloned().unwrap_or_default();
    let entry = map.entry(struct_name.to_string()).or_default();
    for var in theme_vars {
        entry.insert(var);
    }
}

/// Look up all known CSS vars for a struct (called by css! macro).
fn lookup_vars(struct_name: &str) -> Option<HashSet<String>> {
    let guard = VAR_REGISTRY.lock().unwrap();
    guard.as_ref()?.get(struct_name).cloned()
}

/// Look up which theme a component uses (called by css! macro for error messages).
fn lookup_theme(struct_name: &str) -> Option<String> {
    let guard = THEME_REGISTRY.lock().unwrap();
    guard.as_ref()?.get(struct_name).cloned()
}

/// Register theme field names (called by Theme derive).
fn register_theme_fields(theme_name: &str, fields: impl IntoIterator<Item = String>) {
    let mut guard = THEME_FIELDS.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    let entry = map.entry(theme_name.to_string()).or_default();
    for f in fields {
        entry.insert(f);
    }
}

/// Look up known field names for a theme (called by StyledComponent derive).
fn lookup_theme_fields(theme_name: &str) -> Option<HashSet<String>> {
    let guard = THEME_FIELDS.lock().unwrap();
    guard.as_ref()?.get(theme_name).cloned()
}

/// Derive macro for generating typed CSS from a theme struct.
#[proc_macro_derive(StyledComponent, attributes(component, prop))]
pub fn derive_styled_component(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    match derive_styled::derive(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive macro for generating a global CSS theme with custom properties.
#[proc_macro_derive(Theme, attributes(var))]
pub fn derive_theme(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    match derive_theme::derive(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Macro for writing spec-validated CSS with named selectors.
#[proc_macro]
pub fn css(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as css_macro::CssMacroInput);
    match css_macro::expand(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
