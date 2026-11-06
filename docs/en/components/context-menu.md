# ContextMenu

`ContextMenu` is a menu pinned to one coordinate. Its accessible role is menu. It hangs under an `OverlayHost` and opens with `activate_overlay`.

The control table has no `<ContextMenu>`.

## Basic usage

The anchor of `ContextMenu::new(anchor_x, anchor_y)` is a logical coordinate, and `open` is true at construction.

Escape and outside presses are dismissed by the framework under menu semantics, so you do not write a second outside-press check.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::{ContextMenu, ContextMenuItem};

view! {
    <Widget
        of={ContextMenu::new(120.0, 80.0)
            .searchable(true)
            .items([
                ContextMenuItem::new("rename", "Rename"),
                ContextMenuItem::new("delete", "Delete").danger(true),
            ])}
    />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{ContextMenu, ContextMenuItem};

widget(
    ContextMenu::new(120.0, 80.0)
        .searchable(true)
        .items([
            ContextMenuItem::new("rename", "Rename"),
            ContextMenuItem::new("delete", "Delete").danger(true),
        ]),
)
```

:::

## Items

`.items` takes `ContextMenuItem::new(value, label)`.

`.hint`, `.icon`, `.image`, `.disabled` and `.danger` are written on the item.

`.image(slot)` puts a round image in the icon's place, painting the host texture slot `slot`, with the same rules as `.leading_image` of [ActionMenu](https://github.com/sena-nana/NanaUI/blob/main/docs/components/action-menu.md#条目): the image wins over the icon, its edge is the row height, an empty slot paints a placeholder circle, and the image is decoration only.

A value split by slashes (`parent/child`) is one level of tree.

## Search

`.searchable(true)` adds a filter box, and the query lives in the committed `TextInputState`.

`.query(...)` and `set_query` change that text; read it with `query_text()`.

An empty query shows the current level; a query shows the matching leaves.

## Position

Layout places the surface against the real viewport, using the size the menu measured:

- when it does not fit below the anchor but fits above, it opens above;
- when it does not fit on the right, it draws in to the left, always staying inside the window.

When the window is resized, layout replays directly and the component is not projected again.

`place_in(room)` further limits the surface to a smaller area, such as the panel that owns it. Filtering and drilling into a submenu re-anchor, and this boundary keeps applying.

## Highlight

The menu has one highlight, which is also the item the screen reader stops on: whichever item the pointer moves to (disabled ones excepted) takes it, the arrow keys move it too, and Enter selects it. It is cleared when the pointer leaves the menu.

## Dismissal

The events are `ContextMenuEvent::Select(value)`, `Search` and `Dismiss`.

When the framework dismisses it, it writes `open` to `false` and emits `Dismiss`, the same receipt as the dismissal after a selection.

A right press on its own only emits `SecondaryPress`.

You decide in the handler whether to pop up this menu.

The keyboard `ContextMenu` key and Shift+F10 emit the same `SecondaryPress` with `keyboard` true, and the coordinate is the center of the focused node's box, usable as the anchor directly.

ArrowDown / ArrowUp on a menu button (`has_popup`) emit it too, with `focus` as `Some(RovingEdge::Start)` / `Some(RovingEdge::End)`: focus the first / last item after opening. Otherwise `focus` is `None` and you choose. When the menu items are in an arrow-key focus group, use `AppContext::focus_roving_edge(group, edge)` to focus that end.

`activate_overlay(host, menu)` requires the menu to be a direct child of that host.

`dismiss_overlay(host)` stops interaction and restores focus first, then leaves the painting until the exit animation ends.

Before the exit animation ends, the node is still in the tree.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `ContextMenu::new(anchor_x, anchor_y)` | — | The anchor is a logical coordinate, and `open` is true at construction |
| `.items` | — | Takes `ContextMenuItem::new(value, label)` |
| `.hint` | — | Written on the item |
| `.icon` | — | Written on the item |
| `.image(slot)` | `TextureSlotName` | Written on the item: a round image in the icon's place, painting a host texture slot |
| `.disabled` | — | Written on the item |
| `.danger` | — | Written on the item |
| `.searchable` | — | `.searchable(true)` adds a filter box, and the query lives in the committed `TextInputState` |
| `.query(...)` | — | With `set_query`, changes that text; `query_text()` reads the current value |
| `set_query` | — | Changes that text |
| `open` | — | True at construction. The framework writes it to `false` when it dismisses the menu |
| `place_in(room)` | — | Limits the surface to the intersection of this area and the window. Without the call, it is limited to the window only |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| `ContextMenuEvent::Select(value)` | `value` | The events are `ContextMenuEvent::Select(value)`, `Search` and `Dismiss` |
| `Search` | — | The events are `ContextMenuEvent::Select(value)`, `Search` and `Dismiss` |
| `Dismiss` | — | When the framework dismisses it, it writes `open` to `false` and emits `Dismiss`, the same receipt as the dismissal after a selection |
| `SecondaryPress` | `target`, `x`, `y`, `keyboard`, `focus` | A right press on its own only emits this. The `ContextMenu` key and Shift+F10 emit it too, with `keyboard` true. You decide in the handler whether to pop up this menu |

## Slots

| Slot | Description |
| --- | --- |
| None | None |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
