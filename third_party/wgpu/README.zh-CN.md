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

暂无：目前就是发布出来的 wgpu 30.0.1。
