# nana-app-icon

[English](README.md) · 简体中文

NanaUI 的默认应用图标，以及 PNG、ICO、ICNS 的编码和 Windows 可执行文件的图标嵌入。

应用没有注册图标时，宿主用这里内置的 Nana 剪影作为窗口和 Dock 图标，各个尺寸都从同一张源图缩放。`resolved_application_icon` 按单窗口图标、已注册图标、默认图标的顺序取用。在 `build.rs` 里调用 `embed_windows` 可以把图标编进 Windows 可执行文件，非 Windows 目标上什么也不做。

注册图标的接口在 [`nana-ui-platform`](../nana-ui-platform)，打包目录布局在 [`nana-packager`](../nana-packager)。自己注册图标的应用可以打开 `nana-ui` 的 `no-default-app-icon`，不链接默认图标。

## 示例

`build.rs`，并在 `[build-dependencies]` 中给 `nana-app-icon` 打开 `embed`：

```rust
fn main() {
    nana_app_icon::embed_windows();
}
```

## Features

- `runtime`（默认开启）：默认窗口图标和图标解析，宿主用。
- `embed`：在 `build.rs` 中嵌入 Windows 图标资源。

## 测试

```bash
cargo test -p nana-app-icon --all-features -- --test-threads=1
```

`runtime` 下的测试会改动进程级的已注册图标，所以要单线程跑。

更多内容见 [打包与分发](../../docs/reference/packaging.md)。
