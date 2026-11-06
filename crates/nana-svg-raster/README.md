# nana-svg-raster

English · [简体中文](README.zh-CN.md)

Rasterizes SVG into RGBA8 bitmaps on the CPU, based on resvg and tiny-skia.

[`nana-ui`](../nana-ui) uses it to build the icon atlas, decode SVG in URL images, and draw Canvas Image. The `rasterize_*` functions differ in sizing and alpha form by purpose: `rasterize_white_mask` fits an icon proportionally into a square and outputs a white premultiplied mask, `rasterize_document` outputs straight alpha at the document's own size, and `rasterize_document_capped` additionally limits the long side. External `href`s are never read; only `data:` embedded resources are accepted. It returns `None` when parsing fails or the size is invalid.

This crate does not cache results or upload to the GPU; `nana-ui` handles that.

## Example

```rust
let raster = nana_svg_raster::rasterize_document(
    br##"<svg xmlns="http://www.w3.org/2000/svg" width="3" height="2"><rect width="3" height="2" fill="#60a5fa"/></svg>"##,
)
.expect("doc");
assert_eq!((raster.width, raster.height), (3, 2));
```

## Features

- `system-fonts`: lets SVG `<text>` use system fonts; forwards to `resvg/system-fonts`.

## Tests

```bash
cargo test -p nana-svg-raster
```

See also [Supporting crates](../../docs/en/learn/supporting-crates.md) and [Third-party code and licenses](../../docs/en/reference/third-party.md).
