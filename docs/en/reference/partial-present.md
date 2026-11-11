# Partial present and damage tracking

Today every frame repaints and presents the whole window. WebGPU has no partial present, so a caret blink or a hover, which change a few pixels, pay for a full-window paint and a full-window present. This page is the design and the measurement: how the wgpu fork presents with dirty rectangles, and how NanaUI derives dirty rectangles from `UiScene` changes and repaints only what they cover.

**Verdict: not now.** On the measured machine a caret-blink frame costs about 45 µs of GPU in NanaUI plus about 0.3 ms of DWM composition, and about 0.7 ms of CPU, most of which is not painting. Partial present can at most remove the DWM share and about half of NanaUI's GPU share. A caret toggles twice a second, so that is under 1 ms of GPU per second. The fork side also has to move the DX12 swapchain from `FLIP_DISCARD` to `FLIP_SEQUENTIAL` and thread a new call through six layers of wgpu. The gain does not pay for it. The conditions for revisiting it are in the last section.

## Measurement (2026-11-11)

Same machine as [Live GPU content §6](./gpu.md): Windows 11, RTX 5060 Ti driving a 1920×1080 60 Hz display, release build, DX12.

The probe is `crates/nana-ui/examples/partial-present-probe.rs`. It opens a 1600×1000 window (scale 1.0) on the hosted GPU path. The page holds a row of 12 buttons, a 120×32 box, a 2×18 caret bar and 90 lines of about 80 characters of code (about 7,000 glyphs). Each frame changes one thing:

- `caret`: the caret bar flips between the text colour and the page background, the repaint a blinking caret asks for;
- `hover`: the box flips between its surface and hover fill;
- `still`: nothing changes, so the painter reuses the dest texture it kept from the last frame.

Every frame is painted and presented in full, as the scene host does today. Load is bounded: one frame per 60 Hz period, each frame waited for on the GPU before the next starts, and the run stops after 30 s. The first 240 frames time the CPU (runtime flush, painter recording, submit, the wait in acquire, the `present` call); the next 240 write GPU timestamps around the painter's work. While it runs, `\GPU Engine(*engtype_3D)\Utilization Percentage` is read once a second and summed for `dwm.exe` and for the probe. The table gives p50s over two runs; the DWM column is the range of four one-second samples. An idle desktop reads 0.05–0.11%.

| Scene | Changed area (physical px) | Draws | Flush | Painter recording | Submit | `present` | GPU (painter) | Probe 3D % | DWM 3D % |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `caret` | 3×18 | 4 | 0.068–0.080 ms | 0.116–0.132 ms | 0.34–0.35 ms | 0.18–0.19 ms | 0.045 ms | 0.35–0.63 | 1.2–3.0 |
| `hover` | 121×32 | 4 | 0.070 ms | 0.122 ms | 0.30 ms | 0.17 ms | 0.045 ms | 0.49–0.62 | 1.6–2.5 |
| `still` | none | 1 | 0.015 ms | 0.016 ms | 0.23–0.25 ms | 0.17 ms | 0.025 ms | 0.37–0.55 | 1.5–3.1 |

How to read it:

- Repainting 1600×1000 with 7,000 glyphs takes 45 µs of GPU. 25 µs of that is the full-window dest → surface blit (with the presentation transfer) that `still` pays too; repainting the page itself is about 20 µs.
- The batch is rebuilt every frame (`batch_rebuilds` 1 per frame), and flush plus recording is about 0.2 ms of CPU. Partial present does not touch that: dirty rectangles clip GPU pixels, they do not build a smaller batch. That item is "retaining the whole frame's draw list" from the [text engine](./text-engine.md) page, a separate change.
- Submit plus the `present` call is about 0.5 ms, fixed driver and DXGI cost independent of how many pixels were drawn.
- DWM load follows the number of presents, not the changed area: `still` presents an identical image every frame and DWM sits at the same 1.5–3% as `caret`. At 60 presents a second that is about 0.25–0.5 ms of GPU per full-window present. That share is what dirty rectangles would save.

Commands (repository root, PowerShell; `S` is a scratch folder, and `run-probe.ps1` only starts the probe and reads the counter above four times while it runs):

```powershell
cargo build -j 12 --release -p nana-ui --example partial-present-probe --features hosted,bundled-fonts,wgpu-interop
$env:WGPU_BACKEND = "dx12"
target\release\examples\partial-present-probe.exe caret   # also hover, still; prints one JSON line
(Get-Counter '\GPU Engine(*engtype_3D)\Utilization Percentage').CounterSamples |
  Where-Object InstanceName -like "pid_$((Get-Process dwm).Id)_*"
```

Not measured: Vulkan, Metal, GLES; a display driven by an integrated GPU (this machine's Intel iGPU has no display attached, and cross-adapter present would not be representative); 4K and high DPI; DirectComposition surfaces; the fp16 HDR working target; scenes with backdrop blur or GPU animation on screen.

## How a frame goes today

1. The runtime commits changes into `UiWorld` and extracts a `UiScene`.
2. `SceneWgpuPainter::paint` batches the scene and draws it into the painter's own retained **dest texture** (`DestTarget` in `scene_paint/dest.rs`, the linear scRGB working format, with 4× MSAA when wanted).
3. The whole dest is blitted to the surface; the blit shader applies the presentation transfer (the `ScenePresentationProfile` colour space, HDR parameters, alpha encoding).
4. The host presents. wgpu-hal's DX12 backend uses a `FLIP_DISCARD` swapchain and `IDXGISwapChain3::Present`, with no dirty rectangles.

When neither scene nor viewport changed (same `PaintedDest`, no live GPU animation), step 2 is skipped and only step 3 runs. That is the precondition partial repaint needs: **the previous frame's complete image is already kept in dest**. It does not have to rely on swapchain images keeping their contents, and the presentation transfer does not have to move into a partial path.

## Design: the fork

Public API changes are additive, and each is listed under *Changes from upstream* in `third_party/wgpu/README.md`.

**Interface.**

- `wgpu::Surface::supports_present_damage(&Adapter) -> bool`: whether this surface on this adapter can present with dirty rectangles.
- Opting in: `SurfaceConfiguration` is a public struct built by literal, and a new field would break existing code, so instead `Surface::set_present_damage(true)` is called before `configure`. It only decides which swapchain the next `configure` builds.
- Presenting: `Queue::present_with_damage(SurfaceTexture, &[PresentRect])`, with a new `PresentRect { x, y, width, height }` in `wgpu-types` (physical pixels, top-left origin). An empty slice means the whole frame. Unsupported or not opted in, it is `present`.
- Below that: `wgpu`'s dispatch traits and `wgpu-core`'s `Queue::present` each get a variant with rectangles; `wgpu-hal`'s `Queue` trait and `DynQueue` get a `present_with_damage` with a default implementation (calling `present`) that backends override. wgpu-core clips rectangles to the surface, drops empty ones, and above a limit (8 suggested) replaces them with their bounding box.

**DX12.** `IDXGISwapChain1::Present1` with `DirtyRectsCount` and `pDirtyRects` in `DXGI_PRESENT_PARAMETERS`. The constraints all come from DXGI:

- `FLIP_DISCARD` cannot be used with partial presentation; it has to be `FLIP_SEQUENTIAL`. So an opted-in `configure` builds `FLIP_SEQUENTIAL` (for both window and DirectComposition surfaces), and surfaces that do not opt in are untouched.
- Every pixel inside each dirty rectangle must be up to date, and no pixel outside them may change.
- With N buffers, the back buffer handed out at frame k still holds frame k−N; the dirty areas of the frames in between are not in it.
- Multisampled swapchains do not support dirty rectangles (NanaUI's swapchain is single-sampled already; MSAA lives in dest).

NanaUI meets the last two (next section): the blit writes only "this frame's damage ∪ the previous N−1 frames' damage", and outside that union the buffer's pixels are already right. If `Present1` fails, fall back to a full `Present` and treat the next frame as full. The frame-latency waitable and `ALLOW_TEARING` are orthogonal to `Present1` and stay as they are.

**Vulkan.** Enable `VK_KHR_incremental_present` where the device has it and chain `VkPresentRegionsKHR` into `vkQueuePresentKHR`. It is a hint: the driver may ignore it, and image contents are not guaranteed to persist. NanaUI does the same either way (write the union from dest) and does not depend on persistence. On Wayland, Mesa turns it into `wl_surface.damage_buffer`, which is where Linux actually saves composition.

**GLES.** `EGL_KHR_swap_buffers_with_damage` (or the `EGL_EXT_` form), same semantics as Vulkan.

**Metal.** `CAMetalLayer` has no dirty-rectangle interface; Core Animation always composites the whole drawable. The present half cannot be done on Metal; what remains is NanaUI repainting only part of dest (stage 2 below). Moving a constantly animating small region into its own sublayer would let the compositor update only that layer, but that is a different composition architecture and outside this design.

## Design: NanaUI

**Where damage comes from.** Compare this frame's `UiScene` with the last one primitive by primitive. A primitive with the same stable id whose content changed (geometry, style, text, transform, clip, containing group) contributes the union of its old and new paint bounds; added and removed primitives contribute their bounds; primitives that only changed draw order contribute the bounds of every reordered primitive. Paint bounds are the `primitive_bounds` the batch already computes, inflated for shadow spread, outlines, one pixel of anti-aliasing and glyph ink overhang, then transformed into dest's physical pixels and clipped to the viewport.

**Always a full frame.** The first frame; a change of size, scale, `ScenePresentationProfile` or presentation parameters (HDR headroom); a theme change; the glyph atlas being cleared or repacked; device loss or a new device generation; a frame that was recorded and then dropped (the painter's retained writes rolled back); damage covering more than half the viewport.

**Layers and groups.** A group with opacity, a blend mode or isolated composition is still composited as one. Changes inside a group are mapped through the group's transform into dest; a change of the group's own opacity or transform damages the group's whole extent (`group_extents`). A modal scrim sits outside its card's opacity group and is damaged on its own.

**Backdrop blur.** A blur reads the pixels behind it. For a backdrop-blur element B with radius r, any damage intersecting B's extent inflated by r adds B's whole extent; a change to B itself does the same. Iterate to a fixed point (an added B may reach another blur). The backdrop copy in dest (`copy_backdrop`) copies only that area.

**Animation.** CPU-side animation commits new style every frame and produces damage by the rules above. GPU-side animation (live `motion_gpu_descriptors`, `glyph_presentation_live`, `chart_presentation_live`) bypasses the scene comparison: while it is live, damage is the area those nodes can sweep this frame, or the full frame when that cannot be bounded. Stage 1 can simply say "any live GPU animation means a full frame", since those frames repaint in full today anyway. Compositor animation never writes samples back into base style, and damage reads only the presentation layer, never logical state.

**Partial repaint of dest (stage 2).** Open dest with `LoadOp::Load`, use the damage rectangle as the scissor, draw the clear colour over the rectangle, then replay, in order, the batch commands that intersect it. The MSAA attachment is not kept between frames, but every pixel in the rectangle is redrawn starting from the clear, so clearing the rectangle, redrawing and resolving only the rectangle is correct. The existing prefix store/restore (`store_prefix`/`restore_prefix`) is a special case of the same idea and can fold into it.

**Presentation transfer.** dest stays in the linear working space and the transfer stays in the blit. A partial blit is the same pipeline with a scissor, covering "this frame's damage ∪ the previous N−1 frames' damage" (N is the swapchain buffer count; the painter keeps the last N damages per surface). A change of presentation parameters is a full frame, so the union never mixes two transfers.

**Rectangle count.** At most 4 per frame; beyond that, merge the pair whose bounding box adds the least area. Above half the viewport, present the full frame.

## Estimated gain

From the table, the most a small-change frame can save:

- NanaUI GPU: 45 µs → with the blit writing a few small rectangles and the page replaying only the commands inside them, close to 0. About 45 µs a frame.
- DWM: about 0.25–0.5 ms per full-window present → composing only the damage, close to 0.
- CPU: unchanged (about 0.7 ms). The batch is still rebuilt, and submit and `present` are fixed costs.

A caret toggles every 500 ms, two frames a second: 0.6–1 ms of GPU saved per second, under 0.1% of a 60 Hz budget. A hover produces one or two frames on enter and leave (a dozen or so with a transition). What saves on every frame is a **continuous small animation**: a 60 fps spinner, progress bar or live mini-chart costs about 0.3–0.5 ms of GPU a frame, 2–3% of the frame time on this discrete GPU. That scales roughly with pixel count — a 4K window is about 5× 1600×1000 — and integrated GPUs have less bandwidth. That is where it could reach several percent, and this machine cannot measure it.

## Staged plan

| Stage | Content | Saves | Precondition |
| --- | --- | --- | --- |
| 0 (done) | `partial-present-probe` and this page | — | — |
| 1 | NanaUI computes damage by scene comparison (primitive comparison and the full-frame rules only; GPU animation and backdrop blur always full); dest still repainted in full; the blit writes the union of the last N damages; the fork's DX12 `Present1` with a `FLIP_SEQUENTIAL` opt-in | DWM composition; blit bandwidth | One of the revisit conditions below holds |
| 2 | Partial repaint of dest (scissored replay), the backdrop-blur rule | NanaUI's own GPU pixels | After stage 1, and GPU painting reaches 1 ms or more on the target machines |
| 3 | Vulkan `VK_KHR_incremental_present`, GLES swap-with-damage; GPU animation damage by swept area | Composition on Linux/Android | Measurements on those platforms |

Stage 1 is the smallest step that pays on its own: the fork half is one backend, the NanaUI half is only primitive comparison, both can be unit-tested, and the gain is exactly the DWM share, which the same probe plus the DWM counter measures directly. Its risk is `FLIP_SEQUENTIAL`: opted-in windows change swap effect, so independent flip and multiplane overlay (MPO) have to be confirmed on real drivers, and the input-to-present latency from [Live GPU content §6](./gpu.md) re-measured.

**Revisit when** any of these holds (start at stage 1):

- The product has continuous small-area animation (spinners, progress, live charts, video thumbnails), and on integrated-GPU or 4K target machines this probe plus the DWM counter measures ≥ 1 ms of DWM per full-window present, or ≥ 1 ms of GPU painting in the app;
- Laptop battery complaints trace back to a constantly animating small area;
- Remote Desktop becomes a main use (RDP sends only dirty areas, so partial present saves network bandwidth there directly).
