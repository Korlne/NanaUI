# nana-ui-input

NanaUI 的规范输入事件，以及路由器向宿主要服务时用的接口。

宿主把原生的指针、滚轮、按键、文本提交、IME 组字和文件拖放事件降维成 `CanonicalInputEvent`。每个输入源一个 `InputSequencer`，给事件盖上严格递增的序列号和时间戳。`InputEndpoint` 是按事件数和字节数限额的队列，相邻的指针移动和滚轮会合并，队列满时 `push` 把事件原样还给调用方。`HostServices` 是路由器反过来调用宿主的那一面：光标、IME 输入上下文和剪贴板。

这个 crate 没有平台依赖，不读原生事件，也不做路由。命中测试、焦点、捕获和 IME 状态都在 `nana-ui-runtime` 的路由器里，原生事件的降维在各个宿主。

## 示例

```rust
use std::time::Duration;
use nana_ui_input::{
    CanonicalInputEvent, DeviceId, EndpointGeneration, InputEndpoint, InputPayload,
    InputSequencer, InputSourceId, PointerInput, PointerPhase,
};

let mut sequencer = InputSequencer::new(InputSourceId(1), EndpointGeneration(1));
let mut endpoint = InputEndpoint::new(8, 0);
for step in 1..=1_000u64 {
    let metadata = sequencer.stamp(DeviceId(2), Duration::from_micros(step));
    let payload = InputPayload::Pointer(PointerInput::mouse(PointerPhase::Move, step as f32, 2.0));
    endpoint.push(CanonicalInputEvent { metadata, payload }).unwrap();
}
// 相邻的移动合并成一条
assert_eq!(std::iter::from_fn(|| endpoint.pop()).count(), 1);
```

## 测试

```bash
cargo test -p nana-ui-input --lib
```

路由侧的合同测试在 `nana-ui-runtime`，完整命令见输入验收矩阵。

更多内容见 [输入链路](../../docs/learn/runtime-input.md) 和 [输入验收矩阵](../../docs/reference/input-conformance.md)。
