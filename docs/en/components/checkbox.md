# Checkbox

`checkbox(label)` creates a checkbox. `label` is both the constructor argument and the accessible name. The template tag is `<Checkbox>`.

## Basic usage

Child text and `label` both write this one field.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Checkbox disabled={locked} v-model={done}>"Done"</Checkbox>
}
```

```rust rust
use nana_ui::runtime::view::checkbox;

checkbox("Done").disabled(locked).model(done)
```

:::

`v-model={done}` expands to `.model(done)`. `done` is a `Signal<bool>`. When the user clicks, the control flips `checked` itself and then emits `ToggleChanged`. The application does not have to write the check back before it shows. `model` writes `event.checked` back to that one signal.

## Listening

To listen to this change yourself, use `.on_change(|event: &ToggleChanged| …)`. In templates, write `on:ToggleChanged={|event: &ToggleChanged| …}`. When `save` is already a function, `@change={save}` passes it as is, and its parameter must be `&ToggleChanged`. Other expressions expand to `move |_| { … }` and the event is ignored. The three-parameter `on:ToggleChanged={|checkbox, event, cx| …}` expands to `.on_cx`.

`ToggleChanged` has only `checked`, which is the value after the toggle, not before.

## Vetoing

To veto this toggle, write the state you want back in the handler. Writing the value it already has does not cause an extra commit.

## Binding only checked

When you bind only `.checked(done)`, the signal drives the display. A user toggle changes the control's own `checked` and the signal does not change; the next write to the signal puts the control back to the signal's value. To have the signal follow, use `model`.

## Disabled

When `disabled` is true it does not receive the pointer and is not focusable.

`checked` and `disabled` can be constants, signals or closures. Closures are recomputed when signals change. Calling `.get()` while building the view gives a constant from that moment.

## Accessibility

The accessibility role is Checkbox, and the name is `label`.

The visible text and the accessible name are the same `label`.

## Layout

The indicator comes first and the label after, separated by one `indicator_gap` step, with the label taking the remaining width; without a label there is no gap. Both are parts published by layout (indicator `LeadingContent`, label `TextContent`), and size, wrapping width, paint position and text fragments all come from that one source. So a Checkbox with no width set is exactly as wide as the indicator, the gap and the label text, and when the label wraps it wraps at the label's own width. Checking and hovering only change painting, without relayout. In right-to-left direction the indicator stays on the left.

## Mixed state

The mixed state is `indeterminate` on the `Checkbox` component, and it takes priority over `checked` for both painting and accessibility. `invalid` draws the border in the danger color, and `size` changes the size. These two and the mixed state are not in the field table.

When only some items under a parent are checked, use `indeterminate` on the component; do not fake a partial check with `checked`.

## Properties

The fields are `label: String`, `checked: bool` and `disabled: bool`. The event is `on_change`, of type `ToggleChanged`. The handler receives `&ToggleChanged`. `model` binds `checked`.

| Property | Type | Description |
| --- | --- | --- |
| `label` | `String` | Constructor argument and the accessible name. Child text writes the same field. The visible text and the accessible name are the same `label` |
| `checked` | `bool` | `model` binds this field. Can be a constant, signal or closure. When only `.checked(done)` is bound, the signal drives the display and user toggles do not change the signal |
| `disabled` | `bool` | When true it does not receive the pointer and is not focusable. Can be a constant, signal or closure |
| `indeterminate` | — | On the `Checkbox` component, not in the field table. Takes priority over `checked` for both painting and accessibility. Use it when only some items under a parent are checked; do not fake a partial check with `checked` |
| `invalid` | — | Draws the border in the danger color. Not in the field table |
| `size` | — | Changes the size. Not in the field table |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| `on_change`, template `on:ToggleChanged` or `@change` | `&ToggleChanged` | The type is `ToggleChanged`. It has only `checked`, which is the value after the toggle, not before. A function is passed as is, and its parameter must be `&ToggleChanged`. Other expressions ignore the event. `model` writes `event.checked` back to the signal |
| `.on_cx`, template `on:ToggleChanged` with three parameters | `checkbox`, `event`, `cx` | `on:ToggleChanged={|checkbox, event, cx| …}` expands to `.on_cx` |

## Slots

| Slot | Description |
| --- | --- |
| None | Child text and `label` both write this one field |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
