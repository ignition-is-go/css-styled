use proc_macro2::{Span, TokenStream, TokenTree, Delimiter};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{braced, Error, Ident, Result, Token};

/// Top-level input: `StructName, { ...css tokens... }`
pub struct CssMacroInput {
    pub struct_name: Ident,
    css_tokens: proc_macro2::TokenStream,
    css_span: Span,
}

impl Parse for CssMacroInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let struct_name: Ident = input.parse()?;
        input.parse::<Token![,]>()?;

        let content;
        let brace = braced!(content in input);

        // Collect all tokens inside the braces as-is
        let css_tokens: proc_macro2::TokenStream = content.parse()?;

        Ok(CssMacroInput {
            struct_name,
            css_tokens,
            css_span: brace.span.join(),
        })
    }
}

/// An UPPERCASE name reference found in the CSS tokens.
struct NameRef {
    ident: Ident,
    placeholder: String,
}

/// A var(--name) reference found in the CSS tokens.
struct VarRef {
    name: String,
    start_span: Span,
    end_span: Span,
}

/// Walk the token stream, find UPPERCASE idents and var() references.
/// Returns: (css_string_with_placeholders, name_refs, var_refs)
fn process_tokens(
    tokens: proc_macro2::TokenStream,
) -> (String, Vec<NameRef>, Vec<VarRef>) {
    let mut css = String::new();
    let mut name_refs: Vec<NameRef> = Vec::new();
    let mut var_refs: Vec<VarRef> = Vec::new();
    let token_vec: Vec<TokenTree> = tokens.into_iter().collect();

    process_token_list(&token_vec, &mut css, &mut name_refs, &mut var_refs, false);

    (css, name_refs, var_refs)
}

fn process_token_list(
    tokens: &[TokenTree],
    css: &mut String,
    name_refs: &mut Vec<NameRef>,
    var_refs: &mut Vec<VarRef>,
    inside_var: bool,
) {
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            TokenTree::Group(group) => {
                let (open, close) = match group.delimiter() {
                    Delimiter::Brace => (" { ", " } "),
                    Delimiter::Parenthesis => ("(", ")"),
                    Delimiter::Bracket => ("[", "]"),
                    Delimiter::None => ("", ""),
                };

                // Check if this is a var(...) call — previous token was "var"
                let is_var_call = css.trim_end().ends_with("var");

                css.push_str(open);
                let inner: Vec<TokenTree> = group.stream().into_iter().collect();
                if is_var_call && group.delimiter() == Delimiter::Parenthesis {
                    // Inside var() — look for --name pattern
                    process_var_args(&inner, css, var_refs);
                } else {
                    process_token_list(&inner, css, name_refs, var_refs, inside_var);
                }
                css.push_str(close);
            }
            TokenTree::Ident(ident) => {
                let s = ident.to_string();

                // Check if this is an UPPERCASE name (selector reference)
                if is_uppercase_name(&s) {
                    // Check if we already have this name
                    let existing = name_refs.iter().find(|r| r.ident == *ident);
                    let placeholder = if let Some(existing) = existing {
                        existing.placeholder.clone()
                    } else {
                        let p = format!("css-s-{}", name_refs.len());
                        name_refs.push(NameRef {
                            ident: ident.clone(),
                            placeholder: p.clone(),
                        });
                        p
                    };
                    // Add space before if needed (e.g., descendant combinator)
                    if needs_space_before(css, &placeholder) {
                        css.push(' ');
                    }
                    css.push_str(&placeholder);
                } else {
                    // Regular ident — check if we need a space before it
                    if needs_space_before(css, &s) {
                        css.push(' ');
                    }
                    css.push_str(&s);
                }
            }
            TokenTree::Punct(punct) => {
                let ch = punct.as_char();
                // Don't add space before certain puncts
                match ch {
                    ';' | ',' | '.' | '%' | ')' | ']' | '}' => {
                        css.push(ch);
                    }
                    ':' => {
                        css.push(':');
                        // In pseudo-selectors, the next token attaches directly.
                        // In declarations, there's a space. We peek at the next token
                        // to decide: if next is ':' (double colon pseudo-element) or
                        // an ident that's a pseudo name, no space. Otherwise space.
                        // Simpler: just don't add space here. The needs_space_before
                        // on the NEXT token handles it (colon returns false).
                    }
                    '-' => {
                        // Hyphen: could be part of a hyphenated ident or a negative number
                        // Don't add space if previous char is a letter (hyphenated ident)
                        // or if at start / after space (negative value)
                        if css.ends_with(|c: char| c.is_alphanumeric() || c == '-') {
                            css.push('-');
                        } else {
                            css.push('-');
                        }
                    }
                    '#' => {
                        // Hash for colors: #fff
                        css.push('#');
                    }
                    '@' => {
                        if needs_space_before(css, "@") {
                            css.push(' ');
                        }
                        css.push('@');
                    }
                    '&' => {
                        if needs_space_before(css, "&") {
                            css.push(' ');
                        }
                        css.push('&');
                    }
                    _ => {
                        css.push(ch);
                    }
                }
            }
            TokenTree::Literal(lit) => {
                let s = lit.to_string();
                if needs_space_before(css, &s) {
                    css.push(' ');
                }
                css.push_str(&s);
            }
        }
        i += 1;
    }
}

/// Process tokens inside a var() call, extracting the --name reference.
fn process_var_args(
    tokens: &[TokenTree],
    css: &mut String,
    var_refs: &mut Vec<VarRef>,
) {
    // Look for --name pattern: '-' '-' ident ('-' ident)*
    let mut i = 0;
    let mut first = true;

    while i < tokens.len() {
        match &tokens[i] {
            TokenTree::Punct(p) if p.as_char() == '-' => {
                // Check for -- prefix
                if i + 1 < tokens.len() {
                    if let TokenTree::Punct(p2) = &tokens[i + 1] {
                        if p2.as_char() == '-' && i + 2 < tokens.len() {
                            if let TokenTree::Ident(name_start) = &tokens[i + 2] {
                                let start_span = p.span();
                                let mut name = name_start.to_string();
                                let mut end_span = name_start.span();
                                let mut j = i + 3;

                                // Consume hyphenated parts
                                while j + 1 < tokens.len() {
                                    if let TokenTree::Punct(dash) = &tokens[j] {
                                        if dash.as_char() == '-' {
                                            if let TokenTree::Ident(part) = &tokens[j + 1] {
                                                name.push('-');
                                                name.push_str(&part.to_string());
                                                end_span = part.span();
                                                j += 2;
                                                continue;
                                            }
                                        }
                                    }
                                    break;
                                }

                                let full_var = format!("--{}", name);
                                var_refs.push(VarRef {
                                    name: full_var.clone(),
                                    start_span,
                                    end_span,
                                });
                                css.push_str(&full_var);
                                i = j;
                                first = false;
                                continue;
                            }
                        }
                    }
                }
                // Regular minus
                css.push('-');
            }
            TokenTree::Punct(p) if p.as_char() == ',' => {
                css.push(',');
                css.push(' ');
            }
            TokenTree::Ident(id) => {
                if !first && needs_space_before(css, &id.to_string()) {
                    css.push(' ');
                }
                css.push_str(&id.to_string());
            }
            TokenTree::Literal(lit) => {
                if needs_space_before(css, &lit.to_string()) {
                    css.push(' ');
                }
                css.push_str(&lit.to_string());
            }
            _ => {
                css.push_str(&tokens[i].to_string());
            }
        }
        first = false;
        i += 1;
    }
}

fn is_uppercase_name(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_uppercase() || c == '_')
}

fn needs_space_before(css: &str, next: &str) -> bool {
    if css.is_empty() {
        return false;
    }
    let last = css.chars().last().unwrap();
    let first_next = next.chars().next().unwrap_or(' ');

    // Never add space after these — they attach to the next token
    if last == '(' || last == '[' || last == '.' || last == '#' || last == ':' || last == '-' {
        return false;
    }

    // Never add space before these — they attach to the previous token
    if first_next == ')' || first_next == ']' || first_next == '.' || first_next == ';' || first_next == ',' || first_next == '%' || first_next == ':' {
        return false;
    }

    // For everything else, add a space if both sides are "wordy" tokens
    true
}

/// Validate and generate the output code.
pub fn expand(input: CssMacroInput) -> Result<TokenStream> {
    let struct_name = &input.struct_name;
    let struct_str = struct_name.to_string();

    // Walk tokens: substitute UPPERCASE names, extract var refs
    let (placeholder_css, name_refs, var_refs) = process_tokens(input.css_tokens);

    // Validate var() references against the proc-macro registry
    for var_ref in &var_refs {
        if let Some(known_vars) = crate::lookup_vars(&struct_str) {
            if !known_vars.contains(var_ref.name.as_str()) {
                let start = proc_macro2::Ident::new("_", var_ref.start_span);
                let end = proc_macro2::Ident::new("_", var_ref.end_span);
                let spanned = quote!(#start #end);
                let msg = match crate::lookup_theme(&struct_str) {
                    Some(theme) => format!(
                        "unknown CSS variable `{}`; not declared with #[prop(var = \"{}\")] on `{}` or in theme `{}`",
                        var_ref.name, var_ref.name, struct_str, theme,
                    ),
                    None => format!(
                        "unknown CSS variable `{}`; not declared with #[prop(var = \"{}\")] on `{}` (no theme set)",
                        var_ref.name, var_ref.name, struct_str,
                    ),
                };
                return Err(Error::new_spanned(spanned, msg));
            }
        }
    }

    // Validate with lightningcss and get properly formatted CSS
    let formatted_css = validate_and_format_css(&placeholder_css, input.css_span)?;

    // Build the runtime format string: replace each placeholder with {}
    // and track which struct constant each slot maps to
    let mut format_string = formatted_css;
    let mut format_args: Vec<TokenStream> = Vec::new();

    // Deduplicate name refs — same name gets same constant but may appear multiple times
    // We need to replace all occurrences and add one arg per occurrence
    // Sort by longest placeholder first to avoid partial replacement
    let mut replacements: Vec<(String, &Ident)> = Vec::new();
    for nr in &name_refs {
        replacements.push((nr.placeholder.clone(), &nr.ident));
    }

    // Build format string by finding and replacing placeholders
    let mut result_format = String::new();
    let mut remaining = format_string.as_str();

    loop {
        // Find the next placeholder occurrence
        let mut earliest: Option<(usize, &str, &Ident)> = None;
        for (placeholder, ident) in &replacements {
            if let Some(pos) = remaining.find(placeholder.as_str()) {
                if earliest.is_none() || pos < earliest.unwrap().0 {
                    earliest = Some((pos, placeholder.as_str(), ident));
                }
            }
        }

        match earliest {
            Some((pos, placeholder, ident)) => {
                // Escape literal braces in the CSS before this placeholder
                let before = &remaining[..pos];
                result_format.push_str(&before.replace('{', "{{").replace('}', "}}"));
                // Insert format slot — add '.' prefix only if not already preceded by one
                if !result_format.ends_with('.') {
                    result_format.push('.');
                }
                result_format.push_str("{}");
                format_args.push(quote! { #struct_name::#ident });
                remaining = &remaining[pos + placeholder.len()..];
            }
            None => {
                // No more placeholders — escape remaining CSS
                result_format.push_str(&remaining.replace('{', "{{").replace('}', "}}"));
                break;
            }
        }
    }

    Ok(quote! {
        {
            static CSS: ::std::sync::OnceLock<String> = ::std::sync::OnceLock::new();
            CSS.get_or_init(|| {
                format!(#result_format, #(#format_args),*)
            }).as_str()
        }
    })
}

/// Validate CSS with lightningcss and return the re-formatted output.
/// This normalizes spacing and catches any syntax errors.
fn validate_and_format_css(css: &str, span: Span) -> Result<String> {
    use lightningcss::stylesheet::{ParserOptions, ParserFlags, StyleSheet, PrinterOptions};
    use lightningcss::printer::PrinterOptions as _;

    let opts = ParserOptions {
        flags: ParserFlags::NESTING,
        ..Default::default()
    };

    let stylesheet = StyleSheet::parse(css, opts).map_err(|err| {
        Error::new(span, format!("CSS syntax error: {}", err.kind))
    })?;

    let printed = stylesheet.to_css(PrinterOptions::default()).map_err(|err| {
        Error::new(span, format!("CSS printing error: {}", err))
    })?;

    Ok(printed.code)
}
