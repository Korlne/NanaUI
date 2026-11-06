# Conditionals

For a piece of UI that appears and disappears with the data, use `v-if`. It expands to `when`, not the `if` of tree building. An `if` at tree-building time decides only whether the node exists at that moment.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::IntoView;

fn tasks(empty: nana_ui::runtime::view::Signal<bool>) -> impl IntoView {
    view! {
        <Column gap=8>
            <Text v-if={empty}>"No tasks yet"</Text>
            <Text v-else>"Has tasks"</Text>
        </Column>
    }
}
```

```rust rust
use nana_ui::runtime::view::*;

fn tasks(empty: Signal<bool>) -> impl IntoView {
    column().gap(8).children(
        empty
            .then_show(|| text("No tasks yet"))
            .otherwise(|| text("Has tasks")),
    )
}
```

:::

`v-if` / `v-else` expand to `when(condition, || branch).otherwise(|| other branch)`. `empty.then_show(|| …)` is `when(empty, || …)`; the name is not `show`. `v-else-if` nests inside `otherwise`:

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Text v-if={loading}>"Loading"</Text>
    <Text v-else-if={failed}>"Failed"</Text>
    <Text v-else>"Done"</Text>
}
```

```rust rust
use nana_ui::runtime::view::{text, when};

when(loading, || text("Loading"))
    .otherwise(|| when(failed, || text("Failed")).otherwise(|| text("Done")))
```

:::

In a template they are three adjacent lines: `v-if`, `v-else-if`, `v-else`. A `v-else` with no `v-if` before it is a template error.

The condition can be a `Signal<bool>`, a `Computed<bool>`, or a closure returning `bool`. An expression such as `list.with(Vec::is_empty)` is wrapped as a closure in templates. In the function style, write `move || list.with(Vec::is_empty)` and then `.then_show`.

The branch that is not shown does not exist by default: the node is torn down, its own scope is collected, and the signals and effects created inside it are dropped with it. When the condition changes back, a new branch is built. A branch already shown at mount does not play an enter transition.

With no `otherwise`, a false condition builds nothing.

## Hide it but keep the node

`v-show` does not tear the node down; it toggles `layout.hidden`. The node stays in the tree but takes no part in layout, painting or hit testing.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Text v-show={shown}>"Still in the tree"</Text>
}
```

```rust rust
use nana_ui::runtime::view::text;

text("Still in the tree").visible(shown)
```

:::

For something toggled often whose inner state must stay, use `v-show`. For a whole block replaced, use `v-if`.

`.keep_alive()` takes a third path: the hidden branch stays, nodes included, in a container that takes no part in layout, painting or hit testing, and comes back with its state intact. In templates, wrap a `v-if` chain in `<KeepAlive>`.

::: warning
One element cannot write `v-for` and `v-if` at the same time; that is a template error. To filter rows out, filter in the `each` data, or let the row view write its own `v-if`. Lists are the next chapter.
:::

`when` and `each` each carry a container, by default a `Stack` column with a gap of 0. To add a class to this chain's container, wrap the chain in `<Block class="…">`, or write `when(…).class(…)` in the function style. The class does not land on the two branches automatically. A branch root's own `class` still belongs to that branch only.

When a branch has to fill the remaining space (a scroll area, an editor), write `when(…).grow()` or `dynamic(…).grow()`: the container takes the rest of the space along the parent's main axis, which gives the branch's `fill_column` something to fill.

`when(…).transition(…)` adds enter and exit. An exiting node first stays where it is, takes no hit testing, and is destroyed once the transition ends. That is enter and exit, not `v-show`.

## Next steps

<div class="next-steps">
  <a class="next-step" href="/guide/essentials/list">
    <p class="next-step-link">Lists</p>
    <p class="next-step-caption">v-for, each, and the key of every row.</p>
  </a>
  <a class="next-step" href="/guide/essentials/class-and-style">
    <p class="next-step-link">Classes and styles</p>
    <p class="next-step-caption">How the conditional class class:empty binds to the same stylesheet.</p>
  </a>
</div>
