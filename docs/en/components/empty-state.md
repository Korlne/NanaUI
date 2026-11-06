# EmptyState

`empty_state(title)` creates an empty state. `title` is a constructor argument with field type `Arc<str>`. It is an explanatory block, not a list item and not a badge.

The template tag is `<EmptyState>`. Child text and `title` both write to this one field.

## Basic usage

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::Icon;

view! {
    <EmptyState message="Drag files here" icon={Icon::Folder}>
        "No files yet"
        <template #action>
            <Button @activate={import}>"Import"</Button>
        </template>
    </EmptyState>
}
```

```rust rust
use nana_ui::runtime::view::{button, empty_state};
use nana_ui::runtime::Icon;

empty_state("No files yet")
    .message("Drag files here")
    .icon(Icon::Folder)
    .action(button("Import").on_activate(import))
```

:::

Icons use catalog constants such as `Icon::Folder`. If you do not need an icon, do not write `icon`. `message` is optional.

## Compact

`compact` defaults to false: content is centered on the horizontal axis. `compact(true)` aligns it to the start edge instead. The width is still 100% of the parent.

## Empty lists

When a list is completely empty, this is often used with `v-if`. For content that appears and disappears with data, see [View authoring](../guide/essentials/view.md).

## Layout

The icon, title, and message form one column at the top of the content area inside the padding (default `xxxl` / `page`, `sm` / `md` when compact; any side set in the style uses the style's value). Each item is centered by default and start-aligned when compact. The title and message are as wide as their text, up to the content area's width, and the content area's width is their line-breaking width. These are parts published by layout (icon `LeadingContent`, title `Header`, message `Detail`); painting, chrome clipping, and accessibility all read this one copy. The action button is an ordinary child node. Layout places the column, plus the gap to the button, ahead of it, so the button starts below the column; the padding is only the declared amount and does not include the column's height. In right-to-left direction, compact mode still aligns left and is not mirrored.

## Properties

The fields are `title: Arc<str>`, `message: Option<Arc<str>>`, `icon: Option<Icon>`, and `compact: bool`. There are no events and no `model`. One action can sit below the title, set with `.action(…)`; it is not a field.

| Property | Type | Description |
| --- | --- | --- |
| `title` | `Arc<str>` | Constructor argument. Child text and `title` both write to this one field |
| `message` | `Option<Arc<str>>` | Optional |
| `icon` | `Option<Icon>` | Catalog constant such as `Icon::Folder`. Omit it if you do not need one |
| `compact` | `bool` | Defaults to false: content is centered on the horizontal axis. `compact(true)` aligns to the start edge. The width is still 100% of the parent |

## Events

No events.

| Event | Payload | Description |
| --- | --- | --- |
| None | None | The action's click is handled on that button; the empty state emits no event of its own |

## Slots

`.action` holds the single action below the title and message, such as a retry button. In a template it is `<template #action>`. This slot takes exactly one root node. The action's click is handled on that button; the empty state emits no event of its own.

| Slot | Description |
| --- | --- |
| `#action` / `.action` | The single action below the title and message; not a field. Takes exactly one root node |

## See also

[Overview](index.md) · [Controls](../reference/components.md)
