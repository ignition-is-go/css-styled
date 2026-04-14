use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Error, Ident, LitStr, Result, Token};

use lightningcss::stylesheet::{ParserFlags, ParserOptions, StyleSheet};

/// Top-level input: `StructName, "...css..."`
pub struct CssMacroInput {
    struct_name: Ident,
    css_template: String,
    css_span: Span,
    /// Unique uppercase name references found in `{NAME}` placeholders
    name_refs: Vec<String>,
    /// CSS variable references: var name including `--` prefix
    var_refs: Vec<String>,
}

impl Parse for CssMacroInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let struct_name: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let css_lit: LitStr = input.parse()?;
        let css_template = css_lit.value();
        let css_span = css_lit.span();

        // Extract {NAME} references (uppercase names in braces)
        let mut name_refs = Vec::new();
        let mut i = 0;
        let bytes = css_template.as_bytes();
        while i < bytes.len() {
            if bytes[i] == b'{' {
                if let Some(end) = css_template[i + 1..].find('}') {
                    let name = &css_template[i + 1..i + 1 + end];
                    if !name.is_empty()
                        && name
                            .chars()
                            .all(|c| c.is_ascii_uppercase() || c == '_')
                    {
                        if !name_refs.contains(&name.to_string()) {
                            name_refs.push(name.to_string());
                        }
                    }
                    i = i + 1 + end + 1;
                    continue;
                }
            }
            i += 1;
        }

        // Extract var(--name) references
        let mut var_refs = Vec::new();
        let search = "var(";
        let mut pos = 0;
        while let Some(idx) = css_template[pos..].find(search) {
            let start = pos + idx + search.len();
            let trimmed = css_template[start..].trim_start();
            if trimmed.starts_with("--") {
                let var_start = start + (css_template[start..].len() - trimmed.len());
                if let Some(end) = trimmed.find(|c: char| c == ')' || c == ',') {
                    let var_name = trimmed[..end].trim().to_string();
                    if !var_refs.contains(&var_name) {
                        var_refs.push(var_name);
                    }
                }
                pos = var_start + 1;
            } else {
                pos = start;
            }
        }

        Ok(CssMacroInput {
            struct_name,
            css_template,
            css_span,
            name_refs,
            var_refs,
        })
    }
}

/// Validate CSS and generate the output code.
pub fn expand(input: CssMacroInput) -> Result<TokenStream> {
    let struct_name = &input.struct_name;
    let struct_str = struct_name.to_string();

    // Validate var() references against the proc-macro registry
    for var_name in &input.var_refs {
        if let Some(known_vars) = crate::lookup_vars(&struct_str) {
            if !known_vars.contains(var_name.as_str()) {
                let msg = match crate::lookup_theme(&struct_str) {
                    Some(theme) => format!(
                        "unknown CSS variable `{}`; not declared with #[prop(var = \"{}\")] on `{}` or in theme `{}`",
                        var_name, var_name, struct_str, theme,
                    ),
                    None => format!(
                        "unknown CSS variable `{}`; not declared with #[prop(var = \"{}\")] on `{}` (no theme set)",
                        var_name, var_name, struct_str,
                    ),
                };
                return Err(Error::new(input.css_span, msg));
            }
        }
    }

    // Build a test CSS string with placeholder class names substituted for {NAME} refs
    let mut test_css = input.css_template.clone();
    for (i, name) in input.name_refs.iter().enumerate() {
        let placeholder = format!("{{{}}}", name);
        let replacement = format!("css-styled-{}", i);
        test_css = test_css.replace(&placeholder, &replacement);
    }

    // Feed the substituted CSS to lightningcss for full syntax validation
    let opts = ParserOptions {
        flags: ParserFlags::NESTING,
        ..Default::default()
    };
    if let Err(err) = StyleSheet::parse(&test_css, opts) {
        let msg = format!("CSS syntax error: {}", err.kind);
        return Err(Error::new(input.css_span, msg));
    }

    // Build the format string and ordered argument list.
    // Each {NAME} occurrence becomes one `{}` slot with a corresponding argument.
    let (format_string, arg_names) = build_format_string(&input.css_template, &input.name_refs);

    let format_args: Vec<TokenStream> = arg_names
        .iter()
        .map(|name| {
            let ident = Ident::new(name, input.css_span);
            quote! { #struct_name::#ident }
        })
        .collect();

    let output = if format_args.is_empty() {
        // No placeholders — the CSS is a static string
        let static_css = input.css_template.clone();
        quote! {
            {
                #static_css
            }
        }
    } else {
        quote! {
            {
                static CSS: ::std::sync::OnceLock<String> = ::std::sync::OnceLock::new();
                CSS.get_or_init(|| {
                    format!(#format_string, #(#format_args),*)
                }).as_str()
            }
        }
    };

    Ok(output)
}

/// Build a format string from a CSS template.
///
/// Replaces each `{NAME}` occurrence (where NAME is a known uppercase ref) with `{}`,
/// and escapes all other `{` and `}` as `{{` and `}}`.
///
/// Returns the format string and the ordered list of name references (one per `{}` slot).
fn build_format_string(template: &str, name_refs: &[String]) -> (String, Vec<String>) {
    let mut result = String::with_capacity(template.len() + 32);
    let mut args = Vec::new();
    let bytes = template.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'{' {
            // Check if this is a {NAME} placeholder
            if let Some(end) = template[i + 1..].find('}') {
                let name = &template[i + 1..i + 1 + end];
                if name_refs.iter().any(|n| n == name) {
                    result.push_str("{}");
                    args.push(name.to_string());
                    i = i + 1 + end + 1;
                    continue;
                }
            }
            // Literal brace — escape it
            result.push_str("{{");
            i += 1;
        } else if bytes[i] == b'}' {
            result.push_str("}}");
            i += 1;
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    (result, args)
}
