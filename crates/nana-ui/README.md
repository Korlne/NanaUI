# nana-ui

NanaUI 的门面 crate，也是应用直接依赖的那一个。

它提供应用入口（`NanaApplication`、`RuntimeApplication`、`RuntimeProgram`）、基于 winit 和 `nana-window` 的桌面宿主，以及把 `UiScene` 画进宿主 WGPU 帧的 `SceneWgpuPainter`。控件、视图和 `AppContext` 经 `nana_ui::runtime` 重新导出，`nana-gpu` 的 `GpuContext`、`FrameContext` 等合同类型也从这里拿。宿主纹理和 `CustomRenderNode` 让 GPU 画面作为普通节点进入界面树。

保留树、组件和布局在 [`nana-ui-runtime`](../nana-ui-runtime)，场景在 [`nana-ui-scene`](../nana-ui-scene)，本 crate 只负责把它们接到窗口和 GPU 上。

## 示例

```rust
fn main() -> Result<(), nana_ui::HostedRunError> {
    let identity = ApplicationIdentity::new(
        "dev.nanaui.counter",
        "NanaUI Counter",
        env!("CARGO_PKG_VERSION"),
    );
    NanaApplication::builder(identity)
        .diagnostics(DiagnosticsConfig::default())
        .run::<RuntimeApplication<Counter>>(
            WindowDescriptor::new("NanaUI Counter").initial_size(480.0, 320.0),
        )
}
```

`Counter` 实现了 `ApplicationState`，完整代码见 `examples/application-counter.rs`：

```bash
cargo run -p nana-ui --example application-counter --features hosted,bundled-fonts
```

## Features

默认开启 `wgpu-backends`、`icu` 和 `snippet-transforms`，桌面应用通常再加 `hosted`，或直接用 `full`。用 `default-features = false` 的应用要自己点名用到的那几项。

- `hosted`：桌面宿主，包括窗口、Surface、帧调度和平台无障碍适配，隐含 `gpu`。
- `gpu`：`SceneWgpuPainter`、宿主纹理、帧交换和无窗口输出。
- `wgpu-backends`（默认）：打开平台上的全部 WGPU 后端，只在 `gpu` 开启时生效。
- `icu`（默认）：用 ICU4X 做数字、日期和复数格式化。
- `snippet-transforms`（默认）：snippet 里的正则变换（`${1/(.*)/${1:/upcase}/}`）。关闭后 snippet 照常展开和联动，变换原样返回，正则引擎（约 0.7 MiB）不进二进制。
- `full`：`bundled-fonts`、`components`、`hosted`、`syntax-highlighting`、`icu` 和 `snippet-transforms`。
- `bundled-fonts`：注册内置的 Noto Sans SC 并设为界面默认字体。
- `components`：打开 `calendar`、`charts`、`controls`、`graph-canvas`、`image-viewer`、`rich-text` 六个控件族，也可以单独开启。
- `syntax-highlighting`：代码块语法高亮。
- `view-macro`：`view!`、`css!` 和 `stylesheet!` 宏。
- `icons-tabler`：完整的 Tabler 图标目录（`nana_ui::icons_tabler`）。
- `packaged-resources`：挂载 `.nrpack` 资源包并响应 `NANA_PACKAGE_VALIDATE` 自检。
- `wgpu-interop`：访问 `GpuContext` 背后的 WGPU 对象，自己录制管线时才需要。
- `native-export`（仅 Windows）：把窗口输出导出为 DX12 共享纹理。
- `accesskit-tree`：只产出 AccessKit `TreeUpdate`，给自己接平台适配器的宿主用。
- `reactive-trace`：记录响应式更新的因果链，用于调试。
- `no-default-app-icon`：不链接默认应用图标，只应由应用二进制打开。

## 测试

```bash
cargo test -p nana-ui --lib --features hosted,gpu
```

不带 `gpu` / `hosted` 时，这两个 feature 后面的模块不会编译，测试也不会运行。GPU 相关测试需要可用的适配器，其余命令见 [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml)。

更多内容见 [快速开始](../../docs/guide/quick-start.md)、[应用 API](../../docs/reference/application-api.md) 和 [实时画面与 GPU 集成](../../docs/reference/gpu.md)。
