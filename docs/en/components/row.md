# Row

`row()` is a horizontal `Stack`. Its template tag is `<Row>`. For vertical layout use `column()`; see [Column](column.md).

## Basic usage

It has no properties in the field table. Spacing is `.gap(…)`, in logical pixels, and accepts any `Px`: an integer or an `f32`.

Internally the function is `Stack::row(0.0)`: width shrinks to content, children are centered on the cross axis, and the gap starts at 0. Only a later `.gap` call writes the gap. In a template, `gap=12` expands to the type-suffixed `.gap(12_f32)`. Hand-written code stays `.gap(12)`.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Row gap=12>
        <Text>"Label"</Text>
        <Button @activate={save}>"Save"</Button>
    </Row>
}
```

```rust rust
use nana_ui::runtime::view::{button, row, text};

row().gap(12).children((
    text("Label"),
    button("Save").on_activate(save),
))
```

:::

## Children

There are two ways to add children. Use `.children((a, b))` when the count is fixed. When tree building needs `if`, `for`, or `let`, use `.with(|c| c.add(…))`. `.with` runs once during tree building. An `if` inside the block decides the structure of this mount only. When data changes later, the block does not run again.

For a list that follows data, use `each`; see [View authoring](../guide/essentials/view.md).

## Difference from column

The difference from `column()` is in the presets, not in `.gap`. A column is vertical, stretches on the cross axis, and fills the parent's width. A row is horizontal, centers on the cross axis, and shrinks its width to content. `.gap` accepts `Px` on both, and negative values are clamped to 0.

## End of the row

To push the following siblings to the end of the row, use `Stack::spacer()`. It is a zero-width, growable gap, not a property of `<Row>`. Put it in the view as a child: `widget(Stack::spacer())`. Its basis width must be 0, or it pushes siblings out.

## Filling the remaining width

To fill the parent's remaining width and let children shrink, use `Stack::fill_row`, not `row()`. The width of `row()` shrinks to content.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| None | None | No properties in the field table |
| `.gap` | `Px` | Spacing in logical pixels. Integer or `f32`. Template `gap=12` expands to `.gap(12_f32)`. Hand-written code stays `.gap(12)`. Internally starts from `Stack::row(0.0)`; only the call writes it. Negative values clamp to 0 |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| None | None | None |

## Slots

| Slot | Description |
| --- | --- |
| Children | Use `.children((a, b))` when the count is fixed. When tree building needs `if`, `for`, or `let`, use `.with(\|c\| c.add(…))`, which runs only once during tree building |

## See also

[Overview](index.md) · [Controls](../reference/components.md)
