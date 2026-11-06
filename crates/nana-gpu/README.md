# nana-gpu

NanaUI 与 GPU 之间的接口层，目前只有 WGPU 一个实现。

`GpuContext` 代表应用唯一的 GPU 设备，由宿主创建，供 NanaUI 的绘制器和应用自己的渲染代码共用。每一帧从 `GpuContext::begin_frame` 拿到一个 `FrameContext`，录制完调用 `submit()`；不提交就丢弃，这一帧也就作废了。纹理会记住自己是在哪一代设备上创建的，设备丢失并重建之后，旧纹理会被拒绝，而不是悄悄画错。

这个 crate 不创建窗口，不调度帧，也不负责绘制界面。那些工作在 [`nana-ui`](../nana-ui) 里。

## 示例

```rust
use nana_gpu::{GpuContext, GpuTextureDescriptor, GpuTextureFormat, GpuTextureUsages};

fn record(gpu: &GpuContext, usage: GpuTextureUsages) {
    let texture = gpu
        .create_texture(&GpuTextureDescriptor {
            label: Some("example"),
            width: 4,
            height: 2,
            format: GpuTextureFormat::RGBA8_UNORM,
            usage,
        })
        .expect("4x2 rgba8 texture");
    assert_eq!(texture.generation(), gpu.generation());

    let frame = gpu.begin_frame("example");
    frame.submit();
}
```

## Features

- `wgpu-interop`：拿到 `GpuContext` 背后的 `wgpu` 设备、编码器和纹理。自己录制渲染管线、或者要接入已有 WGPU 设备时才需要。
- `native-export`（仅 Windows）：把纹理导出为 DX12 共享纹理，交给其他 D3D 设备读取。
- `native-import`（仅 Windows）：接收外部 GPU 生产者写入的 DX12 共享纹理。

## 测试

```bash
cargo test -p nana-gpu
```

测试需要一个可用的 GPU 适配器。设备丢失和 Windows 共享纹理的测试命令见 [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml)。

更多内容见 [GPU 集成](../../docs/reference/gpu.md) 和 [GPU 与帧生命周期](../../docs/learn/gpu-frame.md)。
