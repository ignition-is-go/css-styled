use proc_macro2::Span;
use syn::{DeriveInput, Error, Ident, LitStr, Result, Field};

/// Parsed struct-level configuration from `#[component(...)]` attributes.
#[derive(Debug, Default)]
pub struct ComponentConfig {
    pub scope: Option<LitStr>,
    /// Named child-class aliases: (alias_ident, class_string_literal)
    pub classes: Vec<(Ident, LitStr)>,
    /// Modifier variant names
    pub modifiers: Vec<Ident>,
    /// If true, user will provide their own StyledComponentBase impl
    pub custom_base_css: bool,
    /// Optional theme type path, e.g. `AppTheme`
    pub theme: Option<syn::Path>,
}

/// Parsed field-level configuration from `#[prop(...)]` attributes.
#[derive(Debug)]
pub enum PropConfig {
    /// The field is skipped from CSS generation.
    Skip,
    /// The field maps to a CSS property.
    Mapped {
        css: LitStr,
        on: Option<Ident>,
        pseudo: Option<LitStr>,
        /// Default value source
        default: Option<PropDefault>,
    },
    /// The field declares a CSS custom property (variable).
    Variable {
        var: LitStr,
        /// Default value source
        default: Option<PropDefault>,
    },
}

/// A default value for a prop field.
#[derive(Debug, Clone)]
pub enum PropDefault {
    /// `default = theme.field_name` — resolves to `var(--theme-var-name)`
    ThemeVar(Ident),
    /// `default = "literal"` — a literal default value
    Literal(LitStr),
}

/// A fully-parsed field with its config.
#[derive(Debug)]
pub struct ParsedField {
    pub ident: Ident,
    pub config: PropConfig,
}

/// Parse all `#[component(...)]` attributes from the struct.
pub fn parse_component_config(input: &DeriveInput) -> Result<ComponentConfig> {
    let mut config = ComponentConfig::default();

    for attr in &input.attrs {
        if !attr.path().is_ident("component") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("scope") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                config.scope = Some(lit);
                Ok(())
            } else if meta.path.is_ident("class") {
                let content;
                syn::parenthesized!(content in meta.input);
                // Parse comma-separated: alias = "class-name"
                loop {
                    if content.is_empty() {
                        break;
                    }
                    let alias: Ident = content.parse()?;
                    let _eq: syn::Token![=] = content.parse()?;
                    let class_str: LitStr = content.parse()?;
                    config.classes.push((alias, class_str));
                    if content.is_empty() {
                        break;
                    }
                    let _comma: syn::Token![,] = content.parse()?;
                }
                Ok(())
            } else if meta.path.is_ident("base_css") {
                config.custom_base_css = true;
                Ok(())
            } else if meta.path.is_ident("modifier") {
                let content;
                syn::parenthesized!(content in meta.input);
                loop {
                    if content.is_empty() {
                        break;
                    }
                    let name: Ident = content.parse()?;
                    config.modifiers.push(name);
                    if content.is_empty() {
                        break;
                    }
                    let _comma: syn::Token![,] = content.parse()?;
                }
                Ok(())
            } else if meta.path.is_ident("theme") {
                let value = meta.value()?;
                let path: syn::Path = value.parse()?;
                config.theme = Some(path);
                Ok(())
            } else {
                Err(meta.error("unknown component attribute"))
            }
        })?;
    }

    if config.scope.is_none() {
        return Err(Error::new(
            Span::call_site(),
            "missing required `#[component(scope = \"...\")]` attribute",
        ));
    }

    Ok(config)
}

/// Parse the `#[prop(...)]` attribute from a single field.
pub fn parse_prop_config(field: &Field) -> Result<Option<PropConfig>> {
    for attr in &field.attrs {
        if !attr.path().is_ident("prop") {
            continue;
        }

        // Check for #[prop(skip)]
        let mut is_skip = false;
        let mut css: Option<LitStr> = None;
        let mut on: Option<Ident> = None;
        let mut pseudo: Option<LitStr> = None;
        let mut var: Option<LitStr> = None;
        let mut prop_default: Option<PropDefault> = None;

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                is_skip = true;
                Ok(())
            } else if meta.path.is_ident("css") {
                let value = meta.value()?;
                css = Some(value.parse()?);
                Ok(())
            } else if meta.path.is_ident("on") {
                let value = meta.value()?;
                on = Some(value.parse()?);
                Ok(())
            } else if meta.path.is_ident("pseudo") {
                let value = meta.value()?;
                pseudo = Some(value.parse()?);
                Ok(())
            } else if meta.path.is_ident("var") {
                let value = meta.value()?;
                var = Some(value.parse()?);
                Ok(())
            } else if meta.path.is_ident("default") {
                // Parse `default = theme.field_name` or `default = "literal"`
                let value = meta.value()?;
                if value.peek(LitStr) {
                    let lit: LitStr = value.parse()?;
                    prop_default = Some(PropDefault::Literal(lit));
                } else {
                    let path: Ident = value.parse()?;
                    if path != "theme" {
                        return Err(Error::new_spanned(&path, "expected `theme` or a string literal; use `default = theme.field_name` or `default = \"value\"`"));
                    }
                    let _dot: syn::Token![.] = value.parse()?;
                    let field_name: Ident = value.parse()?;
                    prop_default = Some(PropDefault::ThemeVar(field_name));
                }
                Ok(())
            } else {
                Err(meta.error("unknown prop attribute; expected `css`, `on`, `pseudo`, `var`, `default`, or `skip`"))
            }
        })?;

        if is_skip {
            return Ok(Some(PropConfig::Skip));
        }

        if let Some(var_lit) = var {
            let var_name = var_lit.value();
            if !var_name.starts_with("--") {
                return Err(Error::new_spanned(
                    &var_lit,
                    "CSS custom property name must start with `--`",
                ));
            }
            return Ok(Some(PropConfig::Variable { var: var_lit, default: prop_default }));
        }

        if css.is_none() {
            return Err(Error::new_spanned(
                attr,
                "`#[prop(...)]` requires `css = \"property-name\"` or `var = \"--name\"`",
            ));
        }

        return Ok(Some(PropConfig::Mapped { css: css.unwrap(), on, pseudo, default: prop_default }));
    }

    Ok(None)
}
