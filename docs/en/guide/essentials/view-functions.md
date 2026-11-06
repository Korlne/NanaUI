# Composing Views

A piece of UI is a function that returns `impl IntoView`. `IntoView` is anything that can become retained nodes: an element, a tuple of views, `each`, `when`, and `()`. The call happens at mount time. The function itself does not cache the tree.

A component in a template is a function marked `#[component]`. `<Status label="Idle" />` calls `status("Idle")`.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::{IntoProp, IntoView, component};
use nana_ui::runtime::SemanticColorRole;

#[component]
fn status(label: impl IntoProp<String>) -> impl IntoView {
    view! {
        <Text foreground={SemanticColorRole::Muted}>{label}</Text>
    }
}

fn page() -> impl IntoView {
    view! {
        <Column gap=8>
            <Status label="Idle" />
        </Column>
    }
}
```

```rust rust
use nana_ui::runtime::view::*;
use nana_ui::runtime::SemanticColorRole;

fn status(label: impl IntoProp<String>) -> impl IntoView {
    text(label).foreground(SemanticColorRole::Muted)
}

fn page() -> impl IntoView {
    column().gap(8).children(status("Idle"))
}
```

:::

An unknown tag looks up a function with the same name in snake_case. `Status` is `status`, `TodoRow` is `todo_row`. Attributes go to the parameters with the same names, in any order. Children go to the parameter named `children`. With no children, nothing is passed. One child is that single view. Several children become a tuple; past 12, nest another tuple.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::{IntoView, component};

#[component]
fn frame(children: impl IntoView) -> impl IntoView {
    view! {
        <Column gap=8>
            {children}
        </Column>
    }
}
```

```rust rust
use nana_ui::runtime::view::{column, text, IntoView};

fn frame(children: impl IntoView) -> impl IntoView {
    column().gap(8).children(children)
}
```

:::

`<Frame><Text>"Content"</Text></Frame>` is `frame(text("Content"))`.

## Optional attributes

Mark a parameter `#[prop(default)]` (uses `Default::default()`) or `#[prop(default = expr)]` and the template may omit that attribute. An attribute name with no value (`<Setting disabled />`) means `true`.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::{IntoProp, IntoView, component};

#[component]
fn setting(
    label: &'static str,
    #[prop(default = "")] hint: &'static str,
    #[prop(default = false)] disabled: impl IntoProp<bool>,
) -> impl IntoView {
    view! {
        <Column gap=2>
            <Checkbox label={label} disabled={disabled} />
            <Text>{hint}</Text>
        </Column>
    }
}

fn page() -> impl IntoView {
    view! {
        <Column gap=8>
            <Setting label="Autosave" />
            <Setting hint="Sign in first" label="Sync" disabled />
        </Column>
    }
}
```

```rust rust
use nana_ui::runtime::view::*;

fn setting(label: &'static str, hint: &'static str, disabled: impl IntoProp<bool>) -> impl IntoView {
    column().gap(2).children((checkbox(label).disabled(disabled), text(hint)))
}

fn page() -> impl IntoView {
    column().gap(8).children((
        setting("Autosave", "", false),
        setting("Sync", "Sign in first", true),
    ))
}
```

:::

Defaults live next to the function. Names in them resolve in the function's module, not in the template that uses it. `impl Trait` parameters can have defaults too: `#[prop(default = || true)] shown: impl Fn() -> bool`. An attribute whose type uses the function's own generic parameters cannot have a default.

These mistakes fail to compile, with the error pointing at the template:

- A missing attribute that has no default: `` `<Setting>` needs `label` ``.
- A parameter the function does not have: `no method named 'colour'`. Writing the same attribute twice is also an error.
- A function not marked `#[component]`: the compiler cannot find module `setting`.

`#[component]` does not change the function. In Rust you still pass all arguments in order, `setting("Sync", "Sign in first", true)`. It adds a hidden module with the same name next to the function, holding the builder the template expands to (`setting::Props`). When you `use` the function, the module comes with it. Arguments must be owned or `'static` values. `key` on a component tag has its own meaning (see below), so no parameter may be named `key`.

Custom tags take only these values. Events and fields are written inside the function. `<Status @activate={save} />` is a template error: a component tag cannot carry `@` or `on:`.

A tag with a path always calls a function, even when its last segment matches a built-in tag. Without a path, built-in tags win. If your own function is also called `button`, the built-in `Button` shadows it. Write `<self::Button />` in the same module to call your function.

A `key` on a component tag lands on its first root node and expands to `keyed(key, call)`.

## Returning different types

`impl IntoView` must be one concrete type within a function. When two branches have different types, erase them with `into_any` into `AnyView`. `AnyView` implements `IntoView` itself.

```rust
use nana_ui::runtime::view::{column, text, IntoView};

fn body(empty: bool) -> impl IntoView {
    if empty {
        text("Nothing yet").into_any()
    } else {
        column().gap(8).children(text("Has content")).into_any()
    }
}
```

`Option<view>` is a view too. `None` builds nothing. `show.then(|| text("Here"))` is empty when `show` is false.

::: warning
An `if` inside the function decides the structure for this call only and does not follow the data afterward. To switch `body` on a signal, use `when` from [Conditional rendering](conditional.md); do not write `if signal.get()` inside the function. During construction `signal.get()` is a constant snapshot.
:::

`()` is the empty view. A tuple can contain a `()`, which takes a slot but builds no node. Prefer a tuple for a fixed set of children, `.children((a, b))`. To use `for` or `let` while building the tree, use `.with(|c| c.add(…))`; that block also runs only once.

Built-in tags never call your functions. `Column`, `Row`, `Text`, `Button` and the rest expand to `column()`, `text`, `button`; see the table in [View syntax](view.md).

## Where parameters come from

A parent view passes in signals, path handles, or cloned values. A row view receives a clone of its item from `each`, so a parameter such as `title: String` is the row's own. To follow the original data, pass a signal, a `Computed`, or a field handle from a [Store](reactivity.md), not a copy taken with `get()`.

Callbacks are ordinary parameters: `fn toolbar(start: impl FnMut() + Send + 'static) -> impl IntoView`, with `button("Start").on_activate(start)` inside. There is no separate emits. In a template this is still an attribute, not `@`.

## Next steps

<div class="next-steps">
  <a class="next-step" href="/guide/essentials/lifecycle">
    <p class="next-step-link">Mount and unmount</p>
    <p class="next-step-caption">Which scope these functions evaluate in, and what remains after unmount.</p>
  </a>
  <a class="next-step" href="/guide/essentials/view">
    <p class="next-step-link">View syntax</p>
    <p class="next-step-caption">How built-in tags expand into function calls.</p>
  </a>
</div>
