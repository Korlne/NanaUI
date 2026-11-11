# NanaUI 的 wgpu fork

[English](README.md)

这里是 wgpu 30.0.1 及其组成 crate（`wgpu`、`wgpu-core`、`wgpu-core-deps-*`、`wgpu-hal`、`wgpu-types`、`wgpu-naga-bridge`、`naga`、`naga-types`）发布出来的源码，由 `scripts/vendor-wgpu.py` 放进来。NanaUI 在这里修改它们：把从 naga IR 到各后端的这一段（着色器与管线缓存、后端输出）握在自己手里，同时保留 wgpu 的校验与资源追踪。

包名和版本沿用上游，各 crate 之间按路径互相依赖，因此：

- **每个依赖图里只有这一份 wgpu。** `scripts/check-engine-boundary.py` 会拒绝来自别处的 wgpu 或 naga crate。
- **只用 NanaUI 的应用什么都不用做。** 以 git 依赖本仓库就会带上 fork；通过 `nana_gpu::wgpu`（feature `wgpu-interop`）使用 wgpu，不要直接依赖 `wgpu`。
- **还用到基于 crates.io wgpu 的库**（egui-wgpu、glyphon 等）的应用，要把 crates.io 指到本仓库，rev 与它的 NanaUI 依赖相同：

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

  这些库随后就基于 fork 编译；只要 fork 没改到它们用的接口，就能通过。NanaUI 自己的工作区也用路径这样处理它的 egui 示例。

## 修改 fork

- 公开的 `wgpu::` 接口尽量只增不改；不兼容的改动放在 `wgpu-core` / `wgpu-hal` 内部和新接口上。
- 每一处公开接口的改动都记在下面的"与上游的差异"里。
- 改动 naga 的 HLSL 输出，或 `wgpu-hal` 的 DX12 后端编译着色器的方式时，必须重新生成 `nana-ui` 里 `scene_paint::precompiled` 的预编译 DXIL：它的 wgpu 版本检查看不到 fork 的改动。
- 这些目录不在工作区成员里：它们自己的测试和 dev-dependencies 不在这里构建。

## 升级到新的上游版本

1. 运行 `python scripts/vendor-wgpu.py <版本>` 并提交。
2. 重新应用 fork 的改动：`git diff <上一次放入的提交> <最后一个 fork 提交> -- third_party/wgpu`。
3. 更新根 `Cargo.toml` 里的版本。

## 与上游的差异

### Surface 重配置只等这个 surface 的工作

`Surface::configure`（窗口缩放）不再等整个设备空闲。上游会先等所有队列提交完成，等待期间另一个线程提交就以 `ConfigureSurfaceError::GpuWaitTimeout` 失败，所以应用只好让所有队列工作与缩放串行。fork 只等仍在使用这个 surface 交换链图像的工作；校验不变（仍持有已获取的输出时报 `PreviousOutputExists`）。

- **`wgpu-core`**：每个 `Surface` 记下最后一次用到它纹理的提交序号（提交时盖章）。`configure_surface` 先取走 presentation，使等待期间不能再获取新纹理，然后只等这个序号，而不是 `wait_indefinitely`。期间其他线程的提交不再被等待，也不再让 configure 失败。`TextureInner::Surface` 多了 `last_submission` 字段（`wgpu-core` 的公开枚举；`wgpu` 不暴露它）。
- **`wgpu-hal` DX12**：队列在每次 `Present` 之后立即 signal 一个围栏；`configure` / `unconfigure` 在 `ResizeBuffers` / 释放前只等这个值，不再排空整个队列（设备上的 present 队列 idler 已删除）。
- **`wgpu-hal` Vulkan**：`configure` 不再调用 `vkDeviceWaitIdle`。旧交换链作为 `oldSwapchain` 传入后已退役，保留到绘制其后继交换链的某次提交完成为止（在下一次 acquire 时非阻塞检查）；`unconfigure` 与销毁仍未释放的退役交换链时仍等待设备空闲。
- **Metal、GLES**：它们的 `configure` 本来不等待，只受 `wgpu-core` 改动影响。

`wgpu` 的接口没有变化。`ConfigureSurfaceError::GpuWaitTimeout` 仍保留，在等待本身超时时返回。

### DX12 上的着色器与管线缓存

- **`wgpu::Device::set_default_pipeline_cache(Option<&PipelineCache>)`**（新增）。
  以 `cache: None` 创建的管线改用设备的默认缓存。设备只弱引用它（`PipelineCache`
  本身持有设备），应用留着句柄它才生效。别的设备的缓存是校验错误；没有缓存的后端忽略这个调用。
  背后是 `wgpu-core` 的 `Device::set_default_pipeline_cache` /
  `Global::device_set_default_pipeline_cache` 与锁等级 `DEVICE_DEFAULT_PIPELINE_CACHE`；
  dispatch trait 上的方法带空的默认实现，自定义后端照样编译。
- **DX12 上的 `Features::PIPELINE_CACHE`。** `wgpu-hal` 的 DX12 `PipelineCache`
  存放编译好的着色器字节码（DXC 的 DXIL、FXC 的 DXBC），键是 naga 写出的 HLSL、入口点、
  目标 profile 与编译选项的 128 位哈希。缓存里有的阶段不再经过编译器；编译出来的阶段加进去
  （设备内存缓存里已有的也加）。`create_pipeline_cache` 带 `data` 时载入，`PipelineCache::get_data`
  导出（格式见 `wgpu-hal/src/dx12/pipeline_cache.rs`：逐条校验和，连续 8 次保存未用到的条目丢弃）。
  字节码与适配器无关，所以 `wgpu-core` 的头部在 DX12 上用固定的适配器键，校验键写的是编译器
  （种类、版本和 DXC 提交）：缓存字节可以换适配器使用，不能换编译器。不缓存 PSO
  （`ID3D12PipelineLibrary`），驱动自有缓存。
