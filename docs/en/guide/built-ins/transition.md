# Transition

`when` and `each` can carry an enter/leave transition. A new branch or new row plays from the given opacity and transform to the node's own values. Removed ones are not destroyed immediately. This is Vue's `<Transition>` and `<TransitionGroup>`. `TransitionGroup` does not get a page of its own.

Rows already in the tree at mount time do not play an enter animation.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Transition name="fade" duration="150">
        <Text v-if={open}>"Open"</Text>
    </Transition>
}
```

```rust rust
use std::time::Duration;

use nana_ui::runtime::view::{text, when, Transition};

when(open, || text("Open")).transition(Transition::fade(Duration::from_millis(150)))
```

:::

`<Transition>` wraps one `v-if` / `v-else-if` / `v-else` chain. `name` can be `fade`, `slide-up`, `slide-down`, `slide-left`, `slide-right`, or `scale`; omitted, it is `fade`. `duration` is in milliseconds; omitted, it is 150. `transition={value}` passes a `Transition` directly; do not also write `name` then.

The presets correspond to `Transition::fade`, `slide(dx, dy, duration)`, and `scale(ratio, duration)`. `slide-up` is `slide(0.0, 12.0, duration)`; `scale` is `scale(0.95, duration)`. To build your own, use `Transition::new().enter(Presence::new(duration).opacity(0.0).translate(0.0, 8.0)).leave(…)`. `.ease(easing)` sets one easing for the whole transition; the default is `Easing::EaseOutCubic`. When you only need one half, use `without_enter()` or `without_leave()`.

## Leaving in place

A deleted row or branch has its scope reclaimed immediately and no longer follows the data. The node stays in place, takes no part in hit testing, loses focus, and is destroyed only after the animation finishes.

The old and new branches of a `when` exist at the same time: the old one leaves in place while the new one enters after it. With `.moves`, the new branch slides up smoothly when the old one disappears. Vue's `mode="out-in"` (leave first, then enter) is not available yet.

Animation runs on the node's compositor track and is not written back to the logical style. The end of a leave is driven by the completion event of `advance_animations`. Moves start after layout and before extraction in the same frame, so nothing flashes to the new position first.

## TransitionGroup

For lists use `<TransitionGroup>`, or add the same `.transition` to `each`. `move` is the slide duration for position changes (FLIP). Insertion, deletion, and reordering all count, as does the moment leaving rows are finally destroyed and later rows fill the gap.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <TransitionGroup duration="180" move="200">
        <Text v-for={item in items} key={item.id}>{item.title}</Text>
    </TransitionGroup>
}
```

```rust rust
use std::time::Duration;

use nana_ui::runtime::view::{each, text, Transition};

each(items, |item| item.id, |item| text(item.title.clone())).transition(
    Transition::fade(Duration::from_millis(180)).moves(Duration::from_millis(200)),
)
```

:::

`<Transition>` and `<TransitionGroup>` are interchangeable: one wraps a chain, the other wraps a `v-for`; both generate `.transition`, plus `.moves` when `move` is present. A `Store`'s `keyed(..).each` can take `.transition` the same way.

Virtual lists (`each_virtual`) are not supported. Rows scrolled out of the viewport must be reclaimed immediately anyway, and wrapping one in `<Transition>` is an error when the template compiles.

Class names can be written on both tags; they land on the column container the tag wraps, not on each row. See the enter/leave section of [Declarative views](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/reactive-view.md).

## Next steps

<div class="next-steps">
  <a class="next-step" href="/guide/built-ins/keep-alive">
    <p class="next-step-link">KeepAlive</p>
    <p class="next-step-caption">How a switched-away branch is kept instead of destroyed after its animation.</p>
  </a>
  <a class="next-step" href="/guide/built-ins/teleport">
    <p class="next-step-link">Teleport</p>
    <p class="next-step-caption">Mount part of a view somewhere else in the tree.</p>
  </a>
  <a class="next-step" href="/guide/essentials/conditional">
    <p class="next-step-link">Conditionals</p>
    <p class="next-step-caption">How v-if expands into when.</p>
  </a>
</div>
