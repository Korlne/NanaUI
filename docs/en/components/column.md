# Column

`column()` is a vertically arranged `Stack`. The template tag is `<Column>`. For horizontal arrangement use `row()` or `<Row>`; the properties are the same and are not repeated here.

## Basic usage

Spacing is `.gap(…)` in logical pixels; it accepts any `Px`, integer or `f32`. Writing `gap=8` in a template expands to `.gap(8_f32)` with the type suffix.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Column gap=8>
        <Text>"Title"</Text>
        <Button @activate={save}>"Save"</Button>
    </Column>
}
```

```rust rust
use nana_ui::runtime::view::{button, column, text};

column().gap(8).children((
    text("Title"),
    button("Save").on_activate(save),
))
```

:::

## Child nodes

There are two ways to place child nodes. With a fixed count, use `.children((a, b))`. When you need to write `if`, `for` or `let` while building, use `.with(|c| c.add(…))`. For a list that follows data use `each`, see [Writing views](https://github.com/sena-nana/NanaUI/blob/main/docs/guide/essentials/view.md).

`.with` runs once while building. An `if` in the block decides the structure of this mount. Later data changes do not rerun the block.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `.gap` | `Px` | Spacing in logical pixels; integer or `f32` both work. Writing `gap=8` in a template expands to `.gap(8_f32)` with the type suffix |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| None | None | None |

## Slots

| Slot | Description |
| --- | --- |
| None | With a fixed count, use `.children((a, b))`. When you need to write `if`, `for` or `let` while building, use `.with`. For a list that follows data use `each` |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Writing views](https://github.com/sena-nana/NanaUI/blob/main/docs/guide/essentials/view.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
