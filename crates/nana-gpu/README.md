# nana-gpu

English · [简体中文](README.zh-CN.md)

The interface layer between NanaUI and the GPU. WGPU is currently the only implementation.

`GpuContext` represents the application's single GPU device. The host creates it, and NanaUI's painter and the application's own rendering code share it. Each frame gets a `FrameContext` from `GpuContext::begin_frame` and calls `submit()` after recording; if it is not submitted it is discarded, and that frame is void. Textures remember which device generation they were created on, so after device loss and rebuild, old textures are rejected instead of silently drawing the wrong thing.

This crate does not create windows, schedule frames, or paint the UI. That work lives in [`nana-ui`](../nana-ui).

## Example

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

- `wgpu-interop`: access the `wgpu` device, encoder, and textures behind `GpuContext`. Needed only when you record your own render pipelines or integrate an existing WGPU device.
- `native-export` (Windows only): export a texture as a DX12 shared texture for other D3D devices to read.
- `native-import` (Windows only): receive DX12 shared textures written by an external GPU producer.

## Tests

```bash
cargo test -p nana-gpu
```

The tests need a working GPU adapter. Test commands for device loss and Windows shared textures are in [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml).

See also [GPU integration](../../docs/en/reference/gpu.md) and [GPU and the frame lifecycle](../../docs/en/learn/gpu-frame.md).
