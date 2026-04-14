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

    // Generate pieces
    let scope_const = gen_scope_const(&scope_str);
    let class_consts = gen_class_consts(&config);
    let base_name = {
        let s = struct_name.to_string();
        if s.ends_with("Style") {
            s[..s.len() - 5].to_string()
        } else {
            s.clone()
        }
    };
    let modifier_enum = gen_modifier_enum(struct_name, &base_name, &config);
    let class_method = gen_class_method(struct_name, &base_name, &config, &scope_str);
    let into_css_impl = gen_into_css(struct_name, &parsed_fields, &config, &scope_str);

    Ok(quote! {
        impl #struct_name {
            #scope_const
            #class_consts
            #class_method
        }

        #modifier_enum

        #into_css_impl
    })
}

fn validate_fields(fields: &[ParsedField], config: &ComponentConfig) -> Result<()> {
    let alias_names: Vec<String> = config.classes.iter().map(|(id, _)| id.to_string()).collect();
    let mut seen: Vec<(String, Option<String>, Option<String>)> = Vec::new();

    for field in fields {
        let PropConfig::Mapped { css, on, pseudo } = &field.config else {
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

    for field in fields {
        let PropConfig::Mapped { css, on, pseudo } = &field.config else {
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

        impl css_styled::StyledComponentBase for #struct_name {}
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
