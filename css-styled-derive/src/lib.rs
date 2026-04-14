use proc_macro::TokenStream;

mod parse_attrs;
mod fuzzy;
mod derive_styled;

/// Derive macro for generating typed CSS from a theme struct.
#[proc_macro_derive(StyledComponent, attributes(component, prop))]
pub fn derive_styled_component(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    match derive_styled::derive(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Macro for writing spec-validated CSS with named selectors.
#[proc_macro]
pub fn css(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}
