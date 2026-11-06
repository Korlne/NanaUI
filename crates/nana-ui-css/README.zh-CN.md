# nana-ui-css

[English](README.md) · 简体中文

NanaUI 的 CSS 引擎，负责解析受支持的 CSS 子集、做级联，并把结果映射到 `nana_ui_core::LayoutStyle`。

它只在构建期运行。`view!` 的 `<style>`、`stylesheet!`、`css!` 和 `.vue` 文件的 `<style>` 都经由它编译，运行时拿到的是已经算好的样式数据，不再解析 CSS 文本。`@container` 被编译成运行时求值的区间数据，本 crate 自己从不测量容器。样式表体积、`@import` 深度和嵌套深度都有硬上限，超出的部分跳过或拒绝。

应用一般不直接调用它。盒模型求解和 `@container` 是否成立由 `nana-ui-runtime` 的布局引擎决定，模板展开在 [`nana-ui-view-codegen`](../nana-ui-view-codegen)。

## 测试

```bash
cargo test -p nana-ui-css
cargo run -p nana-css-parity -- compare
```

第二条把布局结果与浏览器盒模型的夹具逐项比对。其余检查见 [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml)。

更多内容见 [CSS 引擎](../../docs/learn/css-engine.md) 和 [布局（CSS 子集）](../../docs/reference/layout.md)。
