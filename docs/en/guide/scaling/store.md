# Store

When any part of a `Signal<App>` changes, everything that read it reruns. `store(value)` keeps the value in one place and tracks each path separately. Path handles can be used directly as attributes. `#[derive(Store)]` sits behind `view-macro` and supports only structs with named fields and no generics. Derivation, `keyed` and writes have no second authoring style.

```rust
use nana_ui::runtime::view::{Store, StoreList, StorePath, store};

#[derive(Clone, Store)]
struct Todo {
    id: u64,
    title: String,
    done: bool,
}

#[derive(Store)]
struct App {
    todos: Vec<Todo>,
    filter: String,
}

let app = store(App {
    todos: Vec::new(),
    filter: String::new(),
});
let todos = app.todos().keyed(|todo| todo.id);
```

:::api

```rust view
use nana_ui::runtime::view;

todos.each(|todo| view! {
    <Row gap=4>
        <Text>{todo.title()}</Text>
        <Checkbox checked={todo.done()}>"Done"</Checkbox>
    </Row>
});
```

```rust rust
use nana_ui::runtime::view::{checkbox, row, text};

todos.each(|todo| {
    row().gap(4).children((
        text(todo.title()),
        checkbox("Done").checked(todo.done()),
    ))
});
```

:::

```rust
todos.at(&7).done().set(true);
app.todos().push(todo);
app.filter().set("done".into());
```

The derive generates `TodoStoreFields` and `AppStoreFields`. To call `.title()` or `.todos()`, the corresponding trait must be in scope. Deriving in the same module is enough. List methods come from `StoreList`; reads and writes come from `StorePath`: `get`, `with`, `try_with`, `set`, `update`.

The argument to `keyed` is `fn(&T) -> K`. A closure that captures nothing can serve as that function pointer. Handles, like signals, are `Copy` ids.

`todos.at(&7).done().set(true)` updates only the bindings that read that one cell. `app.todos().push(todo)` adds a row to the list; existing rows' contents are not reread. `app.filter().set(…)` affects only the places that read `filter`.

## Deep and shallow tracking

Each path has two triggers, both created only on the first tracked read. A path nobody has read occupies no signal.

Reading a value (`get` / `with`) tracks deep. Iterating a list (`keyed(…).each`, `items()`, `len()`) tracks only shallow. Writing a path triggers itself and the existing paths below it, plus the deep trigger of each path above it.

`push`, `insert`, `retain`, `swap` and `sort_by_key` do not change any item's contents, so they trigger only the list itself and the deep triggers above, not the rows. `set` / `update` on the whole list triggers every row. After a rerun, row bindings compare per field and stop if the value did not change.

Rows are located by a 64-bit hash of the key. After reordering they still point to the same item. Two keys in the same list must not hash the same. After a row is removed, its triggers are released on the next lookup by key. `get` on a removed row panics; `try_with` returns `None`.

## Undo

`store_with_history(value, limit)` remembers earlier values. `undo()`, `redo()` and `travel(±n)` move backward and forward. `can_undo()`, `can_redo()` and `steps()` are tracked and can be bound straight to buttons. `steps()` gives the line each step was written on.

All writes within one event handler count as one step, bounded by two flushes. Starting a step clones the whole value, so `T: Clone` is required and the cost scales with the value's size. The limit is at least 1. Undo and redo write back the whole value; only bindings whose value actually changed update, and rows are kept by key. Writing after an undo discards the steps that could have been redone.

In `.vue`, `let x = store(…)`, and props whose type is `Store`, `Subfield` or `Item`, are treated as stores: bindings that use them are tracked at runtime and not folded into constants. Writing the bare name (`:checked="done"`, where `done` is a `Subfield` prop) binds it directly.

The field tracking table and trigger rules are in [Declarative views](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/reactive-view.md).

## Next steps

<div class="next-steps">
  <a class="next-step" href="/guide/scaling/packaging">
    <p class="next-step-link">Packaging</p>
    <p class="next-step-caption">Put resources and the executable into the install layout.</p>
  </a>
  <a class="next-step" href="/guide/essentials/reactivity">
    <p class="next-step-link">Reactivity</p>
    <p class="next-step-caption">Signals, scopes, and how store differs from Signal.</p>
  </a>
  <a class="next-step" href="/guide/essentials/list">
    <p class="next-step-link">Lists</p>
    <p class="next-step-caption">How rows kept by keyed().each match their keys.</p>
  </a>
</div>
