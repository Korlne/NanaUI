# nana-frame-exchange

English · [简体中文](README.zh-CN.md)

A bounded exchange that hands frames drawn by a render thread to a UI window for display, built on the host's existing `GpuContext`.

The producer calls `FrameExchange::copy_from` to copy a completed frame on the GPU into a small texture pool; the consumer takes the latest completed `FrameLease` from a `FrameInbox` and samples it. Neither side waits for the other: when the pool is full the frame is simply dropped (`CopyOutcome::PoolFull`), and a lease returns its slot only after the submission that samples it completes. Pixels are never read back to the CPU. `set_epoch` immediately hides frames from older epochs, and frames can carry a typed application payload; payload changes do not affect token identity or epoch.

This crate does not own the device and is not responsible for submission or color conversion. On the window side, frames are attached to the UI with `FrameBinding` in [`nana-ui`](../nana-ui); cross-process texture sharing is `native-export` in [`nana-gpu`](../nana-gpu).

## Example

```rust
use std::sync::Arc;
use nana_frame_exchange::{CopyOutcome, DEFAULT_CAPACITY, FrameExchange, GpuContext, GpuTexture};

fn produce(gpu: &GpuContext, rendered: &GpuTexture) {
    let mut exchange = FrameExchange::new(gpu, DEFAULT_CAPACITY, 7u64, Arc::new(|| {}));
    let inbox = exchange.inbox(); // hand to the consumer thread

    if let CopyOutcome::PoolFull = exchange.copy_from(rendered, 7) {
        return; // drop this frame without waiting
    }
    if exchange.poll() {
        let lease = inbox.latest().unwrap();
        let _ = (lease.token(), lease.size());
    }
}
```

## Features

- `wgpu-interop`: enables `FrameExchange::copy_from_wgpu*`, for producers that render into raw WGPU textures.

## Tests

```bash
cargo test -p nana-frame-exchange
```

The tests need a working GPU adapter.

See also [GPU integration](../../docs/en/reference/gpu.md) and [GPU and the frame lifecycle](../../docs/en/learn/gpu-frame.md).
