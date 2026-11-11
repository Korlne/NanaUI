# 局部呈现与 damage 跟踪

今天每一帧都重画并呈现整个窗口。WebGPU 没有局部呈现（partial present），所以光标闪烁、悬停这类只改几个像素的帧，也要付整窗绘制和整窗呈现的代价。本文是这件事的设计和实测：wgpu fork 一侧怎样带着脏矩形呈现，NanaUI 一侧怎样从 `UiScene` 的变化算出脏矩形、只重画受影响的区域。

**结论：现在不做。** 在实测机器上，一帧光标闪烁的全部 GPU 开销约 45 µs（NanaUI）加约 0.3 ms（DWM 合成），CPU 约 0.7 ms，而且大部分 CPU 不在绘制里。局部呈现最多省下 DWM 那一份和 NanaUI GPU 的一半。光标每秒只闪两次，省下的是每秒不到 1 ms 的 GPU 时间。fork 侧的改动又必须把 DX12 swapchain 从 `FLIP_DISCARD` 换成 `FLIP_SEQUENTIAL`，并穿过 wgpu 的六层接口。收益和代价不成比例。重新考虑的条件见末节。

## 实测（2026-11-11）

机器同 [实时画面 §6](./gpu.md)：Windows 11、RTX 5060 Ti 驱动 1920×1080 60 Hz 显示器，release 构建，DX12。

探针是 `crates/nana-ui/examples/partial-present-probe.rs`。它走 hosted GPU 路径开一个 1600×1000 的窗口（缩放 1.0），页面是一排 12 个按钮、一个 120×32 的盒子、一个 2×18 的光标条和 90 行约 80 字符的代码文字（约 7,000 个字形），然后每帧只改一样东西：

- `caret`：光标条在文字色和页面底色之间切换，也就是光标闪烁要求的那次重绘；
- `hover`：盒子在 surface 与 hover 填充之间切换；
- `still`：什么都不变，painter 直接用上一帧保留的 dest 纹理。

每帧都像今天的 scene host 一样整窗绘制、整窗呈现。负载有界：每个 60 Hz 周期一帧，下一帧开始前等这一帧在 GPU 上完成，跑满 30 s 必停。前 240 帧计 CPU（runtime flush、painter 录制、提交、acquire 等待、`present` 调用），后 240 帧在 painter 的工作两侧写 GPU 时间戳。运行时另一路每秒读一次 `\GPU Engine(*engtype_3D)\Utilization Percentage`，分别加总 `dwm.exe` 和探针进程。下表是两次运行的 p50；DWM 一列是四个一秒样本的范围，空闲桌面为 0.05–0.11%。

| 场景 | 改动区域（物理像素） | draw | flush | painter 录制 | 提交 | `present` | GPU（painter） | 探针 3D % | DWM 3D % |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `caret` | 3×18 | 4 | 0.068–0.080 ms | 0.116–0.132 ms | 0.34–0.35 ms | 0.18–0.19 ms | 0.045 ms | 0.35–0.63 | 1.2–3.0 |
| `hover` | 121×32 | 4 | 0.070 ms | 0.122 ms | 0.30 ms | 0.17 ms | 0.045 ms | 0.49–0.62 | 1.6–2.5 |
| `still` | 无 | 1 | 0.015 ms | 0.016 ms | 0.23–0.25 ms | 0.17 ms | 0.025 ms | 0.37–0.55 | 1.5–3.1 |

读法：

- 整窗重画 1600×1000、7,000 个字形，GPU 只要 45 µs。其中 25 µs 是 `still` 也要付的那次 dest → surface 的整窗 blit（带呈现色彩转换），真正重画页面的只有约 20 µs。
- 每帧都重建一次 batch（`batch_rebuilds` 每帧 1），CPU 侧 flush 加录制约 0.2 ms。它和局部呈现无关：脏矩形只能裁 GPU 的像素，不能让 batch 少建。这一项是 [文本引擎](./text-engine.md) 里说的「保留整帧绘制列表」，另算。
- 提交和 `present` 调用加起来约 0.5 ms，是驱动和 DXGI 的固定开销，和画了多少像素无关。
- DWM 的负载跟着呈现次数走，不跟着改动面积走：`still` 每帧呈现一张一模一样的画面，DWM 也和 `caret` 一样占 1.5–3%。按 60 次呈现每秒折算，DWM 每次整窗呈现约 0.25–0.5 ms GPU。脏矩形能省的就是这一份。

命令（仓库根目录，PowerShell；`S` 为临时目录，`run-probe.ps1` 只是启动探针并在运行期间读四次上面那个计数器）：

```powershell
cargo build -j 12 --release -p nana-ui --example partial-present-probe --features hosted,bundled-fonts,wgpu-interop
$env:WGPU_BACKEND = "dx12"
target\release\examples\partial-present-probe.exe caret   # 另跑 hover、still；输出一行 JSON
(Get-Counter '\GPU Engine(*engtype_3D)\Utilization Percentage').CounterSamples |
  Where-Object InstanceName -like "pid_$((Get-Process dwm).Id)_*"
```

没有测：Vulkan、Metal、GLES；核显驱动的显示器（这台机器的 Intel 核显没有接显示器，跨适配器呈现不具代表性）；4K 和高 DPI；DirectComposition 表面；HDR 的 fp16 工作目标；背景模糊和 GPU 动画在画面上的场景。

## 今天一帧怎么走

1. Runtime 把改动提交进 `UiWorld`，提取出 `UiScene`。
2. `SceneWgpuPainter::paint` 把场景排成 batch，画进 painter 自己保留的 **dest 纹理**（`scene_paint/dest.rs` 的 `DestTarget`，线性 scRGB 的工作格式，按需带 4× MSAA）。
3. dest 整张 blit 到 surface，blit 的着色器同时做呈现转换（`ScenePresentationProfile` 的色彩空间、HDR 参数、alpha 编码）。
4. 宿主 `present`。wgpu-hal 的 DX12 后端用 `FLIP_DISCARD` swapchain 和 `IDXGISwapChain3::Present`，没有脏矩形。

场景和视口都没变（`PaintedDest` 相同、没有活动的 GPU 动画）时，第 2 步整个跳过，只做第 3 步。这正是局部重画需要的前提：**上一帧的完整画面已经保留在 dest 里**，不必依赖 swapchain 图像保留内容，也不必把呈现转换搬进局部路径。

## 设计：fork 一侧

公开 API 只做加法，每一项记进 `third_party/wgpu/README.md` 的 *Changes from upstream*。

**接口。**

- `wgpu::Surface::supports_present_damage(&Adapter) -> bool`：这个表面在这个适配器上能不能带脏矩形呈现。
- 开启：`SurfaceConfiguration` 是公开的字面量结构，加字段会破坏现有写法，所以改成在 `configure` 之前调用 `Surface::set_present_damage(true)`。它只决定下一次 `configure` 选哪种 swapchain。
- 呈现：`Queue::present_with_damage(SurfaceTexture, &[PresentRect])`，`wgpu-types` 新增 `PresentRect { x, y, width, height }`（物理像素，左上原点）。空切片表示整帧。不支持或未开启时等同 `present`。
- 往下：`wgpu` 的 dispatch trait、`wgpu-core` 的 `Queue::present` 各加一个带矩形的版本；`wgpu-hal` 的 `Queue` trait 和 `DynQueue` 新增有默认实现（转调 `present`）的 `present_with_damage`，各后端按需覆盖。wgpu-core 负责把矩形裁到 surface 尺寸、丢掉空矩形；超过上限（建议 8 个）时合成一个外接矩形。

**DX12。** `IDXGISwapChain1::Present1`，`DXGI_PRESENT_PARAMETERS` 填 `DirtyRectsCount` 和 `pDirtyRects`。约束全部来自 DXGI：

- `FLIP_DISCARD` 不能用于局部呈现，必须是 `FLIP_SEQUENTIAL`。所以开启后 `configure` 改建 `FLIP_SEQUENTIAL`（窗口和 DirectComposition 两种表面都一样），不开启的表面不受影响。
- 每个脏矩形里的每个像素都必须是最新的，矩形外的像素不许改。
- swapchain 有 N 个缓冲时，第 k 帧拿到的后备缓冲停在第 k−N 帧，中间几帧的脏区都没画进去。
- 多重采样的 swapchain 不支持脏矩形（NanaUI 的 swapchain 本来就是单采样，MSAA 在 dest 上）。

后两条由 NanaUI 满足（下节）：blit 只写「本帧脏区 ∪ 前 N−1 帧脏区」，这个并集之外的像素在这个缓冲里已经是对的。`Present1` 失败时退回整帧 `Present` 并在下一帧当整帧处理。帧延迟 waitable、`ALLOW_TEARING` 和 `Present1` 正交，不用改。

**Vulkan。** 设备支持 `VK_KHR_incremental_present` 时启用它，在 `vkQueuePresentKHR` 的 `pNext` 里挂 `VkPresentRegionsKHR`。它只是提示：驱动可以忽略，图像内容也不保证保留。NanaUI 一侧的做法不变（从 dest 写并集区域），不依赖保留与否。Wayland 上 Mesa 把它变成 `wl_surface.damage_buffer`，这是 Linux 上真正省合成的地方。

**GLES。** `EGL_KHR_swap_buffers_with_damage`（或 `EGL_EXT_`），语义同 Vulkan。

**Metal。** `CAMetalLayer` 没有脏矩形接口，Core Animation 总是取整张 drawable 合成。呈现这一半在 Metal 上做不到；能省的只有 NanaUI 只重画 dest 的局部（下节第二阶段）。把常动的小区域拆进自己的子 layer 能让合成器只更新它，但那是另一套合成架构，不在本设计里。

## 设计：NanaUI 一侧

**damage 从哪来。** 以图元为单位比较上一帧和这一帧的 `UiScene`：图元稳定 id 相同而内容（几何、样式、文字、变换、裁剪、所在组）有变化的，damage 取它新旧两份绘制范围的并；新增和删除的图元取各自的范围；只是绘制顺序变了的，取所有换了顺序的图元的范围。绘制范围用 batch 已经在算的 `primitive_bounds`，再按阴影扩展、描边、抗锯齿外扩一像素、字形的墨迹溢出（ink overhang）放大，最后变换到 dest 的物理像素、裁到视口。

**一律当整帧的情况。** 第一帧；尺寸、缩放、`ScenePresentationProfile`、呈现参数（HDR headroom）改变；主题换了；字形图集被清空或重排；设备丢失、换代；某一帧录好了又被丢弃（painter 的 retained writes 回滚过）；damage 的面积超过视口的一半。

**层与组。** 有不透明度、混合模式或隔离合成的组整组一起合成，这一点不变。组里的改动按组的变换映射到 dest；组自身的不透明度、变换变化，damage 是组的整个范围（`group_extents`）。模态遮罩在卡片的不透明度组外面，各算各的。

**背景模糊。** 模糊元素读它后面的像素：半径为 r 的背景模糊元素 B，凡是 damage 与 B 的范围外扩 r 相交，B 的整个范围都加进 damage；B 自己变了，同样加 B 的整个范围。这条规则要迭代到不动点（加进去的 B 可能又碰到另一块模糊）。dest 里 backdrop 拷贝（`copy_backdrop`）只拷这片区域。

**动画。** CPU 侧的动画每帧提交新的样式，按上面的规则自然产生 damage。GPU 侧的动画（`motion_gpu_descriptors` 活动、`glyph_presentation_live`、`chart_presentation_live`）不经过场景比较：活动期间 damage 取这些节点在本帧可能扫过的范围；算不出时当整帧。第一阶段可以直接规定「有活动的 GPU 动画就整帧」，因为今天这类帧本来就整帧重画。合成器动画不能把采样值写回基础样式状态，damage 也只看呈现层，不改逻辑状态。

**局部重画 dest（第二阶段）。** dest 用 `LoadOp::Load` 打开，damage 矩形作 scissor，先在矩形里画一次清屏色的矩形，再按原顺序重放 batch 里与矩形相交的命令。有 MSAA 时多重采样附件不保留，但矩形里的每个像素都会从清屏开始重画，所以先清矩形再重画、只解析矩形区域就是对的。已有的「前缀保存与恢复」（`store_prefix`/`restore_prefix`）是同一思路的特例，可以并进来。

**呈现转换。** dest 仍是线性工作空间，转换照旧在 blit 里做。局部 blit 只是同一个管线加 scissor，blit 的区域是「本帧脏区 ∪ 前 N−1 帧脏区」（N 为 swapchain 缓冲数，painter 按 surface 记最近 N 帧的 damage）。呈现参数一变就整帧，所以并集里不会混进两套转换。

**矩形个数。** 每帧最多 4 个；多了就按「合并后多出的面积最少」两两合并。脏区总面积超过视口一半就整帧。

## 收益估算

按上表，一帧小改动能省的上限：

- NanaUI 的 GPU：45 µs → blit 只写几个小矩形、页面只重画矩形里的命令，接近 0。每帧省约 45 µs。
- DWM：每次整窗呈现约 0.25–0.5 ms → 只合成脏区，接近 0。
- CPU：不变（约 0.7 ms）。batch 照样重建，提交和 `present` 是固定开销。

光标每 500 ms 才切换一次，每秒两帧，加起来每秒省 0.6–1 ms GPU，在 60 Hz 的帧预算里不到 0.1%。悬停只在进出时有一两帧（带过渡动画也就十几帧）。真正每帧都在省的是**持续的小动画**：一个 60 fps 的加载转圈、进度条或实时小图表，每帧约 0.3–0.5 ms GPU，即这块独显上 2–3% 的帧时间。像素数和这差不多成正比，4K 窗口约是 1600×1000 的 5 倍；核显的带宽更紧。那时它才可能变成几个百分点以上，只是这台机器测不到。

## 分阶段计划

| 阶段 | 内容 | 省什么 | 前提 |
| --- | --- | --- | --- |
| 0（已完成） | `partial-present-probe` 与本文 | — | — |
| 1 | NanaUI 按场景比较算 damage（只做图元比较和「整帧」规则，GPU 动画、背景模糊一律整帧）；dest 仍整张重画；blit 写最近 N 帧 damage 的并集；fork 的 DX12 `Present1` 加 `FLIP_SEQUENTIAL` 开关 | DWM 合成；blit 的带宽 | 下面的重启条件之一成立 |
| 2 | dest 按 damage 局部重画（scissor 重放），背景模糊规则 | NanaUI 自己的 GPU 像素 | 阶段 1 之后，GPU 绘制在目标机器上占得到 1 ms 以上 |
| 3 | Vulkan `VK_KHR_incremental_present`、GLES swap-with-damage；GPU 动画按扫过范围算 damage | Linux/Android 的合成 | 有这些平台上的测量 |

阶段 1 是能独立见效的最小一步：fork 那半一个后端、NanaUI 那半只要比较图元，都可以单测；收益就是 DWM 那一份，用同一个探针加 DWM 计数器就能直接测到。它的风险在 `FLIP_SEQUENTIAL`：开启它的窗口都换了 swap effect，要在实际驱动上确认独立翻转（independent flip）和多平面叠加（MPO）是否照常生效，并重测 [实时画面 §6](./gpu.md) 的输入到呈现延迟。

**重启条件**（任一成立就从阶段 1 开始）：

- 产品里出现持续的小区域动画（转圈、进度、实时图表、视频缩略图），并且在核显或 4K 的目标机器上，用本探针加 DWM 计数器测到每次整窗呈现的 DWM 开销 ≥ 1 ms，或探针进程的 GPU 绘制 ≥ 1 ms；
- 有笔记本续航相关的反馈，且归因到常动的小区域；
- 远程桌面成为主要使用场景（RDP 只传脏区，局部呈现在那里直接省网络带宽）。
