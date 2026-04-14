use proc_macro::TokenStream;

/// Derive macro for generating typed CSS from a theme struct.
#[proc_macro_derive(StyledComponent, attributes(component, prop))]
pub fn derive_styled_component(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}

/// Macro for writing spec-validated CSS with named selectors.
#[proc_macro]
pub fn css(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}
