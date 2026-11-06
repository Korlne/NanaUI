# Workspace, Dock, and Region Layout

Desktop productivity software is usually built from a sidebar, a main workspace, an inspector, a bottom panel, and dock panes that can be split or floated out. NanaUI provides the structure and interaction contracts for these desktop shells in Runtime. What business content each region or pane shows, and how business state is routed and persisted, is always decided by your application.

If the application only needs a simple single-page or two-column structure, use `List` or `AppShell` directly. You do not need `Workspace`.

## Workspace: semantic workspace regions

`WorkspaceLayout` keeps a set of `RegionState` values in registration order. Each region is defined by the following fields:

| Field | Purpose |
| --- | --- |
| `id` | Built-in region name (such as `Primary`, `Resources`, `Inspector`) or a stable `Custom(String)` identity |
| `role` | Region role: navigation, resources, primary, inspector, timeline, console, tools |
| `placement` | Dock side: `start` / `primary` / `end` / `top` / `bottom` |
| `scope` | Scope: `workspace` (window level) or `primary` (inside the primary region) |
| `size` / `min` / `max` / `default` | Size in logical pixels and its resize bounds |
| `fill_priority` | How remaining space is shared when there are several `primary` regions |
| `collapsible` / `resizable` | Whether the region can collapse, and whether its edge can be dragged to resize it |
| `narrow_behavior` | Response when the window narrows: `shrink` / `collapse` / `overlay` / `none` |

The built-in `RegionId` provides a standard six-region preset. Applications can also register any number of start/end and top/bottom regions through `Custom(String)` (`register` rejects duplicate IDs).

- **Single source of truth**: region sizes, collapsed state, visibility, and JSON serialization and restore are all owned by `WorkspaceModel` and `WorkspaceMutation`. The host-side `WorkspaceController` only converts pointer events and the clock into `WorkspaceMutation` values. It does not keep a second copy of the state in the adapter.
- **Content mounting and splitters**: the `Workspace` control mounts the application's child nodes into their regions through `WorkspaceRegionSlot { id, content }`. A splitter's `8 px` hit area is centered across the edge between adjacent regions and does not occupy a grid track itself.
- **Collapse and viewport mapping**: a collapsed or hidden region is not painted, does not respond to dragging, and gives up its space in the layout geometry (with a `260 ms` transition that requests frames only while the transition runs). A region in `overlay` mode floats above the primary region and does not take space from `primary`. `WorkspaceGeometry` maps the layout result to logical rectangles and physical-pixel rectangles for GPU viewports. It does not create windows or GPU resources itself.

## Sidebar, settings page, and non-modal panels

- **Sidebar (`SidebarFrame` / `SidebarSection` / `SidebarRow`)**: provides a generic structure for groups and navigation rows. Routing and selection state are managed by the application.
- **Settings page (`SettingsModel` / `SettingsState`)**: keeps stable settings tabs and restore rules. The theme list comes from the application's `ThemeRegistry`, and the `ThemeId` is persisted. Other business settings are held in application state. When the settings page uses its own `WorkspaceController`, entering it does not overwrite the main workspace's region sizes and collapsed state.
- **Non-modal panel over full-window content (`OverlayHost` + `Panel`)**: when you need an interactive task panel over live content that fills the window, mount a `Panel` with `OverlayHost`. It does not change the main workspace size, and it does not lock the pointer or Tab focus traversal outside the panel. The application only provides business content and the viewport insets (`PanelInsets`). The framework handles the panel surface, the close lifecycle, and focus restoration.

## Dock: tab docking and floating windows

Product-level docking is provided by the Runtime `Dock` component and the `DockWorkspace` model. (`DockController` / `DockAction` in `nana_ui::dock::*` are only a host adapter that translates pointer and frame events into mutations. They are not a second Dock.)

- Pane content is ordinary child nodes in the tree. Split ratios, drag hit bars, and drop previews are computed by the framework.
- When the user drags a pane out of the main window into a floating panel, `DockWorkspace` emits a `DockWorkspaceEvent`. The host converts it into `WindowCommand::Open` or `Close` through `runtime_dock_window_update` (see `examples/runtime-host-fixture` for the full wiring).

## AreaLayout and professional-tool inspector surfaces

Besides `Workspace` (fixed semantic regions) and `Dock` (multi-tab docking and floating windows), DCC tools for 3D, animation, and audio/video often need "split the screen anywhere" and "high-density property inspectors":

- **[AreaLayout](../components/area-layout.md)**: recursively splits a rectangle into areas that never overlap, each showing one application-chosen editor. The user can drag inward from the small squares at the top-left/top-right corner of an area header to split off a new area, or drag toward an adjacent sibling to merge. It supports maximizing a single area and switching between whole workspace presets (`set_area_layout`). When a preset is switched, areas shared by the old and new layouts keep their child nodes in place (scroll position and input state are not lost), and areas that do not appear are hidden and kept waiting, not destroyed.
- **[PropertyGrid](../components/property-grid.md)** and **[ScrubField](../components/scrub-field.md)**: an inspector-style dense two-column property grid (with collapsible groups, a shared label-column width that drags in sync across the whole grid, the multi-selection mixed value `mixed`, and row masks that carry a disabled reason), and a numeric property field whose value can be changed by dragging left and right, or typed after a click.
- **[TimelineEditor](../components/timeline-editor.md)**: an editable time base painted by a single node. It includes a ruler and playhead that adapt between frames and seconds, a collapsible track tree on the left, keyframe box selection and whole-frame snapping while dragging, clip selection, moving, and trimming (snapping to clip edges and the playhead, without overlap), and a cubic Bézier curve view with tangent handles.

### Choosing between Workspace, Dock, and AreaLayout

| Shell component | Use case | Key traits |
| --- | --- | --- |
| `Workspace` | Main frame of a standard desktop application | Semantic regions (navigation, primary, inspector, bottom bar), automatic collapse in narrow windows, and a `260 ms` expand transition |
| `Dock` | IDE-style multi-panel docking | Panels stack into tab groups, split when dragged to an edge, or detach into independent floating windows |
| `AreaLayout` | Blender-style DCC workbench | Areas never overlap; split or merge by dragging from corner blocks; supports custom switchers in area headers and instant preset switching |

## Tab reordering, list drag and drop, and the command palette

- **Tab and list reordering**: `Tabs::on_reorder` and the vertical `ReorderList` both describe reorder intent as "the moved item + the item that will directly follow it at the target position (`None` means the end)". The component itself never changes the order of the business array. Dragging tabs across windows uses `TabDragGroup` and `TabDragSurface` (a receiver can use `accepts_external_drop(false)` to allow dragging out only and reject drops from outside).
- **Tree view drag and drop (`TreeView`)**: renders the application's stable node IDs, expansion state, and hierarchy as a collapsible tree. It supports multi-selection, trailing toggle columns, inline rename (double-click or `F2`), and drag-and-drop ordering. A drop reports the target row and a `Before` / `Inside` / `After` intent. It does not read or watch the local file system.
- **Command palette (`CommandPalette`) and `ActionRegistry`**: `ActionRegistry` registers stable action IDs and shortcut contexts. `CommandPalette` only shows the actions available in the current context. Command dispatch and shortcut persistence are performed by the application.
- **Split projection (`PaneTree` / `PaneChrome`)**: provides a lightweight rendering projection of splits and tab bars. Pane IDs, dirty flags, and cross-window move logic belong to the application.
