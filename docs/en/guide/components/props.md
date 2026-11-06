# Props

A field accepts three kinds of value: a constant, a signal (or computed), or a closure. A constant is written once when the node is built. Signals and closures follow changes afterward.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Button disabled={move || draft.with(|text| text.is_empty())} loading={pending}>
        "Save"
    </Button>
}
```

```rust rust
use nana_ui::runtime::view::button;

button("Save")
    .disabled(move || draft.with(|text| text.is_empty()))
    .loading(pending)
```

:::

`pending` is a `Signal<bool>` or `Computed<bool>` handed in directly; only "signal id + this field's write function" is recorded, with no closure. The `disabled` part needs to look at the contents of `draft` every time, so it is a closure, boxed once. Writing `loading={false}` or `.loading(false)` gives a constant, with no cost after building.

These three kinds are the only ones a template can tell apart statically. Literals are constants. Paths and fields (`pending`, `row.title`) are passed in as is. Any other expression is wrapped as `move || expression` and recomputed when a signal it read changes. A `disabled={|| …}` you write yourself is kept as is.

## Values read during construction are constants

Writing `count.get()` in a view function gives the value at the moment the tree was built, and it does not follow changes. This is the same as reading `.value` in Vue's `setup`. To follow changes, hand the signal itself or a closure to the prop; do not read it out first.

:::api

```rust view
use nana_ui::runtime::view;

let count = signal(0u64);
view! {
    <Text>"{count}"</Text>
}
```

```rust rust
let count = signal(0u64);
// This line is always the number from tree-building time
text(count.get().to_string());
// This line follows count
text(move || count.get().to_string());
```

:::

`text!("{count}")` is the same thing: signals read by the interpolation become dependencies.

## Fields outside the control table

The setters of functions like `button`, `slider`, and `text_input` are generated from the control table, each corresponding to one `FieldWrite`. A control not in the table is written as `widget(component)` and changed with `.bind(|component| …)`, or with `.prop::<T, W>(value)` after you implement `FieldWrite`.

`.bind` cannot tell which field changed, so it always treats a run as "changed" and goes through copy and projection. Use a single setter when you can: dynamic fields on the same node share one effect, compare field by field, and stop when everything is equal, with no copy, no projection, and no commit.

`Computed<T>` uses the same direct binding as signals. A computed makes the fields that read it rerun only when its result differs from last time.

Each closure is boxed once. In the function style, two closures on the same node are two boxes. Only when the whole template is visible do closures on the same node have a chance to merge into one effect.

## Next steps

<div class="next-steps">
  <a class="next-step" href="/guide/components/events">
    <p class="next-step-link">Events</p>
    <p class="next-step-caption">How to catch activation, input, and commit.</p>
  </a>
  <a class="next-step" href="/guide/components/v-model">
    <p class="next-step-link">Two-way binding</p>
    <p class="next-step-caption">Which fields write back to a signal.</p>
  </a>
  <a class="next-step" href="/guide/essentials/computed">
    <p class="next-step-link">Computed</p>
    <p class="next-step-caption">Derive a bindable value from signals.</p>
  </a>
</div>
