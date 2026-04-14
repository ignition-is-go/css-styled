use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, Error, LitStr, Result};

/// Lightweight derive that generates only:
/// - VAR_* consts for each field
/// - CSS_VARS const array
/// - An Overrides builder struct with vars() convenience method
///
/// Used for library-internal CSS variables that don't need scopes,
/// base CSS, or consumer-facing configuration.
pub fn derive(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    let fields = match &input.data {
        syn::Data::Struct(s) => match &s.fields {
            syn::Fields::Named(named) => &named.named,
            _ => return Err(Error::new_spanned(struct_name, "CssVars requires named fields")),
        },
        _ => return Err(Error::new_spanned(struct_name, "CssVars can only be derived on structs")),
    };

    let mut var_fields: Vec<(syn::Ident, String)> = Vec::new();

    for field in fields {
        let field_ident = field.ident.as_ref().unwrap().clone();
        let mut var_lit: Option<LitStr> = None;

        for attr in &field.attrs {
            if attr.path().is_ident("var") {
                let lit: LitStr = attr.parse_args()?;
                let val = lit.value();
                if !val.starts_with("--") {
                    return Err(Error::new_spanned(&lit, "CSS variable name must start with `--`"));
                }
                var_lit = Some(lit);
            }
        }

        let var_lit = var_lit.ok_or_else(|| {
            Error::new_spanned(&field_ident, format!(
                "field `{}` is missing a `#[var(\"--name\")]` attribute",
                field_ident,
            ))
        })?;

        var_fields.push((field_ident, var_lit.value()));
    }

    // Register in the proc-macro registry so css! can validate references
    crate::register_vars(
        &struct_name.to_string(),
        var_fields.iter().map(|(_, v)| v.clone()),
    );

    // Generate VAR_* consts
    let var_consts: Vec<TokenStream> = var_fields.iter().map(|(ident, var_name)| {
        let const_name = format_ident!("VAR_{}", ident.to_string().to_uppercase());
        quote! { pub const #const_name: &'static str = #var_name; }
    }).collect();

    // CSS_VARS array
    let var_names: Vec<&str> = var_fields.iter().map(|(_, v)| v.as_str()).collect();

    // Overrides builder
    let overrides_name = format_ident!("{}Overrides", struct_name);

    let struct_fields: Vec<TokenStream> = var_fields.iter().map(|(ident, _)| {
        quote! { #ident: Option<String> }
    }).collect();

    let setter_methods: Vec<TokenStream> = var_fields.iter().map(|(ident, _)| {
        quote! {
            pub fn #ident(mut self, value: impl Into<String>) -> Self {
                self.#ident = Some(value.into());
                self
            }
        }
    }).collect();

    let build_parts: Vec<TokenStream> = var_fields.iter().map(|(ident, var_name)| {
        quote! {
            if let Some(ref val) = self.#ident {
                parts.push(format!("{}: {}", #var_name, val));
            }
        }
    }).collect();

    let default_fields: Vec<TokenStream> = var_fields.iter().map(|(ident, _)| {
        quote! { #ident: None }
    }).collect();

    Ok(quote! {
        impl #struct_name {
            #(#var_consts)*
            pub const CSS_VARS: &'static [&'static str] = &[#(#var_names),*];
        }

        pub struct #overrides_name {
            #(#struct_fields,)*
        }

        impl #overrides_name {
            pub fn new() -> Self {
                Self { #(#default_fields,)* }
            }
            #(#setter_methods)*
            pub fn build(self) -> String {
                let mut parts: Vec<String> = Vec::new();
                #(#build_parts)*
                parts.join("; ")
            }
        }

        impl #struct_name {
            pub fn overrides() -> #overrides_name {
                #overrides_name::new()
            }
            pub fn vars(f: impl FnOnce(#overrides_name) -> #overrides_name) -> String {
                f(#overrides_name::new()).build()
            }
        }
    })
}
