# css-styled

Typed, spec-validated CSS generation for Rust component libraries. Define CSS as structs with compile-time validation against W3C specifications.

## Features

- **Compile-time CSS validation** — property names and values checked against W3C specs, with fuzzy suggestions for typos
- **Type-safe CSS generation** — struct fields map to CSS properties, enums for modifier states
- **Scoped styles** — automatic class name scoping for component isolation
- **Static + dynamic CSS** — `css!()` macro for structural styles, struct fields for runtime values
- **CSS custom properties** — first-class `var()` support with compile-time reference checking

## Usage

```rust
use css_styled::{StyledComponent, StyledComponentBase, IntoCss, css};

#[derive(StyledComponent, Clone)]
#[component(scope = "split-handle")]
#[component(class(bar = "split-handle-bar"))]
#[component(modifier(horizontal, vertical))]
#[component(base_css)]
pub struct SplitHandleStyle {
    #[prop(css = "width", on = bar)]
    pub thickness: String,

    #[prop(css = "background", on = bar, pseudo = ":hover")]
    pub hover_color: String,
}

impl StyledComponentBase for SplitHandleStyle {
    fn base_css() -> &'static str {
        css!(SplitHandleStyle, {
            SCOPE {
                display: flex;
                align-items: center;
                flex-shrink: 0;
            }
            SCOPE.HORIZONTAL BAR {
                height: 100%;
            }
            BAR {
                pointer-events: none;
            }
        })
    }
}
```

This generates scoped CSS at runtime:

```css
.split-handle { display: flex; align-items: center; flex-shrink: 0; }
.split-handle.horizontal .split-handle-bar { height: 100%; }
.split-handle-bar { pointer-events: none; }
.split-handle .split-handle-bar { width: 2px; }
.split-handle:hover .split-handle-bar { background: #333; }
```

### Derive attributes

**Struct-level** (`#[component(...)]`):

| Attribute | Description |
|-----------|-------------|
| `scope = "name"` | Base CSS class name (required) |
| `class(alias = "class")` | Child element class aliases |
| `modifier(name, ...)` | State modifier names (generates an enum) |
| `base_css` | Enable `StyledComponentBase` for static CSS |

**Field-level** (`#[prop(...)]`):

| Attribute | Description |
|-----------|-------------|
| `css = "property"` | CSS property this field maps to |
| `on = alias` | Apply to a child element |
| `pseudo = ":state"` | Apply with a pseudo-class or pseudo-element |
| `var = "--name"` | Declare a CSS custom property |
| `skip` | Exclude from CSS generation |

### The `css!()` macro

Write static CSS blocks with compile-time validation. Uppercase identifiers reference generated constants:

```rust
css!(MyComponent, {
    SCOPE {
        display: grid;
    }
    SCOPE.ACTIVE INNER {
        font-weight: bold;
    }
})
```

### Generated API

For a struct `MyStyle` with `modifier(active)` and `class(inner = "my-inner")`:

- `MyStyle::SCOPE` — the scope class name
- `MyStyle::INNER` — child class constant
- `MyStyleModifier::Active` — type-safe modifier variant
- `MyStyle::class(&[MyStyleModifier::Active])` — build class string (`"my-style active"`)
- `style.to_css()` — generate the full CSS string

## Workspace structure

| Crate | Purpose |
|-------|---------|
| `css-styled` | Public API — traits and re-exports |
| `css-styled-derive` | Proc macros (`StyledComponent`, `css!`) |
| `css-spec-data` | W3C spec fetching, caching, and value validation |

## License

MIT
