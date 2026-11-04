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
- `ComponentGeometry` 本来就不存：`component_geometry(id)` 每次按当前盒子现算，仍然给绝对坐标。提取用的本地版本（第 4 阶段）见 3.5。`ModalSurfaceSections` 早就是 surface 本地的。

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
  - 提取：`ExtractedNode.layout_result` / `ExtractedMove.layout_result` 改为本地结果（共享同一个 `Arc`，不复制）；`ExtractedNode.layout` 仍是提取时推导出的绝对盒子，第 3 阶段另加 `local_layout` / `layout_frame`（见 3.5）。Scene 读结果的地方只有裁剪链、参与方式、滚动指标（与坐标无关）和粘性定位（已按上面的约定改）。
  - 无障碍见 3.7。
- **第 1 阶段的过渡**（已在第 3 阶段删除）：提交时 `spread_layout_moves` 从原点变了的盒子往下走，把随之平移的后代标成「只是移动」。现在提交只标写回的盒子本身（`settle_layout_moves`）：只挪了位置的根可以沿用提取记录，原点变了的根另记 `DirtyMask::MOVED`，进 `SystemWork::layout_moved`；后代一个不标。

### 3.4 命中索引（第 2 阶段，已落地）

- 每个条目都在**自己边框盒的坐标**里：`HitEntry.layout` 是 `(0, 0, 宽, 高)`，裁剪、路径裁剪、菜单、变换的轴心都按这个坐标存。条目另存 `origin`：盒子离命中父节点原点的偏移，经父节点交给子节点的变换的线性部分换算（单位线性时就是偏移本身）。查询时 `open_hit` 先减 `origin` 进入条目自己的空间，再减滚动 `shift` 进入子节点的空间；存进父节点边界树的包围盒是「自身 ∪ 子节点（加 `shift`）」再加 `origin`（`entry_bounds`，冷建和增量刷新同一个公式）。
- 偏移的来源：参照系是父节点且命中父节点就是树上的父节点时，直接用 `local_layout_box`（逐位精确）；`Page` 参照系的命中根（文档根、`fixed`）也直接用本地值；其余（包含块在别处的绝对定位、逃逸条目挂在裁剪父节点上、`fixed` 推断不出 `Page` 的情况）用 `layout_box_from` / `layout_origin` 求差。
- 父节点的变换带透视时，平移不能在外层做：偏移并进条目自己的变换，`origin` 为零，于是子孙依赖它的位置——这种条目不是普通条目，写回时走整棵子树修补。
- 平移子树时只重建根条目（`rebuilt_plain_hit_entry`，原地替换，`origin` 变了计入 `hit_test_entries_moved`），再按深度刷新祖先包围盒；后代条目一个不碰。根条目不原地改 `origin`：同一次提交里可能有兄弟增删，`order` 要从重建里拿。
- **钉住的条目**（`HitEntry.pinned`）：读了自身偏移以外的几何——跨参照系求差的偏移、#201 逃逸的桥接变换、组件几何（选择框菜单、悬停卡连接线、模态框的 surface/body 裁剪，都从页面坐标减去本节点原点）、粘性定位。祖先一动它们就可能变，所以 `HitIndex.pinned` 单独记着；打补丁时，脏节点下面的钉住条目即使没被标脏也重建（`pinned_hit_entries_under`，O(钉住数 × 深度)）。
- 合成层叠加时的查询（`visit_presentation_hits`）仍按页面坐标现算变换，读条目里的本地矩形前先乘上「本地 → 页面」的平移；`pointer_layout_position`、`layout_pointer_position`、`menu_region_contains` 同样在本地与页面坐标之间换一次。附带修正：`clip-path` 的命中以前拿页面坐标去比盒子本地的裁剪形状，盒子不在原点时错位，现在两边都在本地坐标里。
- 冷对照：`check_moved_hit_entries` 在所有补丁打完后，对被替换的条目**及其下面所有没被滚动平移过的条目**逐字段（含 `origin`、`pinned`、包围盒）比对全量重建；`reflow_oracle` 的命中几何加上 `entry_offset`。
- 第 3 阶段起后代不再标 INPUT，`layout_followers` 删除：平移子树下需要重建的条目只靠 `pinned_hit_entries_under` 找（参照系没跟着动的后代离命中父节点的偏移靠跨参照系求差得出，本来就是钉住的）。

### 3.5 提取与 Scene（第 3、4 阶段，已落地）

第 3 阶段只发平移的根，但根下面画东西的记录仍在同一次 delta 里换成现在的页面盒子并重建图元，带组件几何的节点仍完整重新提取：painter 把纯平移变换吸附到整设备像素（#223，合成滚动的语义），文字的亚像素相位只取图元自身的位置，靠变换平移、挪了小数像素的子树画出来和冷场景不一样（快照 `area-layout/maximized` 上文字整体偏了 0.28 px）；组件几何又是按页面坐标算的。第 4 阶段把两者都改成本地坐标，平移的子树一个图元也不重建：

- **记录**：`ExtractedNode` 带 `local_layout` 与 `layout_frame`（与 `UiWorld::local_layout_box` / `layout_frame` 相同）；`layout` 是提取那一刻的页面盒子，只作参考。`ExtractedMove` 同样带 `local_layout` / `layout_frame`，只给平移的根发。
- **组件几何本地化**：`ExtractedNode.component_geometry` 在节点自己的坐标里（边框盒在原点）。运行时用同一份 `derive_component_geometry` 求它：`UiWorld::local_component_geometry` 把 `geometry_frame` 设成这个节点，期间 `record_layout_origin` 沿参照系链求和时走到这个节点就停（它自己的原点记 0），所以内容盒、部件、子孙的盒子都是相对这个节点的、和节点站在哪无关；链上碰不到它（读了外面的盒子）或读了视口（`document_viewport_of`）就记 `geometry_read_page`，调试构建断言。对外的 `UiWorld::component_geometry` 不设 frame，仍是页面坐标，逐位和改造前相同，命中、IME、无障碍都读它。
- **读页面的 visual**（`StandardVisual::geometry_reads_page`）：只剩打开的 Select（菜单贴视口）和打开的 MenuSurface（菜单项放在页面上）。它们仍在页面坐标里推导、记录在页面上，节点或它上面的盒子一动就重新提取：`NodeStore` 在 `set_visual` 时把它们记进 `page_geometry`，`settle_layout_moves` 只扫这个集合（原来扫所有带 visual 的节点），`extract_nodes_reusing` 的守卫看记录的 visual。关着的 Select 只有标签、提示和把手，推导时不再读视口（`document_viewport_of` 只在打开时调）；关着的 MenuSurface 够不到它的菜单项（`closed_menu_hides`），表面是空框——两者都按本地坐标推导，平移时沿用记录。打开、关上都改 visual，节点整个重新提取，记录在页面与本地之间切换。NativeMarkdown（收尾一步起）也在本地坐标里：整篇文档从本地内容盒往下排，冷场景同样如此，所以平移后和冷场景逐位一致；与改造前按页面累加的结果在末位不同，Mermaid 曲线的抗锯齿让 `gallery-rich-text-dark` / `-light` 两张基线变了几个像素，已重录。其余带组件几何的节点（按钮、输入框等）平移时直接沿用记录。
- **图元本地化**：`build_node_primitives` 以 `UiScene::record_box`（本地记录为 `(0, 0, 宽, 高)`，页面记录为提取时的页面盒子）为 `bounds` 建图元，所有坐标——`bounds`、批量矩形、笔画点、`Path` / `Chart` 的 `origin`、图层裁剪、节点自己的裁剪（`clips` 里继承部分之后的那些）——都在节点自己的坐标里；`transform` 是页面上的变换（祖先的变换、滚动、节点自己绕现在位置的变换），不含原点。
- **放置**：节点现在的页面原点按参照系链从上往下累加本地偏移（`placed_origin`，与运行时 `record_layout_origin` 同样的加法顺序，按 `placement_epoch` 记忆；第一次平移之前直接取 `layout`，`placement_live`）。新提取的记录这个值与 `layout` 逐位相等（调试构建断言）。`paint_origin` 是图元的落点：本地记录取 `placed_origin`，页面记录取 0。
- **绘制**：`draw_primitive` 给出 `SceneDraw { origin, transform, clips }`：几何先加 `origin`（每个坐标一次加法），再过 `transform`；`clips` 全在页面上——继承的那些由 `fold_ancestor` 从祖先现在的页面盒子（`placed_layout`，组件裁剪是本地几何加 `paint_origin`）现算，节点自己的加上 `origin`（按图元、revision、两个 epoch 缓存在 `placed_clips`）。冷场景走同一个公式，所以平移后的子树画出来和冷场景逐位相同：几何是同一份本地值，原点是同一个链和，变换是同一个页面变换。保留投影（`Retained`，自建图元以来没有任何会改原点的变化）直接用建图元时记下的原点（`RetainedPrimitive::origin`，调试构建断言与现算相同）；重定基（`Rebased`）的原点和继承裁剪一起记在 `DrawAttributes`。
- **平移的根**：只换本地盒子、参照系和结果，抬 `attribute_epoch` 与 `placement_epoch`，图元不动；尺寸或结果变了才重建。整节点重新提取、但只挪了位置且结果相同的（`moved_in_place`）同样不重建。
- **读了自己位置的节点**：分两种。
  - `offset_bound`：只读了自己在父节点坐标里的偏移。对齐父节点文字的小图标在参照系是父节点、两边记录都是本地时，用父节点的本地文字框减自己的 `local_layout.y`，和祖先站在哪无关；祖先平移时沿用，只有它自己是平移的根（偏移变了）时重建，查的是本次的平移根，O(平移根数)。
  - `origin_bound`：读了页面位置，它自己或上面的盒子平移时整节点重建，O(集合大小 × 深度)。剩下的都保持重建，因为改成本地会改变数值末位或语义：
    - 画家（`Painter`）的局部变换绕记录在页面上的角点（`about_origin`，`e + ox - (a·ox + c·oy)`），平移量是页面坐标的函数；改成绕本地角点再放到原点在末位不同。只有非恒等的局部变换才进集合。
    - 图标在参照系不是父节点或宿主记录在页面上时，按两边的页面原点求差。
    - 图表悬停指示线对齐页面半像素：吸附是页面位置的函数，只在指针悬停时存在。
    - 逐字形呈现的行内对象：`present_inline_objects` 把绕页面中心的缩放写进图元的变换，只在呈现时更新，平移后不重算就会停在旧位置。
    - 带转场视口的文字：视口是页面上的裁剪与文字框求交，裁剪不一定跟着平移。
- **可见性**：平移的根（含整节点重新提取、位置变了的）在剔除索引里整棵子树平移（`translate_subtree`，与滚动同一条路，嵌套的根只补自己多出来的那一段），根自己的图元重算；参照系不是父节点或粘性定位的（`unfollowing`：逃逸的绝对定位、`fixed`、菜单项、`sticky`）在平移根下面时重算自己和子树。所有平移先做、再做任何重算，重算出来的边界不会被再平移一次。`inherited_transform_changed` 不再把“位置变了”算作变换变了，只改尺寸或位置的祖先不再重算整棵子树的边界。
- **公开语义**：`ScenePrimitive` 的几何在节点自己的坐标里（读页面的 visual 除外）；页面上的位置用 `UiScene::primitive_layout_bounds` / `place_rect`，或 `SceneDraw::placed_bounds` / `place`；`SceneDraw::geometry_transform` 是“先放到原点再变换”的合成矩阵，给不吸附网格的读者。`node_bounds` 与 `node_layout_bounds` 相同，都是现在的页面盒子。

### 3.6 painter：几何先放到原点，再变换、再吸附

- painter 对每个图元先把几何加上 `SceneDraw::origin`（`placed_bounds`、`place`、笔画点进复用的暂存区、`Path` / `Chart` / 图层蒙版的 `origin`），之后的路径与改造前对页面坐标图元完全一样：`paint_transform` 只把 `transform` 的纯平移吸附到整设备像素（#223 的滚动语义不变），文字的整像素与亚像素相位从放好的页面位置取。布局的平移因此不是一个要吸附的平移，挪了小数像素的标签按新位置重新求相位（正确性要求），挪了整数像素的只换运行的 `origin`、实例照旧沿用。
- 不需要每节点变换缓冲：painter 每帧只遍历 `visible_operations` 裁剪后的图元，每个图元多一次加法和一次原点读取，代价和视口里的内容成正比。`nana-ui` 的 `rows_a_reflow_moves_by_a_fraction_paint_what_a_cold_scene_does` 在 1、1.1、1.2、1.75 倍下把行一帧挪 0.37 px，逐像素对比冷场景（变异实验：只让重定基的原点取整，第一帧就有 1 823 个像素不同）。

### 3.7 无障碍

`AccessibilityNode.bounds` 已经是相对父节点的（47aa93fb2）。写回时本地偏移变了的盒子直接标 ACCESSIBILITY，随父节点平移的后代不写回也不标。第 3 阶段起无障碍只看平移的根（`SystemWork::layout_moved`），收尾一步起连根下面也不走：

- **钉住的盒子**（`NodeRecord::pins_accessibility`）：参照系不是父节点（`fixed`、逃逸的绝对定位、触发式菜单的菜单项）、裁剪溢出（视口坐标的 `clip` 从它而来）、交给子节点的不是恒等（滚动偏移、变换、3D 上下文、`absolute` / `fixed` / `sticky`，即 `hands_accessibility_identity` 不成立），或带 Select、TreeView、MenuSurface、ModalFrame 这类从几何派生虚拟节点或改写自身盒子的 visual。其余盒子随父节点平移时相对偏移逐位不变，它下面没钉住的盒子也一样。`NodeStore` 记着这个集合（`accessibility_pins`，超集）：决定它的字段一写就重算该节点——样式解析（`note_layout_anchor_change`）、滚动偏移、写回或换参照系时参照系变了、visual 设置、插入；节点移除时删掉；读的时候再按现状核一次。
- **从钉子找**（`reach_moved_pins`）：根到页面一路交恒等、根不是模态框、没有动画叠加和裁剪 visual（`moved_root_hands_identity`）时，对每个钉子往上找最近的平移根，路上碰到会改变子节点投影的盒子（参照系不是父节点、裁剪、不交恒等、模态框）就交给那个盒子；找到的钉子从它自己起按原来的规则走（`reach_moved_subtree`：没跟着动的进 `affected`，裁剪的整棵子树重投影，不交恒等的盒子下面逐个 `keeps_relative_bounds`），交恒等的盒子下面不再往下。代价 O(钉子数 × 深度)，和被平移的节点数无关。
- 根本身不交恒等（祖先有滚动或变换、根是 `absolute`、有动画叠加或裁剪 visual）时仍从根往下走整棵子树：滚动或变换下视口盒是 `变换 · 布局盒`，平移后父子相减在末位不一定相等，冷对照要求逐位一致，所以这些子树里的盒子逐个精确判定。
- **相对偏移的公式**（`layout_offset_in_parent`）：盒子的参照系是父节点、两边投影出来的都是布局盒时（`axis_aligned`，或不在对齐路径上但投影值恰好等于布局盒），相对 bounds 直接取本地布局偏移 `local_layout.x/y`，冷投影和保留路径同一个公式。原来用「子节点页面盒 − 父节点页面原点」：页面盒是链和，差值只在末位之内等于本地偏移，按不整齐的量（如每帧 1.37 px）平移祖先后末位会变，而随父节点平移的后代不重投影，保留值（`y = 2.5`）和冷投影（`2.5000038`）就不一致。其余情况（参照系不是父节点、滚动、变换、裁剪、虚拟节点、`Document`）照旧相减，这些盒子本来就逐个精确判定或重投影。`keeps_relative_bounds` 的「逐位不变」由此成立。
- **投影本身**：平移的根自己的相对偏移确实变了，要重投影，代价落在它的子节点筛选（每个子节点问一次隐藏、视口盒）。`axis_aligned` 原来有任何 visual、z-index、浮层宿主就整体退回变换链现算；现在只看链上每个盒子（滚动、变换、`fixed` / `sticky`、3D、父节点是打开的触发式菜单），全局只留动画叠加和裁剪 visual（每个活着的 ModalFrame 都是裁剪 visual，所以不会把 surface 当成布局盒）；被问的盒子本身不进记忆表，只记它上面的链，隐藏判定同样。测试与 `layout-verify` 构建里 `axis_aligned` 每答一次「对齐」都现算一遍视口盒断言等于布局盒。
- 回归：`accessibility_relative_tests.rs` 的 `boxes_moved_by_an_uneven_step_keep_a_cold_projection_to_the_bit` 每帧把首行加高 1.37 px 共 12 次（有、无 visual 两种页面），逐位比对冷投影并断言叶子拿到的就是本地偏移；`moved_extraction.rs` 的平移步长也改回 1.37。变异实验：恢复相减公式，两者都报 `y = 2.5` 对 `2.5000038`。
- 校验：测试与 `layout-verify` 下 4 096 个节点以内的世界，`check_moved_pins` 每次把同一批根按旧规则整棵走一遍，断言两边找出的节点集合与子树集合相同，并逐个节点核对钉住的盒子都记在集合里；`check_kept_accessibility` 照旧对没投影的节点逐个比对冷投影。只改了尺寸的根（不在 `layout_moved` 里）只在自身带变换时重投影子节点。

## 4. 验证

- **冷对照**：`assert_matches_cold`，以及各层的冷重建对照（命中 `check_moved_hit_entries`、提取逐字段比对、Scene 冷建、无障碍冷投影）全部保留，并改为在相对数据上比对；绝对值另外经推导后再比一次。
- **端到端**：`crates/nana-ui-scene/tests/moved_extraction.rs` 加入组件（按钮、输入框）、滚动容器、`fixed`、逃逸绝对定位、粘性、变换祖先等场景。
- **基准**：改行高、改宽度、改控件文字三组交替跑 A/B，读最小值，再加 `RUNS=240 /usr/bin/time -l` 的 instructions retired。
- **快照套件**：预期零像素变化。第 4 阶段改动后与 3e9c83b80 的输出逐字节相同（`gallery-sidebar-collapsed-dark` 为已知抖动）。
- **平移的冷对照**（收尾一步）：
  - 运行时：测试与 `layout-verify` 下 1 024 个节点以内的世界，`extract_nodes_reusing` 每次结束时找出本次批里盒子相对渲染器所持记录变了的节点，对它下面所有这次没重新提取、渲染器仍持有记录的节点逐个冷提取，除提取时的页面盒子外逐字段比对（`check_records_kept_under_moves`）。
  - Scene：调试构建下 512 个节点以内的场景，每个带平移的 delta 之后克隆一份、全部重建图元，对平移根及其下面的每个图元比对放置原点、纯平移变换、裁剪矩形（逐位）和图元本身（`audit_moved_primitives`）。
  - 端到端：`moved_extraction.rs` 的 `moved_controls_that_read_little_of_the_page_keep_what_they_built`：关着的 Select、Popover、对齐父节点文字的图标、带 Mermaid 图的 Markdown（`rich-text`）随行平移，断言不重建图元、每帧与冷场景逐位一致；再让图标在行内自己平移，最后打开 Select 和 Popover 再平移。
  - 变异实验：去掉 `settle_layout_moves` 对 `page_geometry` 的重新标记，运行时冷对照报打开的 Select 的 `component_geometry` 与冷提取不同；去掉 `offset_bound` 的重建，Scene 冷对照报图标图元的 `y` 与重建不同；把打开的 Select 当本地推导，`local_component_geometry` 的断言报它读了页面。
- **painter 冷对照**：`rows_a_reflow_moves_by_a_fraction_paint_what_a_cold_scene_does`（见 3.6）；`moved_extraction.rs` 逐位比对 `SceneDraw` 的原点、纯平移变换、裁剪矩形和剔除结果。

## 5. 分阶段

| 阶段 | 内容 | 依赖 |
| --- | --- | --- |
| 1 | 布局权威改相对；结果、组件几何改本地；对外 API 靠推导给绝对值；`ContentBoundsIndex`、粘性定位改读本地值 | 无 |
| 2 | 命中索引用本地条目，平移只改根原点 | 1 |
| 3 | 提取和 Scene 用本地图元、根平移、投影重定基 | 1 |
| 4 | 图元与组件几何改本地，painter 先放到原点再吸附；平移子树不重建图元 | 3 |
| 5 | 端到端基准，更新 `docs/reference/resource-budgets.md` | 2、3 |

第 2、3 阶段在第 1 阶段落地之后并行推进。

第 1 阶段已落地：42 001 盒改行高，布局 16.4 → 3.9 ms，整帧 29.9 → 17.8 ms，每次编辑指令 296 M → 166 M（数字与做法见 `resource-budgets.md`）。

第 2 阶段已落地：命中 3.6 → 0.66 ms，整帧 17.3 → 13.8 ms，每次编辑指令 168 M → 140 M；改宽度、改控件文字持平（做法见 3.4，数字见 `resource-budgets.md`）。

第 3 阶段已落地：提取 3.74 → 0.08 ms（其中 Scene 的 delta 约 1.95 → 0.05 ms），无障碍 2.86 → 0.80 ms，布局 2.06 → 0.47 ms（`spread_layout_moves` 删除），命中 0.65 → 0.24 ms，整帧 11.7 → 1.7 ms，每次编辑指令 127 M → 32 M；改宽度、改控件文字持平（做法见 3.5、3.7，数字见 `resource-budgets.md`）。

第 4 阶段已落地：图元、节点自己的裁剪和组件几何改为本地坐标，painter 先把几何放到节点原点再变换（3.5、3.6）；`place_painted_records` 与 `component_geometry.is_none()` 守卫删除，平移的子树一个图元也不重建，剔除索引整棵平移。控件页（`CONTROLS=1 edits 2000 height`，62 001 节点）改行高的提取段 59.7 → 0.4 ms、整帧 67 → 4 ms，无头运行提取段 41 → 0.3 ms；简单页提取段 79 → 60 µs；`nana-text-paint-benchmark --workload reflow` 的 flush+batch 合计下降（数字见 `resource-budgets.md`）。快照与 3e9c83b80 逐像素相同。

收尾·无障碍已落地：平移的根下面只处理钉住的盒子，`axis_aligned` 不再因为有 visual 就整体退回变换链现算（3.7）。与 6838427fb 交替 8 轮读最小值：控件页改行高无障碍 3.17 → 0.64 ms、整帧 5.8 → 2.6 ms，简单页无障碍 1.11 → 0.36 ms，每次编辑指令控件页 55 M → 26 M、简单页 31 M → 23 M；改宽度、改控件文字持平（数字见 `resource-budgets.md`）。

收尾·读页面的节点已落地：只有打开的 Select / MenuSurface 仍按页面推导，NativeMarkdown、关着的 Select 与 MenuSurface 改为本地；对齐父节点文字的图标只在自己是平移根时重建（`offset_bound`）；`settle_layout_moves` 只扫 `page_geometry`（3.5）。与 7fa3b81b4 交替 8 轮、每轮交换先后读最小值：每个奇数格换成关着的 Select、每行第一格换成一段 Markdown 的控件页（`CONTROLS=1 MIXED=1`；Markdown 那一格用加了 `rich-text` 特性的临时副本）改行高，提取 24.8 → 0.18 ms、布局 2.9 → 1.1 ms、整帧 34.2 → 5.8 ms；普通控件页布局 0.65 → 0.53 ms、整帧 1.52 → 1.39 ms；简单页、改控件文字持平（数字见 `resource-budgets.md`）。

**目标**：改行高时整帧和 Taffy 布局在同一数量级，个位数毫秒；改宽度和改控件文字不回退。
