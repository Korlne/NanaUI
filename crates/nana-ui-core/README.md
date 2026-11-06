# nana-ui-core

English · [简体中文](README.zh-CN.md)

Foundation types shared by every NanaUI layer, at the bottom of the dependency graph.

This crate defines the style model (`LayoutStyle`, semantic colors, design tokens), themes (`ThemeDefinition`, `CompiledTheme`, `ThemeRegistry`), logical-pixel geometry, the Workspace region model, and motion timelines (`MotionTrack`). Styles written with Rust builders, `view!`, the `.vue` dialect, and `css!` all end up as the same data. Components declare only tiers, such as a radius tier or a motion role; the concrete values come from the current theme.

This crate holds only data and pure computation: it does not parse CSS, lay out text, or own the UI tree. CSS is mapped in at build time by [`nana-ui-css`](../nana-ui-css), text lives in [`nana-text`](../nana-text), and the UI tree lives in [`nana-ui-runtime`](../nana-ui-runtime).

## Example

```rust
use nana_ui_core::{SemanticColorRole, ThemeAppearance, builtin_theme};

let theme = builtin_theme(ThemeAppearance::Dark);
let accent = theme.style_model().color(SemanticColorRole::Accent);
```

## Features

No features are on by default.

- `bundled-font-regular`: embeds the Noto Sans SC variable font (about 4.4 MB), needed by SVG rasterization and Canvas2D text.
- `bundled-fonts`: uses the same font as the theme's UI font, for desktop hosts to register; embeds no extra data.

## Tests

```bash
cargo test -p nana-ui-core
```

Hard-coded values in themes are checked by `python3 scripts/audit-theme-hardcoding.py`; other commands are in [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml).

See also [Core contracts](../../docs/en/learn/core-contracts.md) and [Theme](../../docs/en/reference/theme.md).
