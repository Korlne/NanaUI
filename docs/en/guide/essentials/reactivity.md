# Reactivity

State lives in signals. The tree is built once at mount, and after that each binding changes only the field it writes. There is no whole-tree repaint and no diff.

Both styles call `signal`.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::{IntoView, signal};

fn counter() -> impl IntoView {
    let count = signal(0u64);
    view! {
        <Column gap=8>
            <Text>"Count {count}"</Text>
            <Button @activate={count.update(|c| *c += 1)}>"Increment"</Button>
        </Column>
    }
}
```

```rust rust
use nana_ui::runtime::view::*;

fn counter() -> impl IntoView {
    let count = signal(0u64);
    column().gap(8).children((
        text!("Count {count}"),
        button("Increment").on_activate(move || count.update(|c| *c += 1)),
    ))
}
```

:::

`signal(0u64)` gives a `Signal<u64>`. It is a `Copy` id, and the value lives on the thread that created it. Using it on another thread panics.

## Reading and writing

`get` clones the current value and records this read. `with` lends a reference only, which suits a check that needs no clone, such as `list.with(Vec::is_empty)`. When `T` is not `Clone` there is no `get`; use `with`.

`set` installs a new value and notifies, even when the new value equals the old one. `update` changes in place, then notifies. A closure passed to `update` must not read the same signal.

When `T: PartialEq`, `set_if_changed(value)` compares without tracking first, does nothing when the values are equal, and otherwise calls `set`, returning whether it wrote. Use it for writes that often bring back the same value, such as background results and size callbacks: bindings do not run a wasted round, and calling it inside an effect does not make the effect subscribe to the signal. `try_set_if_changed` returns `false` when the signal is gone instead of panicking, which suits writing back to a row that may already be collected.

`get_untracked` and `with_untracked` read without establishing a dependency. `untrack(|| …)` wraps a whole block, and reads inside it are not recorded. After the scope is collected, reading or writing reports `runtime.reactive.disposed_access` and panics. `try_update` returns `false` when the signal is gone instead of panicking.

`set` and `update` only queue subscribers; they do not change the tree at once. Input routing calls `flush_reactive` at the end of every event, and each frame flushes once more at its start. When you change a signal in `RuntimeProgram::update`, windows with pending bindings are also asked to repaint.

A property takes a constant, a `Signal<T>`, a `Computed<T>`, or an `Fn() -> T`. A constant is written when the node is built. A signal binds directly, without boxing a closure. A signal written on its own in an interpolation, such as `"Count {count}"`, becomes a dependency when it is read.

::: warning
Calling `count.get()` while constructing a view reads the constant of that moment and stops following changes. To follow changes, pass the signal itself, or pass a closure.
:::

In the function style, `.label(count.get())` is exactly that constant. In templates, `{count.get()}` is an expression and is wrapped as `move || count.get()`, which does follow changes. Pass the signal whenever you can.

A signal created in a mount closure, an `each` row or a `when` branch belongs to that scope, and is dropped when the scope is collected. Signals created outside those scopes have no owner and stay until the thread ends. Signals a list row needs go inside the row's view.

## Nested state

When anything inside `Signal<App>` changes, every place that read it reruns. For nested state use `store`, which tracks each path separately. See [Store](https://github.com/sena-nana/NanaUI/blob/main/docs/guide/scaling/store.md).

To hand a value down to a lower view, use [Provide / inject](https://github.com/sena-nana/NanaUI/blob/main/docs/guide/components/provide.md). To derive from a signal, use [Computed values](https://github.com/sena-nana/NanaUI/blob/main/docs/guide/essentials/computed.md). To do one thing when a dependency changes, use [Watchers](https://github.com/sena-nana/NanaUI/blob/main/docs/guide/essentials/watch.md).

## Next steps

<div class="next-steps">
  <a class="next-step" href="/guide/essentials/computed">
    <p class="next-step-link">Computed values</p>
    <p class="next-step-caption">Derive a value from a signal with computed.</p>
  </a>
  <a class="next-step" href="/guide/essentials/watch">
    <p class="next-step-link">Watchers</p>
    <p class="next-step-caption">Do one thing when a dependency changes.</p>
  </a>
  <a class="next-step" href="/reference/reactive-view">
    <p class="next-step-link">Declarative views</p>
    <p class="next-step-caption">The full contract for signals, Store and flushing.</p>
  </a>
</div>
