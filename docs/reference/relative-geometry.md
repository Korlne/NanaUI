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

- `NodeRecord` 存「相对参照系原点的偏移 + 尺寸」（`local_layout`）和参照系本身（`layout_frame: LayoutFrame`）。引擎的 `DocumentLayoutCache.boxes` 和 `ContainerPlan` 的原点也都是相对值，非父节点的参照系另记在 `DocumentLayoutCache.frames`。
- 参照系（`LayoutFrame`）有三种：
  - `Parent`：父节点边框盒原点。普通流里的盒子，以及包含块就是父节点的 `absolute` 盒子。
  - `Block(id)`：包含块的边框盒原点。包含块在父节点之上的 `absolute` 盒子（逃逸的绝对定位）。
  - `Page`：文档原点。根、`fixed` 盒子、包含块是初始包含块的 `absolute` 盒子、触发式菜单的菜单项（它们挂在视口上）。`Page` 下的本地值就是绝对值，和改造前逐位相同。
- 引擎的做法：`place_node_body` 一律以 `Point::ZERO` 作为本容器的坐标系原点放置子节点，`place_node_head` 写出的盒子就是相对参照系的偏移。跨参照系的读取只有几处，逐一改过：
  - 定位子节点的包含块内边距盒在包含块自己的坐标系里（`padding_box`）；
  - 静态位置要从容器换到包含块的坐标系，经 `content_origin_in` 沿参照系链求和；
  - 触发式菜单的菜单项放在页面坐标里，菜单自身的页面原点由 `origin_in` 求出；
  - 行内上下文求部件时，用盒子自己坐标系里的边框盒。
- 静态位置的逃逸绝对定位会随包含块与容器之间的盒子移动。沿用 `place_remote_overlays` 每遍对未放置容器的复查，比较项多一个「静态位置换到包含块坐标系后是否变了」，代价 O(逃逸条目数 × 深度)。
- **逐位精确的约定**：绝对坐标一律定义为 `abs(child) = abs(frame) + local(child)`，按坐标轴从最上面的参照系往下逐级相加（`UiWorld::record_layout_origin`、引擎的 `origin_in`、`RuntimeLayoutEngine::layout_document` 的 `page_boxes` 用同一种加法顺序）。冷求解同样先求本地偏移、再这样累加，所以增量和冷对照逐位相等，平移也不需要做 delta 加法。
- `translate_moved_children` 与 `place_moved_container` / `place_moved_children` 随之删除。父节点重新排布时只写直接子节点的偏移，孙子辈原封不动：只挪了位置的容器在父节点那里 `subtree_unchanged` 不成立，进 `place_node_body` 后计划照旧成立，一个子节点也不碰。

### 3.2 布局结果：本地存储，绝对值按需给

- `LayoutResult` 在本节点坐标里存：边框盒在 (0,0)。`child_placements`、子节点片段和 `ChildPlacement` 部件存子节点的本地盒子（它自己的参照系里），`overflow` 和 `scroll_extent` 在本节点坐标里，只并入参照系是本节点的子节点；参照系在别处的子节点（`fixed`、逃逸的绝对定位、菜单项）在放到页面上时才并入溢出框。
- 节点只是被平移时，它的结果不变，发布时 `geometry_eq` 直接判为相同。`placed` 层、`OwnGeometry`、`moved_local_result` 和 `place_layout_result` 都去掉了。
- `ComponentGeometry` 本来就不存：`component_geometry(id)` 每次按当前盒子现算，仍然给绝对坐标。第 3 阶段要让带组件几何的节点也走轻量路径时，需要一个按本地坐标算的版本。`ModalSurfaceSections` 早就是 surface 本地的。

### 3.3 对外 API：语义不变，绝对值靠推导

对外的 `layout_box`、`layout_result`、`component_geometry`、`canonical_layout_box`、`viewport_layout_box` 等仍返回绝对（文档）坐标：

- `layout_box(id)` 沿参照系链累加，O(深度)，不缓存。
- `layout_result(id)` 返回 `Option<Arc<LayoutResult>>`（原来是 `Option<&LayoutResult>`）。放到页面上的结果缓存在 `page_layout_results` 里，每条记下它是按哪个本地结果（`Arc` 指针）、哪个原点（逐位）放的，读时比一下，不一致才重新放置。运行时内部不走这条路：`canonical_layout_box`、`component_content_box`、部件查询、裁剪链都直接读本地结果再加原点，所以缓存只在外部调用时才填，不会让每个节点多一份结果（`node_memory` 的预算守着这一点）。
- 新增的本地接口：`local_layout_box(id)`、`layout_frame(id)`、`local_layout_result(id)`、`LayoutFrame`；crate 内另有 `layout_origin(id)`（页面原点）和 `layout_box_from(owner, child)`（子节点相对某个祖先的盒子）。
- `RuntimeLayoutEngine::layout_document` / `layout_document_with_frontier` 仍返回页面坐标；框架写回用 crate 内的 `layout_document_with_frontier_local`。
- 外部写的 `WriteLayout` 仍是文档坐标。提交结束时按深度从上往下换成本地值，参照系按样式推断（`layout_frame_from_styles`，与裁剪链的规则一致），所以和写入顺序无关；没写的子节点随父节点平移。引擎写回走隐藏的 `UiMutation::PlaceLayout { id, layout, frame }`。`position` 改变而布局还没重跑时，提交末尾把受影响子树里参照系与样式不符的盒子换到新参照系，页面位置不变（`reframe_layouts`）。
- 帧路径上的消费者读本地值：
  - `ContentBoundsIndex` 每条按自己节点的原点存，父节点读子节点时加上子节点的本地偏移；携带给包含块的范围按落地的包含块的坐标存，所以平移的子树里条目全部不动。落地点可能变的两种情况（`position` 在是否建立包含块之间变化、子树换父节点）会让子树里携带范围的条目重读子节点（`reland`）。
  - 粘性定位从本地盒子和本地结果算，所有盒子都相对节点自己的原点，`StickyAncestor.result` 约定为祖先自己坐标里的结果，按 `bounds` 的原点放置。
  - 提取：`ExtractedNode.layout_result` / `ExtractedMove.layout_result` 改为本地结果（共享同一个 `Arc`，不复制）；`ExtractedNode.layout` 仍是推导出的绝对盒子，这是 Scene 第 3 阶段的事。Scene 读结果的地方只有裁剪链、参与方式、滚动指标（与坐标无关）和粘性定位（已按上面的约定改）。
  - 无障碍见 3.7。
- **第 1 阶段的过渡**（第 2 阶段后只剩 Scene 和无障碍在用）：命中索引和 Scene 仍按绝对盒子工作。提交时 `spread_layout_moves` 从原点变了的盒子往下走，把随之平移的后代标成「只是移动」（INPUT、RENDER，`render_moved_epoch`、`translated_hit`），参照系在子树外的盒子（`fixed`、包含块在外的绝对定位）不动；后代的最近裁剪容器也在动时再标 ACCESSIBILITY。第 2、3 阶段各自本地化后删掉这一步。

### 3.4 命中索引（第 2 阶段，已落地）

- 每个条目都在**自己边框盒的坐标**里：`HitEntry.layout` 是 `(0, 0, 宽, 高)`，裁剪、路径裁剪、菜单、变换的轴心都按这个坐标存。条目另存 `origin`：盒子离命中父节点原点的偏移，经父节点交给子节点的变换的线性部分换算（单位线性时就是偏移本身）。查询时 `open_hit` 先减 `origin` 进入条目自己的空间，再减滚动 `shift` 进入子节点的空间；存进父节点边界树的包围盒是「自身 ∪ 子节点（加 `shift`）」再加 `origin`（`entry_bounds`，冷建和增量刷新同一个公式）。
- 偏移的来源：参照系是父节点且命中父节点就是树上的父节点时，直接用 `local_layout_box`（逐位精确）；`Page` 参照系的命中根（文档根、`fixed`）也直接用本地值；其余（包含块在别处的绝对定位、逃逸条目挂在裁剪父节点上、`fixed` 推断不出 `Page` 的情况）用 `layout_box_from` / `layout_origin` 求差。
- 父节点的变换带透视时，平移不能在外层做：偏移并进条目自己的变换，`origin` 为零，于是子孙依赖它的位置——这种条目不是普通条目，写回时走整棵子树修补。
- 平移子树时只重建根条目（`rebuilt_plain_hit_entry`，原地替换，`origin` 变了计入 `hit_test_entries_moved`），再按深度刷新祖先包围盒；后代条目一个不碰。根条目不原地改 `origin`：同一次提交里可能有兄弟增删，`order` 要从重建里拿。
- **钉住的条目**（`HitEntry.pinned`）：读了自身偏移以外的几何——跨参照系求差的偏移、#201 逃逸的桥接变换、组件几何（选择框菜单、悬停卡连接线、模态框的 surface/body 裁剪，都从页面坐标减去本节点原点）、粘性定位。祖先一动它们就可能变，所以 `HitIndex.pinned` 单独记着；打补丁时，脏节点下面的钉住条目即使没被标脏也重建（`pinned_hit_entries_under`，O(钉住数 × 深度)）。
- 合成层叠加时的查询（`visit_presentation_hits`）仍按页面坐标现算变换，读条目里的本地矩形前先乘上「本地 → 页面」的平移；`pointer_layout_position`、`layout_pointer_position`、`menu_region_contains` 同样在本地与页面坐标之间换一次。附带修正：`clip-path` 的命中以前拿页面坐标去比盒子本地的裁剪形状，盒子不在原点时错位，现在两边都在本地坐标里。
- 冷对照：`check_moved_hit_entries` 在所有补丁打完后，对被替换的条目**及其下面所有没被滚动平移过的条目**逐字段（含 `origin`、`pinned`、包围盒）比对全量重建；`reflow_oracle` 的命中几何加上 `entry_offset`。
- 与 `spread_layout_moves` 的关系：命中索引不再读后代的盒子，`translated_hit` 删除。后代仍被标 INPUT，因为无障碍把「只因移动而标 INPUT」的节点当作移动种子（`accessibility_moved`）；命中索引用 `layout_followers` 跳过其中不是钉住的条目（同一节点再被别的原因标 INPUT 时移出集合）。参照系没跟着动的后代（包含块在外的绝对定位）现在也标 INPUT（`fixed` 除外），因为它们离命中父节点的偏移变了。

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

`AccessibilityNode.bounds` 已经是相对父节点的（47aa93fb2）。改成相对布局后，发布时的那次判断（`boxes_keeping_parent_offset`、推迟标记）去掉了：写回时本地偏移变了的盒子直接标 ACCESSIBILITY，随父节点平移的后代本地偏移不变、不写回也不标；参照系不跟着动的子节点（`fixed` 等）相对父节点的偏移变了，由 `spread_layout_moves` 补标。

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

第 1 阶段已落地：42 001 盒改行高，布局 16.4 → 3.9 ms，整帧 29.9 → 17.8 ms，每次编辑指令 296 M → 166 M（数字与做法见 `resource-budgets.md`）。

第 2 阶段已落地：命中 3.6 → 0.66 ms，整帧 17.3 → 13.8 ms，每次编辑指令 168 M → 140 M；改宽度、改控件文字持平（做法见 3.4，数字见 `resource-budgets.md`）。

**目标**：改行高时整帧和 Taffy 布局在同一数量级，个位数毫秒；改宽度和改控件文字不回退。
