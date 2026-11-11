# Live GPU Content and GPU Integration Contract

This page specifies the full technical contract for NanaUI's host GPU context (`GpuContext` / `FrameContext` / `GpuTexture`), live texture mounting (`HostTexture` / `GpuTextureView`), same-pass direct drawing (`GpuView`), cross-thread latest-frame exchange (`FrameExchange`), and per-scene offscreen production (`SceneResourceProducer`).

For windowless retained output, target planning, and multi-consumer paths (such as Spout or screen recorders), see [Off-Window Output (ExternalSurface)](output.md). The `GpuContext`, `FrameContext`, and `GpuTexture` described here are also the sole device, submission, and resource owners for that output boundary.

## Backend selection, GPU policy, and HDR presentation

### 1. Native RHI decision gate (Issue #186) and choosing WGPU backends

Issue #186 concluded **NO-GO**: WGPU remains NanaUI's only official graphics backend, and the official path stays `Logical GPU ABI -> WgpuBackend -> wgpu` (see the [Issue #186 delivery record](https://github.com/sena-nana/NanaUI/tree/docs-archive/archive/docs-notes/consumer-upgrade-2026-09-25-issue186.md)).

Framework crates do not lock the graphics backend on the application's behalf: the workspace's `wgpu` enables only `std`, `parking_lot`, and `wgsl` by default. When `gpu` is enabled, `nana-ui`'s default feature `wgpu-backends` turns on every standard backend for the current platform (DX12 / Vulkan / GLES on Windows, Metal on Apple, Vulkan / GLES on Linux). To reduce size and ship only specific graphics backends, set `default-features = false` on the dependency and list the backends you need per platform explicitly on the application's own `wgpu` dependency; `hosted_context` only picks adapters among the backends actually compiled in. For how the DX12 shader compiler (`dxcompiler.dll` vs FXC) is chosen, see [Two-phase startup · DX12 shader compiler](startup.md#dx12-shader-compiler).

### Adapter selection

The host chooses only among adapters that can present the main window. Selection happens on the startup thread, and the replacement device after device loss is chosen again by the same rules:

- The application specifies it through `StartupOptions::with_adapter(GpuAdapterChoice)`. `Matching(GpuAdapterId)` matches by vendor/device id; among multiple cards of the same model it then compares the PCI bus address, and when either side has no bus address it compares names. If that card is absent, cannot present the main window, or cannot create a device, selection falls back to automatic; this is not a startup failure.
- `Automatic` (the default) follows the rules of wgpu's `request_adapter`: when `WGPU_ADAPTER_NAME` is set, it takes the first adapter whose name contains it; otherwise it stably sorts by `WGPU_POWER_PREF` and takes the first. When neither is set, it is the first in enumeration order. The application's `Matching` takes precedence over both environment variables.
- A program reads the `GpuAdapterSelection` from this device creation through `RuntimeProgramContext::gpu_adapters()`: the adapters that can present the main window (in enumeration order), the one actually used, and whether the requested card was skipped (`requested_missing`). Contexts in which the host does not choose a device (headless, staged, embedded injection) return an empty list or only the injected device. Settings UIs should read this value instead of enumerating adapters themselves.
- Adapters are not switched while running: the application saves the new choice, and it takes effect on the next launch.

### 2. Unified GPU policy (Issue #184)

`GpuContext::policy()` maintains a shared pipeline registry and work counters per device generation (`DeviceGeneration`). `SceneWgpuPainter`'s static pipelines reuse the underlying WGPU objects across windows through it. `GpuWorkSink` records buffer uploads and reallocations uniformly into the device statistics (`GpuPolicyStats`, including `upload_bytes`, `upload_writes`, `upload_copies`, `upload_flushes`, `upload_ring_allocations`, `upload_ring_waits`, `frame_slot_waits`, `frame_slot_stalls`, and more). The public boundary always stays pure `nana-gpu` types.

### 3. HDR display info and color space contract (Issue #283)

A Surface's HDR capability is determined by `SurfaceCapabilities::format_capabilities`. `wgpu::Surface::display_hdr_info` returns the current display's runtime recommendations, which are used only to adjust the final tone map: Apple EDR provides relative headroom, and Windows provides peak luminance and the SDR reference white in nits. A missing value means only "unknown", not SDR, and it alone must not decide whether to enable an HDR Surface:

- **Event-driven refresh**: the host caches and updates `DisplayHdrInfo` when creating the Surface, on device recovery, and on display events such as window move, scale, and resize (steady-state frames do not poll the operating system, and Metal queries always run on the event thread). When values do not change, no redraw is triggered; when they change, only the presentation uniform is updated, without rebuilding Runtime or Scene state. The safe headroom is `DisplayHdrInfo::tone_map_headroom().unwrap_or(1.0)`, and a missing SDR reference white is treated by the `80 nit` scRGB convention. When an embedded host receives a system display notification, it can call `EmbeddedRuntime::refresh_display_hdr_info()` (or the method of the same name on `HostedGpuContext`) on the window thread.
- **Linear scRGB working space**: `SceneWgpuPainter`'s internal compositing working space is uniformly **premultiplied linear scRGB**. Chart marks (`ScenePrimitiveKind::Chart`) are drawn by three analytic anti-aliased pipelines for polylines, areas, and shapes (edges are a 1-device-pixel transition and do not rely on MSAA). Vertex and style arrays stay resident in the target GPU buffers by revision, and the pipelines are compiled lazily the first time a chart is encountered.
- **Color metadata for cross-thread and offscreen export**: `FrameExchange` copies texels byte for byte and does no color space conversion. When a producer needs to pass a color space across threads, it should call `copy_from_with_metadata` with a `FrameColorMetadata` (declaring primaries, transfer function, and alpha semantics), and the consumer reads it from `FrameLease::color_metadata()` or `FrameBinding::color_metadata()` (the old `copy_from` marks frames `Unknown`, and consumers must never silently assume sRGB). By default, `FrameBinding` directly accepts only full-range sRGB or 80-nit linear scRGB. Frames declared as P3, PQ, HLG, limited range, or non-premultiplied alpha stay in the inbox and are not sampled by mistake; the producer must convert them first or a custom consumer must handle them.
- **`nana-ui-devtools::offscreen` snapshot boundary**: always uses `BGRA8_UNORM_SRGB`. To export a PNG, use the `SnapshotImage` returned by `readback_image` / `paint_image` and call `into_png_srgb` or `write_png_with_metadata` (they convert premultiplied sRGB to straight-alpha sRGB and write the PNG sRGB chunk automatically).

### 4. Frame upload ring (UploadRing), frame slots, and caches

- **Batched upload (`UploadRing`)**: the renderer does not scatter `queue.write_buffer` / `write_texture` calls. Instead, it appends writes through `GpuWorkSink` to the current `FrameContext`'s upload batch. When `FrameContext::submit` is called, the device's pending batch (including off-frame writes and writes from previously discarded frames) and this frame's upload batch are copied with one `memcpy` into a mapped block of the `UploadRing` (capped at `32 MiB`), merged into a single upload command buffer, and land ahead of the render commands in the same `queue.submit`. This removes WGPU's internal staging allocation for each small write. (Text glyph blocks still use an in-encoder `copy_buffer_to_buffer` at the recorded position, so that multiple draws to the same target before a single submit each see the correct glyphs; see #224. Glyph atlas regions written by the second and later paints within the same frame are likewise copied at the recorded position; see "Atlas ordering for multiple targets in one frame" below. These count toward `upload_bytes` but not `upload_writes` / `upload_copies`.)
- **Bounded frame slots**: each `FrameContext` holds a frame slot while recording (discard returns it immediately; submit returns it in the GPU completion callback). When slots are full, `begin_frame()` does one bounded wait on the oldest submitted frame; `try_begin_frame()` returns `None` immediately when slots are full. The window presentation path uses `try_begin_frame()`, and **must call `get_current_texture` only after obtaining a frame slot**. (If no slot is available, it first does one non-blocking `poll`; if still full, it skips this frame and leaves the buffer already on screen in place. This prevents a transparent swapchain from taking a buffer and then waiting, which would blank the whole window.)
- **Generational caches and shared URL textures**: the pipeline and resource layout registries use a `StampedCache` isolated per `DeviceGeneration` (when full, it linearly evicts the least recently used 1/8 and compiles pipelines in parallel outside the lock). `SceneWgpuPainter` holds a single `UrlTextureCache` shared by Quad backgrounds, `border-image`, and HostTexture masks (at most 4 concurrent HTTP fetches, an idle cap of 256 entries / `64 MiB`, automatic release after 120 consecutive painted frames without a reference; raster image decode is capped at `4096 px` / `64 MiB`, SVG at `2048 px`).
- **Painter core shared across profiles**: incompatible output groups of the headless `PresentationDriver` share one `SceneWgpuPainter`. Profile-independent parts are stored once: the text engine, shape/raster caches and glyph atlas, `UrlTextureCache`, and motion resources. The icon atlas is target state, one per painted `RenderTargetId` (plus one for the painter's own default target). Each working format (and its URL source encoding) keeps one pipeline set (quad, mesh, icon and text programs, HostTexture, backdrop, chart), swapped in when the profile switches and built on first use. URL textures are bucketed by egress and source encoding (sRGB or legacy linear direct write), so the two encodings never overwrite each other. Each target uses only one profile for its lifetime. Whenever the shared atlas evicts or moves glyphs for any profile, the placement epoch advances, and other targets rebuild their retained `PreparedBatch`. Window hosts still hold one painter per profile.
- **Atlas ordering for multiple targets in one frame**: a `FrameContext`'s upload batch lands as a whole ahead of all of that frame's passes at submit time, and the glyph atlas is shared by every target that painter draws. So the second and later paints within one frame write their glyph regions with `copy_buffer_to_texture` at the recorded position in that frame's encoder, ordered after the passes already recorded. When a later group evicts, reuses, or compacts an earlier group's glyph rectangles, the earlier group still samples its own glyphs. The first paint still goes through the upload batch, so the cost of a single-target frame is unchanged. The same applies when a window and its same-profile outputs share a painter within one frame. These copies live with the frame's encoder: when a frame is discarded they never land, and before the next paint the painter makes the atlas forget all glyph positions, so each target re-places glyphs and rebuilds its batches. A painter records into only one frame at a time: the host submits or discards the current frame before painting into the next; two frames submitted out of order would put one frame's atlas writes under the other frame's passes.
- **Motion ordering for multiple scenes in one frame**: motion descriptors, keyframes, and clock buffers are also shared by the painter. The second and later paints in a frame upload only the changed data, at the encoder's recorded position, so a later scene does not overwrite an earlier scene's animation. The first paint keeps using the frame upload batch, and identical motion data is not uploaded twice. Ordered copies count toward upload bytes and add no submits or CPU waits. A dropped frame invalidates motion's CPU upload cache through the same shared resource ledger, and the next frame re-uploads. This rule covers both `paint` and `paint_target`.
- **Painter resource counters**: `SceneWgpuPainter::text_glyph_counters()` reports the glyph atlas, raster cache, and glyph uploads. `resource_counters()` (`ScenePainterResourceCounters`) reports icon atlas count and bytes, URL texture count, uploaded GPU bytes (including all mips, excluding driver padding), pending CPU upload bytes, and in-flight loads. Headless hosts read the same numbers for the shared painter through `PresentationDriver::text_glyph_counters()` / `painter_resource_counters()`. All of these are texel bytes allocated by the painter, not driver video memory.
- **Target attachment and pipeline reuse**: when switching between UI-only and mixed GPU-node content, only the 4x MSAA attachment is added or released; the linear color target, bindings, and group caches of the same size, format, and presentation profile are kept. Scaling must rebuild size-dependent textures, but on a device pipeline cache hit no shader module or pipeline layout is created again, so multi-target first frames do not parse WGSL repeatedly. Changing size, output format, or presentation profile still rebuilds the target under the original contract.


### 5. Shader and pipeline cache across launches

Every `GpuContext` whose device has `PIPELINE_CACHE` (the hosted device requests it whenever the adapter offers it) owns one pipeline cache and makes it the device default through the fork's `wgpu::Device::set_default_pipeline_cache`. Every pipeline created without naming a cache goes through it — all scene families, surface and text materials, chart programs, and renderers that build pipelines on the raw device. On DX12 it holds compiled shader bytecode, so a stage found there skips naga's HLSL compile in DXC/FXC; on Vulkan it is the driver's `VkPipelineCache`. Metal and GL have none (`pipeline_cache_key()` is `None`).

The framework hands out and takes back bytes; the host decides where they live:

- `GpuContext::pipeline_cache_key()` names what the bytes belong to; `file_stem()` is `dx12` (bytecode is adapter-independent) or `vulkan-<vendor>-<device>`.
- `load_pipeline_cache(&bytes)` replaces the cache before painters are built; `pipeline_cache_data()` returns the bytes to save. `restore_pipeline_cache` / `persist_pipeline_cache` do the same through a `PipelineCacheStore` (`load` / `save`).
- Hosted applications pass a store with `NanaApplicationBuilder::pipeline_cache(store)` (or `StartupOptions::pipeline_cache`): the `nana-startup-gpu` thread restores it before the painter compiles and saves it once the painter is built. A device replaced after a loss starts with an empty cache.
- The saved bytes carry an envelope (magic, key, length, checksum) over wgpu's own header and validation key, and the DX12 payload checksums each entry. Bytes that are not a cache, belong to another key, are truncated or corrupted, or that the backend rejects are refused (`PipelineCacheLoad::Refused`) and the device compiles as if nothing had been loaded; no backend validation error is raised. A driver update invalidates Vulkan data inside wgpu/the driver, and a DXC update invalidates DX12 data through the validation key; both silently recompile.

## Mounting host textures (`HostTexture` + `GpuTextureView`)

Shader output, preview viewports, and offscreen render textures are nodes in the UI tree just like ordinary buttons: they take part in layout, are clipped, and can be hit. For a first integration, use the default path: draw the content into a sampleable texture, mount a `GpuTextureView` in the tree, and register it in `host_textures()` under the same string slot name.

```bash
cargo run -p nana-ui --example hosted-gpu-demo --features hosted,bundled-fonts,wgpu-interop
```

`run_runtime` creates the one `GpuContext` for the process (additional windows only create a new Surface and share that context; an external event loop injects an existing device through `EmbeddedRuntime` and `HostedGpuShared::from_device` and must never call `request_device` again).

When the host creates or restores the shared device, it automatically enables the BC/ASTC texture compression features the adapter supports, so content producers can sample compressed textures on the same device. This does not change the UI's own texture storage policy. Unsupported features are not requested; producers choose formats or fall back according to the device's actual capabilities. Externally injected devices and application offscreen hosts must request the compression features they need themselves when creating the device.

:::api

```rust view
// In the tree: takes layout space just like a Button
let preview = cx
    .mount_view_root(document_id, || view! {
        <Texture resource="preview" />
    })?
    .root::<GpuTextureView>();
```

```rust rust
// In the tree: takes layout space just like a Button
let preview = cx
    .mount_view_root(document_id, || widget(GpuTextureView::new("preview")))?
    .root::<GpuTextureView>();
```

:::

Register the texture slot in a `RuntimeProgram` and update it per frame:

```rust
// 1. initialize / rebuild_gpu: create the texture on the host device and register the slot
let gpu = context.gpu();
let texture = gpu.create_texture(&GpuTextureDescriptor {
    label: Some("preview"),
    width,
    height,
    format: GpuTextureFormat::RGBA8_UNORM_SRGB,
    usage: GpuTextureUsages::SAMPLED | GpuTextureUsages::COPY_DST,
})?;
self.textures.register(
    "preview",
    HostTexture::new(&texture),
    width,
    height,
    HostTextureAlphaMode::Opaque,
);

fn host_textures(&self, _id: WindowId) -> Option<HostTextureRegistry> {
    Some(self.textures.clone())
}

// 2. Update pixels or replace the texture before flush
fn prepare_window_frame(&mut self, id: WindowId, context: &RuntimeProgramContext<Self::Message>) {
    // Upload CPU pixels: context.gpu().write_texture(&texture, region, &pixels, bytes_per_row)
    // Replace the texture object or size: host.replace_texture(&new_texture); content-only update: host.invalidate()
    // Note: never modify Runtime nodes to swap a texture; this may still be called when the window is occluded or zero-sized, so do not assume a present follows
}

// 3. Only after present may the old texture replaced in the previous frame be released
fn window_frame_presented(&mut self, _id: WindowId, _context: &RuntimeProgramContext<Self::Message>) {}

// 4. Device loss recovery: the program instance is kept; only rebuild GPU resources on the new context.gpu()
fn rebuild_gpu(&mut self, context: &RuntimeProgramContext<Self::Message>) {}
```

- **The slot is the only identity**: a view records only the slot name (`TextureSlotName`), and each painted frame looks up the slot's current binding in the registry by name. `HostTexture::new` allocates its own id. Whether you register a new handle in the same slot, swap the texture object with `replace_texture` (`generation` increments), or refresh content with `invalidate` (`version` increments), the host redraws the windows that use the slot, with no change to Runtime nodes. On device rebuild, the host clears the registry first and then calls `rebuild_gpu`; the program just registers again on the new device. If `HostTexture::device_generation()` does not match the current painter's device generation, the whole frame is rejected with `ScenePaintError::StaleHostTexture`.
- **Linear working space requirement**: `HostTexture` samples entering the Scene must be in premultiplied linear scRGB. Ordinary opaque or straight-alpha sRGB images can use `*_SRGB` texture formats and let the hardware sampler decode them. For translucent premultiplied content, use linear `UNORM` or floating-point formats; do not mislabel gamma-space premultiplied bytes as `SRGB`.

### GPU contract and the wgpu escape hatch

#### Logical GPU ABI (Issue #185)

Ordinary render extensions can declare uniform/storage buffers, sampled/storage textures, samplers, dynamic slices, and optional resource arrays through `nana_gpu::ResourceTable`, and bind portable WGSL with vertex/instance layouts through `ShaderInterface`. (`GpuContext::create_resource_layout` and `create_resource_group` validate `DeviceGeneration`, usage, ranges, and optional capabilities before creating opaque handles; the public API never exposes `wgpu::BindGroupLayout`.)

| Core contract type | Responsibility |
| --- | --- |
| `GpuContext` | The process's single GPU device handle: carries `generation()`, `capabilities()`, and the sticky loss state `is_lost()`, and provides `create_texture` / `write_texture` / `begin_frame` |
| `FrameContext` | An exclusive per-frame encoder: `submit()` submits and returns a `GpuSubmission`; drop / `discard()` invalidates it immediately and automatically rolls back the retained target state the painter registered on that frame |
| `GpuTexture` / `GpuRenderTarget` | 2D textures and render targets carrying a `DeviceGeneration` (resources from another generation are rejected at the entry point) |
| `GpuTextureFormat` / `GpuTextureUsages` | Backend-neutral opaque format and usage flags |

When a custom renderer really needs the underlying native WGPU objects, enable the `wgpu-interop` feature explicitly. It provides `GpuContext::from_wgpu` / `wgpu()`, `FrameContext::wgpu_encoder()`, `GpuTexture::from_wgpu` / `wgpu_view()`, `ScenePass::wgpu()`, and more, and hands out **the same** device and encoder behind the contract. Internal framework crates (`nana-ui`, `nana-frame-exchange`, `nana-ui-devtools`) reach the backend through the hidden `nana_gpu::__framework`, enforced in CI by `scripts/check-engine-boundary.py`.

::: tip Queue work needs no submission guard
NanaUI's wgpu fork makes `Surface::configure` (a window resize) wait only for the work that still uses that surface's swapchain images, not for the whole device to go idle, and another thread submitting at the same time no longer makes it fail with `GpuWaitTimeout` (see `third_party/wgpu/README.md`). So raw `queue.submit` / `write_texture` / `write_buffer` calls through `wgpu-interop` may run on any thread without a lock; the former `gpu.wgpu().lock_submission()` is deprecated and its guard no longer excludes anything. Inside `GpuContext` only the `submission_order` mutex remains, keeping frame submits, upload batches and native-export signals in queue order.
:::

#### Native texture export (`native-export`)

An external producer that creates its own WGPU device and has no existing device-lost callback uses `GpuContext::from_wgpu_with_loss_tracking` to hand the callback explicitly to the same `GpuContext`; all clones share `is_lost()` / `lost_report()` state. The original `from_wgpu` keeps the host's callback and does not observe loss on the host's behalf; existing hosts still forward it to the embedded runtime themselves. The OpenVR / Electron reference producers use the former, so they never claim to check `is_lost()` without having installed the notification. The `controlled_device_loss` integration test actually calls `device.destroy()` to verify callback ownership, sticky loss, and rejection of the replaced generation; this does not mean recovery from a hardware driver reset or from an external compositor has been tested.

On a Windows + DX12 device, enabling the `native-export` feature (capability bit `GpuCapability::NativeTextureExport`) lets `NativeExportPool` create 3 shared `BGRA8` textures and one shared fence on the host's own device and export them as NT handles, so external D3D11 / Spout consumers can sample across APIs with zero CPU readback. For the full fence timing (`2k + 1` ready, `2k + 2` release) and the consumer contract, see [Off-Window Output · DX12 shared textures](output.md#dx12-shared-textures-native-export).

## Cross-thread latest-frame exchange (`FrameExchange` and `FrameBinding`)

When live content is produced on a separate thread (such as model rendering, broadcast compositing, or video decoding), use `FrameExchange` from `nana-frame-exchange` to hand completed frames to the window, and bind them to a slot on the window side with `FrameBinding` (the producer only needs to depend on `nana-frame-exchange`):

```rust
// Producer thread: call poll on every tick; when there is a new frame, record it and copy_from
let mut exchange = FrameExchange::new(&gpu, frame_exchange::DEFAULT_CAPACITY, epoch, notify);
let inbox = exchange.inbox(); // send to the UI thread

let mut frame = gpu.begin_frame("producer");
scene.render(frame.wgpu_encoder()); // wgpu-interop
frame.submit();
match exchange.copy_from(&rendered, epoch) {
    CopyOutcome::Submitted => {}
    CopyOutcome::PoolFull => {} // pool full: drop this frame without blocking
    CopyOutcome::EmptySource | CopyOutcome::IncompatibleSource | CopyOutcome::DeviceMismatch => {}
}
exchange.poll();

// UI window thread
let mut binding = FrameBinding::new(
    context.gpu(),
    textures.slot("program"),
    HostTextureAlphaMode::Premultiplied,
);
// Call in prepare_window_frame: binding.prepare(Some(&inbox), |token| accept(token));
// Call in window_frame_presented: binding.presented(Some(&inbox), |token| accept(token));
```

- **GPU-side copy and lock-free exchange**: `copy_from` performs one `copy_texture_to_texture` inside the texture pool (the source texture must have `COPY_SRC`, be single-sampled 2D, and be a non-depth format). For a single consuming window, allocate `3` slots (copy in flight, currently displayed, replaced and awaiting present); add `2` slots for each additional window sharing the consumer.
- **Lease order and epoch filtering**: after `prepare` swaps in a new frame, the old frame is kept until `presented` and only then returned; the producer reuses that slot only after the UI submission that sampled it completes. An application can advance the scene generation with `set_epoch` (immediately hiding frames of the old generation in the inbox; while no frame of the new generation has been published and `accept` still allows old frames, `FrameBinding` keeps the displayed content instead of flashing back to transparent blank).

## GpuView: same-pass direct drawing without an intermediate texture

When content has no intermediate texture and must be encoded directly into the current UI render pass, use `GpuView::new(slot_id)` (it projects a `CustomRenderNode` with renderer key `"gpu-view"`; see the `gpu-view-demo` example):

- **`Inline` vs `Standalone`**: `GpuViewMode::Inline` reuses the target pass the painter currently has open (setting viewport and scissor through `ScenePass`); `Standalone` opens a separate `Load / Store` pass on the same frame and target through `SceneGpuRenderContext::with_pass`. A custom renderer's output color must be **premultiplied linear scRGB**, and its pipeline cache key should be `(context.gpu.generation(), context.target_format, context.presentation)`.
- **Instanced batching that preserves document order**: if you implement `batch_capacity() > 1` and `draw_batch_in_pass`, the painter hands it a slice of **directly adjacent** nodes of the same renderer in the display list to draw at once. As soon as any ordinary Quad, text, icon, HostTexture, or composition group boundary appears in between, the slice ends, so occlusion order is bit-for-bit identical to drawing node by node. When several nodes share the same shader, use the same `(slot_id, revision)` and put per-node differences in `params`.
- **Implement `preparation_version` to allow frame caching**: custom renderers re-prepare every frame by default. **As long as one custom renderer in the tree does not implement `preparation_version`, the painter marks the whole frame as uncacheable and rebuilds all draw commands.** Implement it by `(revision, params)` as the built-in `DefaultGpuViewRenderer` does.

## Media and GPU slot contract

| Node / interface | Underlying slot contract | Behavior |
| --- | --- | --- |
| `GpuTextureView` | `"nana.host-texture"` + the slot name registered by the host | Standard host texture node |
| `GpuView` | `"gpu-view"` + decimal `slot_id` | The host must register a matching `SceneGpuRenderer` |
| `Video` | `"nana.host-texture"` + `video:{id}` | The host pushes frames (with a slot, `poster` is not drawn; without a slot only the static `poster` image is shown, and no audio/video is decoded) |
| `BrowserView` | Runtime layout anchor + host native child view | macOS `WKWebView` only (Windows/Linux return `Unsupported`); see the next section |

### Preparing content at the actual painted pixel size

`HostTextureRegistry::painted_extent(slot)` returns the largest device-pixel size the slot needed in the most recent complete paint of each render target (accounting for `ContentFit`, the node's own transform, and the current DPI scale factor; when the same slot is referenced by both a list thumbnail and a large image overlay, the largest consumer size is taken automatically). Producers can watch for size requirement changes through `painted_revision()`, `painted_changes_since(rev, &mut out)`, or `subscribe_painted(callback)` and prepare textures at the real physical resolution. Never compute it yourself as "last frame's `LayoutBox` × scale factor".

## In-app browser

`BrowserView` is a native child-view exception of the window host, not a full-window WebView shell: the Runtime node owns layout, accessibility, and the visibility anchor, while the host manages the native engine child view (currently only macOS `WKWebView` is supported, and it does not appear in offscreen Scene screenshots).

### Headless web content (WebSurface)

`WebSurface` runs a platform web engine instance entirely outside the window view tree, captures page frames at `max_fps` (`WebFrame`: tightly packed RGBA8, premultiplied alpha in sRGB space), and delivers them on a background thread to an application-provided `WebFrameSink`, with zero pixel conversion and zero queuing overhead on the UI thread. The application decides where frames go (for example, writing them into a `HostTexture` slot or a custom compositor):

- **Request reconciliation and in-place updates**: a program declares `WebSurfaceRequest { id, policy, desc, restore_url, revision, command, frames }` in `RuntimeProgram::web_surface_requests()` (withdrawing the request releases the engine), and status events flow back through `web_surface_event`. Changing `desc` (size, scale, transparency, frame rate) takes effect in place; changing the sink or the security policy rebuilds the instance.
- **Temporary interactive window (`ShowWindow`)**: the same headless page can be placed temporarily into an interactive native window (for completing a web login or clicking in-page settings). Cookies and page state are preserved and frames keep flowing meanwhile. After the user closes the window or `HideWindow` is sent, the page returns offscreen and reports `WindowClosed`.
- **Platform backends**: macOS is based on an offscreen borderless `WKWebView` (it detects 10 consecutive static frames and throttles to 4 fps, resuming immediately when content moves; measure it with `cargo run -p nana-window --example web-surface-probe`). Windows is based on a WebView2 Composition Controller on a dedicated STA thread + `Windows.Graphics.Capture` (requires the WebView2 Runtime installed on the system and `WebView2Loader.dll` shipped with the package). On Linux, `web_surface_support()` returns `false`.

## Per-scene offscreen production

When an offscreen texture must be encoded dynamically from the current Scene graph **before** it is sampled, still mount a `GpuTextureView` in the tree, and implement `SceneResourceProducer` on the host side:

- **Batched submission for visible frames**: `encode_scene(scene, &mut frame)` records directly into the same `FrameContext` the host opens after acquiring the Surface, and is submitted with the UI drawing in the same `queue.submit` (if either encoding or painting fails, the whole frame is discarded; producers must never submit the encoder themselves or report completion early).
- **Hidden tick**: when the window is occluded or minimized, a due `FrameDemand` still calls `prepare_window_frame`, runs `scene_resource_producers` on a separate Surface-less `FrameContext`, and submits immediately (at zero size it only prepares and does not run producer encoding).
- **One target per frame rule**: `FrameContext` exclusively manages a frame's submission and its discard rollback. If a previous frame painted some `RenderTargetId` and has been neither submitted nor discarded, painting that target again returns `ScenePaintError::TargetInFlight`. Device loss is recorded in the sticky `GpuContext::is_lost()`. When a device is injected externally through `EmbeddedRuntime`, the host should call `EmbeddedRuntime::notify_device_lost()` on receiving a device loss notification, and call `replace_gpu()` once the new device is ready.

### Preparation caching and content revisions for GPU nodes

`SceneGpuRenderer::preparation_version` only controls reuse of UI command preparation. A node that draws according to time or external state may change pixels every frame even when its resources and geometry do not; such renderers should return `None` from `content_version`. Window retained output, the off-window PresentationDriver, and Spout shared output redraw on host request accordingly, while still reusing the stable prepared commands.

Static or explicitly revised renderers can return `Some(revision)` and increment it when pixels change. To preserve existing consumer behavior, `content_version` defaults to `preparation_version`; dynamic renderers need to distinguish the two explicitly. A dynamic renderer that is installed but not referenced by the current scene does not keep static output painting continuously.

For general text and shape fragment materials, see [General Materials](materials.md).

### Per-frame application payloads

The `M` in `FrameExchange<E, M = ()>` is an immutable application payload published together with the GPU copy, separate from the epoch and `FrameToken<E>` identity. Construct it with `new_with_payload`; `copy_from_with_payload` (or `copy_from_wgpu_with_payload` under `wgpu-interop`) submits the texture, color metadata, and payload together. `FrameLease::payload()` and `FrameBinding<E, M>::payload()` return only the data of the corresponding lease / currently bound frame. The existing payload-free constructors and copy interfaces remain compatible.

The payload is kept together with the texture through pending copy, completed publication, binding, and retirement after presentation; dropped frames, generation invalidation, and slot recycling follow the existing rules. Updating a payload does not switch the epoch, does not clear the latest frame, and adds no history index or texture pool growth. Consumers must not pair an old texture with separately read latest producer state. The payload type is defined by the application; the framework does not depend on the application's coordinate or model types.
