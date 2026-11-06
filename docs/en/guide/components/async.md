# Async

A signal can only be used on the thread that created it. Async code that writes signals stays on the UI thread: put the blocking part in `spawn_blocking` and hand the result to a `resource`. To show other content before the first completion, use `suspense`.

`load_user` is your function, not part of the framework. It must be called while `suspense` builds its content, so it goes in a function the content runs. A `resource` written outside `suspense` does not make this fallback appear.

```rust
use nana_ui::runtime::view::{component, resource, spawn_blocking, text, IntoView, Signal};

#[component]
fn profile(id: Signal<u64>) -> impl IntoView {
    let user = resource(move || id.get(), |id| spawn_blocking(move || load_user(id)));
    text(move || user.with(|user| user.map_or(String::new(), |user| user.name.clone())))
}
```

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

`<Profile id={id} />` calls `profile(id)` (`#[component]` lets templates pass arguments by name; see [Composing views](https://github.com/sena-nana/NanaUI/blob/main/docs/guide/essentials/view-functions.md)). The content is built immediately and hidden until the `resource` created inside completes for the first time. Later reloads do not switch back to "Loading…". For the template's `fallback` and `<template #fallback>` in `.vue`, see [Suspense](https://github.com/sena-nana/NanaUI/blob/main/docs/guide/built-ins/suspense.md).

## `resource` remembers the latest result

`source` is tracked like an effect. Every time it changes, `fetch(source)` is called and any previous fetch that has not completed is dropped.

- `get()` / `with()` read the latest result. `None` before the first completion. The old value is kept during a reload.
- `loading()` reports whether a load is in progress.
- `refetch()` loads again with the source's current value.
- `set(value)` changes the result directly, for optimistic updates. A fetch already in flight still writes its value when it completes.

`spawn_blocking` runs the closure on a new thread and returns a future carrying the result. The closure must be `Send`. `spawn_local` runs a future on the UI thread's executor; it does not require `Send` and can read and write signals directly. The `Task` it returns can be canceled with `abort()`. Tasks created during tree building belong to the view and are dropped when it unmounts.

Using a signal on another thread panics. The handle is a `Copy` id and can go into a `Send` handler, but it must be used back on the thread that created it.

## The host must poll

When a waker is woken on any thread, it wakes the event loop through the host's wake hook, which then calls `poll_tasks()` on the UI thread. Tasks are also polled once at the start of every frame. The `nana-ui` window host calls `set_task_wake` at startup, polls while handling host work, and requests a redraw for windows with pending application bindings.

Without a host (tests, embedding) you must call `poll_tasks()` yourself. If nobody polls, the future just sits there.

## Next steps

<div class="next-steps">
  <a class="next-step" href="/guide/built-ins/suspense">
    <p class="next-step-link">Suspense</p>
    <p class="next-step-caption">How long the fallback shows, and whether reloading switches back.</p>
  </a>
  <a class="next-step" href="/guide/scaling/fetch">
    <p class="next-step-link">Fetch</p>
    <p class="next-step-caption">The host allowlist shared by page requests and url().</p>
  </a>
  <a class="next-step" href="/guide/built-ins/transition">
    <p class="next-step-link">Transition</p>
    <p class="next-step-caption">How loaded content enters.</p>
  </a>
</div>
