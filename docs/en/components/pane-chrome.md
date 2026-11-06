# PaneChrome

`PaneChrome` is the title row at the top of a pane, plus a body that fills the space below. The title row holds tabs and a row of icon actions. Tab ids, dirty marks and moving between windows are still yours.

The control table has no `<PaneChrome>`. `.action` takes an action and a view, and cannot be written as `<template #action>`.

## Basic usage

`PaneChrome::new()` makes this pane active by default.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{PaneChrome, PaneChromeAction, PaneChromeActionKind};

widget(PaneChrome::new())
    .tabs(view! { <Text>"main.rs"</Text> })
    .action(
        PaneChromeAction::new(PaneChromeActionKind::CloseItem, "Close"),
        view! { <Button>"Close"</Button> },
    )
    .body(view! { <Text>"Body"</Text> })
```

```rust rust
use nana_ui::runtime::view::{button, text, widget};
use nana_ui::runtime::{PaneChrome, PaneChromeAction, PaneChromeActionKind};

widget(PaneChrome::new())
    .tabs(text("main.rs"))
    .action(
        PaneChromeAction::new(PaneChromeActionKind::CloseItem, "Close"),
        button("Close"),
    )
    .body(text("Body"))
```

:::

## Projection

`PaneTree` is only a lightweight projection for one render.

## Tabs and body

`.tabs(id)` is the tab strip in the title row.

`.body(id)` is the content below.

`.header(id)` replaces it with a whole title row of your own; the chrome styles it and does not change its children.

On a view, `.tabs(view)`, `.body(view)` and `.header(view)` place slots.

## Actions

`.actions` is a list of `PaneChromeAction`.

The `kind` of `PaneChromeAction::new(kind, label)` is a `PaneChromeActionKind`: `Focus`, `SplitHorizontal`, `SplitVertical`, `MoveToWindow`, `MoveToNextPane`, `ClosePane`, `CloseItem`, `Custom`.

When `.icon` has an icon, assembly paints it as an icon button.

`.action(action, view)` appends an action in writing order, with `view` as that button.

Whether an action closes a tab or splits a window when pressed is done in your handler.

The chrome only places the buttons in the title row and separates the surfaces of the active and idle panes.

## Assembly

`assemble_pane_chrome` is the `slot_assembler`.

It runs when the view is committed.

After placing slots by hand, call it once yourself.

Changing `active` does not re-assemble automatically.

## Active

`.active(false)` switches the title row's background one step lighter.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `.tabs` | — | `.tabs(id)` is the tab strip in the title row. On a view, `.tabs(view)` places the slot |
| `.body` | — | `.body(id)` is the content below. On a view, `.body(view)` places the slot |
| `.header` | — | `.header(id)` replaces it with a whole title row of your own; the chrome styles it and does not change its children |
| `.active` | — | `new()` makes this pane active by default. `.active(false)` switches the title row's background one step lighter. Changing `active` does not re-assemble automatically |
| `.actions` | `PaneChromeAction` | A list of actions. The `kind` of `PaneChromeAction::new(kind, label)` is a `PaneChromeActionKind`: `Focus`, `SplitHorizontal`, `SplitVertical`, `MoveToWindow`, `MoveToNextPane`, `ClosePane`, `CloseItem`, `Custom` |
| `.icon` | — | When it has an icon, assembly paints it as an icon button |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| None | None | Whether an action closes a tab or splits a window when pressed is done in your handler |

## Slots

| Slot | Description |
| --- | --- |
| `.tabs` | On a view, `.tabs(view)` places the slot |
| `.body` | On a view, `.body(view)` places the slot |
| `.header` | On a view, `.header(view)` places the slot. Replaces it with a whole title row of your own |
| `.action` | `.action(action, view)` appends an action in writing order, with `view` as that button. Cannot be written as `<template #action>` |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md) · [Workspace](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/workspace.md)
