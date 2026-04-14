use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, Error, Result};

use crate::fuzzy::closest_match;
use crate::parse_attrs::{self, ComponentConfig, ParsedField, PropConfig};

pub fn derive(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;
    let config = parse_attrs::parse_component_config(&input)?;
    let scope_str = config.scope.as_ref().unwrap().value();

    // Extract named struct fields
    let fields = match &input.data {
        syn::Data::Struct(s) => match &s.fields {
            syn::Fields::Named(named) => &named.named,
            _ => {
                return Err(Error::new_spanned(
                    struct_name,
                    "StyledComponent only supports structs with named fields",
                ))
            }
        },
        _ => {
            return Err(Error::new_spanned(
                struct_name,
                "StyledComponent can only be derived on structs",
            ))
        }
    };

    // Parse and validate all fields
    let mut parsed_fields = Vec::new();
    for field in fields {
        let field_ident = field.ident.as_ref().unwrap().clone();
        match parse_attrs::parse_prop_config(field)? {
            Some(prop_config) => {
                parsed_fields.push(ParsedField {
                    ident: field_ident,
                    config: prop_config,
                });
            }
            None => {
                return Err(Error::new_spanned(
                    field,
                    format!(
                        "field `{}` is missing a `#[prop(...)]` attribute; add `#[prop(css = \"...\")]` or `#[prop(skip)]`",
                        field_ident
                    ),
                ));
            }
        }
    }

    // Compile-time validation
    validate_fields(&parsed_fields, &config)?;

    // Register vars in the proc-macro-internal registry so css! can validate them
    let component_var_names: Vec<String> = parsed_fields.iter().filter_map(|f| {
        if let PropConfig::Variable { var, .. } = &f.config { Some(var.value()) } else { None }
    }).collect();
    crate::register_vars(&struct_name.to_string(), component_var_names);
    if let Some(theme_path) = &config.theme {
        let theme_name = quote!(#theme_path).to_string().replace(' ', "");
        crate::register_theme_vars(&struct_name.to_string(), &theme_name);
    }

    // Generate pieces
    let scope_const = gen_scope_const(&scope_str);
    let class_consts = gen_class_consts(&config);
    let var_consts = gen_var_consts(&parsed_fields);
    let theme_vars_const = gen_theme_vars_const(&config);
    let css_vars_const = gen_css_vars_const(&parsed_fields);
    let base_name = {
        let s = struct_name.to_string();
        if s.ends_with("Style") {
            s[..s.len() - 5].to_string()
        } else {
            s.clone()
        }
    };
    let modifier_consts = gen_modifier_consts(&config);
    let modifier_enum = gen_modifier_enum(struct_name, &base_name, &config);
    let class_method = gen_class_method(struct_name, &base_name, &config, &scope_str);
    let into_css_impl = gen_into_css(struct_name, &parsed_fields, &config, &scope_str);
    let default_impl = gen_default_impl(struct_name, &parsed_fields, &config)?;
    let overrides_struct = gen_overrides(struct_name, &base_name, &parsed_fields);

    Ok(quote! {
        impl #struct_name {
            #scope_const
            #class_consts
            #modifier_consts
            #var_consts
            #css_vars_const
            #theme_vars_const
            #class_method
        }

        #modifier_enum

        #into_css_impl

        #default_impl

        #overrides_struct
    })
}

fn validate_fields(fields: &[ParsedField], config: &ComponentConfig) -> Result<()> {
    let alias_names: Vec<String> = config.classes.iter().map(|(id, _)| id.to_string()).collect();
    let mut seen: Vec<(String, Option<String>, Option<String>)> = Vec::new();
    let mut seen_vars: Vec<String> = Vec::new();

    for field in fields {
        // Check for Variable fields
        if let PropConfig::Variable { var, .. } = &field.config {
            let var_name = var.value();
            if seen_vars.contains(&var_name) {
                return Err(Error::new_spanned(
                    var,
                    format!("duplicate CSS custom property `{var_name}`"),
                ));
            }
            seen_vars.push(var_name);
            continue;
        }

        let PropConfig::Mapped { css, on, pseudo, .. } = &field.config else {
            continue;
        };

        // Validate CSS property name
        let css_name = css.value();
        if css_spec_data::property(&css_name).is_none() {
            let all = css_spec_data::all_property_names();
            let suggestion = closest_match(&css_name, all);
            let msg = if let Some(s) = suggestion {
                format!("unknown CSS property `{css_name}`; did you mean `{s}`?")
            } else {
                format!("unknown CSS property `{css_name}`")
            };
            return Err(Error::new_spanned(css, msg));
        }

        // Validate `on` alias
        if let Some(on_ident) = on {
            let on_name = on_ident.to_string();
            if !alias_names.contains(&on_name) {
                let msg = format!(
                    "unknown class alias `{on_name}`; available aliases: {}",
                    alias_names.join(", ")
                );
                return Err(Error::new_spanned(on_ident, msg));
            }
        }

        // Validate pseudo-class/element
        if let Some(pseudo_lit) = pseudo {
            let pseudo_str = pseudo_lit.value();
            if pseudo_str.starts_with("::") {
                if !css_spec_data::is_pseudo_element(&pseudo_str) {
                    let all = css_spec_data::all_pseudo_elements();
                    let suggestion = closest_match(&pseudo_str, all);
                    let msg = if let Some(s) = suggestion {
                        format!("unknown CSS pseudo-element `{pseudo_str}`; did you mean `{s}`?")
                    } else {
                        format!("unknown CSS pseudo-element `{pseudo_str}`")
                    };
                    return Err(Error::new_spanned(pseudo_lit, msg));
                }
            } else if pseudo_str.starts_with(':') {
                if !css_spec_data::is_pseudo_class(&pseudo_str) {
                    let all = css_spec_data::all_pseudo_classes();
                    let suggestion = closest_match(&pseudo_str, all);
                    let msg = if let Some(s) = suggestion {
                        format!("unknown CSS pseudo-class `{pseudo_str}`; did you mean `{s}`?")
                    } else {
                        format!("unknown CSS pseudo-class `{pseudo_str}`")
                    };
                    return Err(Error::new_spanned(pseudo_lit, msg));
                }
            } else {
                return Err(Error::new_spanned(
                    pseudo_lit,
                    "pseudo must start with `:` or `::`",
                ));
            }
        }

        // Check for duplicate (css, on, pseudo) combos
        let key = (
            css_name.clone(),
            on.as_ref().map(|i| i.to_string()),
            pseudo.as_ref().map(|l| l.value()),
        );
        if seen.contains(&key) {
            return Err(Error::new_spanned(
                css,
                format!(
                    "duplicate CSS property `{css_name}` for the same selector and pseudo combination"
                ),
            ));
        }
        seen.push(key);
    }

    Ok(())
}

fn gen_scope_const(scope_str: &str) -> TokenStream {
    quote! {
        pub const SCOPE: &'static str = #scope_str;
    }
}

fn gen_class_consts(config: &ComponentConfig) -> TokenStream {
    let consts: Vec<TokenStream> = config
        .classes
        .iter()
        .map(|(alias, lit)| {
            let const_name = format_ident!("{}", alias.to_string().to_uppercase());
            quote! {
                pub const #const_name: &'static str = #lit;
            }
        })
        .collect();
    quote! { #(#consts)* }
}

/// Convert a CSS variable name like `--sh-thickness` to a const name like `VAR_SH_THICKNESS`.
fn var_to_const_name(var_name: &str) -> String {
    let stripped = var_name.strip_prefix("--").unwrap_or(var_name);
    format!("VAR_{}", stripped.replace('-', "_").to_uppercase())
}

fn gen_var_consts(fields: &[ParsedField]) -> TokenStream {
    let consts: Vec<TokenStream> = fields
        .iter()
        .filter_map(|f| {
            if let PropConfig::Variable { var, .. } = &f.config {
                let var_name = var.value();
                let const_name = format_ident!("{}", var_to_const_name(&var_name));
                Some(quote! {
                    pub const #const_name: &'static str = #var_name;
                })
            } else {
                None
            }
        })
        .collect();
    quote! { #(#consts)* }
}

fn gen_css_vars_const(fields: &[ParsedField]) -> TokenStream {
    let var_names: Vec<String> = fields
        .iter()
        .filter_map(|f| {
            if let PropConfig::Variable { var, .. } = &f.config {
                Some(var.value())
            } else {
                None
            }
        })
        .collect();
    quote! {
        pub const CSS_VARS: &'static [&'static str] = &[#(#var_names),*];
    }
}

fn gen_theme_vars_const(config: &ComponentConfig) -> TokenStream {
    if let Some(theme_path) = &config.theme {
        quote! {
            pub const THEME_VARS: &'static [&'static str] = #theme_path::ALL_VARS;
        }
    } else {
        quote! {
            pub const THEME_VARS: &'static [&'static str] = &[];
        }
    }
}

fn gen_modifier_consts(config: &ComponentConfig) -> TokenStream {
    let consts: Vec<TokenStream> = config
        .modifiers
        .iter()
        .map(|m| {
            let name_str = m.to_string();
            let const_name = format_ident!("{}", name_str.to_uppercase());
            quote! {
                pub const #const_name: &'static str = #name_str;
            }
        })
        .collect();
    quote! { #(#consts)* }
}

fn gen_modifier_enum(_struct_name: &syn::Ident, base_name: &str, config: &ComponentConfig) -> TokenStream {
    if config.modifiers.is_empty() {
        return TokenStream::new();
    }

    let enum_name = format_ident!("{}Modifier", base_name);
    let variants: Vec<syn::Ident> = config
        .modifiers
        .iter()
        .map(|m| format_ident!("{}", to_pascal_case(&m.to_string())))
        .collect();
    let variant_strings: Vec<String> = config.modifiers.iter().map(|m| m.to_string()).collect();

    let match_arms: Vec<TokenStream> = variants
        .iter()
        .zip(variant_strings.iter())
        .map(|(v, s)| quote! { #enum_name::#v => #s })
        .collect();

    quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum #enum_name {
            #(#variants),*
        }

        impl #enum_name {
            pub fn as_str(&self) -> &'static str {
                match self {
                    #(#match_arms),*
                }
            }
        }
    }
}

fn gen_class_method(
    _struct_name: &syn::Ident,
    base_name: &str,
    config: &ComponentConfig,
    scope_str: &str,
) -> TokenStream {
    if config.modifiers.is_empty() {
        return TokenStream::new();
    }

    let enum_name = format_ident!("{}Modifier", base_name);

    quote! {
        pub fn class(modifiers: &[#enum_name]) -> String {
            let mut result = String::from(#scope_str);
            for m in modifiers {
                result.push(' ');
                result.push_str(m.as_str());
            }
            result
        }
    }
}

fn gen_into_css(
    struct_name: &syn::Ident,
    fields: &[ParsedField],
    config: &ComponentConfig,
    scope_str: &str,
) -> TokenStream {
    let mut rule_exprs = Vec::new();

    // Collect CSS variable fields into a single rule on the scope selector
    let var_fields: Vec<&ParsedField> = fields
        .iter()
        .filter(|f| matches!(&f.config, PropConfig::Variable { .. }))
        .collect();

    if !var_fields.is_empty() {
        let var_idents: Vec<&syn::Ident> = var_fields
            .iter()
            .map(|f| &f.ident)
            .collect();
        let var_names: Vec<String> = var_fields
            .iter()
            .map(|f| {
                if let PropConfig::Variable { var, .. } = &f.config {
                    var.value()
                } else {
                    unreachable!()
                }
            })
            .collect();
        let scope_selector = format!(".{}", scope_str);

        rule_exprs.push(quote! {
            {
                let mut decls = Vec::new();
                #(
                    decls.push(format!("{}: {}", #var_names, &self.#var_idents));
                )*
                format!("{} {{ {} }}", #scope_selector, decls.join("; "))
            }
        });
    }

    for field in fields {
        let PropConfig::Mapped { css, on, pseudo, .. } = &field.config else {
            continue;
        };

        let field_ident = &field.ident;
        let css_name = css.value();

        // Build the selector
        let selector = build_selector(scope_str, on, pseudo, config);

        let css_name_str = &css_name;
        rule_exprs.push(quote! {
            {
                let val = &self.#field_ident;
                let result = css_styled::css_spec_data::validate_value(#css_name_str, val);
                match result {
                    css_styled::css_spec_data::ValidationResult::Valid => {},
                    css_styled::css_spec_data::ValidationResult::Warn(msg) => {
                        eprintln!("css-styled warning: property `{}` value `{}`: {}", #css_name_str, val, msg);
                    },
                    css_styled::css_spec_data::ValidationResult::Invalid(msg) => {
                        eprintln!("css-styled error: property `{}` value `{}`: {}", #css_name_str, val, msg);
                    },
                }
                format!("{} {{ {}: {}; }}", #selector, #css_name_str, val)
            }
        });
    }

    let base_css_impl = if config.custom_base_css {
        // User will provide their own StyledComponentBase impl
        TokenStream::new()
    } else {
        quote! { impl css_styled::StyledComponentBase for #struct_name {} }
    };

    quote! {
        impl css_styled::IntoCss for #struct_name {
            fn to_css(&self) -> String {
                let base = <Self as css_styled::StyledComponentBase>::base_css();
                let mut rules: Vec<String> = Vec::new();
                if !base.is_empty() {
                    rules.push(base.to_string());
                }
                #(rules.push(#rule_exprs);)*
                rules.join("\n")
            }

            fn scope(&self) -> &'static str {
                #scope_str
            }
        }

        #base_css_impl
    }
}

fn build_selector(
    scope_str: &str,
    on: &Option<syn::Ident>,
    pseudo: &Option<syn::LitStr>,
    config: &ComponentConfig,
) -> String {
    let mut selector = format!(".{}", scope_str);

    // Add pseudo to the scope part
    if let Some(pseudo_lit) = pseudo {
        selector.push_str(&pseudo_lit.value());
    }

    // Add child class if `on` is specified
    if let Some(on_ident) = on {
        let on_name = on_ident.to_string();
        if let Some((_, class_lit)) = config.classes.iter().find(|(id, _)| id == &on_name) {
            selector.push_str(&format!(" .{}", class_lit.value()));
        }
    }

    selector
}

fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;
    for c in s.chars() {
        if c == '_' || c == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Generate an overrides builder struct for per-instance CSS variable overrides.
///
/// Produces `{BaseName}Overrides` with a builder API:
/// ```ignore
/// ActivityBarStyle::vars(|v| v.icon_opacity("1").icon_color("red"))
/// // → "--ab-icon-opacity: 1; --ab-icon-color: red"
/// ```
fn gen_overrides(
    struct_name: &syn::Ident,
    base_name: &str,
    fields: &[ParsedField],
) -> TokenStream {
    // Collect variable fields only
    let var_fields: Vec<(&syn::Ident, String)> = fields
        .iter()
        .filter_map(|f| {
            if let PropConfig::Variable { var, .. } = &f.config {
                Some((&f.ident, var.value()))
            } else {
                None
            }
        })
        .collect();

    if var_fields.is_empty() {
        return TokenStream::new();
    }

    let overrides_name = format_ident!("{}Overrides", base_name);

    // Struct fields: Option<String> per variable
    let struct_fields: Vec<TokenStream> = var_fields.iter().map(|(ident, _)| {
        quote! { #ident: Option<String> }
    }).collect();

    // Builder setter methods
    let setter_methods: Vec<TokenStream> = var_fields.iter().map(|(ident, _)| {
        quote! {
            pub fn #ident(mut self, value: impl Into<String>) -> Self {
                self.#ident = Some(value.into());
                self
            }
        }
    }).collect();

    // Build method — produces the inline style string
    let build_parts: Vec<TokenStream> = var_fields.iter().map(|(ident, var_name)| {
        quote! {
            if let Some(ref val) = self.#ident {
                parts.push(format!("{}: {}", #var_name, val));
            }
        }
    }).collect();

    // Default field initializers (all None)
    let default_fields: Vec<TokenStream> = var_fields.iter().map(|(ident, _)| {
        quote! { #ident: None }
    }).collect();

    quote! {
        /// Per-instance CSS variable overrides. Use `build()` to get an inline
        /// style string, or pass to an element's `style` attribute.
        pub struct #overrides_name {
            #(#struct_fields,)*
        }

        impl #overrides_name {
            pub fn new() -> Self {
                Self { #(#default_fields,)* }
            }

            #(#setter_methods)*

            /// Produce the inline CSS variable declarations.
            pub fn build(self) -> String {
                let mut parts: Vec<String> = Vec::new();
                #(#build_parts)*
                parts.join("; ")
            }
        }

        impl #struct_name {
            /// Create a per-instance CSS variable override builder.
            pub fn overrides() -> #overrides_name {
                #overrides_name::new()
            }

            /// Convenience: build overrides via a closure.
            pub fn vars(f: impl FnOnce(#overrides_name) -> #overrides_name) -> String {
                f(#overrides_name::new()).build()
            }
        }
    }
}

/// Generate a `Default` impl if all fields have a `default` attribute.
/// Fields with `default = theme.x` resolve to `format!("var({})", ThemePath::VAR_X)`.
/// Fields with `default = "literal"` use the literal value.
/// If any field is missing a default, returns empty (user writes their own Default).
fn gen_default_impl(
    struct_name: &syn::Ident,
    fields: &[ParsedField],
    config: &ComponentConfig,
) -> Result<TokenStream> {
    use crate::parse_attrs::PropDefault;

    // Check if all non-skip fields have defaults
    let mut field_defaults: Vec<(&syn::Ident, TokenStream)> = Vec::new();

    for field in fields {
        let default_opt = match &field.config {
            PropConfig::Skip => continue,
            PropConfig::Mapped { default, .. } => default,
            PropConfig::Variable { default, .. } => default,
        };

        let Some(default) = default_opt else {
            // A field without a default — can't generate Default impl
            return Ok(TokenStream::new());
        };

        let value_expr = match default {
            PropDefault::ThemeVar(theme_field) => {
                let Some(theme_path) = &config.theme else {
                    return Err(Error::new_spanned(
                        theme_field,
                        "cannot use `default = theme.field` without #[component(theme = ...)]",
                    ));
                };
                // Validate the field name against the theme's registered fields
                let theme_name = quote!(#theme_path).to_string().replace(' ', "");
                let field_str = theme_field.to_string();
                if let Some(known_fields) = crate::lookup_theme_fields(&theme_name) {
                    if !known_fields.contains(&field_str) {
                        let mut available: Vec<_> = known_fields.iter().cloned().collect();
                        available.sort();
                        return Err(Error::new_spanned(
                            theme_field,
                            format!(
                                "unknown theme field `{}`; `{}` has fields: {}",
                                field_str, theme_name, available.join(", "),
                            ),
                        ));
                    }
                }
                let const_name = format_ident!(
                    "VAR_{}",
                    theme_field.to_string().to_uppercase()
                );
                quote! {
                    format!("var({})", #theme_path::#const_name).into()
                }
            }
            PropDefault::Literal(lit) => {
                quote! { #lit.into() }
            }
        };

        field_defaults.push((&field.ident, value_expr));
    }

    let field_inits: Vec<TokenStream> = field_defaults.iter().map(|(ident, expr)| {
        quote! { #ident: #expr }
    }).collect();

    Ok(quote! {
        impl Default for #struct_name {
            fn default() -> Self {
                Self {
                    #(#field_inits,)*
                }
            }
        }
    })
}
