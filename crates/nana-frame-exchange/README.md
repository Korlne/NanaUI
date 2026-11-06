# nana-frame-exchange

把渲染线程画好的帧交给 UI 窗口显示的有界交换，建在宿主已有的 `GpuContext` 上。

生产者调用 `FrameExchange::copy_from`，把完成的帧在 GPU 上复制进一个小纹理池；消费者从 `FrameInbox` 取最新一份完成的 `FrameLease` 去采样。双方互不等待：池满时这一帧直接丢弃（`CopyOutcome::PoolFull`），租约在采样它的那次提交完成后才归还槽位。像素从不读回 CPU。`set_epoch` 会立刻隐藏旧 epoch 的帧，帧还可以带上类型化的应用载荷，载荷变化不影响 token 身份或 epoch。

这个 crate 不拥有设备，也不负责提交和颜色转换。窗口侧把帧挂进界面用的是 [`nana-ui`](../nana-ui) 的 `FrameBinding`，跨进程共享纹理在 [`nana-gpu`](../nana-gpu) 的 `native-export`。

## 示例

```rust
use std::sync::Arc;
use nana_frame_exchange::{CopyOutcome, DEFAULT_CAPACITY, FrameExchange, GpuContext, GpuTexture};

fn produce(gpu: &GpuContext, rendered: &GpuTexture) {
    let mut exchange = FrameExchange::new(gpu, DEFAULT_CAPACITY, 7u64, Arc::new(|| {}));
    let inbox = exchange.inbox(); // 交给消费者线程

    if let CopyOutcome::PoolFull = exchange.copy_from(rendered, 7) {
        return; // 丢弃这一帧，不等待
    }
    if exchange.poll() {
        let lease = inbox.latest().unwrap();
        let _ = (lease.token(), lease.size());
    }
}
```

## Features

- `wgpu-interop`：打开 `FrameExchange::copy_from_wgpu*`，给渲染到裸 WGPU 纹理的生产者用。

## 测试

```bash
cargo test -p nana-frame-exchange
```

测试需要一个可用的 GPU 适配器。

更多内容见 [GPU 集成](../../docs/reference/gpu.md) 和 [GPU 与帧生命周期](../../docs/learn/gpu-frame.md)。
