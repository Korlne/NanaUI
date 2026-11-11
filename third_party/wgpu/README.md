# NanaUI's wgpu fork

[中文](README.zh-CN.md)

These are the published sources of wgpu 30.0.1 and the crates it is built from
(`wgpu`, `wgpu-core`, `wgpu-core-deps-*`, `wgpu-hal`, `wgpu-types`,
`wgpu-naga-bridge`, `naga`, `naga-types`), vendored by
`scripts/vendor-wgpu.py`. NanaUI changes them here: owning the path from naga
IR to each backend (shader and pipeline caches, backend output) without giving
up wgpu's validation and resource tracking.

Package names and versions stay upstream's, and the crates depend on each
other by path, so:

- **Every graph holds this one wgpu.** `scripts/check-engine-boundary.py`
  rejects a wgpu or naga crate from anywhere else.
- **An application that only uses NanaUI needs nothing.** A git dependency on
  this repository brings the fork; reach wgpu through `nana_gpu::wgpu`
  (feature `wgpu-interop`) instead of depending on `wgpu` directly.
- **An application that also uses a crate built on crates.io wgpu** (egui-wgpu,
  glyphon, ...) patches crates.io onto this repository, at the same rev as its
  NanaUI dependencies:

  ```toml
  [patch.crates-io]
  naga = { git = "https://github.com/sena-nana/NanaUI.git", rev = "<rev>" }
  naga-types = { git = "https://github.com/sena-nana/NanaUI.git", rev = "<rev>" }
  wgpu = { git = "https://github.com/sena-nana/NanaUI.git", rev = "<rev>" }
  wgpu-core = { git = "https://github.com/sena-nana/NanaUI.git", rev = "<rev>" }
  wgpu-core-deps-apple = { git = "https://github.com/sena-nana/NanaUI.git", rev = "<rev>" }
  wgpu-core-deps-emscripten = { git = "https://github.com/sena-nana/NanaUI.git", rev = "<rev>" }
  wgpu-core-deps-windows-linux-android = { git = "https://github.com/sena-nana/NanaUI.git", rev = "<rev>" }
  wgpu-hal = { git = "https://github.com/sena-nana/NanaUI.git", rev = "<rev>" }
  wgpu-naga-bridge = { git = "https://github.com/sena-nana/NanaUI.git", rev = "<rev>" }
  wgpu-types = { git = "https://github.com/sena-nana/NanaUI.git", rev = "<rev>" }
  ```

  Those crates then build against the fork; they compile as long as the fork
  has not changed an API they use. NanaUI's own workspace does the same with
  paths for its egui example.

## Changing the fork

- Keep the public `wgpu::` API additive where possible; put breaking changes
  in `wgpu-core` / `wgpu-hal` internals and in new APIs.
- List every change to a public API under *Changes from upstream* below.
- A change to naga's HLSL output or to how `wgpu-hal`'s DX12 backend compiles
  shaders must regenerate the precompiled DXIL in `nana-ui`'s
  `scene_paint::precompiled`: its wgpu version check cannot see fork changes.
- These directories are excluded from the workspace: their own tests and
  dev-dependencies do not build here.

## Moving to a new upstream version

1. `python scripts/vendor-wgpu.py <version>` and commit.
2. Re-apply the fork's changes: `git diff <previous vendoring commit> <last fork commit> -- third_party/wgpu`.
3. Update the version in the root `Cargo.toml`.

## Changes from upstream

### Surface reconfiguration waits for that surface only

`Surface::configure` (a window resize) no longer waits for the whole device to
go idle. Upstream drains every queue submission first and fails with
`ConfigureSurfaceError::GpuWaitTimeout` when another thread submits during
that wait, so applications serialized all queue work against resizes. The fork
waits only for the work that still uses that surface's swapchain images;
validation is unchanged (an output still acquired is `PreviousOutputExists`).

- **`wgpu-core`**: each `Surface` keeps the index of the last submission that
  used one of its textures (stamped at submit). `configure_surface` takes the
  presentation first, so no new texture can be acquired, then waits for that
  index instead of `wait_indefinitely`. Submissions made meanwhile from other
  threads are not waited for and no longer fail the configure.
  `TextureInner::Surface` gains a `last_submission` field (a public enum of
  `wgpu-core`; nothing in `wgpu` exposes it).
- **`wgpu-hal` DX12**: the queue signals a fence right after each `Present`;
  `configure` / `unconfigure` wait for that value before `ResizeBuffers` /
  release, instead of draining the queue (the device's present-queue idler is
  gone).
- **`wgpu-hal` Vulkan**: `configure` no longer calls `vkDeviceWaitIdle`. The
  old swapchain, passed as `oldSwapchain` and therefore retired, is kept until
  a submission that drew to its successor completes (checked without
  blocking at the next acquire); `unconfigure` and dropping a retired
  swapchain still wait for the device.
- **Metal, GLES**: their `configure` did not wait; they get the `wgpu-core`
  change only.

The `wgpu` API is unchanged. `ConfigureSurfaceError::GpuWaitTimeout` stays and
is still returned when the wait itself times out.

### Shader and pipeline cache on DX12

- **`wgpu::Device::set_default_pipeline_cache(Option<&PipelineCache>)`** (new).
  A pipeline created with `cache: None` uses the device's default cache. The
  device holds it weakly (a `PipelineCache` holds its device), so it applies
  while the application keeps the handle. A cache from another device is a
  validation error; backends without caches ignore the call. Behind it:
  `wgpu-core` `Device::set_default_pipeline_cache` /
  `Global::device_set_default_pipeline_cache` and lock rank
  `DEVICE_DEFAULT_PIPELINE_CACHE`; the dispatch trait method has a no-op
  default, so custom backends still compile.
- **`Features::PIPELINE_CACHE` on DX12.** `wgpu-hal`'s DX12 `PipelineCache` holds
  compiled shader bytecode (DXIL from DXC, DXBC from FXC) keyed by a 128-bit
  hash of the HLSL naga wrote, the entry point, the target profile and the
  compile flags. A stage found there skips the compiler; a stage compiled is
  added to it (also one the device's in-memory cache already had).
  `create_pipeline_cache` with `data` loads it and `PipelineCache::get_data`
  saves it (format in `wgpu-hal/src/dx12/pipeline_cache.rs`: per-entry
  checksums, unused entries dropped after 8 saves). Bytecode does not depend
  on the adapter, so `wgpu-core`'s header uses a constant adapter key on DX12
  and the validation key names the compiler (kind, version and DXC commit):
  cache bytes move between adapters but not between compilers. PSOs are not
  cached (`ID3D12PipelineLibrary`); drivers keep their own.
