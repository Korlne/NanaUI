# SidebarRow

`SidebarRow` is one navigation row in a sidebar. It draws itself by reusing the list item's slots, but its identity remains a sidebar row. Links, routing and selection are yours to supply; NanaUI ships no product navigation.

## Basic usage

The control table has no `<SidebarRow>`.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::{SidebarRow, SidebarRowState};

view! {
    <Widget of={SidebarRow::new("Overview").state(SidebarRowState::Active).depth(0)} />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{SidebarRow, SidebarRowState};

widget(
    SidebarRow::new("Overview")
        .state(SidebarRowState::Active)
        .depth(0),
)
```

:::

One section can show several rows as `Active`; which row stands for the current page is written into `state` by you.

## State

`SidebarRow::new(label)` is idle, at depth 0, and small by default. `.state` takes a `SidebarRowState`: `Idle`, `Active`, `AncestorActive`, `Disabled`.

Use `Active` for the current page, and `AncestorActive` for an ancestor row while the current page is below it. Use `SidebarRowState::Disabled` for a disabled row.

## Depth

`.depth` is the indentation in the tree, and `sidebar_row_depth_inset` turns a depth into left padding. Depth starts at 0, and each level steps by the sidebar's indent.

`.disclosure(expanded)` puts a disclosure marker at the start of the row.

## Tone and tools

`.tone` is a `SidebarRowTone`: `Default`, `Warning`, `Error`. `.tools(…)` are the tools at the end of the row, shown while the pointer is over this row or keyboard focus is on this row or inside it, replacing the trailing slot; so Tab stops on the row first and then on the tools. When collapsed they leave the layout by default, and a long label uses the whole row; `.tools_rest(RowToolsRest::Reserve)` instead keeps their width and only stops painting them, so the tools always hold the trailing position. `.keep_tools(true)` shows them normally too. The rules are the same as the inline tools of a [ListItem](https://github.com/sena-nana/NanaUI/blob/main/docs/components/list-item.md).

## Activation

Activation goes through `activate_sidebar_row`. Which route to follow after a press is done in your handler. The control keeps no history.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `label` | — | `SidebarRow::new(label)`. Idle, at depth 0, and small by default |
| `.state` | `SidebarRowState` | `Idle`, `Active`, `AncestorActive`, `Disabled`. Use `Active` for the current page and `AncestorActive` for an ancestor row |
| `.depth` | — | The indentation in the tree, starting at 0. `sidebar_row_depth_inset` turns a depth into left padding. Each level steps by the sidebar's indent |
| `.disclosure` | — | `expanded` puts a disclosure marker at the start of the row |
| `.tone` | `SidebarRowTone` | `Default`, `Warning`, `Error` |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| None | None | Activation goes through `activate_sidebar_row`. Routing is done in your handler. The control keeps no history |

## Slots

It draws itself by reusing the list item's slots, but its identity remains a sidebar row.

| Slot | Description |
| --- | --- |
| `.tools` | The tools at the end of the row, shown while hovered or while focus is inside the row. On the component the parameter is a node `id`; in a view it is a child view |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md) · [Workspace](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/workspace.md)
