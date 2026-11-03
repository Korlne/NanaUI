# 相对几何：平移一棵子树只改一个偏移

本文是布局几何从「绝对坐标为准」迁到「相对父节点为准」的设计与分阶段计划。目标是一次编辑的代价跟真正变了的东西成正比，而不是跟被挪动的节点数成正比。

## 1. 现状与问题

42 001 盒、2 000 行的页面上改一行行高（`perf/micro/stack-depth`，`edits 2000 height`，main `ba8ba044f`），整帧约 36 ms：

| 阶段 | 耗时 |
| --- | ---: |
| 布局 | 21.1 ms |
| 提取 | 4.6 ms |
| 无障碍 | 3.7 ms |
| 命中 | 3.3 ms |

Taffy 做同一次编辑只要 0.48 ms。真正重新排布的只有约 1 000 行，其余约 2 万个节点只是平移。计数器显示有 21 001 个结果被改写，命中索引挪了 20 997 个条目。

原因在于几何以页面绝对坐标为准，而且好几层各存了一份绝对副本：

- 权威盒子 `NodeRecord.layout`（`store.rs`），引擎内的副本 `DocumentLayoutCache.boxes`，以及 `ContainerPlan` 的原点；
- `LayoutResult` 的所有位置字段：`bounds`、`border_box`、`padding_box`、`content_box`、基线、`overflow`、`scroll_extent`、`child_placements`、`fragments`、`parts`；
- `ComponentGeometry` 里的每个区域（按钮标签、输入框光标等）；
- `ContentBoundsIndex`；
- 命中条目 `HitEntry.layout`、裁剪和变换；
- 提取记录 `ExtractedNode.layout`，以及 Scene 图元的 `bounds`。

平移一棵子树，每一层都要把子树里的每个节点改写一遍。

之前的几步（c642ac283 起）把求解换成了本地坐标，把每个被挪节点的改写成本压低了，但被挪节点的个数没变。

## 2. 已有的样板：滚动

滚动早就是「只改一个偏移」的形态，三层都有现成路径：

- **命中索引**：滚动容器上的 `shift`，查询时逐层把点换到子节点的坐标里（`open_hit`）。更新只需改一处，再刷新祖先包围盒，代价 O(深度)。
- **Scene**：`scroll_translations` 只抬 `attribute_epoch`。子孙的图元不重建，绘制时由 `node_projection` 按 `current · inverse(base)` 重定基（rebase），可见性索引用 `translate_subtree` 惰性平移。
- **文字**：整像素部分的平移进共享的 `TextPresentationGpu` 行。

本设计就是让「子树换了位置」走和滚动同一类路径。

## 3. 设计

### 3.1 布局：以相对偏移为准

- `NodeRecord` 存「相对父节点边框盒原点的偏移 + 尺寸」，引擎的 `boxes` 和 `ContainerPlan` 的原点也都改成相对值。
- 父节点的边框盒原点就是子节点偏移的参照点。对于逃逸的绝对定位和 `fixed`，参照点是包含块或视口，规则和今天求解时的一致。
- **逐位精确的约定**：绝对坐标一律定义为 `abs(child) = abs(parent) + local(child)`，按坐标轴从根往下逐级相加。冷求解也改成先求出本地偏移、再这样累加，所以增量和冷对照永远逐位相等，平移也永远不需要做 delta 加法。这会让个别坐标和今天差在最后一位（ulp）之内，预期快照零像素变化；如果确实出现像素差，先逐张核对，再决定是否重录。
- `translate_moved_children` 的整棵子树遍历随之取消。父节点重新排布时只写直接子节点的偏移，孙子辈原封不动。

### 3.2 布局结果：本地存储，绝对值按需给

- `LayoutResult` 改为在本节点坐标里存：边框盒在 (0,0)。`child_placements` 用父节点的本地坐标，`overflow` 和 `scroll_extent` 也一样。
- 节点只是被平移时，它的结果不变。这正是 c642ac283 引入的 `OwnGeometry` 的完全体，`placed` 那一层随之去掉。
- `ComponentGeometry` 同样改为节点本地坐标；`ModalSurfaceSections` 早就是 surface 本地的。

### 3.3 对外 API：语义不变，绝对值靠推导

运行时内约 230 处读几何。对外的 `layout_box`、`layout_result`、`component_geometry`、`viewport_layout_box` 等都保持返回绝对（文档）坐标：

- `layout_box(id)` 从根往下累加偏移，代价 O(深度)，不需要缓存。
- `layout_result(id)` / `component_geometry(id)` 由一个惰性的绝对副本缓存给出。每条缓存记下它是按哪个绝对原点放置的，读的时候比一下当前推导出的原点，不一致才重新放置。代价是每次读 O(深度)，摊还 O(1)。返回类型可能从 `&LayoutResult` 变成 `Arc<LayoutResult>`；字段访问能自动解引用，基本兼容，具体影响由第 1 阶段核实。
- 帧路径上的消费者（提取、Scene、命中、无障碍、`ContentBoundsIndex`、粘性定位）一律改读本地值，不触发绝对值的推导。

### 3.4 命中索引

- 每个条目存本地盒子和本地变换。`shift` 推广为「本节点原点 − 滚动」，查询沿用 `open_hit` 逐层换坐标的做法。
- 平移子树时只改根条目的原点，再按深度刷新祖先包围盒，和今天的滚动一样。
- 本来就是绝对的那几类继续单独处理：`fixed` 根、#201 逃逸条目、菜单和悬停卡连接线、粘性定位、透视。
- 在旋转或缩放的祖先下，原点要换到变换后的空间，`update_hit_test_scroll` 已经按线性部分做了缩放。

### 3.5 提取与 Scene

- 提取记录和图元都改为节点本地坐标，节点原点放进节点变换（`node_scene_transform`），平移原点同样走 CSS 变换的轴心。
- 运行时每棵被挪动的子树只发一条根平移，不再每个后代各发一条 `ExtractedMove`。
- Scene 收到根平移时，按 `scroll_translations` 的方式处理：抬 `attribute_epoch`，靠投影重定基，可见性索引做 `translate_subtree`，子孙图元不重建。
- 绘制时仍要按绝对坐标读几何的地方，统一经 `draw_node_rect` 取绝对值，不再直接读 `bounds`：
  - painter 的分组和边缘渐隐；
  - devtools 的 `scene_probe`；
  - `GlyphObject`、`EdgeFadeMask`、`InsetShadowOverlay`、`CompositorLayer.clip`；
  - 动效的轴心。

### 3.6 painter：先测量，再决定

- painter 的整批重建只遍历**可见**图元，代价和视口里的内容成正比，跟页面总节点数无关。改行高的基准不测 painter。
- 先在视口内塞满图元的场景上测量平移帧的 painter CPU 时间。只有这部分显著时，才做每节点变换的存储缓冲（索引方式同 `transform_id`），让平移只需写一次缓冲、批次照旧沿用。
- 文字的亚像素相位跟绝对 x 走，平移了小数像素就要重新求字形，这是正确性要求，不能省。

### 3.7 无障碍

`AccessibilityNode.bounds` 已经是相对父节点的（47aa93fb2）。改成相对布局后，「偏移逐位没变」的判断直接读本地值即可，发布时的那次检查也随之消失。

## 4. 验证

- **冷对照**：`assert_matches_cold`，以及各层的冷重建对照（命中 `check_moved_hit_entries`、提取逐字段比对、Scene 冷建、无障碍冷投影）全部保留，并改为在相对数据上比对；绝对值另外经推导后再比一次。
- **端到端**：`crates/nana-ui-scene/tests/moved_extraction.rs` 加入组件（按钮、输入框）、滚动容器、`fixed`、逃逸绝对定位、粘性、变换祖先等场景。
- **基准**：改行高、改宽度、改控件文字三组交替跑 A/B，读最小值，再加 `RUNS=240 /usr/bin/time -l` 的 instructions retired。
- **快照套件**：预期零像素变化。

## 5. 分阶段

| 阶段 | 内容 | 依赖 |
| --- | --- | --- |
| 1 | 布局权威改相对；结果、组件几何改本地；对外 API 靠推导给绝对值；`ContentBoundsIndex`、粘性定位改读本地值 | 无 |
| 2 | 命中索引用本地条目，平移只改根原点 | 1 |
| 3 | 提取和 Scene 用本地图元、根平移、投影重定基 | 1 |
| 4 | painter 测量，按结果决定是否做每节点变换缓冲 | 3 |
| 5 | 端到端基准，更新 `docs/reference/resource-budgets.md` | 2、3 |

第 2、3 阶段在第 1 阶段落地之后并行推进。

**目标**：改行高时整帧和 Taffy 布局在同一数量级，个位数毫秒；改宽度和改控件文字不回退。
