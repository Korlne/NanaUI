# Provide / inject

`provide(value)` attaches a value to the current scope. Descendant scopes, including rows and branches built later, read it along the parent chain with `use_context::<T>()`; the nearest provider wins. Calling `provide` again with the same type in the same scope replaces the previous one. `T` must be `Clone + 'static`.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::{use_context, IntoView, provide, signal};

fn page() -> impl IntoView {
    let user = signal(String::from("you"));
    provide(user);
    view! {
        <Column gap=8>
            {greeting()}
        </Column>
    }
}

fn greeting() -> impl IntoView {
    let user = use_context::<nana_ui::runtime::view::Signal<String>>().expect("page provides user");
    view! {
        <Text>"Hello, {user}"</Text>
    }
}
```

```rust rust
use nana_ui::runtime::view::{use_context, IntoView, column, provide, signal, text};

fn page() -> impl IntoView {
    let user = signal(String::from("you"));
    provide(user);
    column().gap(8).children(greeting())
}

fn greeting() -> impl IntoView {
    let user = use_context::<nana_ui::runtime::view::Signal<String>>().expect("page provides user");
    text!("Hello, {user}")
}
```

:::

It can only be read while building views. Event handlers do not run inside any scope, so `use_context` returns `None` there.

::: warning
Calling `provide` outside a scope does nothing. Calling `use_context` in an event handler cannot read it either. When you put a signal in `provide`, the child view still gets that same `Signal`.
:::

## Next steps

<div class="next-steps">
  <a class="next-step" href="/guide/components/props">
    <p class="next-step-link">Props</p>
    <p class="next-step-caption">Pass values into a view function.</p>
  </a>
  <a class="next-step" href="/guide/essentials/view-functions">
    <p class="next-step-link">Composing views</p>
    <p class="next-step-caption">A piece of UI is a function returning IntoView.</p>
  </a>
</div>
