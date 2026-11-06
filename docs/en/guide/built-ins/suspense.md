# Suspense

`suspense` shows a fallback until the content is ready for the first time. The content is built immediately, just hidden at first. The fallback is removed only after every `resource` created inside has completed its first load. Later reloads keep the old content and do not switch back.

For loading itself, see [Async](https://github.com/sena-nana/NanaUI/blob/main/docs/guide/components/async.md). `profile` creates a `resource` while the content is built.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::text;

view! {
    <Suspense fallback={text("Loading…")}>
        <Profile id={id} />
    </Suspense>
}
```

```rust rust
use nana_ui::runtime::view::{suspense, text};

suspense(|| text("Loading…"), move || profile(id))
```

:::

The fallback is the view shown while `pending` is still greater than 0 during content building. In templates use `fallback={…}`. In `.vue` it is written `<template #fallback>`; that is a slot, and it does not expand into a method named `fallback`. `<Suspense>` has only this one attribute.

While the content is built, `pending` goes up by one for each `resource` created and down by one when that fetch first completes. `refetch` does not add to it. If the content creates no `resource` at all, the fallback never appears, so the `resource` must be written in a function the content actually runs. When several `resource`s are in the same content, all of them must complete their first load.

The content closure is `FnOnce`. The fallback closure must be `Send`. If this part of the view is unmounted midway, unfinished tasks are aborted with `abort`, the `pending` they still hold is subtracted, and the fallback disappears with them.

## Fallback and content are both in the tree

The content goes into a column, hidden with `visible` until `pending` returns to 0. The fallback is a sibling `when` whose condition is `pending > 0`. After the first completion the fallback is torn down, and the content is not rebuilt; it only goes from hidden to visible.

Later `refetch` calls no longer increase `pending`, so this `when` does not open again.

`provide(SuspenseContext)` happens inside the content closure. A `resource` written before `suspense` cannot see this context and does not make this fallback appear. To have the fallback wait for it, put the `resource` in a function the content calls.

## Reloading does not return to the fallback

A `resource` keeps its previous value while refetching and reports loading through `loading()`. `suspense` only looks at whether a first result has ever existed. If you want to switch to a different piece of UI while refreshing, read `loading()` and write your own `when`; do not expect the boundary to show the fallback again.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Text v-if={move || user.loading()}>"Refreshing…"</Text>
    <Text v-else>{name}</Text>
}
```

```rust rust
use nana_ui::runtime::view::{text, when};

when(move || user.loading(), || text("Refreshing…")).otherwise(|| text(name))
```

:::

For optimistic updates use `user.set(new_value)`. A fetch already in flight still overwrites it when it completes. This has nothing to do with the fallback: the content is already showing.

Without host polling, the first load never completes and the fallback stays forever. Window hosts call `set_task_wake` and `poll_tasks()`. Tests and embedders must poll themselves. See [Async](https://github.com/sena-nana/NanaUI/blob/main/docs/guide/components/async.md).

## Next steps

<div class="next-steps">
  <a class="next-step" href="/guide/components/async">
    <p class="next-step-link">Async</p>
    <p class="next-step-caption">resource, spawn_blocking, and threads.</p>
  </a>
  <a class="next-step" href="/guide/scaling/sfc">
    <p class="next-step-link">.vue dialect</p>
    <p class="next-step-caption">The same calls, written as a single-file component.</p>
  </a>
  <a class="next-step" href="/guide/built-ins/keep-alive">
    <p class="next-step-link">KeepAlive</p>
    <p class="next-step-caption">Keep already loaded state when switching away from a page.</p>
  </a>
</div>
