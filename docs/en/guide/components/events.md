# Events

Controls emit typed events as they are interacted with. You catch them on the view. Events without data use `@`; events with data use `on:` or the corresponding method.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Column gap=8>
        <Button @activate={save}>"Save"</Button>
        <Slider min=0 max=1 step=0.05 on:RangeChanged={|event| applied.set(event.value)} />
    </Column>
}
```

```rust rust
use nana_ui::runtime::view::{button, column, slider};

column().gap(8).children((
    button("Save").on_activate(save),
    slider(0.0, 1.0, 0.05).on_change(|event| applied.set(event.value)),
))
```

:::

When `save` is already a function, `@activate={save}` passes the function in directly. Written as an expression, it expands to `.on_activate(move || { … })`. `Activate` has no payload, and the handler takes no arguments. Activation of buttons, icon buttons, `Chip`, and `ListItem` is this one event.

`on:RangeChanged={|event| …}` expands to `.on::<RangeChanged>`. The method of the same meaning on a slider is `.on_change`. The type of `e` is inferred and needs no annotation. In templates `@change={|event| …}` is the same thing; the compiler picks the event from the control table.

## Preview and commit are different events

While a slider is dragged, every visible value is a `RangeInput` (`.on_input`), suitable for live preview. `RangeChanged` (`.on_change`) is the commit: pointer release with a changed value, a keyboard step, accessibility `SetValue`, or `set_range_value`. A canceled drag does not commit.

Drag start and end have a separate `RangeDragging { dragging }`. Keyboard and accessibility steps are not drags.

Each edit in a text field is `TextChanged` (`.on_input`). Enter in a single-line field is `TextSubmitted` (`.on_submit`); it does not count as a submit while IME composition has not been committed. Checkboxes and switches emit `ToggleChanged` (`.on_change`), with the field `checked`. Number fields emit `NumberChanged`. Dropdown selects emit `SelectChanged`, whose payload is the selected `value`.

A misspelled event name, or listening on a control that has no such event, is reported with line and column when the template compiles.

## Changing the control or sending to the program

`.on` sees only the event. To change the control, emit another event, or hand a message to `RuntimeProgram`, use three parameters:

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::Activate;

view! {
    <Button
        on:Activate={|_button, _event: &Activate, cx| {
            cx.dispatch_program(msg);
        }}
    >
        "Start"
    </Button>
}
```

```rust rust
button("Start").on_cx(|_button, _event: &nana_ui::runtime::Activate, cx| {
    cx.dispatch_program(msg);
})
```

:::

In templates, writing `on:Activate={…}` as `|component, event, cx|` gives this one method. `cx.emit(event)` puts your own event in the queue. Cross-window work, GPU switches, and persistence go through `dispatch_program` and reach `update` in the next frame. Do not push the click itself into `update`.

These listeners are written on the node's `EventListeners`, the authoritative list in Runtime. Extension controls must not keep a second copy outside the type. See [Extension controls](https://github.com/sena-nana/NanaUI/blob/main/docs/guide/components/registry.md).

## Next steps

<div class="next-steps">
  <a class="next-step" href="/guide/components/v-model">
    <p class="next-step-link">Two-way binding</p>
    <p class="next-step-caption">Have input write straight back to a signal.</p>
  </a>
  <a class="next-step" href="/guide/components/slots">
    <p class="next-step-link">Named slots</p>
    <p class="next-step-caption">Hand a piece of view to dialogs and menus.</p>
  </a>
  <a class="next-step" href="/guide/essentials/events">
    <p class="next-step-link">Event basics</p>
    <p class="next-step-caption">Where on, on_activate, and on_cx go.</p>
  </a>
</div>
