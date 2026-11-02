# 资源预算：栈、内存与性能

这一页是框架在栈、常驻内存和 CPU 时间上的合同：每一项给出预算、守住它的门禁、量它的命令，以及能在同一台机器上跑出来的同类实现对照。没有门禁的数字只是观察，不写成预算；没跑过的对照不写数字。

## 栈

### 运行环境

框架的帧路径（挂载、样式、布局、发布、命中索引、无障碍、场景提取、析构）跑在宿主给的线程上，线程栈由平台决定：

| 线程 | 栈 |
| --- | ---: |
| Windows 主线程（MSVC 默认链接） | 1 MiB |
| macOS / iOS 非主线程 | 512 KiB |
| Rust `std::thread::spawn` 默认 | 2 MiB |
| Linux、Android 主线程 | 8 MiB（`ulimit -s`） |

所以预算按最紧的一档定：一整帧，release 和 debug 都必须在 **256 KiB** 内跑完（512 KiB 线程留一半余量）。

### 树深上限

一个节点最多在第 `MAX_TREE_DEPTH = 512` 层（`nana_ui_runtime::MAX_TREE_DEPTH`）；超过时挂载返回 `UiWorldError::TreeTooDeep`，不会走到任何递归。Blink 的 HTML 解析器对 DOM 嵌套用的是同一个上限。

### 规则：哪些遍历可以递归

1. **测量**和**视图建树**是仅有的两处按树深递归的地方。测量里尺寸要从约束传下去的子项里算上来，建树时一个元素要在自己的作用域里建子元素，二者都没有不改变语义的迭代写法。每一层都必须经过兜底：测量走 `layout_engine::recursion::level`，建树走 `crate::stack::level`。剩余栈不足 256 KiB 时这一层换到新分配的 4 MiB 栈段上继续（`stacker::grow`）。剩余栈在桌面平台和 OpenHarmony（Linux 目标）由 `stacker` 读取，在 iOS 和 Android 上读线程自己的栈边界（`pthread_get_stackaddr_np` / `pthread_getattr_np`）——`stacker` 读不到这两个平台，此前兜底在那里不生效。所以兜底在框架支持的每个平台上都生效。
2. **放置**是有界递归：一次调用最多下降 `recursion::SPAN` 层（release 24 层、debug 8 层，两种构建都约 90 KiB），再往下的节点只写下自己的盒子，子树作为边界排进本趟队列，由最外层调用按顺序放完再返回。递归深度与树深无关，放置不需要兜底。见 [布局 · 布局递归与栈](layout.md#布局递归与栈)。
3. **其余所有树遍历**都必须用循环或堆上的显式栈，不得按深度递归。这次改掉的：命中索引建树（`HitIndex::insert_tree`）、命中查询（`HitIndex::visit_hits`）与命中树计数（`count_hit_entries`）、样式解析沿祖先链（`resolve_style`）、行盒查询（`holds_line_box_in`）、原子基线沿祖先上溯（`atom_baseline_reads_content`）。新增遍历时用 `scripts/audit-stack-frames.py` 和下面的门禁验证。
4. **应用自己的嵌套模型**分两类，见下「应用模型的嵌套」：深度没有上限的（树视图的数据）一律不按深度递归；面板切分树（Dock、PaneTree、区域布局）有结构上限 `MAX_SPLIT_DEPTH`，在上限之内递归。
5. **解析运行时内容**（Markdown、Mermaid 图、CSS、ICU 消息、绘制脚本）的递归跟着输入的嵌套深度走，所以每一处都有深度上限或改成了循环，见下「运行时内容的嵌套」。超出上限时返回解析错误、跳过那一段或按纯文本降级，不会 panic。

### 运行时内容的嵌套

| 解析器 | 上限 | 超出时 |
| --- | --- | --- |
| Mermaid：zenuml 花括号 | `MAX_BRACES` 64 | `TooLarge(ClusterDepth)` |
| Mermaid：railroad 分组与调用 | `MAX_DEPTH` 48 | `TooLarge(ClusterDepth)` |
| Mermaid：block、c4、state、class、sequence、architecture、flowchart 的容器 | `Limits::cluster_depth` 16，容器总数 `Limits::clusters` 300 | `TooLarge(ClusterDepth)` / `TooLarge(Clusters)` |
| Mermaid：usecase 的 JSON 表 | 32 | 警告，不成表 |
| Mermaid：`%%{init}%%` 配置 | 8 层 | 更深的对象整段跳过，后面的键照读 |
| Mermaid：缩进树（mindmap、treeView、treemap、ishikawa、kanban） | 源码预算（约 490 层） | 布局不按层递归，无需上限 |
| Markdown（`rich_text`） | 无 | 块是扁平的，`pulldown-cmark` 不递归，`layout_block` 只多一层 |
| ICU 消息（`i18n::message`） | 16 层 | 解析错误，按原文显示 |
| 绘制脚本的路径对象（`paint_script::parse_path`） | 无 | 循环展开；JSON 本身由 `serde_json` 限 128 层 |
| CSS：`@media` / `@container` / `@supports` / `@layer` 块 | `MAX_NESTING` 32 | 跳过那一块，计入 `skipped_at_rules` |
| CSS：容器查询与 `@supports` 条件的括号、`color-mix()` | `MAX_NESTING` 32 | 查询 `Invalid` / 条件未知 / 颜色无效 |
| CSS：`calc()` 的括号与数学函数 | `MAX_CALC_DEPTH` 16 | 值无效 |
| CSS：`var()` 引用链 | `MAX_VAR_DEPTH` 32 | 当作未解析：用回退值；自引用 `--a: var(--a)` 也在此终止 |

每个解析器都有一条测试，在 512 KiB 的线程上解析一万层（缩进树取源码预算能装下的最深）：`nana-ui-diagram/tests/deep_nesting.rs`、`nana-ui-css/tests/deep_nesting.rs`，以及 `rich_text`、`i18n::message`、`paint_script` 各自的单元测试。上限之内的深度同样要受栈预算约束：CSS 嵌套块每层的帧在 debug 下约 6.5 KiB（样式规则的处理拆到了 `parse_style_rule`，不在递归的帧里），32 层约 210 KiB；railroad 48 层在 debug 下要 256–512 KiB，是这些解析器里最紧的。

### 应用模型的嵌套

| 模型 | 深度 | 怎么不溢出 |
| --- | --- | --- |
| `TreeNode`（`TreeView` 的数据：文件树、JSON 文档） | 无上限 | 遍历走 `nana_ui_core::tree_preorder`（堆上开栈的先序迭代器，顺序与递归相同）或堆上显式栈；`TreeNode` 的 `Clone`、`PartialEq` 与 `Drop` 手写为不递归（析构前先把后代摘出来）。只有派生的 `Debug` 仍按层嵌套 |
| `DockNode`、`PaneTreeNode`、`AreaNode`（面板切分树） | `MAX_SPLIT_DEPTH` 64 | 框架自己的切分（Dock 拖放到边上、`AreaLayoutModel::split`）到上限时拒绝；上限深度的布局能经 `serde_json` 保存并读回。应用自己构造的更深的树（含手改的布局文件，最深约 125 层），在进入框架时折叠到上限：经构造函数（`Dock::new`、`DockWorkspace::new`、`PaneTree::new`、`AreaLayout::new`、持久化恢复），以及这三种组件每一次创建和更新（`ComponentView::fit_framework_limits`，所以直接写公开字段也逃不掉）。Dock 把越界子树里的所有项收进一个标签组，一项不丢；PaneTree 和区域布局保留越界切分的第一个面板或区域，其余丢弃（区域布局同时从 `areas` 和 `maximized` 里移除它们）。折叠本身不递归。框架内部的遍历因此只在不超过上限的树上递归；应用自己持有、自己遍历的超深值不在框架管辖内 |
| 内置组件的 JSON 规格（`tree_node_from_json`、`dock_node_from_json`） | `serde_json` 的 128 层 | 遍历的是解析好的 `serde_json::Value`，深度已被解析器封顶 |

测试：`tree_view::tests::data_deeper_than_the_stack_walks_clones_compares_and_drops`（两万层数据在 512 KiB 线程上列行、选中、导航、展开折叠、克隆、比较、析构；去掉迭代析构时栈溢出），`nana-ui-runtime/tests/deep_models.rs`（上限深度的 Dock、区域布局、PaneTree 在 512 KiB 线程上克隆、比较、放置、保存、读回、析构，外加手写的最深 Dock JSON），`dock::tests::a_drop_past_the_split_depth_is_refused`、`area_layout::model::tests::a_split_past_the_split_depth_is_refused`，以及 `deep_models.rs` 里五千层的应用自建树：经构造函数、直接写公开字段后创建并布局、在更新里写入，三条路都在 512 KiB 线程上通过；去掉组件钩子即栈溢出（已做变异实验）。

### 门禁

| 门禁 | 守的是什么 | 命令 |
| --- | --- | --- |
| `nana-ui-scene/tests/deep_tree_stack.rs` | 508 层（上限减 4）的树在 256 KiB 的线程上（release 与 debug 都是）走完整帧：挂载、布局、提取、命中、无障碍、改最深处的文本再走一帧、析构 | `cargo test -p nana-ui-scene --test deep_tree_stack`，再加 `--release` |
| `layout_engine::stack_budget` | 测量和放置的每层栈不超预算（debug 16 / 12 KiB，release 4 / 4 KiB），400 层的树在 1 MiB 线程上布局 | `cargo test -p nana-ui-runtime --lib stack_budget` |
| `view::tests::a_view_deeper_than_the_stack_still_mounts` | 400 层视图在 1 MiB 线程上挂载、布局、析构 | `cargo test -p nana-ui-runtime --lib a_view_deeper` |
| `scripts/audit-stack-frames.py` | 单个函数的栈帧 | 见下 |

`WorkCounters` 发布 `layout_measure_stack_per_level`、`layout_placement_stack_per_level`（每层的栈，字节，一帧取最大）和 `layout_placement_boundaries`（放置从队列里放了几棵子树）。

### 单个函数的栈帧

一个函数的帧是每次调用它都要付的，递归与否都一样。`scripts/audit-stack-frames.py` 读二进制里每个函数序言减掉的栈指针，列出超过预算的函数：

```bash
cargo test --release -p nana-ui-scene --test deep_tree_stack --no-run
python3 scripts/audit-stack-frames.py target/release/deps/deep_tree_stack-<hash> --budget 16384
```

预算：帧路径上的函数 release 不超过 16 KiB。2026-11-02 在 aarch64 macOS 上量到的超标项，都是只在启动时调用一次的构造路径，不在每帧路径上：

| 函数 | release | debug |
| --- | ---: | ---: |
| `AppContext::from_world` | 35 KiB | — |
| `RuntimeDocument::with_context` / `new` | 21 / 18 KiB | 19 / 18 KiB |
| `AppContext::default` | 14 KiB | — |

它们大，是因为 `AppContext`（18 KiB）和 `UiWorld`（14 KiB）按值内联在栈上、构造时搬了几次。

debug 下帧路径上曾有几个函数远超预算：`UiWorld::derive_component_geometry` 111 KiB、`UiScene::build_node_primitives` 74 KiB、`controls_base::build` 72 KiB、`overlays::build` 28 KiB、`text::build` 19 KiB（release 分别是 9、5、6 KiB 以下）。它们都是按 `StandardVisual` 分支的大 `match`，未优化的代码给每个分支的局部变量都留了槽，走哪个分支都要全占。现在每个 12 行以上的分支都在 `own_frame(|| ...)` 里运行（runtime 的 `crate::stack::own_frame` 和 scene 的 `primitives::own_frame`，`#[inline(always)]` 的一层调用）：debug 下闭包是独立的帧，分支的局部变量只在走到它时才占栈；release 下闭包被内联，代码不变。debug 下这几个函数现在是 16、24、16 KiB 以下，最大的分支闭包 20 与 29 KiB。新加的大 `match` 分支照此办理。

### 与同类实现的对照

`perf/micro/stack-depth` 在同一台机器上量「完成一次布局最少要多大的线程栈」，每次探测在子进程里跑、二分到 4 KiB。NanaUI 与 [Taffy](https://github.com/DioxusLabs/taffy) 0.11（Bevy、Dioxus、Zed 等使用的 Rust 布局引擎）布局同一串嵌套的 flex 列：

```bash
cd perf/micro/stack-depth && cargo run --release -- report
```

| 深度 | NanaUI 布局 | NanaUI 整帧 | Taffy 布局 |
| ---: | ---: | ---: | ---: |
| 64 | 116 KiB | 116 KiB | 132 KiB |
| 256 | 116 KiB | 116 KiB | 484 KiB |
| 500 | 116 KiB | 116 KiB | 916 KiB |

（release，aarch64 macOS，2026-11-02。）NanaUI 要的栈与深度无关：放置在线程栈上最多走 `SPAN` 层（每层约 3.5 KiB），测量和建树在剩余栈不足 256 KiB 时换到新栈段，所以这个数基本就是放置的 `SPAN` 层加上帧路径的固定开销。Taffy 每层约 1.8 KiB，500 层在 512 KiB 的线程上会溢出。

## 常驻内存

`nana-ui-runtime/tests/node_memory.rs`（`perf/scenarios/catalog.json` 里的 `component-memory` 门禁）量布局后每个节点常驻的堆（分配减释放，按线程计）：Text 不超过 4.2 KB、Button 5.8 KB、ListItem 6.4 KB，1k 与 10k 节点下每节点数不随规模上涨。

对照（`cargo run --release -- wide`，一个 1 列 × N 行 × 10 格的页面，每格一列套一行；「布局分配」是 `layout_document` 一次调用里的分配次数）：

| 盒子数 | NanaUI 每盒 | NanaUI 布局分配 / 盒 | Taffy 每盒 | Taffy 布局分配 / 盒 |
| ---: | ---: | ---: | ---: | ---: |
| 2 101 | 3.5 KB（原 3.7） | 25.5（原 32.7） | 0.9 KB | 6.9 |
| 10 501 | 3.0 KB（原 3.2） | 25.4（原 32.6） | 1.5 KB | 6.9 |
| 21 001 | 3.0 KB（原 3.2） | 25.3（原 32.6） | 1.5 KB | 6.9 |

（release，aarch64 macOS，2026-11-02，与改动前的二进制交替 8 轮。）二者存的东西不同：Taffy 只存布局样式与布局缓存；NanaUI 的节点还带组件记录、解析后的样式、保留的布局结果、失效与依赖索引和无障碍状态。10 501 盒时 NanaUI 每盒约 3.0 KB 的去向（逐项临时关掉再量的差值）：

| 部分 | 每盒 |
| --- | ---: |
| 布局之前就有的节点本体（记录、组件、样式引用、各索引） | ≈ 1.5 KB |
| 发布的 `LayoutResult`（结构 280 B，外加父节点里每个子节点各一份放置、片段、部件和依赖，约 160 B） | ≈ 0.55 KB |
| 保留的意图与用尺寸缓存（`intrinsics` 每条 112 B 的两槽、`intrinsic_metrics`） | ≈ 0.4 KB |
| 容器计划与测量计划 | ≈ 0.2 KB |
| 盒子、内边距、上下文表 | ≈ 0.2 KB |
| 整页快照（`full_snapshots`，盒子与三张表的副本） | ≈ 0.1 KB |

即便口径不同，每盒约 2 倍仍是真实差距，见「待办」。

## CPU 时间

增量帧的成本由 `perf/` 的性能合同管（`perf/README.md`）：静止帧不做功、paint-only 不重排、一处编辑不扫兄弟，判据是 `WorkCounters`，不是墙钟。

冷启动的整页布局没有门禁，只有观察（同上的 `wide`，release，与改动前的二进制交替 8 轮读最小值）：

| 盒子数 | NanaUI | NanaUI 改动前 | Taffy |
| ---: | ---: | ---: | ---: |
| 2 101 | 1.78 µs / 盒 | 2.01 µs / 盒 | 0.67 µs / 盒 |
| 10 501 | 2.22 µs / 盒 | 2.56 µs / 盒 | 0.71 µs / 盒 |
| 21 001 | 2.64 µs / 盒 | 2.93 µs / 盒 | 0.73 µs / 盒 |

NanaUI 的计时含布局写回的提交与结果发布（Taffy 没有对应步骤）。10 501 盒时一次布局约 39% 在放置、26% 在测量、18% 在两次发布、其余是写回与收尾。深链（`report`，同样交替读最小值）：布局 64 / 256 / 500 层 0.16 / 0.90 / 2.49 ms，整帧 0.84 / 7.4 / 24.7 ms；修掉提取与锚点记忆的平方项后（见「待办」）布局 0.14 / 0.52 / 1.06 ms，整帧 0.66 / 4.1 / 11.6 ms。`cargo run --release -- stages 125,250,500` 按 `FrameStage` 列一帧各阶段的最小耗时。

深链的挂载（`stages` 的 `Mount`，`mount_view_root` 一次）原本也是平方的：125 / 250 / 500 层 0.74 / 2.4 / 8.7 ms，现在 0.28 / 0.55 / 1.11 ms，每翻倍 ×2。原因是每次插入都要沿父链走三遍：查环（`has_ancestor`，还往 SipHash 集合里插）、算深度（`ancestor_depth`）、量子树高（`subtree_height`），每走一步都经 `ValidationPlan::node` 把节点快照从世界里克隆出来暂存；提交后又沿父链两遍清理布局结果。现在：

- 节点在 `Hierarchy::depth` 里存自己的深度，插入与脱离时对移动的子树广度更新，校验在批次还没改结构时直接读它；批次里再按节点记忆已结算的深度（`ValidationPlan::depths`，放下一个没有子节点的节点只改它自己的条目，移动带子树的节点清空）。测试构建里每次读取都与实走比对。
- 只读的父链与子树遍历改用不暂存、不克隆的读取（`parent_of`）；子节点没有子节点时不可能成环，查环直接跳过；查环的上界用步数代替哈希集合。
- 世界里还没有任何布局结果时，两条清理链直接返回（`layout_results_suppressed` 也从 SipHash 换成按 id 哈希的 `NodeSet`）。

比例比绝对值可信，对比要交替多跑几轮读最小值。剖析用 `cargo run --release -- wide-loop` 配 `sample`（宽树栈浅，归因可信）。

之后又做的（2026-11-02）：`LayoutInputMap` 的输入表与样式记忆表从哈希表换成按节点下标的 `DenseNodeMap`（与 `NodeTable` 同一布局：6.5 万以内的 id 直接下标），宽树三档与改动前交替三轮都快 7–8%（2 101 盒 1.79 → 1.66 µs，21 001 盒 2.74 → 2.55 µs）；`holds_line_box_in` 记忆表命中时不再先分配；容器计划按 id 找子项测量尺寸的表只在子项超过 16 个时才建。布局分配每盒 25.3 → 24.3 次。

再往后（2026-11-02）：`LayoutResult::from_box` 的四个空切片改用 `Arc::default()`（原来 `Arc::from([])` 每个结果分配四个头）；`distribute_flex_main` 的八列临时数组放进线程内复用的 `FlexColumns`，嵌套求解拿到空槽就自建；帧驱动提交布局写回时置 `pass_publishes_written_boxes`，提交不再为写入的盒子发布 `CompatibilityWrite` 结果，由紧接着的整批 `RuntimeLayout` 发布一次建好（原来第二次发布只是把第一次的结果逐个判定未变）。宽树与改动前交替三轮读最小值：2 101 / 10 501 / 21 001 盒 1.91 / 2.51 / 2.69 → 1.59 / 2.19 / 2.45 µs，快 9–17%；布局分配每盒 24.4 → 16.2 次。分配次数降了三分之一，时间只降了一成多：macOS 的小块分配本来就便宜，剩下的是计算。`wide` 另列「建树 + 布局」：NanaUI 约 4.1–4.9 µs / 盒，Taffy 0.7–0.85 µs，Taffy 建树几乎不花时间，NanaUI 建树（建组件、排队挂载、提交）约 2.4 µs / 盒。

规模曲线（`wide 10,25,…,3200`，每行 10 格）：NanaUI 211–1 051 盒约 1.5 µs / 盒，之后一路涨到 67 201 盒的 3.16 µs；Taffy 0.69 → 0.83 µs。盒子数相同时形状不影响（3200×10、320×100、32×1000 都约 2.7 µs），各阶段（测量、放置、收尾、发布）按同一比例变慢：上涨来自每遍触及的每节点内存超出缓存，不是某个超线性算法。每遍要读的每节点数据：`NodeRecord` 520 B、每遍重建的 `LayoutInput` 264 B、`LayoutResult` 280 B 加切片，再加各遍的缓存，约 1.2–1.5 KB；Taffy 把一个节点的样式、缓存与输出放在 `SlotMap` 的一格里，缓存是节点内固定的 9 个槽。为此 `DenseNodeMap` 补成与 `HashMap<StableNodeId, V>` 同形的接口（存键、按 id 序迭代），布局遍与保留缓存里每节点都有一条的八张表（盒子、放置、内边距、远端起点、上下文、意图、测量计划、最新意图键）改用它；`ContainerPlan` 每条 264 B、只有容器才有，仍是哈希表。这一步交替三轮只快 0–5%：散列本身不是大头，要压的是每遍读的字节数。

指令与周期（`/usr/bin/time -l … reps <行数> <轮数>`，再用 `SKIP_LAYOUT=1` 只建树，相减得布局本身；每盒）：

| | 建树 指令 | 布局 指令 | 布局 周期 | 布局 IPC |
| --- | ---: | ---: | ---: | ---: |
| 1 051 盒 | 48 249 | 29 143 | 6 558 | 4.44 |
| 67 201 盒 | 48 109 | 30 420 | 11 629 | 2.62 |

Taffy 建树加布局加释放整个进程每盒约 1.7 万条指令，NanaUI 约 7.7 万。布局的指令数不随规模变，周期多了 77%、IPC 从 4.4 掉到 2.6：大树上布局受内存约束。缓存装得下时 NanaUI 每盒指令仍是 Taffy 的两倍多。建树每盒约 4.8 万条，比布局本身还多。

布局竞技场（arena）的第一步（2026-11-02）：每遍按节点读的 `LayoutInput`（264 B，模态框的三份文本度量内联在每个盒子里）在遍内换成私有的 `PassInput`（模态部分装箱，约一条缓存行），公开的 `LayoutInput` 不变；交替三轮大树快 3–5%。逐个关掉试过、各自只占 3–10% 的还有：遍内意图 LRU（`IntrinsicCache`，关掉快 6–12% 但会漏读多算，须换成节点内固定槽位才能拿走）、整页快照（不占时间，每盒 88 B）。第二步：遍内意图事实不再进公开的 LRU `IntrinsicCache`（按键散列、每次插入和命中都要改全表的最近使用链），改为 `PassIntrinsicFacts`：事实按测量顺序排在一个数组里，每个节点在按 id 下标的表里记最新一条、往旧串成链；计数口径与缓存相同（请求、命中、未命中、跨上下文、基线查询、条目与字节），一遍内不淘汰（原预算每节点四个变体，从未用到）。公开的 `IntrinsicCache` 与其合同测试不变。交替三轮快 3–6%。第三步：`seeded_intrinsic`（散列集合）与 `new_metrics`（散列表）并进这张事实表：每条事实另记「查找可见」与「交给保留缓存的最近一次值」，纯叶子只记后者，与原先绕开缓存时相同；交给保留缓存的顺序从散列序变为测量序。交替三轮大树快 8%（21 001 盒 1.91 → 1.75 µs，67 201 盒 2.26 → 2.08 µs）。

**上面的数都是全量路径（`layout_document`），产品帧不走它。** 帧驱动的首帧是 `layout_document_for_viewport`：文档还没有视口时给根种种子，跑一遍覆盖整棵树的作用域遍。`wide` 加 `FRAME_PATH=1` 量这条路径：2 101 / 21 001 / 67 201 盒 2.72 / 3.68 / 4.75 µs，是全量路径的 2.2 倍，每盒多 9.5 次分配、多约 700 B，首帧没有可复用的保留缓存，前沿展开与逐个物化输入全是开销。

第 0 步（2026-11-02）：文档没有任何保留缓存（`viewport` 为空且 `boxes` 为空）时，引擎内部按全量计算，框架的调用计数不变；整页快照只在调用方要求全量时记录与回放。全量遍原本不建测量计划（之后要么扔掉、要么清空），这里由 `records_measure_plans` 让首帧照常建，否则之后的增量编辑会随文档规模变贵（issue259 的成本合同会红）；空的无子节点盒子在作用域遍里也记一条计划，首帧为与之一致不走叶子捷径。产品首帧交替三轮：2.72 / 3.68 / 4.75 → 1.52 / 2.06 / 2.45 µs，快 1.8–1.9 倍；若不记计划能到 1.33 / 1.76 / 2.14，差的这 15% 来自给空叶子记的那条计划，是否值得记留待评估。

第 1 步（子节点几何不再存副本）先量后定：发布时不建父结果里每个子节点的放置与片段，产品首帧时间不变（0–2%，噪声内），每盒内存少 177 B、分配少 1 次。这两个是 `LayoutResult` 的公开字段，为这点收益不做破坏性改动，留到 `LayoutResult` 另有理由改时一并处理。

第 2 步：`wide` 加 `CONTROLS=1`（每格一个标签加一个按钮，Taffy 一行跳过）。在控件页上剖析首帧，SipHash 占布局 5.2%：`scoped_scroll_retention_targets` 首帧对整棵树每个节点往 std `HashSet` 插一次，引擎遍末的 `consulted` 也是 std `HashSet`；换成按 id 散列的 `NodeSet` 与 hashbrown 后两种页面都快 4–6%（控件页 3 101 / 31 001 盒 1.67 / 2.41 → 1.58 / 2.30 µs）。组件存在掩码（外观一位记在 `NodeRecord` 上，查表前先看位）实测没有收益：控件页不变，纯盒子页慢 3%（外观表为空时原本整表判空就返回）；hashbrown 查一个不存在的键通常只读一组控制字节，比预想便宜，所以撤回。

第 3 步（输出只写一次）先按阶段重量了产品首帧（67 201 盒，ns / 盒）：放置 840、测量 525、发布 381（建 228、存 128）、遍末合并 350、写回比较 61、提交 74、预取 60。写回提交只占 6%：给帧驱动单设一条只含 `WriteLayout` 的提交、跳过通用提交对整批的十来遍扫描，实测不变（扫描只是逐条匹配枚举），撤回。遍末合并里做了：`remote_overlays` 从 std `HashSet` 换成 `NodeSet`；保留表为空（首帧）时整表接过遍内的内边距、远端起点、上下文与放置表（`DenseNodeMap::absorb`），不再逐条插入；`DenseNodeMap::reserve` 真正预留，遍内事实与最新意图键表按节点数预留。两种页面快 1–3%。

之后先攻测量与放置本身（冷布局里两者合计 57%）。计数（`last_work_counters`）显示节点被测了不止一次：纯盒子页 2 101 盒测 3 101 次、控件页 3 101 盒测 6 101 次；临时打印每次实算的节点与约束，重测的是格子——测量遍按行拿到的 1280×800 测，放置遍按行实际的内容框 281×16.5 再测，两次结果相同。Taffy 在同一场景（容器以已知尺寸布局时按新的可用宽度重定子项基准）同样重测，差别在单次测量的成本。于是把遍内用尺寸记忆从按整个 `MeasurementKey` 散列的表，改成 `used_order` 加每节点最新下标与更旧链：查找只比较这个节点的几个键。两种页面快 4–7%（纯盒子页 67 201 盒 2.35 → 2.21 µs，控件页 3 101 盒 1.61 → 1.53 µs）。同时：一次性的 `presence_live` 不再新建、填满再丢掉一张祖先记忆表；最新意图键改存事实下标（每槽 56 → 16 B）；二者实测无时间差，作为简化保留。注意发布构建会合并相同机器码的函数（`deduplicated_symbol`），采样里的函数名会张冠李戴，判断要靠计数与关掉再量。

`LayoutInputMap::style` 每盒被调约 20 次（纯盒子页 42 501 次 / 2 101 盒），每次克隆再释放一个 `Arc`；这台机器上一对非争用的克隆加释放比借用多约 5 ns。新增借用版 `style_ref`（返回 `StyleRef`：遍内有输入时借用，只有按需记忆的样式才是自有句柄），70 个调用点里能编译的 44 个改用它（逐个试编译，其余调用方要保留句柄或随后要可变借用 map）。两种页面快 2–6%（纯盒子页 67 201 盒 2.40 → 2.26 µs）。后续各步：放置直接写保留表，省掉遍末合并；发布只为几何变了的节点建结果；建树路径单独剖析。

已做的（2026-11-02）：发布阶段每个结果的四个切片在复用的暂存表里建好、按最终长度一次分配，空切片不分配（`Arc::<[T]>::default()`），按父分组的子集改为一张排序表，不再每个父节点一个 `Vec`；意图缓存键的环境哈希从 SipHash 换成固定种子的 foldhash，`seeded_intrinsic` 换成 hashbrown；`ConstraintClass` 的相等与哈希一趟读出归一化后的标签与值，不再各自归一化两遍；`RetainedIntrinsicMetrics` 每个内容的变体表按一增长（首个 push 原本预留 4 个，320 B 只用 80 B）；`MeasurePlanSlots` 的两个槽装箱（原本每个容器 528 B 内联在表里，通常只用一个槽）；整页 pass 按节点数和容器数预先给各表留好容量；锚点记忆表在父节点已结算时不再分配。

## 待办

- **冷布局吞吐与每盒内存**：上面两张表 NanaUI 仍落后 Taffy 约 3 倍（时间）与 2 倍（内存）。剩下的成本是摊开的，没有单点：每盒约 16 次分配（每个容器的 `descend`、`flow`、`child_sizes`、计划条目等 `Vec`，单去掉它们省不出多少时间）、每个节点在测量里的十来项判定（在场、浮层分支、菜单分支、行盒记忆，各自一次按 id 的查表）、每次写入落四张意图缓存表、十几张按 `StableNodeId` 的哈希表（`LayoutInputMap` 的输入表每条 264 B，一万盒时 2.8 MB，查一次就是一次缓存未命中；`style()` 每盒被调约 18 次，分散在 14 处）、以及约 4.8 KB 的 `LayoutStyle`（一次判定常常要碰几条缓存行）。再往下要换存储：按节点下标的稠密数组代替这些表、`LayoutResult` 不再给每个子节点存三份几何，这是结构改动，需要先对 `WorkCounters` 合同和消费者（Scene、命中、无障碍、`issue205` 等测试）逐个确认。
- **往已布局的界面里挂深子树**：挂载本身已是线性（见下），但插入时沿祖先清理布局结果的两条链（`suppress_layout_result_chain`、`clear_layout_result_chain`）在世界里已有布局结果时仍走到最近的固定尺寸边界，代价是「插入的节点数 × 深度」。首次挂载时表是空的，直接跳过。要在已处理过的祖先处提前停，得先确认边界节点的样式在两次插入之间不会变，否则会读到过期结果。
- **`AppContext` / `UiWorld` 内联大小**：把大块字段装箱，构造路径的帧就能回到预算内。
- **railroad 解析的 debug 帧**：`alt → seq → term → primary` 每层五个帧、合计 5 KiB 以上，48 层在 debug 下余量不到一倍。把 `Result<Expr, Fail>` 的大小压下来，或把上限降到 32。
- **移动端实机**：`stack` 的 iOS 与 Android 分支只做过交叉 `cargo check`（`aarch64-apple-ios`、`aarch64-linux-android`，后者先 `source scripts/android-env.sh`），没有在设备上跑过深树门禁。
