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

**重构已完成**（2026-11-04，见第 5 节末）：同一次改行高，简单页整帧 30.4 → 1.15 ms，62 001 节点的控件页 86.0 → 1.23 ms，每次编辑指令 326 M → 23 M、834 M → 25 M。

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
  - `ContentBoundsIndex` 每条按自己节点的原点存，父节点读子节点时加上子节点的本地偏移；携带给包含块的范围按落地的包含块的坐标存，所以平移的子树里条目全部不动。落地点可能变的两种情况（`position` 在是否建立包含块之间变化、子树换父节点）会让子树里携带范围的条目重读子节点（`reland`）。父节点刷新时只读移动了的子节点：叶子原地写入，上面的节点按层合并一次（`MaxTree::rederive`），一段连续 k 行的平移约 k + log n 次合并；只换了自己盒子的条目一次查表就地结算（收尾·内容范围）。
  - 粘性定位从本地盒子和本地结果算，所有盒子都相对节点自己的原点，`StickyAncestor.result` 约定为祖先自己坐标里的结果，按 `bounds` 的原点放置。
  - 提取：`ExtractedNode.layout_result` / `ExtractedMove.layout_result` 改为本地结果（共享同一个 `Arc`，不复制）；`ExtractedNode.layout` 仍是提取时推导出的绝对盒子，第 3 阶段另加 `local_layout` / `layout_frame`（见 3.5）。Scene 读结果的地方只有裁剪链、参与方式、滚动指标（与坐标无关）和粘性定位（已按上面的约定改）。
  - 无障碍见 3.7。
- **第 1 阶段的过渡**（已在第 3 阶段删除）：提交时 `spread_layout_moves` 从原点变了的盒子往下走，把随之平移的后代标成「只是移动」。现在提交只标写回的盒子本身（`settle_layout_moves`）：只挪了位置的根可以沿用提取记录，原点变了的根另记 `DirtyMask::MOVED`，进 `SystemWork::layout_moved`；后代一个不标。

### 3.4 命中索引（第 2 阶段，已落地）

- 每个条目都在**自己边框盒的坐标**里：`HitEntry.layout` 是 `(0, 0, 宽, 高)`，裁剪、路径裁剪、菜单、变换的轴心都按这个坐标存。条目另存 `origin`：盒子离命中父节点原点的偏移，经父节点交给子节点的变换的线性部分换算（单位线性时就是偏移本身）。查询时 `open_hit` 先减 `origin` 进入条目自己的空间，再减滚动 `shift` 进入子节点的空间；存进父节点边界树的包围盒是「自身 ∪ 子节点（加 `shift`）」再加 `origin`（`entry_bounds`，冷建和增量刷新同一个公式）。
- 偏移的来源：参照系是父节点且命中父节点就是树上的父节点时，直接用 `local_layout_box`（逐位精确）；`Page` 参照系的命中根（文档根、`fixed`）也直接用本地值；其余（包含块在别处的绝对定位、逃逸条目挂在裁剪父节点上、`fixed` 推断不出 `Page` 的情况）用 `layout_box_from` / `layout_origin` 求差。
- 父节点的变换带透视时，平移不能在外层做：偏移并进条目自己的变换，`origin` 为零，于是子孙依赖它的位置——这种条目不是普通条目，写回时走整棵子树修补。
- 平移子树时只重建根条目（`rebuilt_hit_entry_in_place`，原地替换，`origin` 变了计入 `hit_test_entries_moved`），再按深度刷新祖先包围盒；后代条目一个不碰。根条目不原地改 `origin`：同一次提交里可能有兄弟增删，`order` 要从重建里拿。
- **滚动容器里也原地重建**（收尾·滚动容器）：原来只有「普通」条目（恒等变换、无裁剪、上面没有滚动 `shift`）能原地重建，滚动容器下的条目变换里带着滚动，滚动容器自己带裁剪和 `shift`，于是一次改行高把滚动容器整棵子树重建（4.2 万个条目）。现在条件改成「交给子节点的东西没变」：新条目按 `patch_hit_branch` 的做法单独建——种子是父条目现在的变换（存的变换加上面的 `shift`）再减父节点现在的滚动，建好后减去 `child_shift(parent)` 回到索引存它的空间（与 `insert_tree` 同一个公式）——然后要求变换的线性部分是恒等（把自己的布局坐标交给子节点的条目除外，见下一条）、变换和透视（为零）与旧条目逐位相同，`HitEntry.scroll` 相同，子节点列表是同一个 `Arc`，z-index 不变，不是 `fixed`、不逃逸（`clip_escape` 为空：从种子建不会去找裁剪父节点），自身没有 3D 变换或 3D 上下文（种子不带 `parent_blocks_3d`）。`IndexedHit.shift` 原样保留，所以子节点仍按建它们时的滚动加 `shift` 查询。裁剪、菜单、painter 都只属于条目自己，可以随便变。
- **转过的条目把自己的布局坐标交给子节点**（收尾·命中变换，`HitEntry.local_children`）：原来每个条目的 `transform` 是从页面（扣掉一路的 `origin`）一直乘下来的，转过的祖先下平移一行时，行的条目不是线性恒等，不能原地重建；更要紧的是，转过的盒子自己被平移的行撑高时，变换原点（默认中心）跟着动，它下面每个条目的 `transform` 平移部分都变了，于是整棵修补（`ROTATED=1` 改一次行高重建 4.2 万个条目）。现在自身的 2D 变换有缩放、旋转、斜切或翻转（线性部分不是恒等）、一路没有透视、不在 3D 上下文里的盒子，交给子节点的不是「自己的变换 · 减滚动」，而是只有「减滚动」（`scroll_seed`）：子节点的条目在它的本地布局坐标里建，里面不含它的变换，也不含它上面的任何东西。
  - 查询：`open_hit` 进入这种条目后，先用它的 `transform` 的逆把点换进它的布局坐标，再减 `shift` 进入子节点的空间（`transformed_point`，奇异时子节点不可达）。父节点存的包围盒是「子节点的包围盒加 `shift`，四角过 `transform` 的外接框」与自身合并（`entry_bounds` → `turned_bounds`），冷建和每次刷新同一个公式，逐位一致。
  - 于是转过的子树里一切都是正立的：平移的行线性恒等、变换逐位相同，照普通条目原地重建；转过的盒子自己改尺寸、平移或改变换，只重建它自己的条目（`rebuilt_hit_entry_in_place` 对这种条目不要求变换相同，只要求仍是这种条目、滚动和子节点列表不变），子孙一个不碰。变成或不再是这种条目（例如变换改成纯平移）走整棵修补。
  - 跟着改的读取：`inherited_shift` 往上累加滚动 `shift` 到最近的这种条目为止（含它自己的，它上面的不进子孙的变换）；`child_shift` 同理；补丁与原地重建的种子统一走 `live_child_seed`（这种父节点给 `scroll_seed`，其余仍是父节点的变换加上面的 `shift` 再减滚动）；`insert_tree` 往下减 `child_shift` 时到这种条目为止。`shift_hit_test_scroll` 在这种条目上按恒等线性部分换算滚动量。从窗口坐标进出某个条目的地方（`pointer_layout_position`、`layout_pointer_position`、`menu_region_contains`、逃逸条目的 `escaped_hit_at`）改用 `entry_space`：偏移仍是一路 `origin` 与 `shift` 的和（只到最近的这种条目），再加上那个条目的布局坐标到窗口的仿射（从外往里逐个乘，不递归）；链上没有这种条目时与原来逐位相同。
  - 透视和 3D 下不这样做：透视不能并进仿射，3D 上下文要把变换往下传，这两种仍按原来的方式乘下去、整棵修补。普通页一个这种条目也没有，所有路径与原来逐位相同。
  - 合成层叠加时的查询（`visit_presentation_hits`）本来就按页面坐标现算变换，不读条目的 `transform`，不受影响。
- **改变换只标盒子自己**（收尾·改变换）：样式里的 `transform`、`transform_origin` 等变了，原来整棵子树标 TRANSFORM | INPUT，命中对子树里每个条目逐个原地重建；现在只标这个盒子。它是交出本地坐标的条目（前后都是）时，`rebuilt_hit_entry_in_place` 只重建它自己，脏节点下面的钉住条目（`sticky`、跨参照系的绝对定位、#201 逃逸的桥接变换）照旧重建；其余情况（纯平移、变成或不再是这种条目、透视、3D）原地重建不成立，走 `patch_hit_subtree` 整棵修补它的分支，和原来标整棵的结果一样。冷对照 `check_moved_hit_entries` 照旧对被原地替换的条目及其下面所有条目比对全量重建。
- `HitEntry.scroll`：索引让这个条目的子节点处在的滚动偏移，建时取现在的值，每次 `shift` 跟着改。驱动在同一帧既有滚动又有别的脏节点时丢掉滚动增量、交给补丁处理；这时滚动容器自己的 `scroll` 与现在的值不同，不能原地重建（否则保留的 `shift` 少了这一次滚动），走子树补丁。
- **钉住的条目**（`HitEntry.pinned`）：读了自身偏移以外的几何——跨参照系求差的偏移、#201 逃逸的桥接变换、组件几何（选择框菜单、悬停卡连接线、模态框的 surface/body 裁剪，都从页面坐标减去本节点原点）、粘性定位。祖先一动它们就可能变，所以 `HitIndex.pinned` 单独记着；打补丁时，脏节点下面的钉住条目即使没被标脏也重建（`pinned_hit_entries_under`，O(钉住数 × 深度)）。
- 合成层叠加时的查询（`visit_presentation_hits`）仍按页面坐标现算变换，读条目里的本地矩形前先乘上「本地 → 页面」的平移；`pointer_layout_position`、`layout_pointer_position`、`menu_region_contains` 同样在本地与页面坐标之间换一次。附带修正：`clip-path` 的命中以前拿页面坐标去比盒子本地的裁剪形状，盒子不在原点时错位，现在两边都在本地坐标里。
- 冷对照：`check_moved_hit_entries` 在所有补丁打完后，对被替换的条目**及其下面所有条目**逐字段（含 `origin`、`pinned`、`scroll`、包围盒）比对全量重建；`reflow_oracle` 的命中几何加上 `entry_offset`。上面和自己都没有 `shift` 的条目逐位比；有 `shift` 的（原来直接跳过）把变换、裁剪的变换、包围盒加上上面的 `shift` 换到现在的空间再比，容差是舍入量级（`max(1e-4, 1e-6·|v|)`），其余字段仍逐位——滚动的 `shift` 路径本来就不与冷建逐位相同。收尾·命中变换起，`local_children` 也逐位比；自己和上面都没有 `shift`、下面却有的条目（例如转过的包装盒里装着滚过的列表），变换、裁剪逐位比，包围盒在同一舍入容差内比：它合并的子节点包围盒是在另一个滚动下建的再加 `shift`，与冷建差末位（原来这种条目不会被原地重建，所以没碰到）。
- 回归（收尾·命中变换，2026-11-05）：`hit_test/moved_turned_tests.rs` 的 `boxes_moved_under_a_turned_ancestor_build_only_their_own_entries`，旋转、斜切、翻转、缩放、两层旋转、缩放里套旋转、滚过的页面里的旋转、转过的滚动容器、`preserve-3d` 九种祖先，每种三轮：首个普通行和滚动列表的首行各按 1.37 px 加高，中间夹单独滚动、滚动与平移同帧、改最外层的旋转角（单独和与平移同帧）、页面滚动后改最外层的 z-index（整棵修补它的分支，上面有 `shift`）、往最外层里追加一个盒子，最后把最外层改成纯平移再改回来；页里有转过的 `sticky`（装在转过的包装盒里和直接装在页面滚动容器里各一个）、`fixed`、逃逸滚动容器的绝对定位。每帧与只做全量重建的孪生页比对 60 × 36 的指针网格（最上面的命中、全部候选、点在每个候选布局坐标里的位置及映射回窗口的位置，后两项容差 1e-3），并与不读条目变换和包围盒的合成层遍历（`visit_presentation_roots`）及 `layout_projection_transform` 对照：候选相同、位置在 2e-3 内。合成层遍历按另一个顺序乘变换，网格点落在盒子边上舍入以内时两边可能各归一侧，这种盒子不进这一项对照（孪生页逐个相同）；同一份对照放到 1e9322016 上照样在边上的点报不同（翻转页 `(305.53, 262.61)`），所以不是这次引入的。`preserve-3d` 以外，每帧重建的条目不超过行数加包装盒数再加 8。
  - 变异实验（各自让上面的测试变红）：不交出本地坐标（`local_children` 恒假）；`insert_tree` 在这种条目下面仍减 `child_shift`（要页面滚动后转过的 `sticky` 被补丁才看得出，为此加了那个 `sticky`）；`inherited_shift` 越过这种条目；滚动 `shift` 按转过的线性部分换算；包围盒不过变换（孪生页两边一样错，靠合成层对照）；查询不过逆变换；种子仍乘父节点的变换；`child_shift` 计入这种条目上面的 `shift`；变成或不再是这种条目也原地重建；`entry_space` 的点或回窗口的映射不过那个仿射（同样靠对照）。没变红的两项：非这种条目、线性部分不是恒等的也原地重建（只在透视或 3D 下出现，`preserve-3d` 那一页冷对照照样逐位相同，守卫保留为防御）；`escaped_hit_at` 不过仿射（它只决定要不要按绘制顺序重排候选，页里逃逸的盒子顺序本来就对，看不出来）。
- 第 3 阶段起后代不再标 INPUT，`layout_followers` 删除：平移子树下需要重建的条目只靠 `pinned_hit_entries_under` 找（参照系没跟着动的后代离命中父节点的偏移靠跨参照系求差得出，本来就是钉住的）。

### 3.5 提取与 Scene（第 3、4 阶段，已落地）

第 3 阶段只发平移的根，但根下面画东西的记录仍在同一次 delta 里换成现在的页面盒子并重建图元，带组件几何的节点仍完整重新提取：painter 把纯平移变换吸附到整设备像素（#223，合成滚动的语义），文字的亚像素相位只取图元自身的位置，靠变换平移、挪了小数像素的子树画出来和冷场景不一样（快照 `area-layout/maximized` 上文字整体偏了 0.28 px）；组件几何又是按页面坐标算的。第 4 阶段把两者都改成本地坐标，平移的子树一个图元也不重建：

- **记录**：`ExtractedNode` 带 `local_layout` 与 `layout_frame`（与 `UiWorld::local_layout_box` / `layout_frame` 相同）；`layout` 是提取那一刻的页面盒子，只作参考。`ExtractedMove` 同样带 `local_layout` / `layout_frame`，只给平移的根发。
- **组件几何本地化**：`ExtractedNode.component_geometry` 在节点自己的坐标里（边框盒在原点）。运行时用同一份 `derive_component_geometry` 求它：`UiWorld::local_component_geometry` 把 `geometry_frame` 设成这个节点，期间 `record_layout_origin` 沿参照系链求和时走到这个节点就停（它自己的原点记 0），所以内容盒、部件、子孙的盒子都是相对这个节点的、和节点站在哪无关；链上碰不到它（读了外面的盒子）或读了视口（`document_viewport_of`）就记 `geometry_read_page`，调试构建断言。对外的 `UiWorld::component_geometry` 不设 frame，仍是页面坐标，逐位和改造前相同，命中、IME、无障碍都读它。
- **读页面的 visual**（`StandardVisual::geometry_reads_page`）：只剩打开的 MenuSurface（菜单项放在页面上）。它仍在页面坐标里推导、记录在页面上，节点或它上面的盒子一动就重新提取：`NodeStore` 在 `set_visual` 时按节点空间分类（`NodeSpace::READS_PAGE`）把它记进 `page_bound`，`settle_layout_moves` 只扫这个集合（原来扫所有带 visual 的节点），`extract_nodes_reusing` 的守卫看记录的 visual。关着的 Select 只有标签、提示和把手，推导时不读视口；关着的 MenuSurface 够不到它的菜单项（`closed_menu_hides`），表面是空框——两者都按本地坐标推导，平移时沿用记录。打开、关上都改 visual，节点整个重新提取。NativeMarkdown（收尾一步起）也在本地坐标里：整篇文档从本地内容盒往下排，冷场景同样如此，所以平移后和冷场景逐位一致；与改造前按页面累加的结果在末位不同，Mermaid 曲线的抗锯齿让 `gallery-rich-text-dark` / `-light` 两张基线变了几个像素，已重录。其余带组件几何的节点（按钮、输入框等）平移时直接沿用记录。
- **打开的 Select**（收尾·打开的 Select）：也在本地坐标里推导。菜单读页面只有一处：`select_menu_room` 给出的空间（视口与裁剪链，页面坐标，存在 visual 的 `menu_room` 里）决定往下还是往上开、要不要截短、横向要不要左移。本地推导时把这个空间减去字段的页面原点（`local_select_menu_room`，没有空间时用窗口），字段在 `(0, 0, w, h)`；空间之后只经过 `select_menu_surface`（从 `select_menu_geometry` 拆出来的那一段）决定表面的盒子，其余（行、滚动条、提示、选中）都只由这个盒子和字段自己的状态决定。所以盒子逐位相同，整份记录就逐位相同：往下开且放得下时表面是 `(0, h + gap, w, 自然高度)`，往上开时是 `(0, −gap − 自然高度, …)`，与字段站在哪无关；被截短或横向被推回空间时随平移变。平移时：字段自己是平移根，`extract_nodes_reusing` 在沿用记录前比一次（`keeps_select_menu`：渲染器持有记录的 `menu.surface` 与 `local_select_menu_surface` 现算的逐位比较）；字段在平移根下面，`settle_layout_moves` 从 `page_bound` 里带 `NodeSpace::MENU_ROOM` 的节点找出这些字段，只标 RENDER 并记 `render_room_epoch`（不算「别的原因」），提取时同样比一次，相同就什么也不发、渲染器沿用记录，不同才整节点重新提取。跨过上/下翻转的阈值、被截短、空间本身变了（滚动、裁剪、窗口变了，`stale_select_menu_rooms` 改 visual）都会重新提取。对外的 `UiWorld::component_geometry` 照旧在页面上推导，命中、无障碍、指针读它，逐位不变；Scene 里画出来的是本地几何加原点，与页面几何在末位以内相同。
- **图元本地化**：`build_node_primitives` 以 `UiScene::record_box`（本地记录为 `(0, 0, 宽, 高)`，页面记录为提取时的页面盒子）为 `bounds` 建图元，所有坐标——`bounds`、批量矩形、笔画点、`Path` / `Chart` 的 `origin`、图层裁剪、节点自己的裁剪（`clips` 里继承部分之后的那些）——都在节点自己的坐标里；`transform` 是页面上的变换（祖先的变换、滚动、节点自己绕现在位置的变换），不含原点。
- **放置**：节点现在的页面原点按参照系链从上往下累加本地偏移（`placed_origin`，与运行时 `record_layout_origin` 同样的加法顺序，按 `placement_epoch` 记忆；第一次平移之前直接取 `layout`，`placement_live`）。新提取的记录这个值与 `layout` 逐位相等（调试构建断言）。`paint_origin` 是图元的落点：本地记录取 `placed_origin`，页面记录取 0。
- **绘制**：`draw_primitive` 给出 `SceneDraw { origin, transform, clips }`：几何先加 `origin`（每个坐标一次加法），再过 `transform`；`clips` 全在页面上——继承的那些由 `fold_ancestor` 从祖先现在的页面盒子（`placed_layout`，组件裁剪是本地几何加 `paint_origin`）现算，节点自己的加上 `origin`（按图元、revision、两个 epoch 缓存在 `placed_clips`）。冷场景走同一个公式，所以平移后的子树画出来和冷场景逐位相同：几何是同一份本地值，原点是同一个链和，变换是同一个页面变换。保留投影（`Retained`，自建图元以来没有任何会改原点的变化）直接用建图元时记下的原点（`RetainedPrimitive::origin`，调试构建断言与现算相同）；重定基（`Rebased`）的原点和继承裁剪一起记在 `DrawAttributes`。
- **平移的根**：只换本地盒子、参照系和结果，抬 `attribute_epoch` 与 `placement_epoch`，图元不动；尺寸或结果变了才重建。整节点重新提取、但只挪了位置且结果相同的（`moved_in_place`）同样不重建。
- **读了自己位置的节点**：分两种。
  - `offset_bound`：只读了自己在父节点坐标里的偏移。对齐父节点文字的小图标在参照系是父节点、两边记录都是本地时，用父节点的本地文字框减自己的 `local_layout.y`，和祖先站在哪无关；祖先平移时沿用，只有它自己是平移的根（偏移变了）时重建，查的是本次的平移根，O(平移根数)。
  - `origin_bound`：读了页面位置，它自己或上面的盒子平移时整节点重建，O(集合大小 × 深度)。剩下的都保持重建，因为改成本地会改变数值末位或语义：
    - 画家（`Painter`）的局部变换绕记录在页面上的角点（`about_origin`，`e + ox - (a·ox + c·oy)`），平移量是页面坐标的函数；改成绕本地角点再放到原点在末位不同。只有非恒等的局部变换才进集合。 收尾·最后一批逐项评估过，都保留：要改成本地就得让 painter 在加原点之前先过局部变换（`SceneDraw` 多一段、`geometry_transform` 的语义跟着变），合成矩阵的舍入顺序不同（`a·gx + c·gy + e` 再加原点，对 `a·(gx + ox) + c·(gy + oy) + e + ox − (a·ox + c·oy)`），转过的 painter 内容在末位乃至抗锯齿上与现在不同，违反「像素不变」；集合里只有非恒等局部变换的 painter 节点，代价是平移时重建它们自己。
    - 图标在参照系不是父节点或宿主记录在页面上时，按两边的页面原点求差。 保留：要与平移无关，得沿两边的参照系链求到共同祖先再相减，结果与现在的页面原点之差在末位不同，图标的 `y` 会变；这种图标（文字宿主下的绝对定位图标、宿主是打开的 MenuSurface）很少见，每次平移只重建它自己。
    - 图表悬停指示线对齐页面半像素：吸附是页面位置的函数，只在指针悬停时存在。 保留：吸附到页面半像素是要的行为（不吸附线会发虚），只在指针悬停时存在，每次平移重建一个图表节点。
    - 逐字形呈现的行内对象：`present_inline_objects` 把绕页面中心的缩放写进图元的变换，只在呈现时更新，平移后不重算就会停在旧位置。 保留：与 painter 同一个结构（变换在加原点之后），只在逐字形呈现（动画）进行时存在。
    - 带转场视口的文字：视口是页面上的裁剪与文字框求交，裁剪不一定跟着平移。 保留：只在入场、退场转场进行时存在，转场结束后视口清掉，节点回到普通的本地记录。
- **可见性**：平移的根（含整节点重新提取、位置变了的）在剔除索引里整棵子树平移（`translate_subtree`，与滚动同一条路，嵌套的根只补自己多出来的那一段），根自己的图元重算；参照系不是父节点或粘性定位的（`unfollowing`：逃逸的绝对定位、`fixed`、菜单项、`sticky`）在平移根下面时重算自己和子树。所有平移先做、再做任何重算，重算出来的边界不会被再平移一次。`inherited_transform_changed` 不再把“位置变了”算作变换变了，只改尺寸或位置的祖先不再重算整棵子树的边界。根的平移量是页面上的、在根上面那些变换之前的位移，剔除边界在变换之后：平移量先过根继承的变换的线性部分（根自己的变换绕它自己现在的位置，随它一起动，不参与）；继承的变换带透视时，边界不是平移一个量能得到的，整棵子树重算（`refresh_subtree`）。原来直接拿页面位移去平移，变换过的祖先下面（`a_moved_subtree_that_reads_the_page_paints_like_a_cold_scene` 的旋转列）边界偏了 0.3 px。
- **剔除用图元自己的不透明度组**：叶子边界在图元处于非边缘渐隐的不透明度组里时记为未知（保守包含）。原来按节点问 `opacity_groups(node)`，打开的 Popover 的淡入组落在触发器节点上，触发器自己的图元（按钮底、文字）也被当成在组里、边界未知，于是任何视口都报它；而 painter 用 `primitive_opacity_groups`，触发器不在菜单的淡入里。现在剔除问 `primitive_opacity_groups(primitive)`（节点上没有组时直接跳过，代价不变），与 painter 同一套规则，打开 Popover 后的剔除与冷场景相同（`moved_extraction.rs` 去掉了跳过）。模态框的遮罩同样按 painter 的规则只取父节点的组。
- **剔除索引的保留审计**：调试构建下原来只要索引被平移过（`translated`）就不再比对。现在平移过的索引比叶子：每片叶子的边界加上上面所有还欠着的平移，与新建的逐位（8 ulp 内）比，再查每个内部节点放好后仍罩住两个子节点。上面那处 0.3 px 就是这样查出来的。
- **改变换**（收尾·改变换）：本来就只重新提取盒子自己，子孙的图元按 `current · inverse(base)` 重定基。这次补上一处末位：祖先从转过改成纯平移时，`current · inverse(base) · base` 的线性部分是 `0.99999994` 而不是 1，painter 不再把它当纯平移吸附到整像素。现在 `DrawAttributes.rebased` 记下建图元时的投影和现在的投影，图元（及节点自己的裁剪）的变换就是建时投影的，直接画成现在的投影，不再经过逆矩阵；画家的局部变换等其他变换照旧乘 delta。剔除索引的保留审计（`bounds_match`）改为在盒子自己的量级上数 ulp：转过之后贴近 0 的边是几百像素的项相减得来的，按它自己的 ulp 比会把舍入当成漂移（基线上同样会报，`-0.77558` 对 `-0.775_583_27`）。
- **公开语义**：`ScenePrimitive` 的几何在节点自己的坐标里（读页面的 visual 除外）；页面上的位置用 `UiScene::primitive_layout_bounds` / `place_rect`，或 `SceneDraw::placed_bounds` / `place`；`SceneDraw::geometry_transform` 是“先放到原点再变换”的合成矩阵，给不吸附网格的读者。`node_bounds` 与 `node_layout_bounds` 相同，都是现在的页面盒子。

### 3.6 painter：几何先放到原点，再变换、再吸附

- painter 对每个图元先把几何加上 `SceneDraw::origin`（`placed_bounds`、`place`、笔画点进复用的暂存区、`Path` / `Chart` / 图层蒙版的 `origin`），之后的路径与改造前对页面坐标图元完全一样：`paint_transform` 只把 `transform` 的纯平移吸附到整设备像素（#223 的滚动语义不变），文字的整像素与亚像素相位从放好的页面位置取。布局的平移因此不是一个要吸附的平移，挪了小数像素的标签按新位置重新求相位（正确性要求），挪了整数像素的只换运行的 `origin`、实例照旧沿用。
- 不需要每节点变换缓冲：painter 每帧只遍历 `visible_operations` 裁剪后的图元，每个图元多一次加法和一次原点读取，代价和视口里的内容成正比。`nana-ui` 的 `rows_a_reflow_moves_by_a_fraction_paint_what_a_cold_scene_does` 在 1、1.1、1.2、1.75 倍下把行一帧挪 0.37 px，逐像素对比冷场景（变异实验：只让重定基的原点取整，第一帧就有 1 823 个像素不同）。

### 3.7 无障碍

`AccessibilityNode.bounds` 已经是相对父节点的（47aa93fb2）。写回时本地偏移变了的盒子直接标 ACCESSIBILITY，随父节点平移的后代不写回也不标。第 3 阶段起无障碍只看平移的根（`SystemWork::layout_moved`），收尾一步起连根下面也不走：

- **节点自带变换**（收尾·无障碍变换）：`AccessibilityNode` 多了 `transform: Option<PaintTransform>`，对应 AccessKit `Node` 的 `transform`。节点空间到父节点空间是 `translate(bounds.x, bounds.y) · transform`（`AccessibilityNode::to_parent`），节点自己的盒子在自己的空间里是 `(0, 0, width, height)`。每个节点有一个无障碍空间（`AccessibleSpace`）：
  - **按偏移放置**的盒子（下一条）没有自己的变换、或自己的变换只是平移（本地 2×3 为 `(1, 0, 0, 1, e, f)`）时，空间是父节点的空间平移 `bounds` 的偏移：`(a·(x − sx + e), d·(y − sy + f), a·w, d·h)`，`transform` 为 `None`，与原来逐位相同；
  - 自己的变换是正缩放（`(a', 0, 0, d', e, f)`，系数不是 1）时（收尾·视口框架），发出 `transform = diag(a·a', d·d')`，`bounds` 是 `(a·(x − sx + e), d·(y − sy + f), w, h)`；有旋转、斜切或翻转时发出 `transform = diag(a, d) · L`（`L` 是它在本地坐标里绕变换原点的 2×3，`(a, d)` 是父节点空间的缩放），`bounds` 是 `(a·(x − sx), d·(y − sy), w, h)`。两种都叫「自带框架」（`AccessibleSpace::framed`）：尺寸是它自己的布局盒，它的空间就是它的本地布局坐标（缩放归一），下面的盒子在里面按未乘系数的本地偏移放置；
  - 全都与页面位置无关，所以转过或缩放过的子树里平移只改根自己的偏移，子孙逐位不变，改缩放系数也只改它自己；
  - **从视口来的盒子自带框架**（收尾·视口框架，`frames_from_viewport`，`viewport_frame_in_parent`）：不按偏移放置的盒子（页面根、`fixed`、`sticky`、逃逸的绝对定位）在 2D 链上（没有 3D 上下文、模态框、动画叠加、裁剪 visual），父节点空间到视口的映射可逆（父节点按偏移放置或从视口来、空间已知；根的父节点是视口），而且自己的变换不只是平移、或整条链把它的盒子转离视口坐标轴、或父节点空间自带框架时，空间也是它的本地布局坐标：`to_parent = V_P⁻¹ · M · translate(origin)`，`M` 是整条变换链（`accessibility_ancestor_transform`，含 sticky 的平移、`fixed` 的悬挂、逃逸的桥接），`V_P` 是父节点空间到视口的映射（父节点自带框架时是 `accessible_space_to_viewport`，否则是平移到父节点的视口盒原点，与原来的相减同一个约定），平移部分进 `bounds` 的偏移，线性部分进 `transform`，尺寸是它自己的布局盒。平移照旧来自视口，线性部分交给子孙：下面按偏移放置的盒子与页面位置无关；
  - 其余不按偏移放置的盒子（3D、模态框的 surface、动画叠加或裁剪 visual 下的一切，以及父节点空间不可逆时）仍从视口外接框来：父节点空间没有框架时是「视口盒 − 父节点视口原点」（原来的结果），有框架时发出父节点空间到视口的逆的线性部分，`bounds` 原点是视口原点在父节点空间里的位置（`turned_space_inverse`），于是它的空间与视口对齐；
  - 3D 下的取舍：3D 上下文里（`transform_3d`、`perspective`、`preserve-3d`）只退回外接框，不发透视；平台能拿到的只有 2D 仿射，外接框是平台已经在用的近似。

- **按偏移放置**（`offset_placed`，`offset_bounds_in_parent`，`local_placement`）：盒子的参照系是父节点、自己没有 3D 上下文、不是 `fixed` / `sticky`（`NodeSpace::places_by_offset`；自己的 2D 变换不再排除），父节点的空间让它的盒子正立（`AccessibleSpace::unit` 有值：父节点自己按偏移放置，或从视口外接框来、而一路的变换只是正缩放），全局没有动画叠加和裁剪 visual 时，相对 bounds 直接由本地布局盒算出：`(a·(x − sx), d·(y − sy), a·w, d·h)`，`(sx, sy)` 是父节点的滚动偏移，`(a, d)` 是父节点空间的缩放（从最近一个发出 `transform` 的盒子往下，正缩放的样式相乘；没有这样的盒子时就是从页面起的累积缩放，与原来相同），自己的变换按上一条并进去或发出去。链上全是恒等时（`axis_aligned`）就是 `(x, y, w, h)`，与原来逐位相同。冷投影和保留路径用同一个公式，所以平移后逐位一致。`NodeSpace::hands_offsets` 现在只排除 3D：旋转、斜切、翻转的盒子下面同样按偏移放置。测试与 `layout-verify` 下 `check_offset_bounds` 每算一次就把父节点空间到视口的映射（`accessible_space_to_viewport`：父节点盒子的视口变换乘缩放的逆）、自己的偏移和 `transform` 连乘，核对自己盒子的四角落在旧算法「布局盒四角过整条变换链」的位置上（舍入误差内）。
- **钉住的盒子**（`NodeSpace::pins_accessibility`）：不按偏移放置（参照系不是父节点：`fixed`、逃逸的绝对定位、触发式菜单的菜单项；3D 上下文或 `sticky`）、裁剪溢出（视口坐标的 `clip` 从它而来）、交给子节点的不是偏移（3D），或带打开的 Select、TreeView、MenuSurface、ModalFrame 这类从几何派生虚拟节点或改写自身盒子的 visual。自己带 2D 变换（平移、缩放、旋转、斜切、翻转）的盒子和转过的祖先下面的盒子不再是钉子。滚动过的容器不再是钉子；关着的 Select 只投影字段本身（`project_select_menu` 只改 `expanded`），也不再是钉子（`NodeSpace::VIRTUAL`）。节点空间分类存在 `NodeRecord::space` 上，`NodeStore` 记着这个集合（`space_pins`，与分类逐个相同）：决定分类的字段一写就重算该节点（`note_node_space`）——样式解析（`note_layout_anchor_change`）、写回或换参照系时参照系变了、visual 设置（打开、关上 Select 都改 visual）、插入；节点移除时删掉。测试与 `layout-verify` 下每次提交和每次无障碍投影都核对分类与集合（`check_node_spaces`）。
- **从钉子找**（`reach_moved_pins`）：根的空间让它的盒子正立（`moved_root_hands_offsets`：`AccessibleSpace::unit` 有值，转过的根也算）、根不是模态框、没有动画叠加和裁剪 visual 时，对每个钉子往上找最近的平移根。路上碰到裁剪、不交偏移（3D）、模态框、参照系是页面的盒子就交给那个盒子（或者它根本没动）；从视口来的钉子不挡路：自带框架的（转过的祖先下的 `sticky`、从移动的行量起的绝对定位，收尾·视口框架起）空间正立，下面按偏移放置；空间不正立的（3D 上下文下面）往下走时不把下面当按偏移放置（`reach_moved_subtree` 的 `handed` 要它的空间正立），逐个判定；参照系是某个块的盒子（`absolute`）记下那个块，到根之前经过了它才算跟着根动，否则它没动、下面也没动。仍交偏移的钉子（缩放、`sticky`、从随根平移的块量起的绝对定位）不挡路：它下面的盒子按偏移相对它放置。找到的钉子从它自己起按原来的规则走（`reach_moved_subtree`：没跟着动的进 `affected`，裁剪的整棵子树重投影，不按偏移放置的逐个 `keeps_relative_bounds`），交偏移的盒子下面不再往下。代价 O(钉子数 × 深度)，和被平移的节点数无关。
- **原地不动的裁剪盒**：长列表的滚动容器在行变高时盒子不变，只是滚动范围变了，布局照样把它标成「只是布局动了它」。原来裁剪的种子一律整棵子树重投影（视口坐标的 `clip` 可能变了），现在 `SystemWork::layout_reshaped`（`DirtyMask::RESHAPED`：写回时新原点或新尺寸、或发布的结果在滚动范围、溢出、基线之外还有几何变化，`layout_result_reshapes`）里没有它时，它按原来的盒子裁剪，下面的 `clip` 不变，只投影它自己。
- 根的空间不正立（祖先有 3D，或根从视口外接框来而上面有旋转），根是模态框，有动画叠加或裁剪 visual 时仍从根往下走，盒子逐个精确判定。旋转、斜切、翻转的祖先下平移已经只投影根（收尾·无障碍变换）。
- **原地改尺寸的转过的盒子**：自己带变换的盒子原地变了尺寸，变换原点（默认中心）随尺寸动，它的空间像平移过一样动了，里面按偏移放置的盒子相对它不变。它不在 `layout_moved` 里，但根的空间正立时当作平移根一起交给 `reach_moved_pins`（`resized_roots`，与 `layout_moved` 合成 `walk_roots`）：下面只有钉子重投影（例如两层以下的 `fixed`，它相对转过的空间的逆变了）。其余原地改尺寸、空间不正立或在 3D 链上的盒子照旧重投影直接子节点（外接框原点随尺寸变）。
- **子节点筛选**：投影一个节点要问每个子节点有没有视口盒（`visible_accessibility_bounds` 是否为 `Some`）。收尾·视口框架起，父节点交给子节点的映射和页面原点每个父节点只取一次（`HandedMap`），按偏移放置、没有自己变换的子节点只读记录和两个角（`HandedMap::finite_under`），不再每个子节点经过 `has_accessibility_bounds_under` 拷一份 `AccessibleTransform`、重查上面的链和触发式菜单；父节点是文档时照旧走完整判定（文档的子节点页面原点就是偏移，`0 + x` 与 `x` 在 `-0` 上不同）。变换链本身在「只有布局样式和记录」的页面上（没有长度动画、浮层宿主、z-index、游离挂载、visual、动画叠加）对流内、相对定位、没有自己变换的盒子直接取父节点的映射（`plain_transform_step`），不再走页面盒子的参照系链、悬挂与逃逸、sticky、变换样式的查询，页面原点取父节点原点加本地偏移（测试与 `layout-verify` 下逐位比对页面盒子、断言本地变换是恒等、没有逃逸、sticky 平移为 0）；投影前按受影响节点数给记忆表留好容量，一千个移动的行填表时不再扩容重哈希。滚动下 `axis_aligned` 不成立，原来每个子节点都算一遍自己的变换和视口盒、插两次记忆表；现在按偏移放置的子节点直接用父节点已记下的给子节点的变换判断四角是否有限（`has_accessibility_bounds`，测试与 `layout-verify` 下与慢路径对照）。收尾·滚动容器起，父节点那条链的三个答案（上面有没有隐藏、是否对齐、是否交偏移）每个被投影的节点只问一次（`ChildChain`），不再每个子节点各查三次记忆表；子节点的页面盒子取父节点变换记忆里记下的原点加本地偏移（`AccessibleTransform.origin`，与 `record_layout_origin` 同一个加法），不再每个子节点沿参照系链求和。测试与 `layout-verify` 下每个子节点都按原来的逐个判定再判一次、页面盒子与链求和逐位比对。
- **投影本身**：平移的根自己的相对偏移确实变了，要重投影，代价落在它的子节点筛选（每个子节点问一次隐藏、视口盒）。`axis_aligned` 原来有任何 visual、z-index、浮层宿主就整体退回变换链现算；现在只看链上每个盒子（滚动、变换、`fixed` / `sticky`、3D、父节点是打开的触发式菜单），全局只留动画叠加和裁剪 visual（每个活着的 ModalFrame 都是裁剪 visual，所以不会把 surface 当成布局盒）；被问的盒子本身不进记忆表，只记它上面的链，隐藏判定同样。测试与 `layout-verify` 构建里 `axis_aligned` 每答一次「对齐」都现算一遍视口盒断言等于布局盒。
- 回归（滚动、变换、Select，2026-11-04）：`accessibility_relative_tests.rs` 新增
  - `boxes_moved_in_a_scrolled_list_keep_a_cold_projection_to_the_bit`：滚动 30.37 px 的列表里每帧把首行加高 1.37 px，两轮各 6 次，中间滚动和平移同一帧；有、无 visual 两种页面；断言行里的盒子不重投影、叶子拿到本地偏移、行拿到 `y − scroll` 的位模式；
  - `boxes_moved_under_a_transformed_ancestor_match_a_cold_projection`：缩放 1.5 × 1.25 的祖先下 12 次，断言叶子拿到缩放后的偏移；旋转 0.3 rad 的祖先下走旧路径，与全量投影一致；
  - `pins_under_boxes_that_hand_offsets_are_found_from_the_pins`：缩放的、`sticky` 的、从行量起的绝对定位盒子里各放一个裁剪盒和一个普通盒；
  - `a_closed_select_moves_with_its_row_and_an_open_one_projects_again`：滚动列表里关着 → 打开 → 关上，各平移 3 次，断言只有打开时 Select 重投影、只有打开时是钉子；
  - `a_clip_resized_in_place_projects_what_it_holds_again`：原地变高的裁剪盒，下面的盒子都重投影。
  - 变异实验（各自变红）：每个 Select 都钉住 → Select 测试；冷投影改回相减 → 滚动、缩放、Select 三个测试（冷对照报逐位不同）；钉子往上找时在参照系是块的盒子处停 → `check_moved_pins`；在缩放的盒子处停 → `check_moved_pins`；裁剪盒无视 `layout_reshaped` 一律当原地不动 → 原地变高的裁剪测试（冷对照）；裁剪盒一律整棵重投影（旧行为）→ 滚动列表与 Select 测试的「行里的盒子不重投影」；子节点筛选答反 → 两个滚动测试；去掉旋转下原地改尺寸的修复 → 旋转测试（冷对照）。滚动下根退回整棵走只影响代价（新公式下 `keeps_relative_bounds` 照样答「不变」），用基准量：`SCROLLED=1` 无障碍约 1.0 → 1.45 ms。
- 回归（转过的空间，2026-11-04）：`accessibility_relative_tests.rs` 新增
  - `boxes_moved_under_a_turned_ancestor_keep_a_cold_projection_to_the_bit`：旋转 0.3 rad、斜切 0.4、翻转、两层旋转、旋转里套缩放、缩放里套旋转的祖先下，首行每帧加高 1.37 px 共 12 次；断言移动的行只投影自己，格子和叶子拿到 `unit · 本地偏移` 的位模式、`transform` 为 `None`，最里层的转发出 `transform`、正缩放并进偏移；
  - `boxes_placed_from_the_viewport_under_a_turn_match_a_cold_projection`：旋转、斜切、翻转下的 `sticky` 和 `fixed`（各套一个普通盒）拿到逆变换，从钉子找到并重投影；
  - `a_turn_changed_while_rows_move_matches_a_cold_projection`：同一帧改祖先的旋转角并平移，再单独平移，五轮；
  - `a_select_under_a_turn_moves_with_its_row_and_an_open_one_projects_again`：旋转下关着 → 打开 → 关上各平移 3 次，打开时列表框在字段的转过空间里正立（`transform` 为 `None`），只有打开时重投影；
  - `a_turned_box_resized_in_place_moves_its_space`：自带旋转的行原地变高，里面的格子、叶子不重投影，两层以下的 `fixed` 重投影；
  - `placed_corners_are_the_turned_corners_of_a_cold_projection`：等价性。用 `AccessibilityPlacements` 把相对投影沿链累积，按偏移放置的每个盒子四角与旧算法的外接框来源（布局盒四角过 `accessibility_ancestor_transform`）在 2e-3 内相等，其余节点（含虚拟节点、逆变换的钉子）的外接框与 `project_accessibility_in_viewport` 在 2e-3 内相等；旋转、斜切、翻转和缩放-旋转-缩放-斜切四层嵌套各一页。
  - 变异实验见下面「收尾·无障碍变换」一段。
- **改变换**（收尾·改变换）：改变换的盒子交给子孙的空间没变时，只投影它自己和它下面从视口外接框来的盒子。`NodeSpace::keeps_space`（提交时用前后两份样式的分类判定）：前后都没有 3D 上下文，并且自身 2D 变换前后都自带框架（缩放、旋转、斜切、翻转：空间就是它的本地布局坐标，与转多少、缩放多少无关），或前后都只是平移（无变换、纯平移：空间是父节点的平移）。收尾·视口框架之前，缩放要前后同一对系数才算。成立时只标 `TRANSFORM | TRANSFORM_KEPT | INPUT | RENDER`，INPUT 不带 INPUT_ELSEWHERE（和布局移动一样，别处再标 INPUT 就退回整棵），列在 `SystemWork::transform_kept`；不成立时标在 `SystemWork::transform` 上，无障碍从它整棵重投影（与原来相同，只是不再逐个标子孙）。投影时，`transform_kept` 里的盒子还要满足：没被别的原因标 INPUT、本帧没被布局平移、自己不裁剪（视口坐标的 `clip` 从它来）、`moved_root_hands_offsets`（空间让它的盒子正立，因此在偏移放置的链上，没有动画叠加、裁剪 visual，不是模态框）；满足就投影它自己（另要 `style_frames_its_space`：样式自带框架的盒子空间确实自带框架；3D 上下文下面的缩放盒子父节点空间不可逆，空间是视口的缩放，改系数会改子孙），再从钉子找（`reach_turned_pins`）：每个钉子往上走，先碰到这种根就把钉子整棵重投影，先碰到另一个钉子就交给那个钉子。与平移不同，转动带着它下面的一切，参照系在根上面的绝对定位、逃逸到根上面裁剪的盒子照样被它的变换带着，所以不看参照系。不满足的退回整棵。
  - 页面根（参照系是 `Page`，从视口外接框来）自己转、`fixed` / `sticky` / 逃逸的绝对定位自己转，收尾·视口框架起都自带框架，与转过的普通盒子一样只投影它和它下面的钉子；改缩放系数同样只投影它（缩放自带框架，子孙是未乘系数的偏移）。
  - 附带修正平移路径：`reach_moved_subtree` 里没跟着根动的盒子（参照系在根上面）原来只投影它自己；它的空间不正立时（转过的空间里从视口来的绝对定位），它下面的盒子从视口外接框相减得来，链上的原点变了，相减在末位不同（宽 `4.7861786` 对 `4.786186`），现在连同子树重投影。基线上同样会错。
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
- **打开的 Select**（收尾·打开的 Select）：`moved_extraction.rs` 的 `an_open_select_moved_keeps_its_menu_until_it_flips`，字段分别是平移根（列的直接子节点）和平移根下面的节点（包在一个被列平移的盒子里），各三轮：首行按 1.37 px 加高 7 次把字段推过「下面放不下、翻到上面」的阈值，再按 −1.37 px 退回去翻回来，中间夹滚动（空间变了）和改字段所在行的宽度并同帧平移；滚动容器切出空间。每帧逐位与冷场景比对；没换边的平移断言一个图元也不重建，换边的那一帧断言重建了。加 `nana-ui-runtime/layout-verify` 时 `check_records_kept_under_moves`、`check_kept_accessibility`、`check_moved_pins` 一并在里面跑，Scene 的 `audit_moved_primitives` 在调试构建里跑。`select_popup_hit.rs` 改为把图元放到页面上（`place_rect`）再与页面几何比。变异实验（各自变红）：`keeps_select_menu` 一律答「保留」→ 冷场景对照报表面画在 `y = 158.85`、冷场景在 `18.85`，`layout-verify` 下运行时报 `component_geometry` 与冷提取不同；`settle_layout_moves` 不标平移根下面的字段 → 包起来的那一半冷场景对照变红，`check_records_kept_under_moves` 报「kept under … which moved」；打开的 Select 仍算读页面 → 没换边的平移重建了 11 个图元；本地空间不减字段原点 → 换边那一帧没有重建。
- **painter 冷对照**：`rows_a_reflow_moves_by_a_fraction_paint_what_a_cold_scene_does`（见 3.6）；`moved_extraction.rs` 逐位比对 `SceneDraw` 的原点、纯平移变换、裁剪矩形和剔除结果。

## 5. 分阶段

| 阶段 | 内容 | 依赖 |
| --- | --- | --- |
| 1 | 布局权威改相对；结果、组件几何改本地；对外 API 靠推导给绝对值；`ContentBoundsIndex`、粘性定位改读本地值 | 无 |
| 2 | 命中索引用本地条目，平移只改根原点 | 1 |
| 3 | 提取和 Scene 用本地图元、根平移、投影重定基 | 1 |
| 4 | 图元与组件几何改本地，painter 先放到原点再吸附；平移子树不重建图元 | 3 |
| 5 | 端到端基准，更新 `docs/reference/resource-budgets.md`（已完成） | 2、3 |

第 2、3 阶段在第 1 阶段落地之后并行推进。

第 1 阶段已落地：42 001 盒改行高，布局 16.4 → 3.9 ms，整帧 29.9 → 17.8 ms，每次编辑指令 296 M → 166 M（数字与做法见 `resource-budgets.md`）。

第 2 阶段已落地：命中 3.6 → 0.66 ms，整帧 17.3 → 13.8 ms，每次编辑指令 168 M → 140 M；改宽度、改控件文字持平（做法见 3.4，数字见 `resource-budgets.md`）。

第 3 阶段已落地：提取 3.74 → 0.08 ms（其中 Scene 的 delta 约 1.95 → 0.05 ms），无障碍 2.86 → 0.80 ms，布局 2.06 → 0.47 ms（`spread_layout_moves` 删除），命中 0.65 → 0.24 ms，整帧 11.7 → 1.7 ms，每次编辑指令 127 M → 32 M；改宽度、改控件文字持平（做法见 3.5、3.7，数字见 `resource-budgets.md`）。

第 4 阶段已落地：图元、节点自己的裁剪和组件几何改为本地坐标，painter 先把几何放到节点原点再变换（3.5、3.6）；`place_painted_records` 与 `component_geometry.is_none()` 守卫删除，平移的子树一个图元也不重建，剔除索引整棵平移。控件页（`CONTROLS=1 edits 2000 height`，62 001 节点）改行高的提取段 59.7 → 0.4 ms、整帧 67 → 4 ms，无头运行提取段 41 → 0.3 ms；简单页提取段 79 → 60 µs；`nana-text-paint-benchmark --workload reflow` 的 flush+batch 合计下降（数字见 `resource-budgets.md`）。快照与 3e9c83b80 逐像素相同。

收尾·无障碍已落地：平移的根下面只处理钉住的盒子，`axis_aligned` 不再因为有 visual 就整体退回变换链现算（3.7）。与 6838427fb 交替 8 轮读最小值：控件页改行高无障碍 3.17 → 0.64 ms、整帧 5.8 → 2.6 ms，简单页无障碍 1.11 → 0.36 ms，每次编辑指令控件页 55 M → 26 M、简单页 31 M → 23 M；改宽度、改控件文字持平（数字见 `resource-budgets.md`）。

收尾·读页面的节点已落地：只有打开的 Select / MenuSurface 仍按页面推导，NativeMarkdown、关着的 Select 与 MenuSurface 改为本地；对齐父节点文字的图标只在自己是平移根时重建（`offset_bound`）；`settle_layout_moves` 只扫 `page_geometry`（3.5）。与 7fa3b81b4 交替 8 轮、每轮交换先后读最小值：每个奇数格换成关着的 Select、每行第一格换成一段 Markdown 的控件页（`CONTROLS=1 MIXED=1`；Markdown 那一格用加了 `rich-text` 特性的临时副本）改行高，提取 24.8 → 0.18 ms、布局 2.9 → 1.1 ms、整帧 34.2 → 5.8 ms；普通控件页布局 0.65 → 0.53 ms、整帧 1.52 → 1.39 ms；简单页、改控件文字持平（数字见 `resource-budgets.md`）。

收尾·无障碍末位：随父节点平移的盒子相对 bounds 取本地布局偏移（`layout_offset_in_parent`），不整齐的步长（1.37 px）平移后与冷投影逐位一致（2f0479886，3.7）。

收尾·无障碍滚动与 Select：相对 bounds 在平移、滚动和正缩放链上都由本地偏移算出（`offset_bounds_in_parent`），滚动过的容器和缩放的祖先下平移只投影根；关着的 Select 不再是钉子；盒子没变的裁剪种子不再整棵重投影（`layout_reshaped`）。与 21f8058fa 交替 8 轮读最小值：`SCROLLED=1` 改行高无障碍 17.4 → 0.94 ms、控件页 `SCROLLED=1` 30.2 → 1.62 ms，混合页 2.41 → 0.34 ms；简单页、控件页、改控件文字持平（数字见 `resource-budgets.md`）。

收尾·无障碍变换：`AccessibilityNode` 自带 `transform`，节点到父节点是 `translate(bounds.x, bounds.y) · transform`；旋转、斜切、翻转的盒子发出自己的变换（乘上面的正缩放），子孙在它的本地布局坐标里按偏移放置，从视口外接框来的盒子在转过的空间里拿逆变换，3D 退回外接框（3.7）。转过的祖先下平移只投影根，自带 2D 变换的盒子不再是钉子；`AccessibilityPlacements` 从相对树累积视口变换。与 9f4fc87be 交替 8 轮读最小值：`ROTATED=1` 改行高无障碍 9.95 → 0.75 ms、整帧 27.1 → 16.8 ms，控件页 `ROTATED=1` 无障碍 17.7 → 1.29 ms；简单页、控件页、混合页、改控件文字持平，`SCROLLED=1` 无障碍多约 20 µs（数字见 `resource-budgets.md`）。变异实验（各自变红）：转过的盒子不交正立空间（旧规则：转过的祖先就是钉子）→ 六个新测试；交出的转不乘上面的缩放 → 等价性与「缩放里套旋转」；`turned` 不往下传 → 逆变换的钉子、等价性、Select 三个测试；转过空间里的钉子不给逆变换 → 逆变换测试与等价性；转过字段的虚拟节点改回视口相减 → Select 测试（列表框外接框差 37 px）；从视口来、空间不正立的钉子也当正立往下不走 → 逆变换测试（`check_kept_accessibility` 报绝对定位里的盒子与冷投影不同）；原地改尺寸的转过的盒子不当平移根 → 原地改尺寸测试（冷对照）；投影忽略自己的变换 → 九个测试；正立的自身变换不并入平移 → 五个测试；按偏移放置的盒子在转过空间里改回外接框相减 → 七个测试；`AccessibilityPlacements` 乘反顺序 → 等价性与 Select；AccessKit 桥接不把转的平移乘缩放比 → `a_turned_node_hands_its_turn_after_its_offset`。往上找钉子时在「从视口来、空间不正立的钉子」处停的条件去掉了：那个钉子自己会往下走，经过它只会把下面的钉子找两遍，变异实验证实结果不变。`keeps_relative_bounds` 与 `offset_bounds_in_parent` 在转过空间里不走「视口盒等于布局盒」分支的守卫是防御性的，去掉后测试不变红（转过的空间里视口盒不会等于布局盒）。

第 5 阶段已完成：ba8ba044f 对 2f0479886 交替 8 轮、每轮交换先后读最小值（负载 2.4–3.1）。改行高的整帧，简单页 30.4 → 1.15 ms、控件页 86.0 → 1.23 ms（`VISIBLE=1` 86.5 → 1.44 ms）；布局 17.6 → 0.47 ms、31.2 → 0.50 ms；每次编辑指令 326 M → 23 M、834 M → 25 M。改宽度 0.80 → 0.13 ms，改控件文字持平（完整表格见 `resource-budgets.md`）。

**目标**：改行高时整帧和 Taffy 布局在同一数量级，个位数毫秒；改宽度和改控件文字不回退。——已达到。

收尾·滚动容器：滚动容器里改行高，命中原来整棵重建滚动容器子树（`SCROLLED=1` 4.2 万、控件页 6.2 万个条目），现在平移的行只原地重建自己的条目（3.4）；打开 Popover 后剔除按图元自己的不透明度组判断，不再多报触发器，平移量过继承变换的线性部分，剔除索引平移后也做保留审计（3.5）；无障碍的子节点筛选每个父节点只问一次链（3.7）。与 40b898c12 交替 8 轮、每轮交换先后读最小值：`SCROLLED=1` 改行高整帧 18.8 → 1.56 ms（命中 16.0 → 0.26 ms），控件页 `SCROLLED=1` 35.5 → 1.77 ms（命中 31.8 → 0.29 ms）；简单页、控件页、混合页和改控件文字持平（数字见 `resource-budgets.md`）。

收尾·内容范围：滚动容器里改行高，`ContentBoundsIndex` 原来在一半以上子节点移动时把列表的最大值树整棵重填（读全部 2 000 行再 O(n) 重建），现在只读移动了的行、叶子原地写入、上面的节点按层合并一次（与重填得到同一棵树：每个节点照旧由两个子节点合并）；只换了自己盒子的条目一次查表就地结算，读子节点时参照系是父节点的直接取本地偏移（测试与 `layout-verify` 下断言与 `layout_box_from` 逐位相同）。同时无障碍子节点筛选把父节点交给子节点的变换提到循环外，`holds_line_box_in` 每个盒子只读一遍子节点。冷对照：测试与 `layout-verify` 下每次读内容范围都与从零建的索引逐位比对（4 096 个节点以内）。普通页没有滚动容器，索引一条也不建，没有这项重填。与 9600e796a 交替 8 轮、每轮交换先后读最小值：`SCROLLED=1` 改行高整帧 1.58 → 1.45 ms、控件页 `SCROLLED=1` 1.69 → 1.54 ms，每次编辑指令 28.5 M → 26.4 M、30.3 M → 27.9 M；简单页、控件页、混合页、改宽度、改控件文字持平（数字见 `resource-budgets.md`）。

收尾·命中变换：自身 2D 变换有缩放、旋转、斜切或翻转、一路没有透视和 3D 上下文的盒子把自己的布局坐标交给子节点（`HitEntry.local_children`），查询在它这里过一次逆变换，包围盒取子节点包围盒过它的变换的外接框；转过的子树里平移只原地重建根条目，转过的盒子自己改尺寸（变换原点随之动）、平移或改变换只重建它自己（3.4）。与 1e9322016 交替 8 轮、每轮交换先后读最小值（负载 2.0–2.7）：`ROTATED=1` 改行高整帧 16.7 → 1.33 ms（命中 14.7 → 0.24 ms），控件页 `CONTROLS=1 ROTATED=1` 32.5 → 1.44 ms（命中 29.8 → 0.25 ms）；简单页、控件页、混合页、`SCROLLED=1`、改宽度、改控件文字持平（数字见 `resource-budgets.md`）。转过的页和普通页的整帧差从约 15.6 ms 降到约 0.2 ms。

收尾·改变换：改一个盒子的 `transform` 样式不再把整棵子树标 TRANSFORM | INPUT，只标它自己。命中只重建交出本地坐标的条目自己（3.4）；无障碍在交给子孙的空间不变时只投影它和它下面的钉子（`SystemWork::transform_kept`，3.7）；Scene 本来就只重新提取它，补上纯平移的重定基逐位精确与剔除审计的量级（3.5）。基准 `edits 2000 transform` / `scale`（每次改行所在的列：`ROTATED=1` 是转过的列，转角 0.05 ↔ 0.06 rad；普通页是根列，0.01 ↔ 0.02 rad；`scale` 是缩放 1.01 ↔ 1.02）。与 54e3425e7 交替 8 轮、每轮交换先后读最小值（负载 2.3–2.9）：`ROTATED=1` 改转角整帧 37.6 ms → 25 µs（命中 17.6 ms → 0，无障碍 15.5 ms → 20 µs）；普通页根列改转角 39.2 → 16.2 ms、改缩放 38.0 → 15.0 ms，都只省下命中（约 17.5 ms），无障碍照旧整棵（根从视口外接框来、缩放系数变了，原因见 3.7）；改行高、改宽度、改控件文字持平（数字见 `resource-budgets.md`）。冷对照：`crates/nana-ui-scene/tests/turned_container.rs` 每帧与只做全量重建的孪生页比对 60 × 36 的命中网格、无障碍树（逐位）和场景（图元、原点逐位，变换 1e-3，剔除结果），加 `--features nana-ui-runtime/layout-verify,rich-text` 时 `check_kept_accessibility`、`check_moved_hit_entries`、`check_moved_pins` 一并在里面跑。变异实验（各自变红）：一律当空间不变（M1）、不比缩放系数（M2）、不从钉子找（M3）、不投影根自己（M4）、裁剪的根也当空间不变（M5）、重定基仍过逆矩阵（M6，`audit_moved_primitives` 报 `-9.756329` 对 `-9.756328`）、剔除审计只按自己的 ulp 比（M7）、平移路径不重投影空间不正立、没跟着动的盒子的子树（M8）、改回整棵标记（M9）与 `RuntimeDocument` 仍把这种根当子树种子（M10）——后两个由测试里「只投影根和视口来的盒子、命中只重建 7 个条目以内」的断言抓到。防御性、没变红的：投影时再查「没被别的原因标 INPUT」（触发它需要同一帧别处给根标 INPUT 而又不整棵标，测试里构造不出）；`keeps_accessible_space` 里的 3D 判定（3D 下 `moved_root_hands_offsets` 已经不成立）。

收尾·行盒记忆：「这个盒子有没有流内行盒」（`holds_line_box_in`）的记忆跨帧保留，原来每次作用域布局沿受影响的每个节点往上清到根，改 `min-height` 也会让滚动容器把列表的 2 000 行再问一遍。现在按种子的改动类别清（`may_move_line_boxes`）：答案只读盒子自己的文字、文字布局、行内记录与 visual、子节点列表，以及每个子节点是否省略盒子、是否脱离文档流（定位、浮动）、是否行内级、是否替换元素；样式写入的每个字段都有归类（`display` 与隔离归流、`position` 与 `float` 归定位、`hidden` 归可见性），文字、子节点列表、行内原子、替换内容各有自己的原因和字段。只改尺寸、间距、对齐、滚动、变换或指针输入，原因也不涉及这些的种子不清；其余照旧沿种子往上清到根。依赖边带进前沿、自己没变的节点不再清。结构性种子（拓扑、可见性、原因未知）可能借祖先派生的隐藏（挂载、浮层分支、菜单开合）让整棵子树的答案变，带着它的一遍照旧沿所有受影响节点往上清。守卫：测试与 `layout-verify` 下，每次读记忆都与新走一遍比对（原有，4 096 个节点以内），每遍布局后再把文档里留下的每条记忆与新走一遍比对（新增 `verify::retained_line_boxes_are_fresh`，一份新记忆供所有条目共用，O(n)，`skip_layout_verify` 下也跑）。新增 `world/line_box_memo.rs`：滚动容器里一列行，其中一行按首基线对齐，改尺寸、文字清空再填、`display`（行内级、`none`、块级）、`position` 与 `float`、插入与删除行、`align-items: baseline` 开关，每一步与冷布局逐项比对；只改尺寸时一个盒子都不重新结算（`LayoutExecutionStats::line_boxes_settled == 0`）。变异实验：所有非结构性种子都不清（漏清）→ 五个测试变红（`line_box_memo` 的文字、`display`、基线三个与 `issue382_b`、`issue382_c`，都是读后审计报「retained line-box answer is stale」）；改回沿全部受影响节点清 → 只改尺寸的测试变红（每次重新结算）；把流字段当成不影响 → 不变红（改 `display` 时参与方式变了，另有父节点的子节点列表种子，是拓扑种子）；去掉结构性种子的整片清理 → 全套 2 677 个测试不变红，这条是防御性的：测试里没有在菜单关着时读过菜单里盒子答案的情形。与 29f4c3e4d 交替两组各 8 轮、每轮交换先后读最小值（负载 1.8–2.1）：`SCROLLED=1` 改行高布局 523 → 482 µs、529 → 505 µs，整帧 1 372 → 1 353 µs、1 388 → 1 347 µs；简单页、控件页、混合页、`ROTATED=1`、改变换、改宽度、改控件文字持平（数字见 `resource-budgets.md`）。`SCROLLED=1` 和普通页的整帧差约 0.23 ms，多出的主要是无障碍（约 165 µs，滚动链下每个子节点的视口盒判定，见下）和内容范围。

收尾·视口框架与缩放：从视口来的盒子（页面根、`fixed`、`sticky`、逃逸的绝对定位）在 2D 链上被转过或缩放过时自带框架，`to_parent = V_P⁻¹ · M · translate(origin)`，平移照旧来自视口、线性部分交给子孙；带正缩放的盒子发出 `transform = diag(a, d)`，子孙用未乘系数的本地偏移，纯平移（系数 1）的盒子照旧 `None`；`keeps_accessible_space` 改为「前后都自带框架，或前后都只是平移」，另由 `style_frames_its_space` 挡住 3D 上下文下面不自带框架的缩放（3.7）。滚动链下的子节点筛选每个父节点取一次映射（`HandedMap`），变换链在只有布局样式的页面上对普通盒子直接取父节点的映射（`plain_transform_step`），投影前按受影响节点数给记忆表留容量。冷对照：`check_kept_accessibility`、`check_moved_pins`、`check_offset_bounds` 照旧，新增 `check_viewport_frame`（测试与 `layout-verify` 下核对自带框架的视口盒子四角过父节点空间的映射落在布局盒四角过整条变换链的位置上）与 `plain_transform_step` 的逐位守卫；等价性测试（`assert_turned_corners`）把自带框架的视口盒子也按四角比对。`crates/nana-ui-scene/tests/viewport_frames.rs` 每帧与只做全量投影的孪生页逐位比对无障碍树并用 `AccessibilityPlacements` 核对外接框：根、`fixed`、`sticky`、逃逸的绝对定位连续改角度，两层缩放改系数（含 1e-3、1e-4 和单轴），按 1.37 的步长平移，中间夹滚动。与 d8eabffea 交替 8 轮、每轮交换先后读最小值（负载 2.3–3.1）：普通页根列改转角整帧 15.9 ms → 17.7 µs、改缩放 14.6 ms → 17.9 µs，`ROTATED=1` 改缩放 15.0 ms → 17.8 µs、改转角 24.4 → 17.8 µs；改行高无障碍简单页 268 → 235 µs、控件页 279 → 252 µs、混合页 279 → 251 µs、`SCROLLED=1` 439 → 323 µs、`ROTATED=1` 455 → 337 µs；改宽度、改控件文字持平（数字见 `resource-budgets.md`）。变异实验（各自变红）：从视口来的盒子一律不自带框架 → 四个测试（含孪生页与等价性）；放进父节点空间时不乘父节点映射的逆 → `check_viewport_frame` 报四角不在转过的位置（七个测试）；转动的根下面不找钉子 → `check_kept_accessibility`（四个测试）；自带框架的视口盒子跟着父节点平移时当成保留 → 「转后外接框不变的盒子」测试；外接框恰好等于布局盒时走对齐捷径丢掉框架 → 同一测试；`keeps_accessible_space` 一律保留 → `check_kept_accessibility`（3D 上下文下的列）与三个测试；去掉 `style_frames_its_space` → `check_kept_accessibility`（3D 上下文下改缩放）；`diag` 的 `d` 不乘系数 → `check_offset_bounds`（七个测试）；变换链快路径不减滚动 → `check_offset_bounds`；快路径放过 `sticky` → 快路径守卫（粘住的表头）；子节点有限性一律答「有」→ 与完整判定的对照（越过最大浮点数的盒子）。记忆表预留容量没有语义，用基准量：`SCROLLED=1` 无障碍不预留 391–399 µs，预留 330–337 µs，命中段多约 5–10 µs（命中代码没动，同一进程里分配器的状态不同，记为观察）。

收尾·子节点放置与提取记录：盒子自己的几何没变、只有子节点的放置变了的结果（固定高度的列里首行加高，固定宽度的行里首项变宽），从 f73da4b1b（子节点放置不再标脏父节点）起不再标任何脏位。它投影不出新东西，命中和无障碍确实不用动，但提取给它的记录带着结果，Scene 就一直留着旧的子节点放置；等到下一帧祖先改尺寸、平移它所在的子树，`check_records_kept_under_moves`（f47021db4 加的）才在这个保留的记录上报 `layout_result` 与冷提取不同，所以看起来像是相对几何系列引入的。现在 `publish_layout_results` 发布了新结果而 `layout_result_projects_new_geometry` 答「不投影」时只标 RENDER，命中、无障碍不变。冷对照：`crates/nana-ui-scene/tests/child_placement_records.rs` 每帧把 Scene 留着的每条记录与新提取的结果比对，`viewport_frames.rs` 恢复平移嵌套缩放盒里的行。变异实验：去掉这一处标记 → 两个新测试在加高的那一帧变红，`viewport_frames.rs` 在 `layout-verify` 下报嵌套缩放盒的 `layout_result`，`write_layout_does_not_extract_bit_identical_descendants` 也变红（文档节点的记录不再重新提取）。与 736e1b51b 交替 8 轮、每轮交换先后读最小值（负载 4.3–5.1）：简单页、控件页、`SCROLLED=1` 改行高整帧 1 091.5 → 1 092.3、1 171.2 → 1 174.9、1 251.6 → 1 254.2 µs，提取段 55 → 53、58 → 59、56 → 56 µs，控件页改文字 21.3 → 21.6 µs，都在噪声内：这些页面里被改的行的父节点自己的盒子也变了，本来就重新提取。

仍然按节点处理、没有吃到这次好处的地方：

- 平移的根本身（约 1 000 行）要重投影，这是无障碍剩下的约 0.3 ms。
- 根到页面有 3D，根是模态框，有动画叠加或裁剪 visual 时，无障碍仍整棵走旧路径（原因见 3.7）。滚动过的容器、正缩放（收尾·无障碍滚动与 Select）和旋转、斜切、翻转（收尾·无障碍变换）已经只动根。3D 不做：平台的 `transform` 只有 2D 仿射，透视下四角不是任何仿射能给的，只能退回外接框。
- 从视口来、自带框架的盒子（转过的空间里的 `sticky`、从移动的行量起的绝对定位、自己转的 `fixed`）跟着父节点平移时自己仍重投影：它的 bounds 是页面位置过映射得来，末位随平移变（`keeps_relative_bounds` 对它答「不保留」）；它下面按偏移放置的盒子不动（收尾·视口框架）。
- 改一个盒子的 `transform` 样式：改转角、改缩放系数、改变换原点、只改平移，从视口来的盒子（页面根、`fixed`、`sticky`、逃逸的绝对定位）也一样，已经只动它自己和它下面的钉子（收尾·改变换、收尾·视口框架）。仍整棵重投影的：自带框架与只是平移之间切换（空间换了：系数变成或不再是 1，转与不转）；盒子自己裁剪（子孙的 `clip` 在视口坐标里）；3D 上下文下面的缩放（父节点空间不可逆，它不自带框架，子孙是乘了系数的偏移）；3D 和透视（平台只有 2D 仿射）。
- 打开的 Select 仍是钉子，平移时连同列表框和选项重投影。虚拟节点的 bounds 在字段自己的空间里（转过时在字段空间里正立），但菜单的几何读页面：`select_menu_room` 按视口和上面的裁剪链决定菜单往上还是往下开、留多大，字段一动菜单相对字段就可能变，所以必须重投影。 收尾·打开的 Select 评估后保留：列表框和选项的 bounds 是页面上的组件几何与页面上的空间求交、再过视口映射到字段空间得来的，平移后末位就变；要不钉住，得把这条投影改成从本地几何和本地空间算（与现在的输出在末位不同），再给钉子加一个「菜单盒子没变、空间没切到它」才跳过的条件。打开的菜单是临时状态，一次只开一个，重投影的是字段、列表框和可见的行（几个到十几个节点），基准里量不出来（`CONTROLS=1 MIXED=1 OPEN_SELECT=1` 与 `OPEN_SELECT=closed` 的无障碍段在同一轮交替里相差在噪声内）。
- 打开的 MenuSurface、Painter 非恒等变换、参照系不是父节点的图标、图表悬停线、行内字形对象、转场视口里的文字，平移时仍重新提取或重建（3.5，各自保留的原因写在那里）。打开的 Select 已改为本地记录（收尾·打开的 Select），只在菜单换边、被截短或空间变了时重新提取。 MenuSurface 保留：它的表面是菜单项外接框，菜单项的参照系是页面、由布局在每次平移时重新放置（贴视口翻转），本地表面是「菜单项页面盒子 − 触发器原点」，两边各挪同一个量后相减在末位不同，记录本来就保不住；一次只开一个菜单，代价是平移时重新提取这一个节点。
- 滚动过的容器里改行高，整帧仍比普通页多约 0.17 ms（收尾·视口框架之后）。剩下的三处都是 O(移动行数) 或 O(行数)，常数已压低，量级没变：
  - 内容范围每个移动的行一次叶子写入（约 40 µs）。惰性区间平移做不到逐位：叶子是 `fl(子节点范围 + 偏移)`，「旧叶子 + 位移」与它在末位不同（变异实验 M6 让冷对照报了），布局又本来就逐行写偏移、逐行标脏，所以没有 O(log n) 的余地。
  - 无障碍比普通页多约 90 µs（收尾·视口框架之前约 170 µs）。子节点的「有没有视口盒」判定已经每个父节点取一次映射（`HandedMap`），每个子节点只读记录和两个角；剩下的是每个移动的行自己：它的视口盒（四个角、取外接框、进记忆表）、它的裁剪区查表、它的变换记忆（快路径省掉了参照系链和各种查询，但仍要一次矩阵乘和插表）。行自己的视口盒在相对投影里只用来判断有没有、随后被本地偏移盖掉，改成用父节点的映射判有无、不算外接框能再省一些，但 `project_accessibility_node` 同时服务视口投影，要把这条路拆开，这次没做。子节点判定本身不能再免：有没有视口盒要看子节点自己的页面盒子过映射是否有限（宽到 3e38 的盒子在 4 倍缩放下没有视口盒），父节点一次判定不了。
  - 布局里滚动容器量它的子节点（列表）时问基线，`holds_line_box` 的记忆原来从被编辑的叶子一路清到根，列表要把 2 000 行再问一遍。——已由收尾·行盒记忆解决：只改尺寸、间距、对齐、滚动、变换的种子不清记忆，`SCROLLED=1` 布局段少约 25–40 µs，整帧少约 20–40 µs。布局段剩下的约 45 µs 是内容范围的叶子写入（上一条）和 `record_layout_origin` 多出的滚动偏移记录。
- 透视和 3D 上下文（`perspective`、`preserve-3d`、`transform_3d`）下的命中仍按页面一路乘下去：透视并不进仿射，3D 上下文要把变换往下传，那里的盒子不把自己的布局坐标交给子节点，平移照旧整棵修补（3.4）。缩放、旋转、斜切、翻转已经只重建根（收尾·命中变换）。
- 改一个盒子的 `transform` 样式，命中只重建交出本地坐标的盒子自己的条目（收尾·改变换）；纯平移、变成或不再是这种条目、透视和 3D 下照旧整棵修补它的分支：子孙的条目在它交出的变换里，变换一变它们就都变。剔除索引的边界是页面上过了变换的外接框，转过以后不是平移能得到的，代码上祖先的投影变了仍走 `refresh_subtree`；`VISIBLE=1 ROTATED=1 edits 2000 transform` 的整帧与不剔除时相同（约 26 µs），这次没有量到它的成本，记下待查。


收尾·打开的 Select（2026-11-05）：打开的 Select 不再算读页面，菜单在字段自己的坐标里推导：`menu_room` 减去字段的页面原点，菜单只经由表面盒子（`select_menu_surface`）读它；平移时比一次现算的表面盒子与渲染器持有的记录，逐位相同就沿用记录，不同（换边、截短）才重新提取（3.5）。无障碍里它仍是钉子，MenuSurface、Painter 非恒等变换、参照系不是父节点的图标、图表悬停线、行内字形对象、转场视口里的文字评估后保留，原因见 3.5、3.7。基准新增 `CONTROLS=1 MIXED=1 OPEN_SELECT=1`（改第二行的标签，第四行第二格是打开的 Select，在视口里、每次编辑都被平移；`OPEN_SELECT=closed` 是同一页关着）。与 6e21c8579 交替 8 轮读最小值（负载 3.0–4.5），各页整帧都在噪声内（数字见 `resource-budgets.md`）；一次编辑约 4 700 万条指令里打开的 Select 原来多出的部分（重新提取、重建约 11 个图元、重投影）在 `time -l` 的几次测量间波动（±30 万条）以内，所以收益只在测试的计数上看得到：没换边的平移重建图元 11 → 0。