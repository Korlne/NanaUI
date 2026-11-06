# Button

Emits `Activate` when pressed. The visible label is also the accessible name by default. When several buttons on one screen have the same visible text, use `accessible_name` to spell out each one's action. When you want a single icon, use [IconButton](https://github.com/sena-nana/NanaUI/blob/main/docs/components/icon-button.md).

The template tag is `<Button>`. Child text and `label` both write this one field.

## Basic usage

Here is a save button.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Button disabled={pending} @activate={save}>"Save"</Button>
}
```

```rust rust
use nana_ui::runtime::view::button;

button("Save").disabled(pending).on_activate(save)
```

:::

When `save` is already a function, `@activate={save}` passes the function straight through. When written as an expression, it expands to `.on_activate(move || { … })`.

## Disabled and loading

`disabled` and `loading` can be constants, signals or closures.

## Changing the control or sending to the program

When you need a `ViewContext`, use `.on_cx(|_button, _event: &Activate, cx| …)`. In templates, write it as `on:Activate={…}` with three parameters.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `label` | `String` | Constructor argument and the visible text. When `accessible_name` is not written, it is also the accessible name. Child text writes the same field |
| `accessible_name` | `String` | Accessible name. An empty string falls back to `label`. The visible text stays `label`, so a "Settings" in a row can be read as "Settings, standalone capture window" |
| `disabled` | `bool` | Constant, signal or closure |
| `loading` | `bool` | Constant, signal or closure |
| `has_popup` | `bool` | A button that pops up a menu (`aria-haspopup="menu"`): screen readers report it as a menu button; while it has focus, ArrowUp / ArrowDown emit a `SecondaryPress` with `keyboard: true`, the same as the `ContextMenu` key, and you open the menu. See menu buttons in [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md) |
| `Button::content_align` | `TextHorizontalAlignment` | On the component. Places the icon-and-text group at the start, center (default) or end of the content area; buttons that look like list rows use `Start` |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| `on_activate`, template `@activate` | Takes no arguments | The type is `Activate`. A function is passed as is; an expression is wrapped in `move \|\| { … }` |
| `.on_cx`, template `on:Activate` with three parameters | `&Activate` and `ViewContext` | To change the control, or for `cx.dispatch_program` |

## Slots

There are no named slots. Child text writes `label`.

In Rust, children placed in the button with `.children(...)` (custom-drawn content such as keycaps and color swatches) share one content area with the text: centered vertically and arranged horizontally by `content_align`. When the style itself sets `align_items` or `justify_content`, the style wins.

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
