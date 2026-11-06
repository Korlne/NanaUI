# nana-ui-css

English · [简体中文](README.zh-CN.md)

NanaUI's CSS engine. It parses the supported CSS subset, runs the cascade, and maps the result onto `nana_ui_core::LayoutStyle`.

It runs only at build time. `<style>` in `view!`, `stylesheet!`, `css!`, and `<style>` in `.vue` files are all compiled through it; at run time the program receives already computed style data and never parses CSS text again. `@container` is compiled into range data evaluated at run time; this crate never measures containers itself. Stylesheet size, `@import` depth, and nesting depth all have hard limits, and anything beyond them is skipped or rejected.

Applications generally do not call it directly. Box model solving and whether an `@container` query holds are decided by the layout engine in `nana-ui-runtime`, and template expansion is in [`nana-ui-view-codegen`](../nana-ui-view-codegen).

## Tests

```bash
cargo test -p nana-ui-css
cargo run -p nana-css-parity -- compare
```

The second command compares layout results item by item against browser box model fixtures. Other checks are in [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml).

See also [CSS engine](../../docs/en/learn/css-engine.md) and [Layout (CSS subset)](../../docs/en/reference/layout.md).
