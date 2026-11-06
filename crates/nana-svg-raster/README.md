# nana-svg-raster

在 CPU 上把 SVG 栅格化成 RGBA8 位图，基于 resvg 和 tiny-skia。

[`nana-ui`](../nana-ui) 用它生成图标图集、解码 URL 图片里的 SVG，以及绘制 Canvas Image。几个 `rasterize_*` 函数按用途区分尺寸和 alpha 形式：`rasterize_white_mask` 把图标等比放进正方形并输出白色预乘遮罩，`rasterize_document` 按文档自身尺寸输出直通 alpha，`rasterize_document_capped` 再限制长边。外部 `href` 不会被读取，只接受 `data:` 内嵌资源；解析失败或尺寸无效时返回 `None`。

本 crate 不缓存结果，也不上传 GPU，这些由 `nana-ui` 负责。

## 示例

```rust
let raster = nana_svg_raster::rasterize_document(
    br##"<svg xmlns="http://www.w3.org/2000/svg" width="3" height="2"><rect width="3" height="2" fill="#60a5fa"/></svg>"##,
)
.expect("doc");
assert_eq!((raster.width, raster.height), (3, 2));
```

## Features

- `system-fonts`：让 SVG `<text>` 使用系统字体，转发到 `resvg/system-fonts`。

## 测试

```bash
cargo test -p nana-svg-raster
```

更多内容见 [支撑 crate](../../docs/learn/supporting-crates.md) 和 [第三方代码与许可证](../../docs/reference/third-party.md)。
