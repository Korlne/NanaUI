# Mounting and unmounting

`mount_view` and `mount_view_root` take a closure. The view expression is evaluated inside that closure and built in one commit. Signals, `computed`, `watch_effect` and `provide` created in the closure all belong to the scope of this mount.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::{node_ref, on_cleanup, on_mount, IntoView};
use nana_ui::runtime::DocumentId;

fn search(document: DocumentId) -> impl IntoView {
    let input = node_ref();
    on_mount(move |cx| {
        if let Some(id) = input.get_untracked() {
            let _ = cx.focus_node(document, id);
        }
    });
    on_cleanup(|| {
        // Runs when the scope is recycled. Do not read this scope's signals here.
    });
    view! {
        <Column gap=8>
            <Text>"Search"</Text>
            <TextInput placeholder="Keyword" ref={input} />
        </Column>
    }
}
```

```rust rust
use nana_ui::runtime::view::{column, node_ref, on_cleanup, on_mount, text, text_input, IntoView};
use nana_ui::runtime::DocumentId;

fn search(document: DocumentId) -> impl IntoView {
    let input = node_ref();
    on_mount(move |cx| {
        if let Some(id) = input.get_untracked() {
            let _ = cx.focus_node(document, id);
        }
    });
    on_cleanup(|| {
        // Runs when the scope is recycled. Do not read this scope's signals here.
    });
    column().gap(8).children((
        text("Search"),
        text_input().placeholder("Keyword").node_ref(input),
    ))
}
```

:::

To mount under a parent node use `mount_view(parent, || search(document))`. When the root is the document root use `mount_view_root(document, || search(document))`. To build first without putting it into the tree, use `mount_view_detached`. All three put the closure under the same scope rules.

## Mounting leaves no half tree

When building the tree itself errors, the mutation queue does not commit and the scope is recycled at once. When inserting the root under the parent fails, the scope is recycled first and then the roots already built are torn down. When placing a slot fails, or a handle in `with_refs` does not match an element built this time, the tree is already standing, so it goes through `unmount`: recycle the scope first, then tear down each root. The latter returns `FrameworkError::InvalidInput`.

A root inserted under some parent carries no key. Children the parent has already ordered by key are not reassembled because of it.

When the parent is no longer in the tree, `mount_view` returns `FrameworkError::MissingView` and the closure does not run. `mount_view_root` and `mount_view_detached` instead put the root on the `DocumentId` you pass in.

`on_mount` runs once after the view enters the tree: after the commit that mounts it, or after a row of `each` or a branch of `when` has been put in. It does not track reads. Its argument is the `&mut AppContext` that holds this view. The `node_ref` above already has a node id by then. `focus_node` returns `Ok(false)` when the node is not focusable; it does not panic. When mounting fails and the scope is recycled first, this callback is dropped and never runs against a half tree.

`on_cleanup` registers on the current scope and runs when the scope is recycled. The closure is `FnOnce()`.

::: warning
Calling `on_cleanup` outside a scope does nothing. Event handlers are not in a scope, and cleanup registered there never runs. Write `on_mount` while building the view.
:::

## What is recycled with what

| Moment | What is recycled |
| --- | --- |
| `MountedView::unmount` | This mount's scope, then each root is torn down |
| Tearing down a root node | The bindings and structural side effects bound to it, and the scopes anchored on it |
| `each` removing a row, `when` switching away a branch | Only that row's or branch's child scope |
| `AppContext` being dropped | The view scopes still left on this context |

Throwing away the `MountedView` handle does not unmount. The tree is still there and the bindings are still there. To tear it down, call `mounted.unmount(&mut cx)`. It `dispose`s the scope first and then despawns each root.

A row's or branch's own `on_cleanup` runs when it is removed, without waiting for the whole page to unmount. For a node whose leave animation is still playing, the scope is recycled first and the node is destroyed only after the animation finishes. Do not `get` this scope's signals in cleanup.

Signals created outside a scope have no owner and live until the thread ends. A temporary `signal(…)` in an event is like this. For signals a list row needs, write them in the row view so that row's child scope carries them.

`provide` also hangs on the current scope. Rows and branches see values on the parent chain. After the scope is recycled, this provider is gone too.

## Building and updating are different things

The mount closure runs once. The `if`, `for` and `.with` inside it decide the structure for this one time. Changing a signal afterwards runs only the bindings and `watch_effect`s that read it, plus the structural updates of `each` / `when`. Do not call `mount_view` again for the same piece of UI inside a click handler.

`flush_reactive` is called by the input router at the end of an event, and again at the start of every frame. You do not need to flush manually in `on_mount`. Writing a signal in `on_mount` queues it for the next flush, like any other signal write.

Reading or writing a signal after its scope has been recycled reports `runtime.reactive.disposed_access` and panics as well. For cleanup and for tasks that come back later, use `try_update`: it changes the value only while the signal is still there, and returns `false` when it is gone.

## Next steps

<div class="next-steps">
  <a class="next-step" href="/guide/components/registry">
    <p class="next-step-link">Extension controls</p>
    <p class="next-step-caption">How to register your own control when the built-in tags are not enough.</p>
  </a>
  <a class="next-step" href="/reference/reactive-view">
    <p class="next-step-link">Declarative views</p>
    <p class="next-step-caption">The full contract of scopes, refresh and unmounting.</p>
  </a>
</div>
