# List

`List` is a container for a set of entries. Its accessibility role is a list, and screen readers read it as one. Which entries go inside is up to you.

The entry control is `ListItem`, whose page is described separately in the overview.

There is no `<List>` in the control table.

## Basic usage

`List::new()` has no child nodes.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::List;

view! {
    <Widget of={List::new().label("Recent files")}>
        <ListItem>"main.rs"</ListItem>
        <ListItem>"lib.rs"</ListItem>
    </Widget>
}
```

```rust rust
use nana_ui::runtime::view::{list_item, widget};
use nana_ui::runtime::List;

widget(List::new().label("Recent files")).children((
    list_item("main.rs"),
    list_item("lib.rs"),
))
```

:::

`list_item` is the view function for an entry; the label is both the constructor argument and the visible text.

Selection, disabling and a trailing accessory are written on the entry, not on this list.

## Clicks

The list itself does not take the pointer and is not focusable; clicks land on the entries you put inside.

## Workspace

When you do not need an IDE-style workspace, a column of `List`s or a single `AppShell` is enough.

Regions, Dock and splits are left to `Workspace`.

## Style and length

The list does not sort or virtualize; wrap a long list in `ScrollView`, or manage the window yourself with a virtual list.

The default style is an empty `NodeStyle`.

For spacing, padding or a background, write it in `.style`, or put the list inside `Stack::column`.

## Accessible name

The list's accessible name comes from `.label`; without it, screen readers announce only the role, "list".

## Properties

`.label` gives the whole list an accessible name.

`.style` replaces the whole node style.

| Property | Type | Description |
| --- | --- | --- |
| `.label` | — | Accessible name for the whole list. Without it, screen readers announce only the role, "list" |
| `.style` | — | Replaces the whole node style. The default style is an empty `NodeStyle` |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| None | None | The list itself does not take the pointer and is not focusable |

## Slots

| Slot | Description |
| --- | --- |
| None | None |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
