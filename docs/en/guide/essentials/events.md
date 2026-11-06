# Events

There are two kinds of listener on a control. One takes no argument, such as `Button`'s `on_activate`. One takes a reference to the event, such as `TextInput`'s `on_input` whose argument is `&TextChanged`. Which one applies is fixed in the control table, not annotated on your closure.

`save` is a function with no arguments. `draft` is a `Signal<String>`. `Msg` is your own `RuntimeProgram::Message`, and `Start` is just one of its variants.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::{Activate, TextChanged};

view! {
    <Column gap=8>
        <Button @activate={save}>"Save"</Button>
        <Button on:Activate={|_button, _event: &Activate, cx| cx.dispatch_program_all(Msg::Start)}>"Start"</Button>
        <TextInput @input={|event: &TextChanged| draft.set(event.value.to_string())} />
    </Column>
}
```

```rust rust
use nana_ui::runtime::view::{button, column, text_input};
use nana_ui::runtime::{Activate, TextChanged};

column().gap(8).children((
    button("Save").on_activate(save),
    button("Start").on_cx(|_button, _event: &Activate, cx| {
        cx.dispatch_program_all(Msg::Start)
    }),
    text_input().on_input(|event: &TextChanged| draft.set(event.value.to_string())),
))
```

:::

## Takes no argument

For methods the control table marks as `on { … }`, the closure is `FnMut() + Send + 'static`. The event value is dropped.

| Control | Method | Event dropped |
| --- | --- | --- |
| `Button` | `on_activate` | `Activate` |
| `ListItem` | `on_activate` | `Activate` |
| `IconButton` | `on_activate` | `Activate` |
| `Chip` | `on_activate` | `Activate` |

`@activate={save}` passes the function straight in when `save` is already a function. An expression such as `@activate={count.update(|c| *c += 1)}` is wrapped as `move || { …; }`.

`Activate` has no fields of its own. You do not need it to know the button was activated.

## Takes &Event

For methods marked `with { … }`, the closure is `FnMut(&Event) + Send + 'static`.

| Control | Method | Event |
| --- | --- | --- |
| `TextInput` | `on_input` | `&TextChanged` |
| `TextInput` | `on_submit` | `&TextSubmitted` |
| `TextArea` | `on_input` | `&TextChanged` |
| `Checkbox`, `Switch` | `on_change` | `&ToggleChanged` |
| `Slider` | `on_input` | `&RangeInput` |
| `Slider` | `on_change` | `&RangeChanged` |
| `RangeSpan` | `on_input` | `&RangeSpanInput` |
| `RangeSpan` | `on_change` | `&RangeSpanChanged` |
| `Curve` | `on_input` | `&CurveInput` |
| `Curve` | `on_change` | `&CurveChanged` |
| `NumberInput` | `on_change` | `&NumberChanged` |
| `Select` | `on_change` | `&SelectChanged` |
| `Select` | `on_toggle` | `&SelectToggled` |

`TextChanged` has `value` and `selection`. `TextSubmitted`, `RangeInput`, `RangeChanged` and `NumberChanged` all have `value`. `ToggleChanged` has `checked`. `SelectChanged` has `value`, and `SelectToggled` has `open`. `RangeSpanInput` and `RangeSpanChanged` have `low` and `high`. `CurveInput` and `CurveChanged` have `point` (the point that moved, `None` when the whole set is replaced) and `values`.

In a template, `@input={|event: &TextChanged| …}` passes the closure to `on_input` unchanged. Written as an ordinary expression, it is wrapped as `move |_| { …; }` and the event is ignored. To read `event.value`, write a closure with the parameter.

`Slider`'s `on_input` is every value during the drag (`RangeInput`). `on_change` is the committed value (`RangeChanged`): releasing, a keyboard step, or an accessibility value set. A cancelled drag does not emit `RangeChanged`.

For an event not in the control table, use `on`. The closure receives `&E`. Text truncation, for example, listens for `TextClamped`, which has a `clamped` field; container size listens for `SizeChanged`, which has `width` and `height`.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::TextClamped;

view! {
    <Text on:TextClamped={move |event: &TextClamped| clamped.set(event.clamped)}>{body}</Text>
}
```

```rust rust
use nana_ui::runtime::view::text;
use nana_ui::runtime::TextClamped;

text(body).on(move |event: &TextClamped| clamped.set(event.clamped))
```

:::

## Changing the control, or sending a message to the program

`on` and `on_activate` get neither the component nor a `ViewContext`. Use `on_cx`. The closure is `FnMut(&mut Component, &Event, &mut ViewContext)`. Writing `on:Activate={…}` with three parameters in a template expands to it. Two parameters or fewer expand to `.on::<Activate>`.

`cx.dispatch_program(msg)` merges by the message's Rust type, keeping only the last of a type in one frame. `cx.dispatch_program_all(msg)` keeps every message in dispatch order, and they enter `RuntimeProgram::update` on the next frame. An application's `Message` is usually an enum, so all of them are the same type.

::: warning
When `Message` is an enum, do not use `dispatch_program`. Two clicks in the same frame merge into the last one and the earlier one is lost. Use `dispatch_program_all`.
:::

Event handlers are not inside the view scope. Signals created there have no owner, and `use_context` cannot read anything there either.

## Next steps

<div class="next-steps">
  <a class="next-step" href="/guide/essentials/forms">
    <p class="next-step-link">Form bindings</p>
    <p class="next-step-caption">How v-model writes input back to a signal.</p>
  </a>
  <a class="next-step" href="/guide/essentials/application">
    <p class="next-step-link">Creating an app</p>
    <p class="next-step-caption">What update handles is host messages, not every click.</p>
  </a>
</div>
