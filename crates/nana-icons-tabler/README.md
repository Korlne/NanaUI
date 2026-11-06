# nana-icons-tabler

English · [简体中文](README.zh-CN.md)

The complete [Tabler Icons](https://github.com/tabler/tabler-icons) outline icon catalog, with each icon as a `nana_ui_core::icon::Icon` constant.

Constant names are the icon id uppercased with `-` replaced by `_`, for example `HOME`, `TRASH`, `ZOOM_IN`. Each constant points to its own static data, so unused icons are stripped by the linker. Only the outline style is included. Icon rasterization happens in `nana-ui`, and the icons the shell itself uses are in the built-in catalog in `nana-ui-core`.

In an application, turn on `nana-ui`'s `icons-tabler` feature and refer to icons by name:

```rust
use nana_ui::icons_tabler::{HOME, USER};
```

## Generation

`src/lib.rs` is generated; do not edit it by hand:

```bash
python3 scripts/generate_tabler_catalog.py
```

The Tabler version is set in the script's `TABLER_VERSION`.

## Tests

This crate is excluded by `[workspace] exclude` in the root `Cargo.toml` and has its own `Cargo.lock`, which must be refreshed separately when upgrading dependencies. Tests must also name the manifest:

```bash
cargo test --manifest-path crates/nana-icons-tabler/Cargo.toml
```

## License

The icon data comes from Tabler Icons and is released under the MIT license; this crate accordingly declares `license = "MIT"`.

See also [IconGlyph component](../../docs/en/components/icon-glyph.md) and [Third-party code and licenses](../../docs/en/reference/third-party.md).
