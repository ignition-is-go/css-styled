use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, Error, LitStr, Result};

/// Convert a CSS variable name like `--primary-color` to a const name like `VAR_PRIMARY_COLOR`.
fn var_to_const_name(var_name: &str) -> String {
    let stripped = var_name.strip_prefix("--").unwrap_or(var_name);
    format!("VAR_{}", stripped.replace('-', "_").to_uppercase())
}

/// Parsed field from a Theme struct: field ident + var name from `#[var("--name")]`.
struct ThemeField {
    ident: syn::Ident,
    var_name: String,
    var_lit: LitStr,
}

pub fn derive(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Extract named struct fields
    let fields = match &input.data {
        syn::Data::Struct(s) => match &s.fields {
            syn::Fields::Named(named) => &named.named,
            _ => {
                return Err(Error::new_spanned(
                    struct_name,
                    "Theme only supports structs with named fields",
                ))
            }
        },
        _ => {
            return Err(Error::new_spanned(
                struct_name,
                "Theme can only be derived on structs",
            ))
        }
    };

    // Parse #[var("--name")] attributes from each field
    let mut theme_fields = Vec::new();
    for field in fields {
        let field_ident = field.ident.as_ref().unwrap().clone();
        let mut var_lit: Option<LitStr> = None;

        for attr in &field.attrs {
            if !attr.path().is_ident("var") {
                continue;
            }
            let lit: LitStr = attr.parse_args()?;
            let val = lit.value();
            if !val.starts_with("--") {
                return Err(Error::new_spanned(
                    &lit,
                    "CSS custom property name must start with `--`",
                ));
            }
            var_lit = Some(lit);
        }

        let var_lit = var_lit.ok_or_else(|| {
            Error::new_spanned(
                &field_ident,
                format!(
                    "field `{}` is missing a `#[var(\"--name\")]` attribute",
                    field_ident
                ),
            )
        })?;

        let var_name = var_lit.value();
        theme_fields.push(ThemeField {
            ident: field_ident,
            var_name,
            var_lit,
        });
    }

    // Register theme vars in the proc-macro-internal registry
    crate::register_vars(
        &struct_name.to_string(),
        theme_fields.iter().map(|tf| tf.var_name.clone()),
    );

    // Check for duplicate var names
    let mut seen = Vec::new();
    for tf in &theme_fields {
        if seen.contains(&tf.var_name) {
            return Err(Error::new_spanned(
                &tf.var_lit,
                format!("duplicate CSS custom property `{}`", tf.var_name),
            ));
        }
        seen.push(tf.var_name.clone());
    }

    // Generate VAR_* consts (from the CSS variable name, e.g. --ml-surface → VAR_ML_SURFACE)
    // Also generate field-name-based consts (e.g. surface → VAR_SURFACE) for use with
    // `default = theme.surface` in StyledComponent derives.
    let var_consts: Vec<TokenStream> = theme_fields
        .iter()
        .map(|tf| {
            let var_const_name = format_ident!("{}", var_to_const_name(&tf.var_name));
            let field_const_name = format_ident!("VAR_{}", tf.ident.to_string().to_uppercase());
            let var_name = &tf.var_name;
            if var_const_name == field_const_name {
                // Same name — only emit once
                quote! {
                    pub const #var_const_name: &'static str = #var_name;
                }
            } else {
                quote! {
                    pub const #var_const_name: &'static str = #var_name;
                    pub const #field_const_name: &'static str = #var_name;
                }
            }
        })
        .collect();

    // Generate ALL_VARS const
    let all_var_names: Vec<&str> = theme_fields.iter().map(|tf| tf.var_name.as_str()).collect();
    let all_vars_tokens = quote! {
        pub const ALL_VARS: &'static [&'static str] = &[#(#all_var_names),*];
    };

    // Generate IntoThemeCss impl
    let format_parts: Vec<TokenStream> = theme_fields
        .iter()
        .map(|tf| {
            let var_name = &tf.var_name;
            let field_ident = &tf.ident;
            quote! {
                decls.push(format!("{}: {}", #var_name, &self.#field_ident));
            }
        })
        .collect();

    Ok(quote! {
        impl #struct_name {
            #(#var_consts)*
            #all_vars_tokens
        }

        impl css_styled::IntoThemeCss for #struct_name {
            fn to_theme_css(&self) -> String {
                let mut decls: Vec<String> = Vec::new();
                #(#format_parts)*
                format!(":root {{ {} }}", decls.join("; "))
            }
        }
    })
}
