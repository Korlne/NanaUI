# Form bindings

To display a signal and write the user's edits back at the same time, use `v-model`. It expands to `.model(signal)`. The signal's type is decided by the control and cannot be swapped for another.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Column gap=8>
        <TextInput placeholder="Title" v-model={title} />
        <Checkbox label="Done" v-model={done} />
        <Slider min=0 max=1 step=0.05 v-model={volume} />
    </Column>
}
```

```rust rust
use nana_ui::runtime::view::{checkbox, column, slider, text_input};

column().gap(8).children((
    text_input().placeholder("Title").model(title),
    checkbox("Done").model(done),
    slider(0.0, 1.0, 0.05).model(volume),
))
```

:::

`title` is `Signal<String>`, `done` is `Signal<bool>` and `volume` is `Signal<f64>`. `v-model={title}` passes the signal as is and does not wrap a closure.

`.model` does two things: it drives that field on the control with the signal, and it listens once to the change event, `set`ting the event's value back to the signal.

| Control | Signal | Event listened to for the write |
| --- | --- | --- |
| `TextInput`, `TextArea` | `Signal<String>` | `TextChanged`, taking `event.value.to_string()` |
| `Checkbox`, `Switch` | `Signal<bool>` | `ToggleChanged`, taking `event.checked` |
| `Slider` | `Signal<f64>` | `RangeInput`, taking `event.value` |
| `RangeSpan` | `Signal<(f64, f64)>` | `RangeSpanInput`, taking `(event.low, event.high)` |
| `Curve` | `Signal<Vec<f64>>` | `CurveInput`, taking `event.values` |
| `NumberInput` | `Signal<f64>` | `NumberChanged`, taking `event.value` |
| `Select` | `Signal<Option<Arc<str>>>` | `SelectChanged`, taking `Some(event.value.clone())` |

`Button` has no `v-model`. Writing one in a template reports "this tag has no `v-model`".

A slider's `.model` listens to `RangeInput`, so every value during a drag is written back to the signal. The value committed on release is `RangeChanged`, which is `on_change`, not `v-model`.

A text write goes through the field's `text_state`: when the new string equals the one already in the control, the text is not replaced and the caret and selection stay where they were. So writing one input back to the signal does not throw the caret back to the start.

`Select` also needs options. `options` is a `Vec<SelectOption>`, each one made with `SelectOption::new(value, label)`, and each can take `.disabled(true)`. Disabled options are still shown.

:::api

```rust view
use std::sync::Arc;

use nana_ui::runtime::view;
use nana_ui::runtime::view::signal;
use nana_ui::runtime::SelectOption;

let picked = signal(None::<Arc<str>>);
let options = vec![
    SelectOption::new("day", "Day"),
    SelectOption::new("night", "Night"),
];
view! {
    <Select placeholder="Choose one" options={options} v-model={picked} />
}
```

```rust rust
use std::sync::Arc;

use nana_ui::runtime::view::{select, signal};
use nana_ui::runtime::SelectOption;

let picked = signal(None::<Arc<str>>);
select()
    .placeholder("Choose one")
    .options(vec![
        SelectOption::new("day", "Day"),
        SelectOption::new("night", "Night"),
    ])
    .model(picked)
```

:::

In templates it is likewise `options={options}` plus `v-model={picked}`. `placeholder` can be a string, which becomes an `Option<Arc<str>>`.

`.model` adds one listener on top of the ones already there. If you also write `on_input`, both run, and the latter does not replace the former. To only read changes without writing back to a signal, do not use `v-model`; use `on_input` or `on_change` from [Events](https://github.com/sena-nana/NanaUI/blob/main/docs/guide/essentials/events.md).

::: warning
Writing `.value(title.get())` at construction puts in only the string of that moment. When the signal changes later, the field does not follow; when the user types, the signal does not update. Use `.model(title)` for two-way binding.
:::

A number field's `on_change` gives `NumberChanged` when the committed value changes. For read-only, use `.read_only(true)`, a field that both `TextArea` and `NumberInput` have. For disabled, use `.disabled(…)`, which all the controls above have.

## Next steps

<div class="next-steps">
  <a class="next-step" href="/guide/essentials/refs">
    <p class="next-step-link">Handles</p>
    <p class="next-step-caption">Reach the built node by type after mount.</p>
  </a>
  <a class="next-step" href="/guide/essentials/events">
    <p class="next-step-link">Events</p>
    <p class="next-step-caption">Wiring on_input and on_change when nothing writes back to a signal.</p>
  </a>
</div>
