# View Syntax

`view!` and function calls are two first-class authoring styles. They expand to the same calls, such as `column().gap(8)`, `.class(..)` and `each(..)`. The tree is built once at mount, and after that each binding updates only the field it writes.

The view layer is compiled in by default. `view!` sits behind the `view-macro` feature. The macro's path is `nana_ui::runtime::view`. The macro and the module of the same name live in different namespaces (macro and type), so both of these lines can coexist:

```rust
use nana_ui::runtime::view;
use nana_ui::runtime::view::{button, column, text};
```

## A counter

Here is the same counter in both styles.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::{IntoView, signal};

fn counter() -> impl IntoView {
    let count = signal(0u64);
    view! {
        <Column gap=8>
            <Text>"Count: {count}"</Text>
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
        text!("Count: {count}"),
        button("Increment").on_activate(move || count.update(|c| *c += 1)),
    ))
}
```

:::

`<Text>"Count: {count}"</Text>` expands to `text!("Count: {count}")`. A string without interpolation expands to `text("…")`.

`@activate={expr}` expands to `.on_activate(move || { expr; })`. If `add` is already a function, `@activate={add}` passes that function directly.

## Children

For a fixed number of children, use a tuple: `.children((a, b))`. The group is boxed once.

If building the tree needs `for`, `if` or `let`, write `.with(|c| { c.add(a); })`. The block runs only once, at tree building, and its branches decide the structure at that moment. For structure that follows data, use `each` or `when`.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Column gap=8>
        <TextInput placeholder="New task" v-model={draft} />
        <Button @activate={add}>"Add"</Button>
        <Text v-for={t in list} key={t.id}>{t.title}</Text>
        <Text v-if={list.with(Vec::is_empty)}>"No tasks yet"</Text>
        <Text v-else>"Has tasks"</Text>
    </Column>
}
```

```rust rust
use nana_ui::runtime::view::*;

column().gap(8).with(|c| {
    c.add(text_input().placeholder("New task").model(draft));
    c.add(button("Add").on_activate(add));
    c.add(list.each(|t| t.id, |t| text(t.title)));
    c.add(
        list.with(Vec::is_empty)
            .then_show(|| text("No tasks yet"))
            .otherwise(|| text("Has tasks")),
    );
})
```

:::

## Attributes

An attribute can be a constant, a `Signal<T>`, a `Computed<T>`, or an `Fn() -> T` closure. Constants are written when the node is built. Signals are bound directly. Closures are boxed once.

::: warning
Calling `count.get()` while constructing a view reads a constant snapshot that does not follow later changes. To follow changes, pass the signal itself or a closure.
:::

A misspelled class name is a compile error in `view!`. In the function style, class names are also checked against the stylesheet at compile time.

## Reference

The macro expands to the right-hand column. Numeric literals get a type suffix in the macro. By hand, `gap` accepts any `Px`, so `gap(8)` is enough.

| Template | Function style |
| --- | --- |
| `<Column gap=8>` / `<Row>` | `column().gap(8).children((…))` / `row()` |
| `<Text>"Count: {count}"</Text>` | `text!("Count: {count}")` |
| `<Button>"Save"</Button>` | `button("Save")` |
| `<TextInput/>` | `text_input()` |
| `@activate={save}` | `.on_activate(save)` |
| `v-model={sig}` | `.model(sig)` |
| `v-for={t in list} key={t.id}` | `list.each(\|t\| t.id, \|t\| …)` |
| `v-if` / `v-else` | `when(…)`, or `cond.then_show(…).otherwise(…)` |
| `v-show={x}` | `.visible(x)` |
| `class="a"` / `class:empty={c}` | `.class(s::a)` / `.class_when(s::a, c)` |

`v-for` and `v-if` cannot be on the same element; that is a template error. `v-for` requires a `key`. An unknown tag calls the `#[component]` function with the same snake_case name; see [Composing views](view-functions.md).

The full text on signals, Store, stylesheets and the `.vue` dialect is in [Declarative views](../../reference/reactive-view.md). Computed values, conditionals and lists each get their own chapter later.
