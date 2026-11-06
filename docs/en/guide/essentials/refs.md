# Handles

Once a view is built, you sometimes need to change that one control by type, or hand a node id to another interface. Use `entity_ref` to record an `Entity<control>` and `node_ref` to record a node id. Both are signals, written when the element is built.

```rust
use nana_ui::runtime::view::{button, column, entity_ref, text, with_refs};
use nana_ui::runtime::{Button, Text};

let (mounted, (label, action)) = cx.mount_view_root(document_id, || {
    let label = entity_ref::<Text>();
    let action = entity_ref::<Button>();
    with_refs(
        column().gap(8).children((
            text("Ready").entity_ref(label),
            button("Start").entity_ref(action),
        )),
        (label, action),
    )
})?;
```

`entity_ref::<Text>()` gives an `EntityRef<Text>`. `.entity_ref(label)` writes the entity into it when this `Text` is built. `with_refs(view, handles)` turns the mount's return value into `(MountedView, resolved handles)`: `EntityRef<C>` becomes `Entity<C>` and `NodeRef` becomes `StableNodeId`. Tuples, arrays and `Vec` resolve item by item as they are.

Handles have to be created inside the mount closure to belong to this mount. The `label` and `action` above are in the closure, and are collected with the other signals on unmount.

The types have to match. `El<Text>` accepts only `EntityRef<Text>`. Writing `EntityRef<Button>` does not compile. One element can fill several handles at once without one overwriting another: `.entity_ref(label).node_ref(slot)`.

::: warning
Every handle in `with_refs` must point at an element built this time. Pointing at a branch that was not built (a `when` whose condition is false, say) makes the mount return `FrameworkError::InvalidInput`, and nothing is left behind.
:::

## Without with_refs

`EntityRef::get` returns `Some(Entity<C>)` once the element is built, and `None` before. This read does not track and does not become a binding. `NodeRef` is just `Signal<Option<StableNodeId>>`. Use `get_untracked` for the id, so another binding does not subscribe to it.

The template property `ref={input}` expands to `.node_ref(input)`. `ref="input"` looks the same `NodeRef` up by name in `.vue`. `entity_ref` has no template property.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::{node_ref, IntoView};

fn field() -> impl IntoView {
    let input = node_ref();
    view! {
        <TextInput placeholder="Keyword" ref={input} />
    }
}
```

```rust rust
use nana_ui::runtime::view::{node_ref, text_input, IntoView};

fn field() -> impl IntoView {
    let input = node_ref();
    text_input().placeholder("Keyword").node_ref(input)
}
```

:::

`EntityRef::node_ref()` reads the same reference as a `NodeRef`, for interfaces that take only a node id, such as a teleport target.

The `MountedView` returned by a mount also offers `root::<C>()`. When the view has one root and you know its type, that gives the `Entity<C>` directly. With several roots, `roots()` gives a `&[StableNodeId]`.

## After you have it

Hand an `Entity<C>` to `AppContext::update_component`. The `cx` above is the context taken from `RuntimeDocument::context_mut()`. The closure receives `&mut C` and a `ViewContext`. When the component equals the old one and no event was emitted and the tree was not changed, the update returns at once, with no projection and no commit.

```rust
cx.update_component(label, |text, _| {
    text.value = "Started".to_string();
})?;
```

Do not write `*text = Text::new("Started")` in this closure. That paints over the interaction state too. To replace the whole thing, use `set_component(label, Text::new("Started"))` and let the control's own `reconcile` decide what stays.

For a field the view already binds with a signal, change the signal instead. Handles are for code that still has to act on an entity after mount: a label the program keeps, a teleport target, focus. To mount a whole page again on a click, see the contract in [Creating an app](https://github.com/sena-nana/NanaUI/blob/main/docs/guide/essentials/application.md).

## Next steps

<div class="next-steps">
  <a class="next-step" href="/guide/essentials/view-functions">
    <p class="next-step-link">View functions</p>
    <p class="next-step-caption">Wrap a piece of UI in a function returning impl IntoView.</p>
  </a>
  <a class="next-step" href="/guide/essentials/lifecycle">
    <p class="next-step-link">Mount and unmount</p>
    <p class="next-step-caption">Which scope a handle belongs to, and how it is collected on unmount.</p>
  </a>
</div>
