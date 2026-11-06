# Components

This page lists the existing controls by family. You use them to attach nodes to the tree. The Rust entry point is `nana_ui::runtime`. Names and template tags come from the same control table: a misspelled tag or attribute is a compile error in `view!`.

Each item has its own page with a property table and an event table. The examples switch with the toggle at the top of the sidebar. For controls without their own template tag, the template side is written as `<Widget of={...}>`, and slots as `<template #name>`. The contract for behavior and events is still [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md).

## Easily confused

- [Select](https://github.com/sena-nana/NanaUI/blob/main/docs/components/select.md) is a single-value dropdown. [Dropdown](https://github.com/sena-nana/NanaUI/blob/main/docs/components/dropdown.md) is single- or multi-value and shares Select's menu. [SearchDropdown](https://github.com/sena-nana/NanaUI/blob/main/docs/components/search-dropdown.md) runs its query through a committed input box.
- [Dialog](https://github.com/sena-nana/NanaUI/blob/main/docs/components/dialog.md) has its own body and footer. [ConfirmDialog](https://github.com/sena-nana/NanaUI/blob/main/docs/components/confirm-dialog.md) is confirm and cancel. [Drawer](https://github.com/sena-nana/NanaUI/blob/main/docs/components/drawer.md) also has the dialog role and enters from an edge.
- [AppShell](https://github.com/sena-nana/NanaUI/blob/main/docs/components/app-shell.md) is a shell without a sidebar or Dock. [DesktopShell](https://github.com/sena-nana/NanaUI/blob/main/docs/components/desktop-shell.md) is a desktop shell with a grid. [Workspace](https://github.com/sena-nana/NanaUI/blob/main/docs/components/workspace.md) content is child nodes, and what goes in each region is up to the application; see [Workspace](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/workspace.md).

## Actions and input

- [Button](https://github.com/sena-nana/NanaUI/blob/main/docs/components/button.md) — Emits `Activate` when pressed. The label is the accessible name.
- [IconButton](https://github.com/sena-nana/NanaUI/blob/main/docs/components/icon-button.md) — A single icon. `label` is the accessible name. Activation does not change `selected` on its own.
- [TextInput](https://github.com/sena-nana/NanaUI/blob/main/docs/components/text-input.md) — One line of text. `model` binds `value`.
- [TextArea](https://github.com/sena-nana/NanaUI/blob/main/docs/components/text-area.md) — Multi-line text. Has `on_input`, no `on_submit`.
- [NumberInput](https://github.com/sena-nana/NanaUI/blob/main/docs/components/number-input.md) — Numbers. Writing `value` is `assign`.
- [Checkbox](https://github.com/sena-nana/NanaUI/blob/main/docs/components/checkbox.md) — Check box. `model` binds `checked`.
- [Switch](https://github.com/sena-nana/NanaUI/blob/main/docs/components/switch.md) — Switch. Has one more state than a checkbox: `loading`.
- [RangeField](https://github.com/sena-nana/NanaUI/blob/main/docs/components/range-field.md) — Slider. The template tag is `<Slider>` and the function is `slider(min, max, step)`.
- [TimelineEditor](https://github.com/sena-nana/NanaUI/blob/main/docs/components/timeline-editor.md) — An editable time base: frame / second ruler and playhead, collapsible tracks, keyframe selection and dragging (snapped to frames), movable and trimmable clips, a Bézier curve view with tangent handles; content belongs to the application, and edits are reported through events.
- [PropertyGrid](https://github.com/sena-nana/NanaUI/blob/main/docs/components/property-grid.md) — Inspector-style two-column property panel: collapsible groups, a shared draggable label column, mixed values, and disabled rows with a reason; controls are placed by row id.
- [ScrubField](https://github.com/sena-nana/NanaUI/blob/main/docs/components/scrub-field.md) — A property field whose number changes as you drag left and right over it; click to type, with mixed-value support.
- [RangeSpanField](https://github.com/sena-nana/NanaUI/blob/main/docs/components/range-span-field.md) — Two-thumb range, horizontal or vertical, with an optional live value marked on the track. The template tag is `<RangeSpan>` and the function is `range_span(min, max, step)`.
- [CurveField](https://github.com/sena-nana/NanaUI/blob/main/docs/components/curve-field.md) — A polyline in a unit square; each point has a fixed horizontal position and you drag it up and down to change its height. The template tag is `<Curve>` and the function is `curve(step)`.
- [Select](https://github.com/sena-nana/NanaUI/blob/main/docs/components/select.md) — Single-value dropdown.
- [Dropdown](https://github.com/sena-nana/NanaUI/blob/main/docs/components/dropdown.md) — Single or multiple values. No `<Dropdown>` tag.
- [SearchDropdown](https://github.com/sena-nana/NanaUI/blob/main/docs/components/search-dropdown.md) — The query goes through a committed `TextInput`.
- [FindReplaceBar](https://github.com/sena-nana/NanaUI/blob/main/docs/components/find-replace-bar.md) — Find-and-replace toolbar; the application handles searching and editing.
- [SegmentedControl](https://github.com/sena-nana/NanaUI/blob/main/docs/components/segmented-control.md) — Segmented selection. The options are its child nodes.
- [Tabs](https://github.com/sena-nana/NanaUI/blob/main/docs/components/tabs.md) — Tabs. Selection, reordering, closing, and dragging between strips are reported from here.
- [XYPad](https://github.com/sena-nana/NanaUI/blob/main/docs/components/xy-pad.md) — Two-dimensional pad. Press and move are `Input`; release is `Change`.
- [ColorField](https://github.com/sena-nana/NanaUI/blob/main/docs/components/color-field.md) — Swatch, hex, and HSV. The control assembles itself.
- [PathField](https://github.com/sena-nana/NanaUI/blob/main/docs/components/path-field.md) — A path input plus an icon button. The control assembles itself.
- [DatePicker](https://github.com/sena-nana/NanaUI/blob/main/docs/components/date-picker.md) — A header and 6×7 date buttons.

## Layout and text

- [Text](https://github.com/sena-nana/NanaUI/blob/main/docs/components/text.md) — Text. Child text with interpolation recomputes with its signals.
- [RichTextView](https://github.com/sena-nana/NanaUI/blob/main/docs/components/rich-text-view.md) — Text with styled ranges: font, color, stroke, shadow, decoration lines. The application owns the value; changing a color does not relayout.
- [RichTextEditor](https://github.com/sena-nana/NanaUI/blob/main/docs/components/rich-text-editor.md) — Edits a `RichText` in place, breaking lines the same way as RichTextView. Toolbars send commands through `cx.rich_edit`.
- [Column](https://github.com/sena-nana/NanaUI/blob/main/docs/components/column.md) — Vertical arrangement. The spacing is `.gap`, in logical pixels.
- [Row](https://github.com/sena-nana/NanaUI/blob/main/docs/components/row.md) — Horizontal arrangement. Width shrinks to fit content, and children are centered on the cross axis.
- [Stack](https://github.com/sena-nana/NanaUI/blob/main/docs/components/stack.md) — The container behind `row` / `column` / `bar` / `spacer` / `layers` / `overlay_layer`. Rows and columns are flex; `layers` stacks.
- [Divider](https://github.com/sena-nana/NanaUI/blob/main/docs/components/divider.md) — Divider line. No label, and it takes no part in interaction.
- [IconGlyph](https://github.com/sena-nana/NanaUI/blob/main/docs/components/icon-glyph.md) — Decorative icon. Takes no pointer input and cannot be focused.
- [ScrollView](https://github.com/sena-nana/NanaUI/blob/main/docs/components/scroll-view.md) — Scrolling. The position is in `ScrollOffset`, the sizes in `ScrollMetrics`.

## Tables and trees

- [Table](https://github.com/sena-nana/NanaUI/blob/main/docs/components/table.md) — Table (`TableRow` / `TableCell`). It does not compare data itself.
- [TreeView](https://github.com/sena-nana/NanaUI/blob/main/docs/components/tree-view.md) — Tree. Does not read paths or watch the file system.
- [ReorderList](https://github.com/sena-nana/NanaUI/blob/main/docs/components/reorder-list.md) — Drag to reorder. It reports the moved value and the value after it, and does not change the order itself.

## Display

- [Card](https://github.com/sena-nana/NanaUI/blob/main/docs/components/card.md) — Card. Actions go on the controls inside; the card itself does not act as a button.
- [List](https://github.com/sena-nana/NanaUI/blob/main/docs/components/list.md) — List. Screen readers read it as a list.
- [ListItem](https://github.com/sena-nana/NanaUI/blob/main/docs/components/list-item.md) — One row. Selection is not toggled by the control itself.
- [FormField](https://github.com/sena-nana/NanaUI/blob/main/docs/components/form-field.md) — Form row. The control is your child node.
- [DynamicForm](https://github.com/sena-nana/NanaUI/blob/main/docs/components/dynamic-form.md) — Dynamic form driven by stable keys; the application owns state and validation.
- [InteractionRequestCard](https://github.com/sena-nana/NanaUI/blob/main/docs/components/interaction-request-card.md) — A neutral request card; the application supplies fields and actions.
- [EmptyState](https://github.com/sena-nana/NanaUI/blob/main/docs/components/empty-state.md) — Empty state. One action can go below the title with `.action`.
- [Progress](https://github.com/sena-nana/NanaUI/blob/main/docs/components/progress.md) — Progress. Only `value` and `label`.
- [Skeleton](https://github.com/sena-nana/NanaUI/blob/main/docs/components/skeleton.md) — Holds the size while content has not arrived.
- [Spinner](https://github.com/sena-nana/NanaUI/blob/main/docs/components/spinner.md) — Spinner. An empty label is just the ring; with a label, it sits to the left of the text.
- [StatusBadge](https://github.com/sena-nana/NanaUI/blob/main/docs/components/status-badge.md) — Status badge. Only describes a status; owns no action.
- [Chip](https://github.com/sena-nana/NanaUI/blob/main/docs/components/chip.md) — Chip. Activation does not change `selected` on its own.
- [Avatar](https://github.com/sena-nana/NanaUI/blob/main/docs/components/avatar.md) — Round avatar. Does not take part in hit testing by default.
- [Thumbnail](https://github.com/sena-nana/NanaUI/blob/main/docs/components/thumbnail.md) — A host texture shown at its aspect ratio. Hits belong to the parent row; when `pickable`, it reports the pointer position within the image.
- [Texture](https://github.com/sena-nana/NanaUI/blob/main/docs/components/texture.md) — A host texture that fills its own box.
- [Tooltip](https://github.com/sena-nana/NanaUI/blob/main/docs/components/tooltip.md) — Tooltip. The role is tooltip, and the label is the displayed text.
- [ValidationMessage](https://github.com/sena-nana/NanaUI/blob/main/docs/components/validation-message.md) — Validation text. Together with `invalid` on the control and `error` on `FormField`, it is one of three separate displays.
- [QrCode](https://github.com/sena-nana/NanaUI/blob/main/docs/components/qr-code.md) — QR code. Not focusable; the role is image.
- [ImageViewer](https://github.com/sena-nana/NanaUI/blob/main/docs/components/image-viewer.md) — Scrim, surface, stage, and caption row.
- [NativeMarkdown](https://github.com/sena-nana/NanaUI/blob/main/docs/components/native-markdown.md) — Markdown. Copying takes a snapshot of its selection.
- [CalendarHeatmap](https://github.com/sena-nana/NanaUI/blob/main/docs/components/calendar-heatmap.md) — Calendar heatmap. You supply dates, values, and titles.
- [Chart](https://github.com/sena-nana/NanaUI/blob/main/docs/components/chart.md) — Charts: line, bar, pie and donut, scatter, radar, gauge. The option mirrors ECharts; you supply data and labels.
- [GraphCanvas](https://github.com/sena-nana/NanaUI/blob/main/docs/components/graph-canvas.md) — Graph canvas. You keep the model and its persistence.
- [GraphMinimap](https://github.com/sena-nana/NanaUI/blob/main/docs/components/graph-minimap.md) — Draws nodes and the viewport according to `GraphModel::bounds`.

## Overlays

- [Dialog](https://github.com/sena-nana/NanaUI/blob/main/docs/components/dialog.md) — Title, body, and footer. No `<Dialog>` tag.
- [ConfirmDialog](https://github.com/sena-nana/NanaUI/blob/main/docs/components/confirm-dialog.md) — Confirmation box. The role is alert dialog.
- [Drawer](https://github.com/sena-nana/NanaUI/blob/main/docs/components/drawer.md) — Drawer. The role is dialog.
- [Popover](https://github.com/sena-nana/NanaUI/blob/main/docs/components/popover.md) — Popover. Press, focus, Enter, Space, and the panel anchor all go through the trigger.
- [ActionMenu](https://github.com/sena-nana/NanaUI/blob/main/docs/components/action-menu.md) — Action menu. Alignment changes to the start edge.
- [ContextMenu](https://github.com/sena-nana/NanaUI/blob/main/docs/components/context-menu.md) — Context menu. The role is menu.
- [AppMenuBar](https://github.com/sena-nana/NanaUI/blob/main/docs/components/app-menu-bar.md) — A menu bar drawn inside the application UI that follows the theme, with cascading submenus; selected items go through the same id queue as native menus.
- [CommandPalette](https://github.com/sena-nana/NanaUI/blob/main/docs/components/command-palette.md) — Command palette. You do the actual dispatch and save shortcuts.
- [Toast](https://github.com/sena-nana/NanaUI/blob/main/docs/components/toast.md) — Passive notification. Once assembled, it has progress, one action, and a close button; you remove it.
- [ToastStack](https://github.com/sena-nana/NanaUI/blob/main/docs/components/toast-stack.md) — Notification corner. Measures before showing, queues, and animates in and out; placed above the modal layer, it stays pressable while a modal is open.

The framework places overlays in the window. For opening, closing, anchors, and focus return, see [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md).

## Shell

- [AppShell](https://github.com/sena-nana/NanaUI/blob/main/docs/components/app-shell.md) — Use it when you need no sidebar, inspector, or Dock.
- [DesktopShell](https://github.com/sena-nana/NanaUI/blob/main/docs/components/desktop-shell.md) — Desktop shell. You still supply the content in the grid.
- [AppTitleBar](https://github.com/sena-nana/NanaUI/blob/main/docs/components/app-title-bar.md) — Title bar. Its height is `TITLE_BAR_HEIGHT`.
- [Toolbar](https://github.com/sena-nana/NanaUI/blob/main/docs/components/toolbar.md) — Toolbar. You place the buttons in it.
- [ToolGroup](https://github.com/sena-nana/NanaUI/blob/main/docs/components/tool-group.md) — Tool group. A set of tool buttons on one shared trough, horizontal or vertical, read as single choice, toggles, or actions.
- [StatusBar](https://github.com/sena-nana/NanaUI/blob/main/docs/components/status-bar.md) — Status bar. Changes are announced as live status.
- [MediaTransportBar](https://github.com/sena-nana/NanaUI/blob/main/docs/components/media-transport-bar.md) — Play, progress, volume, settings, and fullscreen.
- [Workspace](https://github.com/sena-nana/NanaUI/blob/main/docs/components/workspace.md) — Workspace. Content is child nodes in the tree.
- [SidebarFrame](https://github.com/sena-nana/NanaUI/blob/main/docs/components/sidebar-frame.md) — Sidebar frame. No built-in product navigation.
- [SidebarSection](https://github.com/sena-nana/NanaUI/blob/main/docs/components/sidebar-section.md) — Sidebar group. You supply the rows in the body.
- [SidebarRow](https://github.com/sena-nana/NanaUI/blob/main/docs/components/sidebar-row.md) — One sidebar row. Drawn by reusing the list item's slots.
- [SettingsRow](https://github.com/sena-nana/NanaUI/blob/main/docs/components/settings-row.md) — Settings row; `label_column` switches to an inspector-style aligned label column. Your state still owns the actual values.
- [Dock](https://github.com/sena-nana/NanaUI/blob/main/docs/components/dock.md) — Docking. A pane's content is child nodes.
- [AreaLayout](https://github.com/sena-nana/NanaUI/blob/main/docs/components/area-layout.md) — DCC-style region splitting: drag from a corner to split or join, regions can be maximized, headers and content are placed by key, and the layout is serializable.
- [SplitPane](https://github.com/sena-nana/NanaUI/blob/main/docs/components/split-pane.md) — Split. Sizes come from `SplitPaneModel`.
- [PaneChrome](https://github.com/sena-nana/NanaUI/blob/main/docs/components/pane-chrome.md) — A tab strip and a row of icon actions.

What goes in each region is up to your application; see [Workspace](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/workspace.md).

## Surfaces

Sizes, colors, fonts, and control surfaces come from the theme. Switching between dark and light changes colors. Single-line controls have three heights: small, medium, and large. For the exact values and how to switch themes, see [Look](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/look.md).

Some families need their own Cargo feature, such as `calendar`, `charts`, `graph-canvas`, and `rich-text`. `components` enables all optional families. See the table in [Application API](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/application-api.md).
