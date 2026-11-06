# nana-ui-input

English · [简体中文](README.zh-CN.md)

NanaUI's canonical input events, and the interface the router uses to request services from the host.

Hosts lower native pointer, wheel, key, text commit, IME composition, and file drop events into `CanonicalInputEvent`. Each input source has one `InputSequencer`, which stamps events with strictly increasing sequence numbers and timestamps. `InputEndpoint` is a queue bounded by event count and byte size; adjacent pointer moves and wheel events are coalesced, and when the queue is full `push` hands the event back to the caller unchanged. `HostServices` is the side where the router calls back into the host: cursor, IME input context, and clipboard.

This crate has no platform dependencies, reads no native events, and does no routing. Hit testing, focus, capture, and IME state are in the router in `nana-ui-runtime`, and lowering native events happens in each host.

## Example

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
// adjacent moves coalesce into one
assert_eq!(std::iter::from_fn(|| endpoint.pop()).count(), 1);
```

## Tests

```bash
cargo test -p nana-ui-input --lib
```

The routing-side contract tests are in `nana-ui-runtime`; the full commands are in the input conformance matrix.

See also [Input pipeline](../../docs/en/learn/runtime-input.md) and [Input conformance matrix](../../docs/en/reference/input-conformance.md).
