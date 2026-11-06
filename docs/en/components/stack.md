# Stack

`Stack` arranges child nodes in one row, one column, or a stack of layers. Rows and columns are flex; stacking has its own layout. The full set of size, alignment and overflow fields is on `LayoutStyle`.

## Basic usage

There is no `<Stack>` in the control table. The view function `column()` is `widget(Stack::column(0.0))`, and `row()` is `widget(Stack::row(0.0))`. With a non-zero gap, construct it directly.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::Stack;

view! {
    <Widget of={Stack::row(8.0)}>
        <Text>"Title"</Text>
        <Widget of={Stack::spacer()} />
        <Button>"Save"</Button>
    </Widget>
}
```

```rust rust
use nana_ui::runtime::view::{button, text, widget};
use nana_ui::runtime::Stack;

widget(Stack::row(8.0)).children((
    text("Title"),
    widget(Stack::spacer()),
    button("Save"),
))
```

:::

## Presets

`Stack::row(gap)` arranges children horizontally, with the width following the content and the children centered vertically. It suits toolbars and button groups.

`Stack::fill_row(gap)` fills the parent's remaining width horizontally, and the children may shrink.

`Stack::column(gap)` arranges children vertically, with the height following the content and the width filling the parent.

`Stack::fill_column(gap)` fills the remaining height vertically; use it for a main content area.

`Stack::bar(gap)` fills the whole row horizontally without stretching itself; use it for a top or bottom bar.

`Stack::spacer()` is a zero-width flex-grow that pushes the siblings after it to the end of the row. Its base width is 0, because otherwise it would fill and squeeze the siblings out.

## Layers

`Stack::layers()` makes all children share one area, with later children painted over earlier ones, the same as `ZStack` in SwiftUI, `Box` in Compose and `Stack` in Flutter.

- The container follows the largest child. When a width and height are given, that area is the container's content box.
- Each child aligns inside that area: `.align` controls the vertical axis, `.justify` the horizontal axis, top left by default. A child's own `align-self` / `justify-self` wins, and `stretch` fills the whole area. The horizontal start follows the writing direction, so it is the right side under RTL.
- Absolutely positioned children are positioned by their containing block as usual and do not grow the container.
- Hit testing and painting both follow document order; an overlap is caught by the layer written later.

CSS writes `display: layers`, a NanaUI extension value; align with `justify-items` / `align-items` / `place-items`. It lays out like a grid: one row, one column, every child placed in that single cell, and a child's own grid position has no effect.

`Stack::overlay_layer()` is a layer that fills an already positioned parent: absolutely positioned, overflow clipped, pointer events off. Use it as the container for stage HUDs and danmaku; the node pool is still mounted by you. Rust presets do not hit by default. When a press in the empty area should also land on this container, add `.hittable()`.

## Layout fields

`Stack::from_layout` takes an already parsed `LayoutStyle` and hits by default. `.with_layout` changes layout fields in place.

`.gap`, `.align`, `.justify`, `.width`, `.height`, `.grow`, `.shrink` and `.padding` change the matching field on the preset. `.surface` writes a semantic background. `.outline` needs both the border color and the width; with one missing, no border is drawn.

## Self-drawing

`.painter` can draw the container's look itself. To add decoration on top of the built-in painting, call `cx.draw_default()` inside `paint`.

The container's gradient `mask-image` (`layout.paint.mask`) also applies to self-drawn content, just as it applies to the background. One shadow plus one mask draws a glow that only faces one side; the part extending outside the container takes the mask value at the box edge. Children are unaffected. For details, see the painting section of [Layout](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/layout.md).

## Hitting

When the empty area of a QR-code card must also receive the pointer, use `Stack::column(...).hittable()` and the event belongs to this container.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `Stack::row(gap)` | — | Horizontal arrangement, width following the content, children centered vertically |
| `Stack::fill_row(gap)` | — | Fills the parent's remaining width horizontally; children may shrink |
| `Stack::column(gap)` | — | Vertical arrangement, height following the content, width filling the parent. The view function `column()` is `widget(Stack::column(0.0))` |
| `Stack::fill_column(gap)` | — | Fills the remaining height vertically |
| `Stack::bar(gap)` | — | Fills the whole row horizontally without stretching itself |
| `Stack::spacer()` | — | Zero-width flex-grow. Its base width is 0, because otherwise it squeezes the siblings out |
| `Stack::layers()` | — | Layers: children share one area, align independently, and the container follows the largest child. CSS writes `display: layers` |
| `Stack::overlay_layer()` | — | A layer filling an already positioned parent: absolutely positioned, overflow clipped, pointer events off. Rust presets do not hit by default |
| `Stack::from_layout` | `LayoutStyle` | An already parsed layout. Hits by default |
| `.with_layout` | — | Changes layout fields in place |
| `.gap` | — | Changes the matching field on the preset. With a non-zero gap, construct it directly |
| `.align` | — | Changes the matching field on the preset |
| `.justify` | — | Changes the matching field on the preset. A layer stack has no main axis, so this changes the children's horizontal alignment |
| `.width` | — | Changes the matching field on the preset |
| `.height` | — | Changes the matching field on the preset |
| `.grow` | — | Changes the matching field on the preset |
| `.shrink` | — | Changes the matching field on the preset |
| `.padding` | — | Changes the matching field on the preset |
| `.surface` | — | Semantic background |
| `.outline` | — | Both the border color and the width are required; with one missing, no border is drawn |
| `.hittable` | — | A press in the empty area lands on this container |
| `.painter` | — | Self-drawn look. For decoration, call `cx.draw_default()` inside `paint` |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| None | None | None |

## Slots

| Slot | Description |
| --- | --- |
| Children | Arranged in one row, one column, or spread into one stack of layers. On the function side this is `.children`. The node pool on a layer is still mounted by you |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
