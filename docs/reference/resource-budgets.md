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

第 0 步（2026-11-02）：文档没有任何保留缓存（`viewport` 为空且 `boxes` 为空）时，引擎内部按全量计算，框架的调用计数不变；整页快照只在调用方要求全量时记录与回放。全量遍原本不建测量计划（之后要么扔掉、要么清空），这里由 `records_measure_plans` 让首帧照常建，否则之后的增量编辑会随文档规模变贵（issue259 的成本合同会红）；空的无子节点盒子在作用域遍里也记一条计划，首帧为与之一致不走叶子捷径。产品首帧交替三轮：2.72 / 3.68 / 4.75 → 1.52 / 2.06 / 2.45 µs，快 1.8–1.9 倍；若不记计划能到 1.33 / 1.76 / 2.14，差的这 15% 来自给空叶子记的那条计划。后来评估过（2026-11-03）：没有子节点可回放的计划，记忆表答的是同一个问题，所以纯叶子（`plain_leaf`）在首帧和作用域遍里都不再记，成了叶子的容器清掉旧计划；控件页布局指令少 7.5%、时间快 5–6%，纯盒子页不变。

第 1 步（子节点几何不再存副本）先量后定：发布时不建父结果里每个子节点的放置与片段，产品首帧时间不变（0–2%，噪声内），每盒内存少 177 B、分配少 1 次。这两个是 `LayoutResult` 的公开字段，为这点收益不做破坏性改动，留到 `LayoutResult` 另有理由改时一并处理。

第 2 步：`wide` 加 `CONTROLS=1`（每格一个标签加一个按钮，Taffy 一行跳过）。在控件页上剖析首帧，SipHash 占布局 5.2%：`scoped_scroll_retention_targets` 首帧对整棵树每个节点往 std `HashSet` 插一次，引擎遍末的 `consulted` 也是 std `HashSet`；换成按 id 散列的 `NodeSet` 与 hashbrown 后两种页面都快 4–6%（控件页 3 101 / 31 001 盒 1.67 / 2.41 → 1.58 / 2.30 µs）。组件存在掩码（外观一位记在 `NodeRecord` 上，查表前先看位）实测没有收益：控件页不变，纯盒子页慢 3%（外观表为空时原本整表判空就返回）；hashbrown 查一个不存在的键通常只读一组控制字节，比预想便宜，所以撤回。

第 3 步（输出只写一次）先按阶段重量了产品首帧（67 201 盒，ns / 盒）：放置 840、测量 525、发布 381（建 228、存 128）、遍末合并 350、写回比较 61、提交 74、预取 60。写回提交只占 6%：给帧驱动单设一条只含 `WriteLayout` 的提交、跳过通用提交对整批的十来遍扫描，实测不变（扫描只是逐条匹配枚举），撤回。遍末合并里做了：`remote_overlays` 从 std `HashSet` 换成 `NodeSet`；保留表为空（首帧）时整表接过遍内的内边距、远端起点、上下文与放置表（`DenseNodeMap::absorb`），不再逐条插入；`DenseNodeMap::reserve` 真正预留，遍内事实与最新意图键表按节点数预留。两种页面快 1–3%。

之后先攻测量与放置本身（冷布局里两者合计 57%）。计数（`last_work_counters`）显示节点被测了不止一次：纯盒子页 2 101 盒测 3 101 次、控件页 3 101 盒测 6 101 次；临时打印每次实算的节点与约束，重测的是格子——测量遍按行拿到的 1280×800 测，放置遍按行实际的内容框 281×16.5 再测，两次结果相同。Taffy 在同一场景（容器以已知尺寸布局时按新的可用宽度重定子项基准）同样重测，差别在单次测量的成本。于是把遍内用尺寸记忆从按整个 `MeasurementKey` 散列的表，改成 `used_order` 加每节点最新下标与更旧链：查找只比较这个节点的几个键。两种页面快 4–7%（纯盒子页 67 201 盒 2.35 → 2.21 µs，控件页 3 101 盒 1.61 → 1.53 µs）。同时：一次性的 `presence_live` 不再新建、填满再丢掉一张祖先记忆表；最新意图键改存事实下标（每槽 56 → 16 B）；二者实测无时间差，作为简化保留。注意发布构建会合并相同机器码的函数（`deduplicated_symbol`），采样里的函数名会张冠李戴，判断要靠计数与关掉再量。

`LayoutInputMap::style` 每盒被调约 20 次（纯盒子页 42 501 次 / 2 101 盒），每次克隆再释放一个 `Arc`；这台机器上一对非争用的克隆加释放比借用多约 5 ns。新增借用版 `style_ref`（返回 `StyleRef`：遍内有输入时借用，只有按需记忆的样式才是自有句柄），70 个调用点里能编译的 44 个改用它（逐个试编译，其余调用方要保留句柄或随后要可变借用 map）。两种页面快 2–6%（纯盒子页 67 201 盒 2.40 → 2.26 µs）。后续各步：放置直接写保留表，省掉遍末合并；发布只为几何变了的节点建结果；建树路径单独剖析。

增量编辑的计时（2026-11-03）：`edits` 在 `RuntimeDocument::flush` 的产品帧里每帧改一处，读帧剖析里 `Layout` 阶段与整次 flush 的最小值，与 Taffy 改同一叶子后的 `compute_layout` 对照（`edits 200,2000,6400`；`CONTROLS=1` 改标签文字；`PRINT_COUNTERS=1` 打印最后一次编辑的布局计数）。`width` 改中间一行第六格叶子的宽度，只动这一行；`height` 改它的最小高度，下面每一行都要移动。第一次量出来：

| 盒子数 | 编辑 | 布局 | 整帧 | Taffy 布局 |
| ---: | --- | ---: | ---: | ---: |
| 4 201 | width | 364 µs | 2.2 ms | 47 µs |
| 42 001 | width | 6.3 ms | 41.7 ms | 1.0 ms |
| 134 401 | width | 28.0 ms | 152.8 ms | 5.4 ms |
| 134 401 | height | 240.8 ms | 680.5 ms | 5.3 ms |

控件页改标签文字 9 301 / 93 001 盒都只要 11 µs（整帧 19 / 26 µs），不随规模变。`width` 的计数很小（测 5、放 12、2 000 次记忆命中），时间却随规模涨，有两处 O(N)：

- **行盒记忆每遍重建。** 根被重测时要问自己是否含行盒（`holds_line_box_in`），纯盒子页上没有任何行盒，只能走遍整棵树才能答「否」；记忆表原本每遍新建。现在保留在 `DocumentLayoutCache::line_boxes`：答案只看一个盒子和它里面的东西，所以作用域遍沿每个受影响节点往上清到根（不在没有答案的祖先处停：自己就有行盒的盒子不问子节点就结算），其余沿用。测试构建里 4 096 个节点以内的世界每次都与新走一遍比对。134 401 盒 `width` 布局 28.0 → 6.7 ms。
- **父结果里的子节点放置让父节点也算「几何变了」。** 行按内容定宽，格子变宽它也变宽（281 ↔ 289），根的结果里记着每行的放置（`FlexChildPlacement` 片段和 `ChildPlacement` 部件），于是根被标上命中、渲染、无障碍脏位，命中索引从根重建整份文档（`patch_hit_subtree`）。这两类记录只有布局自己写、Scene、命中与无障碍都不读，而位置变了的子节点自己会发布并投影；`layout_result_projects_new_geometry` 比较时把它们除外。134 401 盒 `width` 整帧 117 → 6.7 ms，4 201 盒 2.1 → 0.13 ms。

两处修完：

| 盒子数 | 编辑 | 布局 | 整帧 | Taffy 布局 |
| ---: | --- | ---: | ---: | ---: |
| 4 201 | width | 111 µs | 131 µs | 46 µs |
| 42 001 | width | 0.83 ms | 0.85 ms | 0.46 ms |
| 134 401 | width | 6.7 ms | 6.7 ms | 5.1 ms |
| 42 001 | height | 22.2 ms | 71.2 ms | 0.48 ms |
| 134 401 | height | 118 ms | 346 ms | 5.0 ms |

`height` 仍差 20–70 倍，原因在坐标表示：NanaUI 的布局盒是文档绝对坐标，一行变高，下面每个盒子的坐标都变了（42 001 盒时放置 21 001 个、只动原点的 20 997 个），之后写回、命中索引、无障碍、Scene 图元、提取各自按变了的盒子做一遍；Taffy 存相对父节点的位置，同样的编辑只改这一行之后各行自己的 `location`。要追平得让保留的盒子相对父节点（或相对最近的合成层），整条 `UiWorld → UiScene` 路径都要跟着改，是结构改动，没有做。

平移方案（[设计](https://claude.ai/artifact/6VoxW6MKE6XVYkyfkjfHeF)，方案 B）的 P0 与 P1 引擎部分（2026-11-03）：`WorkCounters` 新增 `layout_translations` 与 `layout_translated_nodes`；只是挪了位置的容器不再对子孙逐个走 `place_node_body`，`translate_moved_children` 自上而下按新的内容原点加计划里记的偏移重放，与新鲜一遍逐位相同（内容原点按 `原点 + 边框 + 用过的内边距 + 前导部件` 的同一加法顺序，并先用旧原点重算一遍与计划比对）；子树里有不能这样移动的盒子（无保留盒子、计划含定位子项或丢了偏移、动态行、孤岛、行内上下文、位置与计划不符）就整棵退回原路径。issue259 的成本门禁改为「放置 + 平移 ≥ 移动的盒子」。42 001 盒改行高：放置 21 001 → 1 012 个，19 989 个由 1 008 次平移带走；采样里引擎放置 424 → 248、布局阶段约少 15%；整次运行指令少 1.8%。

剩下的布局阶段主要是结果发布与写回：结果里的内边距框、内容框、文本行与部件都由原点再经加法派生，原地平移做不到与冷重建逐位相同，而现有合同（`assert_matches_cold`）要求逐位相同，所以 P1 没有跳过结果重建，待定。

结果改为本地坐标（2026-11-03）：结果自身的几何——边框盒、内边距盒、内容盒、文本行、部件、基线——在边框盒置于原点时求出，再整体加上原点得到对外的结果（`build_own_layout_result` → `place_layout_result`）；与子节点有关的量（子节点放置、行内原子片段、溢出框、滚动范围）直接取子节点精确的绝对盒子，不做减法。对外仍是 `layout_result(id) -> Option<&LayoutResult>`、绝对坐标，公开接口与语义不变；派生框的数值由 `(原点 + 边框) + 内边距` 变为 `原点 + (边框 + 内边距)`，末位可能不同，冷重建与增量走同一条路径，逐位一致。部件求解器读插槽盒子的地方（列表项的前导、尾随插槽，标签值的操作按钮）改为从同一原点量起（`solve_content_parts_in`）；第一版漏了这一点，原点被加了两遍，冷重建比对与快速路径比对都一致地错，是语义快照抓到的。

这样一棵只是挪了位置的子树里的节点，发布时不必重建结果：自身几何不变，只按新原点放置并重接子节点。结果表每条另记一个 `OwnGeometry`：`Empty`（普通容器）、`PlainLine { baseline }`（一行普通文本，片段与文本部件都是内容盒），其余为 `Unkept`，平移时照常重建；部件读插槽位置的列表项与标签值也照常重建。测试构建里每个走快速路径的节点都再完整构建一次逐位比对。存一份完整的本地结果会让每个排好版的文本、按钮、列表项多约 330–350 B，超出 `node_memory` 的预算，所以只存这一个小枚举：三类节点各多 16 B（文本 3 442 → 3 458 B）。与引擎平移后的提交相比：42 001 盒改行高整次运行指令少 1.7–2.0%，改宽度持平，首帧布局每盒指令多 0.6%（32 766 → 32 970），时间在噪声内。改行高一帧里命中索引、提取、无障碍三段仍按移动的盒子逐个重做，是接下来几步的对象。

命中索引（2026-11-03）：被标脏的条目先分流，能原地处理的不再走子树修补（`rebuild_hit_test_scoped`）。条目是「普通」的——单位变换、无裁剪、无路径裁剪、无菜单与绘制器、未逃逸（`plain_hit_entry`）——时，它传给子节点的状态与位置无关：平移带走的节点只把世界里的新盒子拷进条目；其余脏节点只重建自己这一条（`build_hit_entries_reaching` 不下探子节点），新条目仍普通、子节点列表与层叠不变、没有改成 `fixed` 时原地替换，子节点保留原条目。会影响子节点条目的样式变化（pointer-events、可见性、变换、层叠）本来就把整棵子树标为 INPUT 脏，子节点各自重建。原地处理过的条目按深度从下往上刷新边界，每个节点一次；边界树按槽位固定配对，叶子相同则内部节点逐位相同。冷重建比对（`assert_matches_cold`）比较前会先整个重建增量索引，测不到增量修补，所以测试构建里 4 096 个节点以内的文档每次原地处理后都另建一份完整索引，逐字段比较这些条目与它们在索引里的父节点。引擎在被移动的容器尺寸不变、自身不受影响时也把它记为平移，结果发布同样少重建一个。42 001 盒改行高：命中阶段 16.5 → 3.2 ms，整帧 65.6–68.8 → 52.4–52.8 ms，整次运行指令少 13.6%；134 401 盒整帧 303–305 → 249–252 ms；改宽度与改标签文字不变。整帧剩下的大头是无障碍（约 12 ms）与提取（约 14 ms）。

Scene（2026-11-03）：图元是用绝对坐标的布局盒（加祖先变换）建的，要像结果那样逐位精确地平移，得把 Scene 的图元整体改成本地坐标并改 GPU 绘制，这一步没有做。做了的是：节点在 Scene 里一个图元都没有、新提取的记录除位置与对外结果外与旧的相同（`moved_only`，位置没变或尺寸变了直接排除，免得逐条比较宽容器结果里的子节点放置）时，不再重建图元。它传给子节点的状态由 Scene 的节点表现算（`project_ancestor_state`），投影只在画它自己的图元时读，所以没有图元的节点跳过重建不影响别人；调试构建在 512 个节点以内的场景上对每个跳过的节点真重建一次，断言仍是 0 个图元。42 001 盒改行高：提取阶段约 14 → 9.6 ms，整帧最小值 53.8 → 47.0 ms，指令少 7.3%；134 401 盒 248 → 223 ms；带绘制的节点照常重建。提取阶段剩下的主要是世界侧为每个脏节点生成 `ExtractedNode`（约 3.5 ms）：只发「新盒子 + 新结果」的轻量路径要先能分清「只因移动而脏」和「还有别的原因」（同一帧换主题、行内可见性、确认按钮忙碌态等都在提取里处理），留作后续。

只因移动而脏的节点（2026-11-03）：节点记录多两个提取纪元（`render_full_epoch` / `render_moved_epoch`），世界的纪元每次 `extract_nodes_reusing` 后加一。在弄脏的源头分开记：只有结果发布走了平移快速路径（自身几何不变、按新原点重新放置）的节点记「移动」；`WriteLayout` 只挪了位置的标记与该节点投影出的标记不算原因；其余所有 RENDER 标记（样式、文字、主题、可见性、确认按钮忙碌态、粘性、重建的结果、新建、失败后恢复）都记「其它」。本纪元只有「移动」、没有「其它」、且 Scene 持有它上一次的记录时，世界不再生成 `ExtractedNode`，只发 `ExtractedMove`（新盒子 + 新结果），Scene 的 `apply_delta_with_moves` 原地改写记录，原循环里读到盒子或结果的判断（继承几何、变换与裁剪、裁剪链、逃逸、层叠、继承不透明度、无图元移动、组候选计数）在改写前后各算一次；拿不准一律走完整提取。先试过让世界克隆 Scene 的旧记录再换盒子，反而更慢（世界侧 3.4 → 5.8 ms）：ExtractedNode 1 160 B，提取时 Scene 的旧记录是冷的，读它的代价和重新提取差不多，所以改成两边都不读整条记录。对照：`layout-verify`（及 runtime 测试构建）下每个走轻量路径的节点都把旧记录换上新盒子后与完整提取逐字段比较（布局盒按位），用 `cargo test -p nana-ui-scene --features nana-ui-runtime/layout-verify` 跑到；`nana-ui-scene/tests/moved_extraction.rs` 每帧移动所有行并同帧改标签颜色、色块填充、同长文字或主题，之后逐节点比较 Scene 的记录与冷提取、图元与冷建 Scene。变异实验：忽略「其它」标记时对照报 `["source_style", "style"]`、四种同帧改动各自独立变红；轻量路径不更新盒子、或 Scene 把会画的移动节点当无图元时也都变红。42 001 盒改行高（`PRINT_STAGES=1` 现在打印各阶段在全部轮次里的最小值，交替跑）：提取阶段 9.1–10.2 → 3.9–4.7 ms，整帧最小值 44.3 → 40.4 ms，整次运行指令 26.1–26.4 G → 22.2–23.0 G（约每帧少 9 千万条）；134 401 盒：提取 50–55 → 21–25 ms，整帧最小值 247 → 229 ms；改宽度与控件页改标签文字的阶段时间与指令都在噪声内（6.31–6.42 G、8.84–8.93 G）。`render_nodes_extracted` 把移动也计入，`extracted_text_spans` 只计完整提取的节点。

无障碍（2026-11-03）：`AccessibilityNode::bounds` 改为相对父节点（偏移从父节点 `bounds` 的原点量起，尺寸不变；`clip` 仍是视口坐标），AccessKit 宿主把偏移设成节点的 `transform`、尺寸设成本地 `bounds`。一行变高后，跟着父节点一起平移的盒子相对值不变，不再投影，也不进原生更新。判定在布局发布时做：引擎平移带走的盒子（`translated`），旧结果与新结果里「自己减父节点」的偏移逐位相等、尺寸相同、裁剪链上没有被这次发布移动或改尺寸的容器，就不标 ACCESSIBILITY（布局写回只是挪了位置的盒子时把这一位留给紧随其后的发布决定，没走到发布的由下一次 drain 补标）。只因布局移动而标的 INPUT 不带新的 `DirtyMask::INPUT_ELSEWHERE`，drain 时进 `SystemWork::accessibility_moved`；这类节点不再作为整棵子树的种子，只投影自己（若标了 ACCESSIBILITY）和布局没挪动的直接子节点（例如视口固定的 `fixed` 盒），会裁剪子孙的容器或存在裁剪视觉时仍按整棵子树处理。投影端再核一次：留下的节点及其父节点的视口盒必须就是布局盒（无变换、无滚动偏移、非模态框面板、非选择框/树/菜单这类带虚拟子节点的控件），否则照常投影。测试构建与 `layout-verify` 下，4 096 个节点以内的世界记下每次投影交出的节点，delta 每次把「旧规则会投影、新规则留下」的节点从头投影一遍，逐字段与交出过的比对。42 001 盒改行高（与 12f5fb909 之后的 main 交替 8 轮读最小值，负载 12–24）：无障碍阶段 13.2 → 3.9 ms（中位数 14.8 → 4.5），整帧 48.1 → 36.9 ms（中位数 57.1 → 43.3）；布局阶段因发布时的判定多 0.4–1 ms（20.9 → 21.3 ms）；每次编辑的指令（`RUNS=240` 与 `RUNS=40` 两次 `time -l` 之差除以 200）409 M → 340 M（−17%；rebase 前在旧基线上量过，不做发布判定、只改种子规则时只省约一半）。改宽度每次编辑 15.98 M → 15.81 M，整帧 0.93 → 0.88 ms；控件页改标签文字整帧 24 µs 不变；冷首帧（含建树与 Taffy 对照的整次运行）指令在 ±0.5% 内。剩下的无障碍成本是被移动的 1 000 行本身（相对父节点的偏移确实变了）和 `accessibility_tree_is_plain` 的全量扫描。

公开 API 变化：`AccessibilityNode::bounds` 的语义（相对父节点）；新增 `UiWorld::project_accessibility_in_viewport` 与 `project_accessibility_nodes_in_viewport`（给自动化与测试的视口坐标）；`SystemWork::accessibility_moved`（新字段，空时行为与以前相同）。读 `bounds` 当屏幕坐标的消费者要换成视口投影：仓库内的 devtools agent 导出已换；NanaLive（`nanalive-control/src/native_ui/`）按 `project_accessibility(...).bounds` 的位置点击或断言的有 `offscreen.rs`（离屏自动化按节点中心点击，产品路径）、`offscreen/receiver.rs`、`form/right_stack.rs`、`form/mode_rail.rs`、`mod.rs`、`kit/tests.rs`、`tools/content/tracking.rs`、`tools/content/guide_model/tests.rs`，需要改用 `project_accessibility_in_viewport`；只看宽高是否为正的读法不受影响。

带绘制节点的图元重建（2026-11-03）：评估过对只移动的带绘制节点精确平移现有图元，不做。图元的位置在 `build_node_primitives` 里按种类各自算出（如 `(x + border) + padding`），加位移得不到逐位相同的值，从本地几何重算要为每种图元复制一份位置公式；Button 的 `component_geometry` 本身是绝对坐标，进不了 `moved_only`。上限实验——让这类节点整个跳过重建（输出错误，只为量上限）——在控件页（`CONTROLS=1 edits 1355 height`，42 006 盒，6 779 个这类节点）上只少 3.3% 的指令，整帧最小值的差与噪声同量级；指定的裸盒页上 Scene 一个图元都没有，收益为 0。painter 只遍历 `visible_operations` 裁剪后的图元，每个图元多加一次原点的代价与场景总数无关。重建本身的热点是祖先走查与绘制顺序 key 的分配，与坐标无关，已改：`ancestor_answer` 的走查表放进 `RebuildScratch` 复用；`group_prefix_memo` 在父节点已记住时一步得出，不收集链；`order_stack` 一次分配建好，重建时若槽位原有的栈内容相同就直接沿用那个 `Arc`，`insert_primitive` 在 key 未变时不再换掉它。控件页改行高：每次编辑 6.96–6.97 → 6.79–6.81 亿条指令（−2.3%），提取阶段最小值 25.3 → 24.3 ms（交替 8 轮），整帧在噪声内；裸盒页指令不变。

布局阶段的常数（2026-11-03）：42 001 盒改行高，布局阶段在 ba8ba044f 上约 19–21 ms，Taffy 做同一次编辑 0.44–0.48 ms。真正重新排布的只有约 1 000 行，其余约 2 万个节点是平移。用 `benchmark` feature 的分段时钟（临时加了细分槽，已撤）拆开，40 次编辑各段取最小值（µs，改前 → 改后）：

| 段 | 改前 | 改后 | 内容 |
| --- | ---: | ---: | --- |
| 测量 | 1 500 | 1 500 | 根列重新测量，2 000 行逐行命中缓存，约 0.7 µs/行 |
| 放置 · 平移遍历 | 2 450 | 2 060 | `translate_moved_children`：每盒约 9 次表查询，每个容器克隆一份计划（`moved_to`） |
| 放置 · 写入 | 650 | 610 | 平移结果写进 `output` 与本轮计划表 |
| 引擎尾段 · 容器计划回收 | 1 700 | 990 | 11 000 份计划去重、换进保留表、丢掉旧的 |
| 引擎尾段 · 其余 | 450 | 450 | 输出排序、写保留盒子 |
| 写回比较 + 视图扫描 + 提交 | 1 750 | 1 750 | 每个移动的盒子一条 `WriteLayout`，提交时标脏 |
| 发布 · 前置 | 700 | 530 | 祖先闭包、排序、按父分组 |
| 发布 · 构建结果 | 4 200 | 3 950 | 每个移动的节点按新原点重建绝对坐标结果：三个切片 + 一个 `Arc` |
| 发布 · 几何比对 | 330 | 330 | `geometry_eq` 与旧结果比 |
| 发布 · 保持偏移判定 | 700 | 650 | 47aa93fb2 的无障碍判定，每个平移节点查父节点新旧盒子 |
| 发布 · 存储与标脏 | 2 130 | 1 060 | 换上新结果、丢掉旧结果、标 INPUT/RENDER |
| 收尾 | 550 | 550 | 主要是 `scoped_scroll_retention_targets` 走一遍整个作用域 |

改了的（每项与前一版二进制交替 3 轮，插桩构建，各段最小值）：
- `dedupe_plan_rows`：16 行以内直接两两比较，不再为每份计划建一张 SipHash 的 `HashSet`（每帧 11 000 次分配）；更长的改用 `NodeSet`。计划回收 1 670–1 810 → 980–1 030 µs。
- `LayoutResult::placed_frame` 改为按值取：原来 `..self.clone()` 克隆整份结果（四个 `Arc` 加减计数）再丢掉。构建结果 4 140–4 370 → 3 920–4 040 µs。两项合计布局 19.0–19.4 → 18.1–18.7 ms。
- 平移遍历：一个容器的边框、内边距与前导部件只读一次，旧原点与新原点共用（原来各走一遍 `effective_layout_style`，含在场、浮层分支、菜单分支判定），加法顺序不变；工作列表放进 `LayoutInputMap` 复用，不再每个容器分配、扩容三个 `Vec`。平移遍历 2 430–2 690 → 2 130–2 280 µs，布局 18.2–18.6 → 17.5–17.8 ms。
- 发布：`layout_result_projects_new_geometry` 原来对每个发布的节点整份克隆旧结果再打补丁比较，改成逐字段比，自身的盒子变了就直接得出结论；存储循环对旧结果只查一次表（`entry`），「是否请求投影」用随 `published` 前进的游标代替每个节点一次二分；前置的 `unique` 从输入顺序（已排序）出发做稳定排序，不再从集合的哈希顺序重排。存储与标脏 2 110–2 130 → 1 050–1 070 µs，前置 670–700 → 530–540 µs，布局 17.6–17.9 → 15.9–16.3 ms。

合起来（生产构建，与 ba8ba044f 交替 8 轮读最小值，负载 4–5）：布局阶段 18.49 → 15.95 ms（−14%），整帧 31.69 → 29.35 ms；命中、无障碍、提取不变。每次编辑的指令（`RUNS=240` 与 `RUNS=40` 两次 `time -l` 之差除以 200）325 M → 296 M（−9%）。试过不收的：写回把三次文档缓存查找合成一次（−0.1 ms，三个访问器因此成了死代码）；计划的 `by_child` 改成共享切片（回收段省 0.13 ms、遍历多 0.15 ms，原子计数抵掉了少的分配）。

剩下的里面，必须改坐标表示才能去掉的：构建结果（约 4 ms，每个移动节点四次分配，存储时再释放旧的四块）、写回与提交（约 1.75 ms）、存储里的标脏（约 1 ms，每个节点在提交和发布各标一次 INPUT/RENDER，每次两次集合插入）、保持偏移判定（0.65 ms）——它们都按「坐标变了的盒子」逐个做，盒子存相对父节点的偏移后，平移子树只改根偏移，这些都不再发生。不改架构还能收的候选：平移计划就地更新（本轮只记「这份计划移到哪」，在尾段对保留表 `get_mut` 改原点，免掉 `moved_to` 的克隆、本轮表的插入和旧计划的释放；要让读 `nodes.container_plans` 的十几处认得这种状态，估计 ≤1 ms）；`scoped_scroll_retention_targets` 在文档里没有 `ScrollView` 时跳过（约 0.5 ms，需要维护一个计数）。

根列的重新测量（2026-11-03，接上段）：那 1.5 ms 不是缓存命中路径慢，是根列根本没用上测量计划。`Stack` 默认 `height: Shrink`，而顺序计划（`sequential`，一个子项变了只把它的差补进记下的总和）要求主轴尺寸 `is_none()`；`Shrink` 与 `Auto` 在 `resolve_axis` 里同样解析为没有长度，`finish_intrinsic_size` 取内容派生的默认值，与不写完全一样，于是放宽为「没写、`auto` 或 `shrink`」。其余内容尺寸关键字（`min-content` 等）与带 min/max 的仍排除。改高度的这一帧，根列不再逐行命中 2 000 次（测量命中 2 025 → 28，测量计划复用 1 → 2）。新测试 `content_growth_under_a_child_moves_its_siblings_when_the_container_shrinks` 让 `Shrink`/`Auto` 列与不写高度的列重测同样多的子项（2 个；改前 14 个，变异实验确认会变红），每一步都与完整重算逐盒比对。与 275f9229c 交替 8 轮读最小值（负载约 5.5）：改行高布局 16.33 → 15.17 ms，整帧 30.37 → 29.05 ms，每次编辑指令 297 M → 286 M（−3.5%）；改宽度（行也是 `Shrink`）布局 0.84 → 0.12 ms，已低于 Taffy 的 0.46 ms；控件页（`CONTROLS=1 edits 1355 height`）布局 20.9 → 19.7 ms。rebase 到相对几何第 1 阶段（35045d816）之后再量（交替 6 轮读最小值，负载约 6.6）：改行高布局 3.81 → 2.29 ms（−40%），整帧 17.67 → 16.31 ms，每次编辑指令 166 M → 155 M（−6.7%）；改宽度 0.78 → 0.11 ms。第 1 阶段把平移的成本拿掉以后，这 1.5 ms 的根列重测就成了改行高布局里最大的一块。

相对几何第 1 阶段（2026-11-03，设计见 [relative-geometry.md](relative-geometry.md)）：布局盒改为相对参照系（父节点、包含块或页面）的偏移，`NodeRecord`、引擎的保留盒子与计划原点、`LayoutResult` 都按本地存；`translate_moved_children` 删除，只挪了位置的容器父节点只写它自己的偏移，子孙一个也不碰，结果不重建也不重发布。对外的 `layout_box` 等沿参照系链从上往下累加得到绝对值。命中索引和 Scene 仍读绝对盒子，过渡期由提交时的 `spread_layout_moves` 把平移子树的后代标成「只是移动」。42 001 盒（与 275f9229c 交替 8 轮读最小值，负载 5.5–7.1，µs）：

| 编辑 | 阶段 | 275f9229c | 第 1 阶段 |
| --- | --- | ---: | ---: |
| height | 布局 | 16 391 | 3 911 |
| height | 命中 | 2 917 | 3 793 |
| height | 无障碍 | 3 392 | 3 379 |
| height | 提取 | 4 274 | 3 896 |
| height | 整帧 | 29 928 | 17 794 |
| width | 布局 / 整帧 | 780 / 800 | 771 / 787 |
| 控件页 text | 布局 / 整帧 | 11 / 23 | 11 / 23 |

每次编辑的指令（`RUNS=240` 与 `RUNS=40` 两次 `time -l` 之差除以 200，两轮）：height 295.7–295.9 M → 166.4 M（−44%），width 15.3–15.4 M 持平，控件页改文字 0.27–0.36 M → 0.40–0.47 M（多约 10 万条）。整次运行（`RUNS=40`，含建树与首帧）height 17.49 → 12.23 G、width 6.26–6.28 → 6.28 G（+0.3%）、控件页 8.77–8.80 → 8.84–8.85 G（+0.7%）：冷路径上多出来的是各处推导绝对盒子的 O(深度) 求和。命中阶段多约 0.9 ms：它挪动的 2 万个条目每个都要推导一次绝对盒子，被写回的 1 000 行也各重建自己那一条（写回只挪了位置的盒子不能原地移动条目：同一次提交里可能有兄弟增删，条目的顺序跟着变，`layout-verify` 抓到过），第 2 阶段改成本地条目后消失。`node_memory` 不变：运行时内部读结果都读本地的，按页放置的结果只在外部调用 `layout_result` 时缓存，提取也直接共享本地结果。

公开接口变化：`UiWorld::layout_result` 返回 `Option<Arc<LayoutResult>>`（原来是 `Option<&LayoutResult>`）；新增 `LayoutFrame`、`local_layout_box`、`layout_frame`、`local_layout_result`；`ExtractedNode::layout_result`、`ExtractedMove::layout_result` 与 `StickyAncestor::result` 改为节点自身坐标里的结果；`UiMutation::PlaceLayout`（隐藏，引擎写回用）。`MutationQueue::write_layout` 仍收文档坐标，提交结束时按深度换成本地值，没写的子节点随父节点平移（以前保持原位）。

相对几何第 2 阶段（2026-11-03）：命中条目改为本地的——每个条目在自己边框盒的坐标里，另存离命中父节点原点的 `origin`，查询时逐层减去（与滚动的 `shift` 同一条路）；平移子树只重建根条目，后代条目不碰。读了自身偏移以外几何的条目（跨参照系的偏移、逃逸、组件几何、粘性）记为钉住，脏节点下面的钉住条目单独重建。42 001 盒（与 35045d816 交替 8 轮读最小值，负载 3.2–3.9，µs）：

| 编辑 | 阶段 | 35045d816 | 第 2 阶段 |
| --- | --- | ---: | ---: |
| height | 布局 | 3 792 | 3 779 |
| height | 命中 | 3 617 | 660 |
| height | 无障碍 | 3 230 | 3 003 |
| height | 提取 | 3 769 | 3 767 |
| height | 整帧 | 17 325 | 13 789 |
| width | 布局 / 整帧 | 760 / 776 | 762 / 778 |
| 控件页 text | 布局 / 整帧 | 10 / 22 | 10 / 22 |

每次编辑的指令（`RUNS=240` 与 `RUNS=40` 两次 `time -l` 之差除以 200）：height 167.7 M → 139.5 M（−17%）。命中阶段剩下的 0.66 ms：1 012 个写回的行各重建自己那一条（`hit_test_entries_moved` 1 008，`hit_test_nodes_rebuilt` 1 012）、按深度刷新祖先包围盒，以及在约 2 万个仍标 INPUT 的后代里跳过随父节点平移的那些（`layout_followers`，第 3 阶段已随这些标记一起去掉）。

相对几何第 3 阶段（2026-11-03）：提交不再把平移子树的后代标脏（`spread_layout_moves`、`layout_followers` 删除），只发平移的根。Scene 按参照系链放置记录，不画东西的记录留在提取时的空间、靠变换跟上，画东西的记录在 Scene 里重新放置并重建（painter 把纯平移吸附到整像素，见 [relative-geometry.md](relative-geometry.md) 3.5）；无障碍只从平移的根往下走一遍记录。42 001 盒（与 715cc0331 交替 8 轮、每轮交换先后读最小值，负载 3.3–4.1，µs；Scene 一行是另做的临时计时，提取阶段里 `apply_delta_with_moves` 那一段，6 轮）：

| 编辑 | 阶段 | 715cc0331 | 第 3 阶段 |
| --- | --- | ---: | ---: |
| height | 布局 | 2 059 | 470 |
| height | 命中 | 648 | 244 |
| height | 无障碍 | 2 856 | 795 |
| height | 提取（含 Scene） | 3 735 | 80 |
| height | 其中 Scene | 1 954 | 48 |
| height | 整帧 | 11 650 | 1 695 |
| width | 布局 / 整帧 | 105 / 120 | 108 / 121 |
| 控件页 text | 布局 / 整帧 | 10 / 22 | 10 / 22 |

每次编辑的指令（`RUNS=240` 与 `RUNS=40` 两次 `time -l` 之差除以 200）：height 127.1 M → 32.0 M（−75%）。无障碍剩下的 0.8 ms 里约 0.17 ms 是往下走的那一遍，其余是 1 008 个平移的行本身的重投影。

画东西的子树平移时（控件页 `CONTROLS=1 edits 2000 height`，62 001 节点，每行 10 个标签加按钮；读最小值，3 轮）：带可见性索引（每帧后调 `visible_operations`，即有 painter 时）整帧 81.6 → 66.2 ms，Scene 54.6 → 50.7 ms，无障碍 9.7 → 2.9 ms；不带可见性索引的无头运行 Scene 27.3 → 34.1 ms，多出的是 `place_painted_records` 扫一遍整张图元表（约 7.6 ms），有 painter 的路径只看根下面的图元。按钮这类带组件几何的节点在平移时仍完整重新提取。painter 段（`nana-text-paint-benchmark --workload reflow`，新增：首个标签高度每帧来回 0.5 px，下面的标签全部平移）：1 000 标签 `batch` p50 0.15 ms、10 000 标签 2.5 ms，前后一致；不重建文字实例（`text_instance_rebuilds` 0），每帧只上传每节点的平移行（10 000 标签 478 KB `text_presentation_upload_bytes`）；`flush` 10 000 标签 26.4 → 27.7 ms（每个标签都是直接平移的根，没有可省的子树）。

公开接口变化：`ExtractedNode` 新增 `local_layout`、`layout_frame`（字面量构造要补上）；`ExtractedMove` 新增同名两项，只给平移的根发；`SystemWork` 新增 `layout_moved`；`UiScene` 新增 `node_layout_bounds`。`UiScene::extracted_node(id).layout` 与 `node_bounds` 现在是记录所在的空间，平移子树里不画东西的记录可能停在提取时的位置；现在的位置用 `node_layout_bounds`，绘制位置仍是 `draw_node_bounds` / `draw_primitive`。

相对几何第 4 阶段（2026-11-04）：图元、节点自己的裁剪和 `ExtractedNode.component_geometry` 改为节点本地坐标，painter 先把几何放到节点原点（每个坐标一次加法）再变换、再把平移吸附到整像素，所以布局平移不再被当成滚动吸附，挪了小数像素的子树和冷场景逐像素相同（做法见 [relative-geometry.md](relative-geometry.md) 3.5、3.6）。`place_painted_records` 与提取的 `component_geometry.is_none()` 守卫删除：平移的子树一个图元也不重建、按钮输入框不再重新提取，剔除索引整棵平移（`translate_subtree`），只有读页面的 visual（Select、MenuSurface、NativeMarkdown）和读了自己位置的节点（`origin_bound`）还按节点处理。与 3e9c83b80 交替 8 轮、每轮交换先后读最小值（负载 2–6.5，µs；VISIBLE=1 是 `perf/micro/stack-depth` 新增的开关，每帧后调 `visible_operations`，即有 painter 时）：

| 页面 | 编辑 | 阶段 | 3e9c83b80 | 第 4 阶段 |
| --- | --- | --- | ---: | ---: |
| 控件页 62 001 节点，VISIBLE=1 | height | 提取（含 Scene） | 59 183 | 330 |
| 控件页，VISIBLE=1 | height | 整帧 | 66 494 | 4 076 |
| 控件页，无头 | height | 提取（含 Scene） | 41 292 | 122 |
| 控件页，无头 | height | 整帧 | 48 846 | 3 731 |
| 控件页 | height | 布局 / 无障碍 | 2 934 / 2 928 | 801 / 2 207 |
| 简单页 42 001 盒，无头 | height | 提取 / 整帧 | 81 / 1 782 | 58 / 1 728 |
| 简单页，VISIBLE=1 | height | 提取 / 整帧 | 86 / 1 736 | 60 / 1 748（另 3 轮） |
| 简单页 | width | 整帧 | 122 | 119 |
| 控件页 | text | 整帧 | 22 | 22 |

每次编辑的指令（`RUNS=240` 与 `RUNS=40` 之差除以 200）：控件页改行高 407 M → 54 M（无头）、597 M → 64 M（VISIBLE=1），简单页 31 M → 31 M。控件页布局段的下降来自 `settle_layout_moves` 不再把平移根下面的 2 万个按钮标 RENDER。剩下的大头是无障碍（2.2 ms，`project_viewport_entries`；已在收尾一步降到亚毫秒，见下）。

painter 段（`nana-text-paint-benchmark --workload reflow`，交替 8 轮读最小值，ms）：1 000 标签 flush 2.07 → 1.76、batch 0.151 → 0.192，合计 2.22 → 1.95；10 000 标签 flush 27.7 → 23.4、batch 2.47 → 2.59，合计 30.1 → 26.0。两边都不重建文字实例（`text_instance_rebuilds` 0），平移行上传量相同；batch 多出的约 20 ns/标签是重定基时为每个节点读一次放置原点（`DrawAttributes.origin`），flush 少的是 Scene 不再为每个平移的标签重建图元、剔除索引不再逐个重算。快照套件与 3e9c83b80 的输出逐字节相同（680 张，`gallery-sidebar-collapsed-dark` 为已知抖动）。NativeMarkdown 先试过放进本地坐标：整篇文档的排版从内容盒一路累加，按本地累加再放到页面上和按页面累加在末位不同，`gallery-rich-text` 两张快照的 Mermaid 曲线有 7 个像素差 1 级，所以它留在页面坐标里（平移时重新提取）。

公开接口变化（第 4 阶段）：`ScenePrimitive` 的几何（`bounds`、批量矩形、笔画点、`Path` / `Chart` 的 `origin`、`LayerBegin` 的裁剪、`LayerMask.origin`、节点自己的裁剪）改为节点本地坐标（读页面的 visual 除外）；页面位置用新增的 `UiScene::primitive_layout_bounds` / `place_rect`。`SceneDraw` 新增 `origin` 字段与 `place`、`placed_bounds`、`geometry_transform`，`transform` 不再含原点，`clips` 全在页面上。`ExtractedNode.component_geometry` 改为本地坐标（`UiWorld::component_geometry` 仍是页面坐标，不变）；新增 `StandardVisual::geometry_reads_page`。`UiScene::node_bounds` 恢复为现在的页面盒子（与 `node_layout_bounds` 相同）。

`component-gallery` 的语义快照另有 8 个与此无关的差异（对话框、空状态等文字的 `line_height` 由 `Relative` 变为 `normal`），来自 b1bc92819，不含这些改动的树上同样出现，基线没有重录。

相对几何收尾·无障碍（2026-11-04）：平移的根下面不再逐个走记录（`reach_moved_subtree` 原来每个根整棵走一遍），改为从 `NodeStore` 记着的钉住盒子（参照系不是父节点、裁剪、不交恒等、Select/TreeView/MenuSurface/ModalFrame）往上找最近的平移根（`reach_moved_pins`，O(钉子数 × 深度)）；根到页面不交恒等时仍整棵走。剩下的是平移的根本身的重投影：`axis_aligned` 原来在有任何 visual 时整体退回变换链现算（控件页每个子节点一次 `hit_motion_layout` 与变换合成），现在逐个看链上的盒子；被问的盒子与它的隐藏判定不再进记忆表（原来每个子节点两次哈希插入、表反复扩容），子节点筛选按子节点数预留。做法见 [relative-geometry.md](relative-geometry.md) 3.7。与 6838427fb 交替 8 轮、每轮交换先后读最小值（负载 11–17，µs，绝对值比安静时高）：

| 页面 | 编辑 | 阶段 | 6838427fb | 本次 |
| --- | --- | --- | ---: | ---: |
| 控件页 62 001 节点 | height | 无障碍 | 3 165 | 635 |
| 控件页 | height | 整帧 | 5 783 | 2 567 |
| 简单页 42 001 盒 | height | 无障碍 | 1 113 | 362 |
| 简单页 | height | 整帧 | 2 454 | 1 460 |
| 简单页 | width | 整帧 | 127 | 128 |
| 控件页 | text | 整帧 | 24 | 24 |

分段（临时计时，负载 4–8，读最小值）：控件页原来往下走的那一遍 680 µs → 16 µs，1 013 个平移行的投影 1 833 → 595 µs；简单页 210 → 14 µs、635 → 344 µs。每次编辑的指令（`RUNS=240` 与 `RUNS=40` 之差除以 200）：控件页改行高 55 M → 26 M，简单页 31 M → 23 M。无障碍剩下的是每个平移行自己的投影（相对偏移确实变了）：节点本身、十个子节点的隐藏与对齐判定、放进 delta 的整份 `AccessibilityNode`。

相对几何收尾·读页面的节点（2026-11-04）：NativeMarkdown、关着的 Select 与 MenuSurface 的组件几何改为本地坐标，平移时沿用记录；只有打开的 Select / MenuSurface 记在 `NodeStore` 的 `page_geometry` 里，`settle_layout_moves` 只扫这个集合（原来每次平移扫所有带 visual 的节点）。做法见 [relative-geometry.md](relative-geometry.md) 3.5。与 7fa3b81b4 交替 8 轮、每轮交换先后读最小值（负载 7–14，µs，`RUNS=40 PRINT_STAGES=1 edits 2000 height`）：

| 页面 | 阶段 | 7fa3b81b4 | 本次 |
| --- | --- | ---: | ---: |
| 控件页 62 001 节点 | 布局 | 650 | 529 |
| 控件页 | 整帧 | 1 525 | 1 389 |
| 控件页 VISIBLE=1 | 整帧 | 1 760 | 1 579 |
| 混合页（`MIXED=1`，`rich-text`） | 提取 | 24 818 | 183 |
| 混合页 | 布局 | 2 946 | 1 106 |
| 混合页 | 无障碍 | 4 280 | 3 688 |
| 混合页 | 整帧 | 34 237 | 5 802 |
| 混合页 VISIBLE=1 | 整帧 | 45 030 | 5 690 |
| 简单页 42 001 盒 | 整帧 | 1 212 | 1 245 |
| 控件页 | text 整帧 | 23.2 | 23.5 |

混合页是控件页每个奇数格的按钮换成关着的 Select、每行第一格换成一段 Markdown（Markdown 那一格要用 `--features rich-text` 构建临时副本，提交的基准只带 Select）。简单页没有 visual，两边走同一条路径，差别在噪声内。混合页的无障碍仍是大头：Select 从几何派生虚拟节点，是钉住的盒子（3.7）。

相对几何端到端（2026-11-04）：改造前 ba8ba044f 对最终的 2f0479886，同一份 `perf/micro/stack-depth` 各自构建 release，交替 8 轮、每轮交换先后读最小值（负载 2.4–3.1，µs，`PRINT_STAGES=1 edits 2000 …`）：

| 页面 | 编辑 | 阶段 | ba8ba044f | 2f0479886 |
| --- | --- | --- | ---: | ---: |
| 简单页 42 001 盒 | height | 布局 | 17 623 | 465 |
| 简单页 | height | 命中 / 无障碍 / 提取 | 2 763 / 3 118 / 4 150 | 238 / 298 / 53 |
| 简单页 | height | 整帧 | 30 424 | 1 149 |
| 简单页 | width | 整帧 | 795 | 125 |
| 控件页 62 001 节点 | height | 布局 | 31 238 | 501 |
| 控件页 | height | 命中 / 无障碍 / 提取 | 4 986 / 8 741 / 36 643 | 247 / 309 / 57 |
| 控件页 | height | 整帧 | 85 988 | 1 233 |
| 控件页 VISIBLE=1 | height | 整帧 | 86 539 | 1 441 |
| 控件页 | text | 整帧 | 22.4 | 22.9 |
| 控件页 VISIBLE=1 | text | 整帧 | 22.5 | 24.0 |

每次改行高的指令（`/usr/bin/time -l`，`RUNS=240` 减 `RUNS=40` 再除以 200）：简单页 326 M → 23 M，控件页 834 M → 25 M。改行高两页都从几十毫秒降到约 1.2 ms，与 Taffy 单做布局（约 0.48 ms）同一数量级；改宽度 0.80 → 0.13 ms；改控件文字持平（差别在 1–1.5 µs，噪声量级）。控件页的 ba8ba044f 一行包含当时按钮、输入框每次平移都重新提取的代价。混合页（`MIXED=1`）在 ba8ba044f 上不存在，没有端到端数字，见上一段。

已做的（2026-11-02）：发布阶段每个结果的四个切片在复用的暂存表里建好、按最终长度一次分配，空切片不分配（`Arc::<[T]>::default()`），按父分组的子集改为一张排序表，不再每个父节点一个 `Vec`；意图缓存键的环境哈希从 SipHash 换成固定种子的 foldhash，`seeded_intrinsic` 换成 hashbrown；`ConstraintClass` 的相等与哈希一趟读出归一化后的标签与值，不再各自归一化两遍；`RetainedIntrinsicMetrics` 每个内容的变体表按一增长（首个 push 原本预留 4 个，320 B 只用 80 B）；`MeasurePlanSlots` 的两个槽装箱（原本每个容器 528 B 内联在表里，通常只用一个槽）；整页 pass 按节点数和容器数预先给各表留好容量；锚点记忆表在父节点已结算时不再分配。

## 待办

- **冷布局吞吐与每盒内存**：上面两张表 NanaUI 仍落后 Taffy 约 3 倍（时间）与 2 倍（内存）。剩下的成本是摊开的，没有单点：每盒约 16 次分配（每个容器的 `descend`、`flow`、`child_sizes`、计划条目等 `Vec`，单去掉它们省不出多少时间）、每个节点在测量里的十来项判定（在场、浮层分支、菜单分支、行盒记忆，各自一次按 id 的查表）、每次写入落四张意图缓存表、十几张按 `StableNodeId` 的哈希表（`LayoutInputMap` 的输入表每条 264 B，一万盒时 2.8 MB，查一次就是一次缓存未命中；`style()` 每盒被调约 18 次，分散在 14 处）、以及约 4.8 KB 的 `LayoutStyle`（一次判定常常要碰几条缓存行）。再往下要换存储：按节点下标的稠密数组代替这些表、`LayoutResult` 不再给每个子节点存三份几何，这是结构改动，需要先对 `WorkCounters` 合同和消费者（Scene、命中、无障碍、`issue205` 等测试）逐个确认。
- **往已布局的界面里挂深子树**：挂载本身已是线性（见下），但插入时沿祖先清理布局结果的两条链（`suppress_layout_result_chain`、`clear_layout_result_chain`）在世界里已有布局结果时仍走到最近的固定尺寸边界，代价是「插入的节点数 × 深度」。首次挂载时表是空的，直接跳过。要在已处理过的祖先处提前停，得先确认边界节点的样式在两次插入之间不会变，否则会读到过期结果。
- **`AppContext` / `UiWorld` 内联大小**：把大块字段装箱，构造路径的帧就能回到预算内。
- **railroad 解析的 debug 帧**：`alt → seq → term → primary` 每层五个帧、合计 5 KiB 以上，48 层在 debug 下余量不到一倍。把 `Result<Expr, Fail>` 的大小压下来，或把上限降到 32。
- **移动端实机**：`stack` 的 iOS 与 Android 分支只做过交叉 `cargo check`（`aarch64-apple-ios`、`aarch64-linux-android`，后者先 `source scripts/android-env.sh`），没有在设备上跑过深树门禁。
