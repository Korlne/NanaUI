# Watchers

To do something when dependencies change, rather than derive a value, use `watch_effect`. For derivation, use [computed values](computed.md).

It runs once right away and records the signals it reads. When those signals change later, it runs again in the next flush of any context. It returns an `Effect`; there is no separate `Watch` type. It does not receive old and new values; keep a copy yourself if you need to compare.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::{IntoView, signal, watch_effect};

fn echoed_label() -> impl IntoView {
    let count = signal(0u64);
    let echoed = signal(String::new());
    watch_effect(move || {
        echoed.set(count.get().to_string());
    });
    view! {
        <Column gap=8>
            <Text>{echoed}</Text>
            <Button @activate={count.update(|c| *c += 1)}>"Increment"</Button>
        </Column>
    }
}
```

```rust rust
use nana_ui::runtime::view::*;

fn echoed_label() -> impl IntoView {
    let count = signal(0u64);
    let echoed = signal(String::new());
    watch_effect(move || {
        echoed.set(count.get().to_string());
    });
    column().gap(8).children((
        text(echoed),
        button("Increment").on_activate(move || count.update(|c| *c += 1)),
    ))
}
```

:::

This effect reads `count` and writes a different signal, `echoed`. When it is created inside a mount closure, a row, or a branch, it belongs to that scope and stops when the scope is reclaimed. To stop it early, call `effect.dispose()`. Reclaiming the scope also stops it.

If effects keep triggering each other for more than 64 rounds, Runtime reports `runtime.reactive.did_not_settle` and drops the rest of the queue. Do not write, inside a `watch_effect`, a signal that the same effect reads. See [Diagnostic codes](../../reference/errors.md) for the fix.

The `watch_effect` closure is `FnMut`, so it may mutate state it captures.

## Relation to bindings

The dynamic fields on a node share one effect. When any input changes, each field is first compared against the retained view. If everything is equal it stops there: no copy, no commit. When a `computed` produces the same value, that step does not even start.

Refresh happens on read, not on write. A `computed` nobody reads just stays marked dirty.

## Next steps

<div class="next-steps">
  <a class="next-step" href="/guide/essentials/class-and-style">
    <p class="next-step-link">Classes and styles</p>
    <p class="next-step-caption">Class names, build-time stylesheets, and surfaces that follow the theme.</p>
  </a>
  <a class="next-step" href="/guide/essentials/computed">
    <p class="next-step-link">Computed values</p>
    <p class="next-step-caption">Derive a value from other signals.</p>
  </a>
</div>
