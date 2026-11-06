# Computed values

When a value is derived entirely from other signals, use `computed`. It belongs to the current scope and is computed once on creation. Afterwards, when a dependency changes it is only marked dirty, and it recomputes when next read.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::{IntoView, computed, signal};

fn summary() -> impl IntoView {
    let count = signal(0u64);
    let label = computed(move || match count.get() {
        0 => "None yet".to_string(),
        n => format!("{n} items"),
    });
    view! {
        <Column gap=8>
            <Text>{label}</Text>
            <Button @activate={count.update(|c| *c += 1)}>"Add"</Button>
        </Column>
    }
}
```

```rust rust
use nana_ui::runtime::view::*;

fn summary() -> impl IntoView {
    let count = signal(0u64);
    let label = computed(move || match count.get() {
        0 => "None yet".to_string(),
        n => format!("{n} items"),
    });
    column().gap(8).children((
        text(label),
        button("Add").on_activate(move || count.update(|c| *c += 1)),
    ))
}
```

:::

`computed` requires `T: PartialEq + 'static`. When a recomputed result equals the previous one, bindings that read it do not run again. So when `computed(|| count.get() % 2)` goes from `1` to `3`, whatever displays that parity does not move.

`Computed<T>` is also `Copy`. Read it with `get` or `with`, which track just like a signal. It can be passed to a property, and read by another `computed`. A read wrapped in `untrack` inside the closure does not count as a dependency.

To merely format a signal, without caching it separately, use `Signal::map`. `text(count.map(|n| format!("{n} items")))` gives a closure property computed fresh each time the binding runs, without the `PartialEq` cut-off.

::: warning
Calling `label.get()` directly while constructing the view writes the string of that moment into the node, which no longer follows changes. Pass `label` itself to `text`, or pass a closure.
:::

To do something when a dependency changes, rather than derive a value, use `watch_effect` from [Watchers](https://github.com/sena-nana/NanaUI/blob/main/docs/guide/essentials/watch.md).

## Relation to bindings

The dynamic fields on a node share one effect. When any input changes, they are first compared by field against the retained view. If all are equal it stops, copying nothing and committing nothing. When `computed` yields the same value, not even that step begins.

Refresh happens on read, not on write. A `computed` nobody reads stays at the dirty mark.

## Next steps

<div class="next-steps">
  <a class="next-step" href="/guide/essentials/watch">
    <p class="next-step-link">Watchers</p>
    <p class="next-step-caption">Run a side effect when a dependency changes.</p>
  </a>
  <a class="next-step" href="/guide/essentials/class-and-style">
    <p class="next-step-link">Classes and styles</p>
    <p class="next-step-caption">Class names, build-time stylesheets, and surfaces that follow the theme.</p>
  </a>
  <a class="next-step" href="/guide/essentials/reactivity">
    <p class="next-step-link">Reactivity</p>
    <p class="next-step-caption">How a signal itself is read and written.</p>
  </a>
</div>
