# Named slots

Composite controls take your content as nodes. A slot is a method: build that piece of view first, then hand its root node to the control. For controls that have a tag in the control table, templates use `<template #name>`. For those without a tag, do not invent one.

Empty states have `<EmptyState>`. `#action` expands to `.action`. Hyphens become underscores: `#close-action` is `.close_action`.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <EmptyState title="No tasks" message="Add one to get started.">
        <template #action>
            <Button @activate={add}>"Add"</Button>
        </template>
    </EmptyState>
}
```

```rust rust
use nana_ui::runtime::view::{button, empty_state};

empty_state("No tasks")
    .message("Add one to get started.")
    .action(button("Add").on_activate(add))
```

:::

`#default` is just ordinary children. A slot must have exactly one root; otherwise this mount fails with `InvalidInput` and leaves nothing behind. The slot and the control are built in the same commit. Signals created inside belong to this mount and are reclaimed when it unmounts.

A slot's root does not change while the view is alive. When the content inside needs to change, write `when` / `each` / `dynamic` inside the slot.

## Dialogs have no `<Dialog>`

`Dialog` and `Drawer` are not in the control table. Both authoring styles are `widget(Dialog::new(title))` followed by `.body`, `.footer`, and `.close_action`. Toggling swaps the UI inside these slots, not the outer line. `Drawer::new(title)` has the same three methods. The title row also has `.title_icon` and `.title_content`: the latter is a piece of view that replaces the title text; see [Dialog](https://github.com/sena-nana/NanaUI/blob/main/docs/components/dialog.md#标题内容).

:::api

```rust view
use nana_ui::runtime::view::widget;
use nana_ui::runtime::Dialog;

widget(Dialog::new("Rename"))
    .body(view! {
        <Column gap=8>
            <TextInput placeholder="Name" v-model={name} />
        </Column>
    })
    .footer(view! {
        <Row gap=8>
            <Button @activate={cancel}>"Cancel"</Button>
            <Button @activate={save}>"Save"</Button>
        </Row>
    })
    .close_action(view! {
        <Button @activate={cancel}>"Close"</Button>
    })
```

```rust rust
use nana_ui::runtime::view::{button, column, row, text_input, widget};
use nana_ui::runtime::Dialog;

widget(Dialog::new("Rename"))
    .body(column().gap(8).children((
        text_input().placeholder("Name").model(name),
    )))
    .footer(row().gap(8).children((
        button("Cancel").on_activate(cancel),
        button("Save").on_activate(save),
    )))
    .close_action(button("Close").on_activate(cancel))
```

:::

Slots are built before the panel, so `.initial_focus_on(entity_ref)` can focus a control in a slot when the dialog opens. If the target has not been built yet, the panel keeps its own focus. `Drawer` and `ConfirmDialog` have this method too.

The slots of `ConfirmDialog::new(title, description)` are `.title_icon`, `.body`, `.footer`, `.close_action`, `.cancel`, `.secondary`, and `.confirm`. Cancel and confirm buttons not given are generated from `cancel_label` / `confirm_label`, defaulting to "Cancel" and "Confirm".

## Forms, transport bars, and menus

These types have no template tags of their own either. On the template side the outer element is `<Widget of={...}>`, and named slots are `<template #name>`. Do not invent tags like `<FormField>`.

| Control | Method | What goes in |
| --- | --- | --- |
| `FormField::new(label)` | `.control` | The control below the label and above the description |
| `MediaTransportBar::new()` | `.leading` | After play: next, skip |
| | `.trailing` | Before volume: speed, quality, subtitles |
| | `.secondary` | A second row below the progress bar; collapses when empty |
| | `.settings` | Items in the settings menu |
| `Popover::new()` | `.trigger` | Icon, text, and count on the trigger |
| `ActionMenu::new()` | `.trigger` | Same as above |

`Popover` and `ActionMenu` are still the control themselves: press, focus, Enter / Space, and the anchor all belong to them. Do not put another pressable control inside the trigger. `.children(..)` are the items on the panel.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::FormField;

view! {
    <Widget of={FormField::new("Name")}>
        <template #control>
            <TextInput label="Name" />
        </template>
    </Widget>
}
```

```rust rust
use nana_ui::runtime::view::{text, text_input, widget};
use nana_ui::runtime::FormField;

widget(FormField::new("Name")).control(text_input().label("Name"))
```

:::

Shell regions (`DesktopShell`, `Workspace`, and so on) are also named slots; for the method names see the slot section of [Declarative views](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/reactive-view.md).

## Next steps

<div class="next-steps">
  <a class="next-step" href="/guide/components/async">
    <p class="next-step-link">Async</p>
    <p class="next-step-caption">How a resource loads, and what shows before the first completion.</p>
  </a>
  <a class="next-step" href="/guide/built-ins/transition">
    <p class="next-step-link">Transition</p>
    <p class="next-step-caption">How branches and list rows enter and leave.</p>
  </a>
  <a class="next-step" href="/components/dialog">
    <p class="next-step-link">Dialog</p>
    <p class="next-step-caption">Title, size, and close policy.</p>
  </a>
</div>
