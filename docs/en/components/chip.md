# Chip

`chip(label)` creates a compact selectable token, pill-shaped. `label` is the constructor argument and its field type is `Arc<str>`. The template tag is `<Chip>`.

## Basic usage

Child text and `label` both write this one field.

The fields are `label: Arc<str>`, `selected: bool`, `disabled: bool`. The event is `on_activate`, of type `Activate`. The handler takes no arguments. There is no `model`. Activation does not change `selected` itself.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Chip selected={picked} @activate={toggle}>"Rust"</Chip>
}
```

```rust rust
use nana_ui::runtime::view::chip;

chip("Rust").selected(picked).on_activate(toggle)
```

:::

`@activate={toggle}` passes `toggle` straight through when it is already a function. An expression expands to `.on_activate(move || { … })`, taking no arguments. When `selected` is true the background is Selected, otherwise Subtle. The application writes this state.

## Disabled

While `disabled` is true the chip emits no events and cannot be focused.

While disabled, neither the body nor the close button emits events.

## Dismissal

Dismissibility lives on the `Chip` component, not in the field table: `dismissible(true)`. This is a leaf compound, so it runs `assemble_chip` itself after a write, and once when the view is built, with nothing to remember separately. The view function has no `.dismissible`. For a constant switch use `.bind(|chip| chip.dismissible = true)`, one write that assembly follows.

Pressing the close button emits `ChipDismissed` and no longer sends `Activate` to this Chip. Dismissal is a request; the application removes it. Listen with `.on(|_: &ChipDismissed| …)`, or `on:ChipDismissed={…}` in templates. The default accessible name for close is "Remove"; change it on the component with `close_label`.

While the close button is pressed, hit testing stops on it and does not become an `Activate`.

## Versus buttons and badges

Against `Button`: a button is an action, with no token `selected` and no close. Against `StatusBadge`: a badge displays only; it neither activates nor closes.

## Accessibility

A Chip is not a list item. For accessibility the body is an activatable Button, with `selected` mapped to the selected state; the close button is another focusable Button.

## Size

The default size is `ControlSize::Small`. `Chip::size` changes it, and it is not in the field table.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `label` | `Arc<str>` | Constructor argument. Child text and `label` both write this one field |
| `selected` | `bool` | When true the background is Selected, otherwise Subtle. The application writes this state. There is no `model`. Activation does not change `selected` itself |
| `disabled` | `bool` | While true the chip emits no events and cannot be focused. While disabled, neither the body nor the close button emits events |
| `dismissible` | — | On the `Chip` component, not in the field table. `dismissible(true)`. The view function has no `.dismissible`. Write a constant switch once with `.bind`, and assembly follows it |
| `close_label` | — | The default accessible name for close is "Remove"; change it on the component with this |
| `size` | — | The default size is `ControlSize::Small`. `Chip::size` changes it, and it is not in the field table |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| `on_activate`, template `@activate` | Takes no arguments | The type is `Activate`. A function is passed as is. An expression expands to `.on_activate`, taking no arguments. Activation does not change `selected` itself |
| `.on`, template `on:ChipDismissed` | `&ChipDismissed` | Pressing the close button emits `ChipDismissed` and no longer sends `Activate` to this Chip. Dismissal is a request; the application removes it. While the close button is pressed, hit testing stops on it and does not become an `Activate` |

## Slots

| Slot | Description |
| --- | --- |
| None | Child text and `label` both write this one field |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
