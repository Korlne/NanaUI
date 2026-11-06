---
title: Input Path
---

# Input Path

An input event travels from a native operating system window (or a headless test session) to a control in the retained tree through two contracts with a strict division of labor: **`nana-ui-input` defines "what canonical events and host services look like"**, and **`framework/input.rs` in `nana-ui-runtime` decides "which node an event is routed and dispatched to"**. This chapter follows the complete path: canonical events and sequence numbers, bounded endpoints and coalescing, the five-step validation and pointer localization of `AppContext::route_input`, the four ordered dispatch chains for pointer, wheel, key, and IME, focus and roving focus, IME and `HostServices` state write-back, and incremental accessibility projection.

## Responsibilities and boundaries

- **`nana-ui-input` (canonical input contract)**: defines the shape, identity, monotonic sequence numbers, bounded inbox endpoints (`InputEndpoint`), and host service callbacks (`HostServices`) of canonical input events. It contains no platform-specific code (`crates/nana-ui-input/src/lib.rs:5`). `nana-ui-platform` re-exports the whole contract unchanged.
- **`nana-ui-runtime` (the single input router)**: each `AppContext` holds its own routing state (`crates/nana-ui-runtime/src/framework/input.rs:1`). It is the **only path** by which host input events enter the retained world. Different windows bind their own input sources and never share pointer capture, IME focus, or cursor state.
- **Host adapter layer (lowering and write-back)**: native windows stamp winit events and push them into endpoints in `crates/nana-ui/src/scene_host/input.rs`.

## Dependencies and neighbors

| Layer / Crate | Role |
| --- | --- |
| `nana-ui-input` | Defines canonical events, the sequencer, bounded endpoints, and the `HostServices` contract |
| `nana-ui-platform` | Re-exports the input contract unchanged for the window layer and the host layer |
| `nana-ui-runtime` | The single routing implementation (`route_input`), four dispatch chains, focus management, IME state, and incremental accessibility projection |
| `nana-ui` | Lowers native platform events into canonical events and, at the end of a drain, writes `HostServices` state back to the native window |
| `nana-text` | Provides `TextShaper` metrics when routing text editing |
| `nana-diagnostics` | Records high-frequency endpoint and routing metrics (`runtime.input.*`, through atomic `metric!` only, never per-frame events) |

## Key types and data

| Type | File and line | Purpose |
| --- | --- | --- |
| `CanonicalInputEvent` | `crates/nana-ui-input/src/canonical.rs:44` | Unified event envelope: source ID, device ID, endpoint generation, sequence number, timestamp, and payload |
| `InputPayload` | `crates/nana-ui-input/src/canonical.rs:209` | 13 canonical payloads: pointer, wheel, key, committed text, IME composition, file drop, focus, disconnect, and more |
| `CommittedText` | `crates/nana-ui-input/src/canonical.rs:269` | Committed text payload; carries the sequence number of the key that produced the text (used to suppress character insertion after a shortcut fires) |
| `CompositionInput` | `crates/nana-ui-input/src/canonical.rs:191` | IME composition lifecycle payload (`Start` / `Update` / `Commit` / `End` / `DeleteSurrounding`) |
| `InputSequencer` | `crates/nana-ui-input/src/canonical.rs:333` | Stamps events with strictly increasing sequence numbers and animation-clock-domain timestamps; manages endpoint generation advancement |
| `InputEndpoint` | `crates/nana-ui-input/src/canonical.rs:410` | Bounded single-source inbox queue that coalesces adjacent pointer move and wheel events |
| `HostServices` | `crates/nana-ui-input/src/host_services.rs:97` | Callback contract for cursor style, IME text input context, and the system clipboard |
| `InputRouteOutcome` / `InputRouteError` | `crates/nana-ui-runtime/src/framework/input.rs:36` / `:74` | Per-event routing result and the five validation rejection reasons |
| `KeyStroke` | `crates/nana-ui-runtime/src/framework/input/dispatch.rs:55` | Key-chain dispatch carrier shared by key presses and `CommittedText` |
| `SourceEffects` | `crates/nana-ui-runtime/src/framework/input/effects.rs:43` | Latest-value slots for cursor and IME context (the host is notified only when state actually changes) |
| `InputState` / `SourceState` | `crates/nana-ui-runtime/src/framework/input.rs:247` / `:167` | Router global state and independent per-input-source state |
| `HeadlessInput` | `crates/nana-ui-runtime/src/framework/input/headless.rs:19` | Deterministic input source used by headless tests and Agent sessions |

## Internal structure

### 1. Canonical events: identity, sequence numbers, and bounded coalescing endpoints

`canonical.rs` (`crates/nana-ui-input/src/canonical.rs:1`) sets three hard rules: the host lowers native messages once, at the boundary; event timestamps all use nanoseconds in the Runtime animation clock domain (so hover throttling, double-click detection, and animation share one clock); and sequence numbers increase strictly monotonically across devices within one endpoint generation (`EndpointGeneration`).

- **Keys and text suppression (`CommittedText`)**: a key payload carries both the layout-independent physical key `PhysicalKey` and the logical key `LogicalKey`. `CommittedText` (`:269`) also records the sequence number of the key that produced it. When Runtime has consumed a key as a shortcut, a Tab focus move, or an Enter submit, `SourceState::handled_key` records that sequence number. When the router later receives a `CommittedText` carrying the same sequence number, it suppresses and drops it (counted in `text_suppressed`). This matches the browser semantics of skipping the `input` event after `keydown.preventDefault()`.
- **Bounded endpoints and safe coalescing (`InputEndpoint`)**: the native window endpoint has a capacity of `ENDPOINT_EVENTS = 1024` and `ENDPOINT_PAYLOAD_BYTES = 1 MiB` (`crates/nana-ui/src/scene_host/host_services.rs:22`). When the queue is full, the event is returned to the caller, which drains first and then retries. Keys, buttons, and IME state transitions are never silently dropped. `coalesce` (`canonical.rs:470`) merges only adjacent `PointerMove` and `Wheel` events (wheel deltas are summed, position takes the latest), and only when the new event's sequence number is strictly greater and its timestamp does not go backward, so coalescing cannot mask an ordering error.

### 2. Routing entry: five-step validation and pointer ID localization

The host first calls `bind_input_source` to bind an input source to a document, then routes events through the repository's single `AppContext::route_input` (`crates/nana-ui-runtime/src/framework/input.rs:397`) or the batched `drain_input` (`:430`). `route_validated` (`:508`) runs five validation steps in a fixed order. If any step fails, it returns the matching `InputRouteError` (`:74`):

| Validation order and error variant | Rejection condition |
| --- | --- |
| 1. `UnknownSource` | The input source has not called `bind_input_source` or has been unbound |
| 2. `StaleGeneration` | The event's `EndpointGeneration` is older than the current binding generation |
| 3. `Disconnected` | The device has disconnected and the event's sequence number is later than the disconnect event |
| 4. `OutOfOrder` | The event's sequence number is not strictly greater than the last processed event of that source |
| 5. `TimestampRegression` | The event's timestamp is less than that of the last processed event of that source |

After validation, pointer events first go through `localize` (`:64`), which maps the platform pointer ID to an `AppContext`-local pointer ID, then enter the dispatch chains. At the end of handling each event, `flush_reactive()` is called immediately to settle signal writes, and `pending_work_revision` determines whether frame work was marked dirty (`InputRouteOutcome::invalidated_work`). When there are no new input events but scrolling, layout, or animation has moved UI elements, the host calls `refresh_input_effects` (`:478`) after the frame to rederive hover and cursor state at the stationary pointer position.

### 3. Four ordered dispatch chains (`dispatch.rs`)

`crates/nana-ui-runtime/src/framework/input/dispatch.rs` splits events into four chains of responsibility with explicit priority (once an earlier link consumes an event, later links do not respond):

1. **Pointer chain (`dispatch_pointer:88`)**: each move of an uncaptured pointer runs **1** hit query (`UiWorld::hit_test`); a captured pointer runs **0** hit queries. A press (Down) passes through, in order: dismiss on outside click for overlays → scrollbar lane interception → reachable handles → `focus_node_in_place` (focuses in place without triggering auto-scroll, so content does not scroll away at the moment of the press and leave the release landing elsewhere) → text editor and document selection press → pointer hooks of compound component nodes (TreeView / AreaLayout / ScrubField / TimelineEditor / PropertyGrid / charts / Dock / SplitPane / Workspace, and others) → generic `press_pointer`.
2. **Wheel chain (`dispatch_wheel:690`)**: line wheel deltas are multiplied by `LINE_SCROLL_EXTENT = 60.0` and passed from the hit node to the nearest scrollable ancestor container that can still change its target, advancing with a 180 ms ease-out cubic transition. Pixel wheel deltas commit the offset directly and cancel that container's transition. Overlays still bound the ancestor walk, and dedicated wheel hooks such as charts keep priority.
3. **Key and text chain (`dispatch_keystroke:800`)**: menu bar key routing (`Alt` / `F10` and arrow keys for `AppMenuBar`) and modal blocking overlays → application-level key policy → focused terminal / rich text editor / plain text editor (including TreeView inline rename and ScrubField typing) → Tab-order focus traversal (`navigate_sequential_focus`) → segmented controls and undo/redo → clipboard shortcuts (through `HostServices`) → the focused control's own key handling → arrow-key navigation for tables and roving focus (`roving_focus_key`) → keyboard triggering of popup menus (`keyboard_menu_key`). With `Ctrl` / `Cmd` combinations, `shortcut_key` (`framework/input.rs:214`) maps non-Latin keyboard layouts such as Cyrillic, Greek, and Hebrew back to Latin letters by physical key position, so shortcuts work consistently across keyboard layouts.
4. **IME composition chain (`dispatch_composition:1252`)**: routes to the editable field that currently owns the IME session (`Start` → `set_ime_preedit("")`, `Update` → update the preedit overlay, `Commit` → commit text through the component, `DeleteSurrounding` → delete characters around the caret).

### 4. `HostServices` state slots and incremental accessibility projection (including virtual nodes)

- **`HostServices` is a latest-state slot, not a request queue**: `HostServices` (`crates/nana-ui-input/src/host_services.rs:97`) and `SourceEffects` (`framework/input/effects.rs:43`) model the mouse cursor and the IME `TextInputContext` as "latest-value slots". The router notifies the host only when state actually changes, and the native window's `NativeWindowServices` syncs to the system window once at the end of each drain. Input is never blocked by a backlog of stale cursor requests.
- **Incremental accessibility projection and virtual children**: `project_accessibility_delta` (`crates/nana-ui-runtime/src/world/accessibility.rs:630`) produces an incremental accessibility transaction with tombstones after layout converges each frame. For self-painted single-node controls such as `TreeView` and open menus, the projector uses **virtual node IDs** marked in the top two bits to project each visible row as an accessibility child of the parent control: menu rows use `virtual_menu_item_id` (bit 63 is `1`, bit 62 is `0`), and tree rows use `virtual_tree_item_id` (`0xC000_0000_0000_0000 | (tree << 16) | slot`, bits 63 and 62 both `1`, role `AccessibilityRole::TreeItem`, carrying `level`, `expanded`, `selected`, `disabled`, and the row's geometry rectangle). An `apply_accessibility_action` (`Click` / `Focus` / `Expand` / `Collapse`) issued by a screen reader first decodes the virtual row ID and then hands off to the same component logic, so screen reader operations produce exactly the same event sequence as mouse clicks and arrow keys.

## Collaboration and data flow

The complete loop for one native window input:

1. A winit native window event enters `handle_window_event` (`crates/nana-ui/src/scene_host/input.rs:54`).
2. `push_input` (`:475`) stamps a sequence number and timestamp with that window source's `InputSequencer` and pushes it into the `InputEndpoint` (if the queue is full, it drains once and retries).
3. `drain_window_input` (`:509`) calls `AppContext::route_input` to complete validation, pointer localization, and dispatch through the four chains. Signals and tree changes produced by event callbacks are settled at the end through `flush_reactive()`.
4. At the frame boundary, `RuntimeDocument::flush` drains pending system work (see [Retained Tree: UiWorld](/learn/runtime-tree)). The host then calls `refresh_input_effects` and applies the latest cursor and IME state once in `NativeWindowServices`.

## Code map

| File path | Core content |
| --- | --- |
| `crates/nana-ui-input/src/canonical.rs` | `CanonicalInputEvent`, identity types, `InputSequencer`, `InputEndpoint`, and adjacent-event coalescing |
| `crates/nana-ui-input/src/host_services.rs` | `HostServices` contract, cursor enum, `TextInputContext`, and the headless implementation |
| `crates/nana-ui-runtime/src/framework/input.rs` | The single router `route_input`, five-step validation, pointer localization, focus routing, and disconnect cleanup |
| `crates/nana-ui-runtime/src/framework/input/dispatch.rs` | The four dispatch chains for pointer, wheel, key, and IME composition |
| `crates/nana-ui-runtime/src/framework/input/effects.rs` | `SourceEffects` latest-value slots for cursor and IME, 8 ms handle probe throttling |
| `crates/nana-ui-runtime/src/framework/input/headless.rs` | `HeadlessInput` deterministic headless input source |
| `crates/nana-ui-runtime/src/world/hit_test.rs` | Incremental hit index and transform-aware hit queries |
| `crates/nana-ui-runtime/src/world/accessibility.rs` | Incremental accessibility projection, tombstones, and virtual row node encoding for menus and trees |
| `crates/nana-ui/src/scene_host/input.rs` | Native window event enqueueing, drain, and cleanup on document rebind |
| `crates/nana-ui/src/scene_host/host_services.rs` | The native window's `HostServices` implementation and endpoint capacity constants |

## Further reading

- [Retained Tree: UiWorld](/learn/runtime-tree): how the `MutationQueue` written by event callbacks is committed, and how in-frame system work is drained
- [Layout Engine](/learn/runtime-layout): where the `LayoutResult` geometry consumed by hit testing and accessibility projection comes from
- [Window and Platform](/learn/window-platform): how native window events and DPI scaling reach `handle_window_event`
- [Input Conformance Matrix (Reference)](/reference/input-conformance) · [Cost of a Single Pointer Event (Reference)](/reference/input-cost)
