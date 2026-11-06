# nana-ui-core

[English](README.md) · 简体中文

NanaUI 各层共用的基础类型，位于依赖图最底层。

这里定义样式模型（`LayoutStyle`、语义色、设计 token）、主题（`ThemeDefinition`、`CompiledTheme`、`ThemeRegistry`）、逻辑像素几何、Workspace 区域模型和动效时间线（`MotionTrack`）。Rust 构建器、`view!`、`.vue` 方言和 `css!` 写出的样式最后都落到同一份数据上。组件只声明档位，比如圆角档和动效角色，具体数值由当前主题决定。

本 crate 只有数据和纯计算，不解析 CSS、不排版文字、不持有界面树。CSS 由 [`nana-ui-css`](../nana-ui-css) 在构建期映射过来，文字在 [`nana-text`](../nana-text)，界面树在 [`nana-ui-runtime`](../nana-ui-runtime)。

## 示例

```rust
use nana_ui_core::{SemanticColorRole, ThemeAppearance, builtin_theme};

let theme = builtin_theme(ThemeAppearance::Dark);
let accent = theme.style_model().color(SemanticColorRole::Accent);
```

## Features

默认不开启任何 feature。

- `bundled-font-regular`：嵌入 Noto Sans SC 可变字体（约 4.4 MB），SVG 光栅化和 Canvas2D 文本需要它。
- `bundled-fonts`：把同一字体作为主题的界面字体，供桌面宿主注册，不额外嵌入数据。

## 测试

```bash
cargo test -p nana-ui-core
```

主题里的硬编码数值由 `python3 scripts/audit-theme-hardcoding.py` 检查，其余命令见 [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml)。

更多内容见 [核心合同](../../docs/learn/core-contracts.md) 和 [主题](../../docs/reference/theme.md)。
