---
title: End-to-end flows
---

# End-to-end flows

The preceding chapters took each subsystem apart vertically. This chapter does the opposite: it picks four key data flows that cross several layers and traces each from its input source all the way to screen pixels or an application callback. Every step names the crate it lives in, the core types, and a source line anchor — the best way to check that you really understand the whole framework.

## Flow one: how a state change becomes screen pixels

Scenario: application code modifies the retained tree, until new pixels appear on the window Surface.

```text
① Queue the change   nana-ui-runtime UiMutation (Create / Insert / Detach / SetStyle / …)
                                     crates/nana-ui-runtime/src/mutation.rs:15
② Frame scheduling   nana-ui         FrameDemand (OnDemand / deadline / Continuous)
                                     crates/nana-ui/src/runtime_host.rs:1035
                                     the host decides from demand whether this frame redraws; with
                                     no change, flush returns without work
③ Message digestion  nana-ui         run_runtime_scene passes dispatch_program messages to
                                     RuntimeProgram::update (open window / switch GPU / persistence)
                                     crates/nana-ui/src/scene_host.rs:99
④ Prepare textures   nana-ui         prepare_window_frame: the application prepares the latest
                                     texture this frame samples
⑤ flush transaction  nana-ui-scene   RuntimeDocument::flush: style → text shaping → layout → hit
                                     index → accessibility projection → dirty node extraction,
                                     converged in a single transaction
                                     crates/nana-ui-scene/src/runtime_document.rs:109
⑥ Acquire surface    nana-ui         acquire the window Surface; external resource producers are
                                     compiled into FramePlan and share the host encoder with the UI
                                     (any failure discards the whole frame and rolls back)
⑦ Record drawing     nana-ui         SceneWgpuPainter::paint_target: visible operations recorded
                                     into the target in document order
                                     crates/nana-ui/src/scene_paint/mod.rs:899
⑧ Submit on screen   nana-gpu        FrameContext::submit (merges UploadRing batches) + present
                                     crates/nana-gpu/src/frame.rs:148
⑨ Post-frame tidy    nana-ui         window_frame_presented: only now is it safe to release old
                                     textures retired by the previous frame; bind_window mounts
                                     follow-up content for newly ready windows
```

Key points:

- `RuntimeDocument` is defined in **`nana-ui-scene`** (`crates/nana-ui-scene/src/runtime_document.rs:18`) and holds both `AppContext` (the retained tree) and `UiScene` (the retained drawing graph) — `flush` advances the former's dirty changes into the latter.
- Compositor animation and live GPU content wake the frame just like ordinary UI, but in isolation: `UiScene::compositor_needs_tick` only switches the current window into the lightweight `FrameDemand::Continuous` presentation and does not trigger a global style or layout recomputation.
- Occluded or minimized windows: the host still calls `prepare_window_frame` on the beat, but skips `flush`, does not acquire the Surface, and does not `present`.

## Flow two: how a click becomes an application callback

Scenario: the user presses a button in the window, from the OS native event all the way to the callback closure the application registered.

```text
① Native event   nana-window / winit OS window events (platform-specific structures)
② Boundary       nana-ui-input       the host lowers them at the boundary into CanonicalInputEvent
                                     (InputPayload / KeyInput / PointerInput,
                                     crates/nana-ui-input/src/canonical.rs:44)
③ Sequencing     nana-ui-input       InputSequencer stamps a strictly increasing sequence number and
                                     an animation-domain timestamp
                                     (crates/nana-ui-input/src/canonical.rs:333)
④ Host services  nana-ui-input       HostServices: cursor, IME and clipboard capabilities call back
                                     through this contract
                                     (crates/nana-ui-input/src/host_services.rs:97)
⑤ Routing        nana-ui-runtime     AppContext::route_input — the only routing implementation
                                     in the whole repository
                                     crates/nana-ui-runtime/src/framework/input.rs:397
⑥ Hit and focus  nana-ui-runtime     query the incremental hit index (world/hit_test.rs) and the
                                     focus chain to locate the target node
⑦ Listeners      nana-ui-runtime     fire NodeStore's event_listeners side table (where event_flags
                                     is authoritative)
⑧ Callback       Application         cx.on(button, |_, Activate, cx| …) or .on_activate(...)
```

Key points:

- Headless tests use `HeadlessInput::route_shaped` (`crates/nana-ui-runtime/src/framework/input/headless.rs:119`), injecting a deterministic input sequence straight into the same `route_input`.
- `nana-ui-devtools`'s Agent sessions reuse the same headless channel (`agent/runtime.rs:120`), so the input behavior observed by automated tests matches a production window exactly.

## Flow three: how a piece of text becomes screen pixels

Scenario: a `Text` component displays a string, from style and text source to the glyph atlas on screen.

```text
① Text source       nana-ui-core       string + TextStyle (TextKind, size, variation axes, rich-text
                                       spans) form a TextSource
② Engine entry      nana-text          TextEngine::layout(kind, source, base, constraints, counters)
                                       → Arc<TextLayout>
                                       crates/nana-text/src/engine.rs:36
③ Text shaping      nana-text          Shaper::shape(fonts: &mut FontSystem, request: &ShapeRequest)
                                       → Arc<ShapedText>
                                       crates/nana-text/src/shaping/shaper.rs:116
                                       (HarfRust OpenType shaping lives in shaping/opentype.rs:73)
④ Paragraph layout  nana-text          Layouter wraps and truncates by TextConstraints (the ellipsis
                                       goes through the standard shaping path too), producing an
                                       immutable TextLayout (crates/nana-text/src/layout/ir.rs:182)
⑤ Tiered cache      nana-text          the same text and constraints return the same Arc<TextLayout>
                                       from the cache; only a width change reruns step ④ without
                                       rerunning step ③ shaping; FontGeneration increments on every
                                       font-set change and invalidates old-generation layouts
                                       automatically
⑥ Shared engine     nana-ui            SharedTextEngine = Arc<Mutex<NativeTextEngine>>
                                       crates/nana-text/src/engine.rs:50
                                       one engine instance shared process-wide: one label's shaping
                                       result is reused directly by every copy
⑦ Layout measure    nana-ui-runtime    RuntimeLayoutEngine queries the text engine for intrinsic
                                       size during layout and leaves the generated TextLayoutId
                                       generational handle on the node (TextLayoutStore)
⑧ Rasterization     nana-ui-scene → ui the retained handle enters UiScene with the ExtractedNode;
                                       SceneWgpuPainter::text paints the retained layout directly
                                       and rasterizes glyphs into a multi-page GPU atlas on screen
```

Key points:

- Text layout is **retained**: it is not relaid out from the string every frame, but reused across frames by handle and cache key; a change of plain color, opacity or geometric transform does not touch the text engine at all.
- Variable font axis animation (`font_axis` / `font-variation-settings`) compiles into per-axis tracks and drives real glyph reshaping and rasterization from the `SHAPE_STYLE` dirty graph; it is never faked with a geometric scale.

## Flow four: how a live GPU frame is attached to the tree

Scenario: the application puts the output of a shader render or a video stream into the UI and wants it to take part in layout like an ordinary button, be clipped by rounded corners, be occluded by popups above it, and respond to clicks.

```text
① Frame production   app / frame-exchange  draw the frame into a sampleable GpuTexture (cross-thread
                                           producers hand it over through nana-frame-exchange and do
                                           not own the UI device or submission rights)
② Slot registration  nana-ui               HostTextureRegistry::slot("preview") → TextureSlot
                                           crates/nana-ui/src/gpu_texture.rs:555, 766
③ Mount node         nana-ui-runtime       mount GpuTextureView::new("preview") on the UI tree
                                           (template tag <Texture>)
                                           crates/nana-ui-runtime/src/gpu_slots.rs:203
④ Tree treatment     nana-ui-runtime       exactly like a Button: takes part in Flexbox/Grid layout,
                                           is clipped by parent rounded corners and overflow, enters
                                           the incremental hit index, and takes part in occlusion in
                                           document order
⑤ Update notify      nana-ui               when texture content updates, TextureSlot wakes only the
                                           windows that reference that slot; a 0-size window still
                                           runs prepare, and the producer is encoded only when the
                                           size is drawable
⑥ Scene extraction   nana-ui-scene         during flush it enters UiScene as a CustomRenderNode
                                           (renderer = "nana.host-texture")
⑦ Document order     nana-ui               SceneWgpuPainter::paint_target samples in place in
                                           document order inside the current target pass; several
                                           layers of live content just mount several GpuTextureView
                                           nodes side by side
⑧ Lifetime           nana-ui               old textures replaced in the previous frame are released
                                           only at window_frame_presented (step ⑨ of flow one); when
                                           the texture size or object changes, generation increments
                                           and the Runtime node is not destroyed and rebuilt
```

Key points:

- Two ways to connect GPU content: the default path with an intermediate texture uses `GpuTextureView` (a string slot name); the advanced path with no intermediate texture, encoding directly into the current UI pass, uses `GpuView` (a `u64` slot number, paired with an explicitly registered `SceneGpuRenderer`).
- Declaration and execution are decoupled: GPU slot declarations live in `nana-ui-runtime` (no WGPU dependency), while actual pipeline sampling and recording live in `nana-ui`.

## The common structure of the four flows

Stack the four flows above and you see the same five-layer funnel across NanaUI:

| Stage | Layer | Structural trait |
| --- | --- | --- |
| **1. External input / production** | Outermost (application, OS) | Shapes vary: native window events, strings, styled text, GPU textures |
| **2. Canonical lowering** | Boundary crates (`input` / `text` / `gpu` / `css`) | Translated once at the boundary into framework contracts: `CanonicalInputEvent`, `TextSource`, `TextureSlot`, `LayoutStyle` |
| **3. Authoritative convergence** | `nana-ui-runtime` (`UiWorld`) | All converge into node state on the retained tree and atomic `UiMutation` |
| **4. Scene projection** | `nana-ui-scene` (`UiScene`) | A single bounded `flush` transaction completes layout, hit testing, accessibility and incremental drawing-graph updates |
| **5. Presentation within the frame** | `nana-ui` (`SceneWgpuPainter`) | Recorded in document order into the host-owned `FrameContext` and submitted on screen |

**Boundary crates only translate, the authoritative state lives only in `UiWorld`, and on-screen presentation happens only in `SceneWgpuPainter`.** Hold on to that sentence and you can tell at once which layer any module sits in and who it answers to.

## Read next

- [Retained tree: UiWorld](/learn/runtime-tree) · [Scene extraction](/learn/scene-extraction): the internals of the convergence point and drawing-graph projection
- [GPU and frame lifecycle](/learn/gpu-frame): `FrameContext`, the upload ring, submission order and the 35-step redraw total order
- [How it works (reference)](/reference/how-it-works): the same runtime model for application developers
