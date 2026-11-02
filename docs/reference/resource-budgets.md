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

对照（`cargo run --release -- wide`，一个 1 列 × N 行 × 10 格的页面，每格一列套一行）：

| 盒子数 | NanaUI 每盒 | Taffy 每盒 |
| ---: | ---: | ---: |
| 2 101 | 3.7 KB | 0.9 KB |
| 10 501 | 3.2 KB | 1.5 KB |
| 21 001 | 3.2 KB | 1.5 KB |

二者存的东西不同：Taffy 只存布局样式与布局缓存；NanaUI 的节点还带组件记录、解析后的样式、保留的布局结果、失效与依赖索引和无障碍状态。即便如此，每盒约 2 倍是一个真实差距，见「待办」。

## CPU 时间

增量帧的成本由 `perf/` 的性能合同管（`perf/README.md`）：静止帧不做功、paint-only 不重排、一处编辑不扫兄弟，判据是 `WorkCounters`，不是墙钟。

冷启动的整页布局没有门禁，只有观察（同上的 `wide`，release，最快一次）：

| 盒子数 | NanaUI | Taffy |
| ---: | ---: | ---: |
| 2 101 | 2.2 µs / 盒 | 0.7 µs / 盒 |
| 10 501 | 2.5 µs / 盒 | 0.7 µs / 盒 |
| 21 001 | 2.9 µs / 盒 | 1.0 µs / 盒 |

本机负载常年在 5–15 之间，单次墙钟可以差一倍；比例比绝对值可信，对比要交替多跑几轮读最小值。

## 待办

- **冷布局吞吐与每盒内存**：上面两张表 NanaUI 都落后 Taffy 2–3 倍。布局本身的热点是分配、`SipHash` 的默认哈希表和意图缓存的键；发布阶段在深链上仍是超线性的（`publish_layout_results`）。
- **深链上的整帧**：500 层的整帧要 26 ms，256 层 7.7 ms，有一处按深度平方增长，在挂载、提取或无障碍里，尚未定位。
- **debug 的大 `match` 帧**：把 `derive_component_geometry`、`build_node_primitives`、`controls_base::build` 的大分支拆成各自的函数，debug 整帧就能降到 256 KiB。
- **`AppContext` / `UiWorld` 内联大小**：把大块字段装箱，构造路径的帧就能回到预算内。
- **运行时内容解析的递归**：`nana-ui-diagram` 的各家族解析（zenuml、usecase、mindmap、block、c4、railroad）、`rich_text::layout_block`、`i18n::message`、`paint_script::parse_path` 和 `nana-ui-css` 的嵌套规则都按输入嵌套深度递归，需要逐个加深度上限或改成显式栈，并给每个加一条深嵌套输入的测试。
- **移动端实机**：`stack` 的 iOS 与 Android 分支只做过交叉 `cargo check`（`aarch64-apple-ios`、`aarch64-linux-android`，后者先 `source scripts/android-env.sh`），没有在设备上跑过深树门禁。
