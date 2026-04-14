use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{braced, Error, Ident, Result, Token};

use crate::fuzzy::closest_match;

/// A single CSS declaration: `property-name: value;`
struct CssDeclaration {
    property: String,
    property_span: Span,
    value: String,
    value_span: Span,
    /// CSS variable references found in the value, e.g. `["--w-size"]`
    var_refs: Vec<(String, Span)>,
}

/// A segment in a compound selector (names joined by dots).
/// e.g. `SCOPE.ACTIVE` is two names in one compound segment.
struct CompoundSelector {
    names: Vec<Ident>,
}

/// A full selector is one or more compound selectors separated by whitespace (descendant combinator).
struct Selector {
    compounds: Vec<CompoundSelector>,
}

/// A CSS rule: selector { declarations }
struct CssRule {
    selector: Selector,
    declarations: Vec<CssDeclaration>,
}

/// Top-level input: `StructName, { rules... }`
pub struct CssMacroInput {
    struct_name: Ident,
    rules: Vec<CssRule>,
}

impl Parse for CssMacroInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let struct_name: Ident = input.parse()?;
        input.parse::<Token![,]>()?;

        let content;
        braced!(content in input);

        let mut rules = Vec::new();
        while !content.is_empty() {
            rules.push(parse_rule(&content)?);
        }

        Ok(CssMacroInput {
            struct_name,
            rules,
        })
    }
}

/// Parse a single rule: SELECTOR { property: value; ... }
fn parse_rule(input: ParseStream) -> Result<CssRule> {
    let selector = parse_selector(input)?;

    let decl_content;
    braced!(decl_content in input);

    let mut declarations = Vec::new();
    while !decl_content.is_empty() {
        declarations.push(parse_declaration(&decl_content)?);
    }

    Ok(CssRule {
        selector,
        declarations,
    })
}

/// Parse a selector like `SCOPE.ACTIVE INNER`
/// Uppercase idents separated by dots (compound) or spaces (descendant).
fn parse_selector(input: ParseStream) -> Result<Selector> {
    let mut compounds = Vec::new();
    compounds.push(parse_compound_selector(input)?);

    // Keep parsing compound selectors while the next token is an uppercase ident
    // (not followed by a brace, which would start declarations).
    while !input.peek(syn::token::Brace) && input.peek(Ident) {
        compounds.push(parse_compound_selector(input)?);
    }

    Ok(Selector { compounds })
}

/// Parse a compound selector like `SCOPE.ACTIVE` (dot-separated uppercase idents).
fn parse_compound_selector(input: ParseStream) -> Result<CompoundSelector> {
    let mut names = Vec::new();
    let first: Ident = input.parse()?;
    validate_uppercase_ident(&first)?;
    names.push(first);

    while input.peek(Token![.]) {
        input.parse::<Token![.]>()?;
        let name: Ident = input.parse()?;
        validate_uppercase_ident(&name)?;
        names.push(name);
    }

    Ok(CompoundSelector { names })
}

fn validate_uppercase_ident(ident: &Ident) -> Result<()> {
    let s = ident.to_string();
    if s.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
        Ok(())
    } else {
        Err(Error::new(
            ident.span(),
            format!(
                "selector names must be UPPERCASE; got `{}`",
                s
            ),
        ))
    }
}

/// Parse a CSS declaration like `align-items: center;`
fn parse_declaration(input: ParseStream) -> Result<CssDeclaration> {
    // Parse hyphenated property name
    let property_span = input.span();
    let property = parse_hyphenated_ident(input)?;

    input.parse::<Token![:]>()?;

    // Parse value tokens until `;`
    let value_span = input.span();
    let (value, var_refs) = parse_value(input)?;

    input.parse::<Token![;]>()?;

    Ok(CssDeclaration {
        property,
        property_span,
        value,
        value_span,
        var_refs,
    })
}

/// Parse a hyphenated identifier like `align-items` or `font-weight`.
fn parse_hyphenated_ident(input: ParseStream) -> Result<String> {
    let first: Ident = input.parse()?;
    let mut name = first.to_string();

    while input.peek(Token![-]) {
        input.parse::<Token![-]>()?;
        let part: Ident = input.parse()?;
        name.push('-');
        name.push_str(&part.to_string());
    }

    Ok(name)
}

/// Parse a CSS value (everything up to the semicolon).
/// Returns the value string and any `var(--name)` references found.
fn parse_value(input: ParseStream) -> Result<(String, Vec<(String, Span)>)> {
    let mut parts = Vec::new();
    let mut var_refs = Vec::new();

    while !input.peek(Token![;]) {
        if input.is_empty() {
            return Err(input.error("expected `;` after CSS value"));
        }

        // Handle negative numbers (e.g. `-1px`)
        if input.peek(Token![-]) {
            input.parse::<Token![-]>()?;
            parts.push("-".to_string());
            continue;
        }

        // Handle idents (possibly hyphenated, possibly function calls like `var(...)`)
        if input.peek(Ident) {
            let ident: Ident = input.parse()?;
            let mut word = ident.to_string();

            // Check for function call syntax: ident(...)
            if input.peek(syn::token::Paren) {
                let content;
                let _paren_span = syn::parenthesized!(content in input);
                let inner = parse_function_args(&content)?;
                let func_str = format!("{}({})", word, inner.0);

                // If this is a var() call, extract the variable name reference
                if word == "var" {
                    if let Some(var_name) = &inner.1 {
                        var_refs.push((var_name.clone(), ident.span()));
                    }
                }

                parts.push(func_str);
                continue;
            }

            // Check for hyphenated values like `no-repeat`
            while input.peek(Token![-]) && !input.peek2(Token![;]) {
                // Peek further: if after `-` there's an ident, it's hyphenated
                if input.peek(Token![-]) && input.peek2(Ident) {
                    input.parse::<Token![-]>()?;
                    let next: Ident = input.parse()?;
                    word.push('-');
                    word.push_str(&next.to_string());
                } else {
                    break;
                }
            }
            parts.push(word);
            continue;
        }

        // Handle literal values (numbers, strings, etc.)
        if input.peek(syn::Lit) {
            let lit: syn::Lit = input.parse()?;
            let mut s = match &lit {
                syn::Lit::Int(i) => i.to_string(),
                syn::Lit::Float(f) => f.to_string(),
                syn::Lit::Str(s) => s.value(),
                _ => format!("{}", quote!(#lit)),
            };
            // Attach a trailing `%` without a space (e.g. `100%`)
            if input.peek(Token![%]) {
                input.parse::<Token![%]>()?;
                s.push('%');
            }
            parts.push(s);
            continue;
        }

        // Handle punct (for things like `#fff`, commas, etc.)
        let tt: proc_macro2::TokenTree = input.parse()?;
        parts.push(tt.to_string());
    }

    // Join with spaces, but collapse spaces around `-` that was pushed standalone
    let mut result = String::new();
    for (i, part) in parts.iter().enumerate() {
        if part == "-" {
            // Negative sign: attach to next token, no space
            result.push('-');
            continue;
        }
        if i > 0 && !result.ends_with('-') && !result.is_empty() {
            result.push(' ');
        }
        result.push_str(part);
    }

    Ok((result, var_refs))
}

/// Parse the inside of a function call like `var(--w-size)`.
/// Returns (the string content, optional var name if this looks like a CSS variable reference).
fn parse_function_args(input: ParseStream) -> Result<(String, Option<String>)> {
    let mut parts = Vec::new();
    let mut var_name = None;

    // Check for `--name` pattern (CSS variable reference)
    if input.peek(Token![-]) {
        input.parse::<Token![-]>()?;
        if input.peek(Token![-]) {
            input.parse::<Token![-]>()?;
            // Now parse hyphenated ident
            let first: Ident = input.parse()?;
            let mut name = first.to_string();
            while input.peek(Token![-]) && input.peek2(Ident) {
                input.parse::<Token![-]>()?;
                let next: Ident = input.parse()?;
                name.push('-');
                name.push_str(&next.to_string());
            }
            let full_var = format!("--{}", name);
            var_name = Some(full_var.clone());
            parts.push(full_var);
        } else {
            parts.push("-".to_string());
        }
    }

    // Parse remaining tokens
    while !input.is_empty() {
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            parts.push(",".to_string());
            continue;
        }
        let tt: proc_macro2::TokenTree = input.parse()?;
        parts.push(tt.to_string());
    }

    let result = parts.join(" ").replace(" ,", ",");
    Ok((result, var_name))
}

/// Convert a CSS variable name like `--sh-thickness` to a const name like `VAR_SH_THICKNESS`.
fn var_to_const_name(var_name: &str) -> String {
    let stripped = var_name.strip_prefix("--").unwrap_or(var_name);
    format!("VAR_{}", stripped.replace('-', "_").to_uppercase())
}

/// Validate CSS properties and values, then generate the output code.
pub fn expand(input: CssMacroInput) -> Result<TokenStream> {
    let struct_name = &input.struct_name;

    // Collect all var() references for compile-time validation
    let mut var_checks: Vec<TokenStream> = Vec::new();

    // Validate all declarations at compile time
    for rule in &input.rules {
        for decl in &rule.declarations {
            // Validate property name
            if css_spec_data::property(&decl.property).is_none() {
                let all = css_spec_data::all_property_names();
                let suggestion = closest_match(&decl.property, all);
                let msg = if let Some(s) = suggestion {
                    format!("unknown CSS property `{}`; did you mean `{}`?", decl.property, s)
                } else {
                    format!("unknown CSS property `{}`", decl.property)
                };
                return Err(Error::new(decl.property_span, msg));
            }

            // Skip value validation for values containing var() references
            let has_var_refs = !decl.var_refs.is_empty();

            if !has_var_refs {
                // Validate value
                let result = css_spec_data::validate_value(&decl.property, &decl.value);
                match result {
                    css_spec_data::ValidationResult::Valid => {}
                    css_spec_data::ValidationResult::Warn(_) => {
                        // Warnings are acceptable at compile time; don't fail
                    }
                    css_spec_data::ValidationResult::Invalid(msg) => {
                        return Err(Error::new(
                            decl.value_span,
                            format!(
                                "invalid CSS value `{}` for property `{}`: {}",
                                decl.value, decl.property, msg
                            ),
                        ));
                    }
                }
            }

            // Generate compile-time checks for var() references
            for (var_name, span) in &decl.var_refs {
                let const_name = format_ident!("{}", var_to_const_name(var_name), span = *span);
                var_checks.push(quote! {
                    let _ = #struct_name::#const_name;
                });
            }
        }
    }

    // Generate runtime code
    let rule_pushes: Vec<TokenStream> = input
        .rules
        .iter()
        .map(|rule| {
            let (format_str, args) = build_format_for_rule(struct_name, rule);
            quote! {
                parts.push(format!(#format_str, #(#args),*));
            }
        })
        .collect();

    Ok(quote! {
        {
            // Compile-time validation of var() references against struct consts
            #[allow(unused)]
            const _: () = {
                fn _check_var_refs() {
                    #(#var_checks)*
                }
            };

            static CSS: ::std::sync::OnceLock<String> = ::std::sync::OnceLock::new();
            CSS.get_or_init(|| {
                let mut parts: Vec<String> = Vec::new();
                #(#rule_pushes)*
                parts.join("\n")
            }).as_str()
        }
    })
}

/// Build a format string and arguments for a single CSS rule.
fn build_format_for_rule(struct_name: &Ident, rule: &CssRule) -> (String, Vec<TokenStream>) {
    let mut format_parts = Vec::new();
    let mut args: Vec<TokenStream> = Vec::new();

    // Build selector portion
    for (ci, compound) in rule.selector.compounds.iter().enumerate() {
        if ci > 0 {
            format_parts.push(" ".to_string());
        }
        for (ni, name) in compound.names.iter().enumerate() {
            if ni > 0 {
                // Compound: dot between, no space
                format_parts.push(".{}".to_string());
            } else {
                format_parts.push(".{}".to_string());
            }
            args.push(quote! { #struct_name::#name });
        }
    }

    // Build declarations portion
    let decl_strs: Vec<String> = rule
        .declarations
        .iter()
        .map(|d| format!("{}: {};", d.property, d.value))
        .collect();
    let decl_body = decl_strs.join(" ");

    format_parts.push(format!(" {{{{ {} }}}}", decl_body));

    let format_string = format_parts.join("");
    (format_string, args)
}
