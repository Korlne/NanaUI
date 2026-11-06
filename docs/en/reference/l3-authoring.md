# L3: Building Retained UI in Rust

This page covers how Rust declarative views (L3) are mounted, the node identity (Key) rules, the fine-grained update contract, and the fundamental difference from immediate-mode / GPUI-style frameworks. For your first window, start with [Quick Start](../guide/quick-start.md) and [Creating an Application](../guide/essentials/application.md).

Rust applications build UI through [declarative views](reactive-view.md): a view is an ordinary Rust expression (written with the `view!` macro or the equivalent function calls). It is **evaluated only once, at mount**, and committed in one batch into the retained `UiWorld`. Later state changes update only the affected fields, through signals (`Signal` / `Store`) or component handles; the whole view function is never rerun per frame.

## Mounting and extracting handles

```rust
let (view, start) = cx.mount_view_root(document_id, || {
    let start = entity_ref::<Button>();
    let page = column().gap(12).children((
        text("Hello"),
        button("Start")
            .entity_ref(start)
            .on_cx(|_, _: &Activate, cx| cx.dispatch_program(Start)),
        row().gap(8).children((button("Open"), button("Popup"))),
    ));
    with_refs(page, start)
})?;
```

Common mounting and composition entry points:

| Entry point / method | Purpose |
| --- | --- |
| `mount_view_root(document, \|\| view)` | Mounts the view root as the document root |
| `mount_view(parent, \|\| view)` | Mounts the view under an existing parent (the root itself does not take an assembly key in the parent) |
| `mount_view_detached(document, \|\| view)` | Creates a detached view in the document first, for a compound control to place into a slot by ID |
| `with_refs(view, refs)` | Returns `(MountedView, resolved handles)` directly after mounting, turning `EntityRef<C>` into `Entity<C>` |
| `.on(\|e\| …)` / `.on_cx(\|component, e, cx\| …)` | Binds an event in place; `on_cx` can modify component state in place or call `cx.dispatch_program` |
| `.slot` / `.child_slot` / named slots | Passes child views to compound controls (such as Shell regions, grouped toolbars, list row icons) |
| `#[component]` | Lets templates call the application's own view functions by attribute name (`<SettingsRow label="…" on={…}/>`); parameters with `#[prop(default = …)]` can be omitted. Rust still calls the same function positionally |

In declarative views, `Stack` (`column()` / `row()`) is only a style and layout container. Compound controls run their own assemblers automatically at mount (no need to call `assemble_*` by hand).

## Node identity (Key) and path resolution

- **Static structure is named by position**: static nodes without an explicit `.key("…")` get internal keys by position automatically. Write `.key("…")` only when you need to find a node by path or pass content to a container that receives children by key (such as `Dock`, `AreaLayout`, `PropertyGrid`).
- **Key validity constraints**: a duplicate key under the same parent returns `DuplicateAssemblyKey`; an empty key or a key containing the path separator `/` (`ASSEMBLY_PATH_SEPARATOR`) returns `InvalidInput`, and the whole batch is rejected.
- **Lookup by path**: any keyed node in a view can be resolved from the view root by relative path: `resolve_assembly_path(root, "content/list")` or the typed `resolve_assembly_entity::<C>(root, path)`. If you only want a control's `Entity<C>` at the mount site, prefer `entity_ref` + `with_refs`.
- **Dynamic child lists**: one-off loops during tree building can use `.children(items.iter().map(…).collect::<Vec<_>>())` or `.with(|c| for … { c.add(…) })`; lists that must add, remove, or reorder with reactive data use `each`, `each_virtual`, or the conditional branch `when`.

## Fine-grained updates (do not rebuild the whole tree)

| Change | Correct approach |
| --- | --- |
| A control field changes with reactive state | Bind a signal or closure directly: `.label(sig)`, `.bind(\|c\| …)`, `text!("{count}")` |
| Changing a single control in the message loop (`RuntimeProgram::update`) | Keep the handle at mount with `entity_ref` / `with_refs`, then call `update_component(entity, \|v, _\| …)` |
| Adding or removing children dynamically, or switching branches | Use `each` / `when` (the underlying imperative `cx.mount(parent, …)` is still available for assembly inside compound controls) |
| Large long lists / trees / tables | Use `each_virtual` (backed by `materialize_virtual_*_retained`) |

::: warning
Never unmount and re-`mount_view_root` the whole tree in `RuntimeProgram::update` or a click callback. For quick previews after code changes during development, see [Hot Reload in Development](hot-reload.md).
:::

## Division of authoring APIs

| API | Role and use cases |
| --- | --- |
| Declarative views (`view!` / function views) | **The default product authoring surface**: the first choice for application UI and compound views |
| `create_component` / `append_child` / `on` | Low-level primitives: for unit tests, benchmarks, or single-node imperative patches |
| `mount` (`AssemblyScope`) | Imperative keyed assembly: used by the `assemble_*` functions of the framework's built-in compound controls |
| `bind_component` | Only for bridge layers that already have fixed IDs |
| `RuntimeProgram` / `ApplicationState` | Host lifecycle and cross-window/GPU message orchestration, used together with declarative views |

## Performance characteristics

One `mount_view_root` is one atomic `world.commit` (covering creation, projection, and parent-child insertion of every node). The mount phase does not trigger text shaping, layout solving, Scene extraction, or GPU upload. By contrast, calling imperative `create_component` + `append_child` in a loop commits each call separately, so always build whole pages with declarative views or batched queues.

### Measured data

- **Steady-state rendering**: over 60 seconds and 210,000 frames of a pure Rust scene, CPU prepare P95 is `0.0105 ms`.
- **First mount and large structural changes**: Runtime layout P95 for 5,000 nodes is `11.6–29.7 ms`; above 5,000 nodes use virtual scrolling or mount in batches.
- **A single pointer-move event**: $O(1)$ constant cost, about `0.0003 ms`, not growing with tree size (see [Input Cost](input-cost.md)).

### Differences from GPUI-style frameworks

GPUI and similar frameworks use the model "an `Entity` holds state, and every frame or every `cx.notify()` reruns `render()` to return a whole `div().child(...)` tree." NanaUI declarative views are also written as intuitive nested expressions with events bound in place, but they **run only once, at mount**: the runtime is a retained tree `UiWorld` with stable `StableNodeId`s, and state changes are written directly into the matching node's property slots through fine-grained signal bindings.

So in NanaUI, avoid these anti-patterns:
- Do not invent a chained atomic style API like `div().px_4().bg_red()` (it conflicts with the unified Style Model; when you need a stylesheet, use the build-time compiled `<style>`, `stylesheet!`, or `css!`);
- Do not rerun view constructors every frame, and do not put event closures in component struct fields (`ComponentView` requires a `Clone`-able pure state snapshot; event handlers are managed centrally by `AppContext`).
