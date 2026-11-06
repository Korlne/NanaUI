# AppShell

`AppShell` is a title bar, a body that fills the rest, and an optional overlay layer. When you do not need a sidebar, inspector or Dock, it is all you need. What goes into each region is up to you.

There is no `<AppShell>` in the control table.

## Basic usage

`AppShell::new()` starts with all three slots empty.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::AppShell;

view! {
    <Widget of={AppShell::new()}>
        <template #body>
            <Column />
        </template>
    </Widget>
}
```

```rust rust
use nana_ui::runtime::view::{column, widget};
use nana_ui::runtime::AppShell;

widget(AppShell::new()).body(column())
```

:::

## Assembly

`assemble_app_shell` is a `slot_assembler`.

It runs when the view commits, so do not call it again in the view.

After placing slots by hand with `create_component`, call it yourself.

A write that changes a field does not reassemble automatically.

When there is no title bar slot and no title bar child on the tree, this assembly does not create a title bar.

When the slot exists, or the child is already a title bar, assembly uses that one and then calls `assemble_app_title_bar`.

## Overlay

The overlay slot is absolutely positioned to fill the shell. If it is not an `OverlayHost`, it takes the pointer only when it has content: at least one child that is not hidden (`hidden` / `display: none`). While it is empty, the pointer falls through to the body below.

When the content changes, whether it takes the pointer is re-evaluated: when children are added or removed (`when`, `dynamic` or `each` used directly as slot content switches branch or rows; a branch collapsed by `keep_alive` also counts as removed), or when a child is shown or hidden, AppShell reprojects this layer without reassembly. Only direct children count: when `when` is wrapped in a `column()`, the outer `column` is always content, so hand the `when` directly to `.overlay(..)`.

Dialogs in the overlay go through `activate_overlay`, or declare `.open(..)` / `.model(signal)` on the dialog (see [Dialog](https://github.com/sena-nana/NanaUI/blob/main/docs/components/dialog.md#声明式开合)).

The shell only stacks this layer over the body.

Title text, window buttons and dragging belong on `AppTitleBar`.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `AppShell::new()` | — | All three slots empty |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| None | None | None |

## Slots

`.title_bar(id)`, `.body(id)` and `.overlay(id)` write an existing node.

In views, methods of the same name take a piece of view: `.title_bar(view)`, `.body(view)`, `.overlay(view)`.

The body slot fills the remaining height.

| Slot | Description |
| --- | --- |
| `.title_bar` | `.title_bar(id)` writes an existing node. In views, `.title_bar(view)` takes a piece of view |
| `.body` | `.body(id)` writes an existing node. In views, `.body(view)` takes a piece of view. The body slot fills the remaining height |
| `.overlay` | `.overlay(id)` writes an existing node. In views, `.overlay(view)` takes a piece of view. The overlay slot is absolutely positioned to fill the shell; if it is not an `OverlayHost`, it takes the pointer only when it has a child that is not hidden, and this is re-evaluated when children are added, removed, shown or hidden |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md) · [Workspace](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/workspace.md)
