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

所以预算按最紧的两档定：release 的一整帧必须在 **256 KiB** 内跑完（512 KiB 线程留一半余量），debug 在 **512 KiB** 内。

### 树深上限

一个节点最多在第 `MAX_TREE_DEPTH = 512` 层（`nana_ui_runtime::MAX_TREE_DEPTH`）；超过时挂载返回 `UiWorldError::TreeTooDeep`，不会走到任何递归。Blink 的 HTML 解析器对 DOM 嵌套用的是同一个上限。

### 规则：哪些遍历可以递归

1. **测量**和**视图建树**是仅有的两处按树深递归的地方。测量里尺寸要从约束传下去的子项里算上来，建树时一个元素要在自己的作用域里建子元素，二者都没有不改变语义的迭代写法。每一层都必须经过兜底：测量走 `layout_engine::recursion::level`，建树走 `crate::stack::level`。剩余栈不足 256 KiB 时这一层换到新分配的 4 MiB 栈段上继续（`stacker::grow`）。剩余栈在桌面平台由 `stacker` 读取，在 iOS 和 Android 上读线程自己的栈边界（`pthread_get_stackaddr_np` / `pthread_getattr_np`）——`stacker` 读不到这两个平台，此前兜底在那里不生效。只有 wasm 读不到。
2. **放置**是有界递归：一次调用最多下降 `recursion::SPAN`（24）层，再往下的节点只写下自己的盒子，子树作为边界排进本趟队列，由最外层调用按顺序放完再返回。递归深度与树深无关，放置不需要兜底。见 [布局 · 布局递归与栈](layout.md#布局递归与栈)。
3. **其余所有树遍历**都必须用循环或堆上的显式栈，不得按深度递归。这次改掉的：命中索引建树（`HitIndex::insert_tree`）、样式解析沿祖先链（`resolve_style`）、行盒查询（`holds_line_box_in`）、原子基线沿祖先上溯（`atom_baseline_reads_content`）。新增遍历时用 `scripts/audit-stack-frames.py` 和下面的门禁验证。
4. 解析运行时内容（Markdown、Mermaid 图、CSS）的递归按输入嵌套深度走，同样要有深度上限或显式栈；它们不在下面这道门禁里，逐个审计的结果见文末「待办」。

### 门禁

| 门禁 | 守的是什么 | 命令 |
| --- | --- | --- |
| `nana-ui-scene/tests/deep_tree_stack.rs` | 508 层（上限减 4）的树在 256 KiB（release）/ 512 KiB（debug）的线程上走完整帧：挂载、布局、提取、命中、无障碍、改最深处的文本再走一帧、析构 | `cargo test -p nana-ui-scene --test deep_tree_stack`，再加 `--release` |
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

它们大，是因为 `AppContext`（18 KiB）和 `UiWorld`（14 KiB）按值内联在栈上、构造时搬了几次。debug 下帧路径上有三个函数远超预算：`UiWorld::derive_component_geometry` 111 KiB、`UiScene::build_node_primitives` 74 KiB、`controls_base::build` 72 KiB（release 分别是 9、5、6 KiB）。它们都是按 `StandardVisual` 分支的大 `match`，未优化的代码给每个分支的局部变量都留了槽。不在递归里，但每帧都在调用链上，是 debug 下整帧栈预算要 512 KiB 而不是 256 KiB 的主要原因。

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

NanaUI 的计时含布局写回的提交与结果发布（Taffy 没有对应步骤）。10 501 盒时一次布局约 39% 在放置、26% 在测量、18% 在两次发布、其余是写回与收尾。深链（`report`，同样交替读最小值）：布局 64 / 256 / 500 层 0.16 / 0.90 / 2.49 ms，整帧 0.84 / 7.4 / 24.7 ms。

本机负载常年在 5–15 之间，单次墙钟可以差一倍；比例比绝对值可信，对比要交替多跑几轮读最小值。剖析用 `cargo run --release -- wide-loop` 配 `sample`（宽树栈浅，归因可信）。

已做的（2026-11-02）：发布阶段每个结果的四个切片在复用的暂存表里建好、按最终长度一次分配，空切片不分配（`Arc::<[T]>::default()`），按父分组的子集改为一张排序表，不再每个父节点一个 `Vec`；意图缓存键的环境哈希从 SipHash 换成固定种子的 foldhash，`seeded_intrinsic` 换成 hashbrown；`ConstraintClass` 的相等与哈希一趟读出归一化后的标签与值，不再各自归一化两遍；`RetainedIntrinsicMetrics` 每个内容的变体表按一增长（首个 push 原本预留 4 个，320 B 只用 80 B）；`MeasurePlanSlots` 的两个槽装箱（原本每个容器 528 B 内联在表里，通常只用一个槽）；整页 pass 按节点数和容器数预先给各表留好容量；锚点记忆表在父节点已结算时不再分配。

## 待办

- **冷布局吞吐与每盒内存**：上面两张表 NanaUI 仍落后 Taffy 约 3 倍（时间）与 2 倍（内存）。剩下的成本是摊开的，没有单点：每盒约 25 次分配（每个容器的 `descend`、`flow`、`child_sizes`、计划条目等 `Vec`）、十几张按 `StableNodeId` 的哈希表（`LayoutInputMap` 的输入表每条 264 B，一万盒时 2.8 MB，查一次就是一次缓存未命中；`style()` 每盒被调约 18 次，分散在 14 处）、以及冷布局把整批结果发布两次（提交写回时一次 `CompatibilityWrite`，帧里再按 `RuntimeLayout` 校验一次，第二次全部判定未变）。再往下要换存储：按节点下标的稠密数组代替这些表、`LayoutResult` 不再给每个子节点存三份几何，这是结构改动，需要先对 `WorkCounters` 合同和消费者（Scene、命中、无障碍、`issue205` 等测试）逐个确认。
- **深链上的整帧**：已定位。500 层 24.7 ms、256 层 7.4 ms 的平方项不在发布（计数确认发布是线性的：每次 501 次锚点查询、501 步上溯，`sample` 把时间归到 `layout_anchors` 是深栈截断造成的误归因），而在场景提取：250 → 500 层时 Extract 2.7 → 10.9 ms（×4），TextShape 0.56 → 1.7 ms（×3），布局与挂载都是 ×2。提取里每个节点都从根走一遍祖先：`compute_ancestor_state`（同父兄弟只有单槽缓存，链上每个节点都是独子，全部未命中）、`is_descendant_of_icon_visual`、`is_descendant_of_rasterized_svg`、`filter_groups_from`、`group_prefix`。要线性就得在重建期间按父节点记忆「给子节点的折叠状态」：子节点不逃逸裁剪链、本身不是 `fixed`、不是工作区调整手柄、祖先里没有 `ModalFrame` 时，`F(P) = step(F(parent(P)), P)`，遇到 `fixed` 祖先在它那里重置几何、透明度照乘；其余情况退回现有算法。TextShape 的 ×3 还没拆。
- **debug 的大 `match` 帧**：把 `derive_component_geometry`、`build_node_primitives`、`controls_base::build` 的大分支拆成各自的函数，debug 整帧就能降到 256 KiB。
- **`AppContext` / `UiWorld` 内联大小**：把大块字段装箱，构造路径的帧就能回到预算内。
- **运行时内容解析的递归**：`nana-ui-diagram` 的各家族解析（zenuml、usecase、mindmap、block、c4、railroad）、`rich_text::layout_block`、`i18n::message`、`paint_script::parse_path` 和 `nana-ui-css` 的嵌套规则都按输入嵌套深度递归，需要逐个加深度上限或改成显式栈，并给每个加一条深嵌套输入的测试。
- **移动端实机**：`stack` 的 iOS 与 Android 分支只做过交叉 `cargo check`（`aarch64-apple-ios`、`aarch64-linux-android`，后者先 `source scripts/android-env.sh`），没有在设备上跑过深树门禁。
