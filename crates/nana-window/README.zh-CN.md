# nana-window

[English](README.md) · 简体中文

NanaUI 应用的原生窗口层：系统材质、自绘标题栏与无框窗口、实时缩放、工作区全屏、系统菜单栏和文件对话框、系统偏好读取，以及辅助窗口与原生启动画面。

函数在调用期间通过 `impl HasWindowHandle` 借用窗口，不保存句柄，原始句柄不会离开这个 crate 和宿主层。申请系统材质时，`apply_system_material` 返回实际生效的 `MaterialOutcome`，做不到就给出显式的 `MaterialFallback`，不会悄悄换成另一种材质。`WebSurface` 是无头网页画面，帧交给应用的 sink，不进入任何窗口的视图树。

只有 [`nana-ui`](../nana-ui) 的 `hosted` 宿主依赖它，普通控件拿不到这些 API。Window、Surface 和事件循环归宿主，平台中立的 `WindowDescriptor`、`WindowEvent` 等合同在 [`nana-ui-platform`](../nana-ui-platform)。

各平台能力不同：macOS 和 Windows 有系统材质、`Underlay` 与 `NativeSplash`，Linux 上材质回退为不透明，这两项也不支持。完整对照见窗口参考。

## 示例

`examples/` 下是原生路径的探针，在主线程上运行并读回平台实际持有的状态：

```bash
cargo run -p nana-window --example file-dialog-probe
cargo run -p nana-window --example web-surface-probe
```

## 测试

```bash
cargo test -p nana-window --lib
```

单元测试只覆盖路径分类、命令拼写等纯逻辑，原生行为要在目标平台上手动验证。

更多内容见 [窗口参考](../../docs/reference/window.md) 和 [原生窗口、标题栏与系统材质](../../docs/architecture/window.md)。
