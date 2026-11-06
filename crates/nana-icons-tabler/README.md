# nana-icons-tabler

完整的 [Tabler Icons](https://github.com/tabler/tabler-icons) outline 图标目录，每个图标是一个 `nana_ui_core::icon::Icon` 常量。

常量名是图标 id 换成大写、`-` 换成 `_`，例如 `HOME`、`TRASH`、`ZOOM_IN`。每个常量指向独立的静态数据，没用到的图标会被链接器剔除。只收录 outline 风格。图标的栅格化在 `nana-ui` 里完成，壳层自己用的图标在 `nana-ui-core` 的内置目录中。

在应用里打开 `nana-ui` 的 `icons-tabler` feature 后按名引用：

```rust
use nana_ui::icons_tabler::{HOME, USER};
```

## 生成

`src/lib.rs` 是生成的，不要手改：

```bash
python3 scripts/generate_tabler_catalog.py
```

Tabler 的版本写在脚本的 `TABLER_VERSION` 里。

## 测试

本 crate 被根 `Cargo.toml` 的 `[workspace] exclude` 排除，有自己的 `Cargo.lock`，升级依赖时要单独刷新，测试也要指定清单：

```bash
cargo test --manifest-path crates/nana-icons-tabler/Cargo.toml
```

## 许可证

图标数据来自 Tabler Icons，以 MIT 许可证发布，本 crate 相应声明 `license = "MIT"`。

更多内容见 [IconGlyph 组件](../../docs/components/icon-glyph.md) 和 [第三方代码与许可证](../../docs/reference/third-party.md)。
