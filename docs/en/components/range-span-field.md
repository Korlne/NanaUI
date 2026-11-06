# RangeSpanField

Two thumbs enclose a span, and the track can also mark a live value. The template tag is `<RangeSpan>`, the function is `range_span(min, max, step)`, and the type is `RangeSpanField`. All three parameters are `f64`. The starting span is the whole range: the function calls `RangeSpanField::new(minimum, maximum, minimum, maximum, step)` internally.

## Basic usage

:::api

```rust view
use nana_ui::runtime::{view, RangeSpanOrientation};

view! {
    <RangeSpan min=0 max=1 step=0.01 orientation={RangeSpanOrientation::Vertical}
        low_label="Input low" high_label="Input high" indicator={live} v-model={span} />
}
```

```rust rust
use nana_ui::runtime::{view::range_span, RangeSpanOrientation};

range_span(0.0, 1.0, 0.01)
    .orientation(RangeSpanOrientation::Vertical)
    .low_label("Input low")
    .high_label("Input high")
    .indicator(live)
    .model(span)
```

:::

`span` is a `Signal<(f64, f64)>` holding the low then the high bound. `v-model={span}` expands to `.model(span)`, listens to `RangeSpanInput`, and also writes back the preview during a drag. `live` is a `Signal<Option<f64>>`; it only drives the display and is never written back.

The two thumbs can also be bound separately: `low={…}`, `high={…}`. When written separately, one side crossing the other pushes the other along instead of stopping there, so the result is the same whichever binding lands first. `span` writes both sides at once and sorts them if they are reversed.

## Orientation

Horizontal by default: the minimum is on the left. `orientation={RangeSpanOrientation::Vertical}` (or `RangeSpanField::vertical()`) makes it vertical: the minimum is at the bottom, the maximum at the top, and Up increases the value.

When horizontal, the width fills by default and the height is at least the control height of the `size` step. When vertical, the width by default is just enough for the thumb and the focus ring, and the height fills; in a column with no definite height, give it one yourself:

```rust
let mut field = RangeSpanField::new(0.2, 0.8, 0.0, 1.0, 0.01).vertical();
Arc::make_mut(&mut field.style.layout).height = Some(LengthSpec::Px(180.0));
```

## Track inset

The minimum and maximum are not at the edges of the node; each is inset by `range_span_track_inset(size)` (on an instance, `track_inset()`): half a thumb plus the focus ring, so the ring still sits inside the node when a thumb rests at either end. The control has no padding and no border of its own, so this distance is measured from the node edge. To draw ticks or a live level beside it, align to the same formula: the position of a value `v` on the main axis is `inset + (v - min) / (max - min) * (length - 2 * inset)`, measured from the bottom edge when vertical.

## Look

Between the two thumbs is a solid accent-colored (`Accent`) bar about two thirds the thickness of the track, centered and rounded at both ends; outside the span is a neutral track (`BorderStrong`). The thumbs match those of [RangeField](https://github.com/sena-nana/NanaUI/blob/main/docs/components/range-field.md). When disabled, the track turns `Border`, the span turns `Faint`, and the thumbs turn the disabled color.

## Value thumbs

`value_thumbs={true}` (or `RangeSpanField::value_thumbs(true)`) draws the two thumbs as accent-colored capsules that carry their own value, the same as `value_thumb` on [RangeField](https://github.com/sena-nana/NanaUI/blob/main/docs/components/range-field.md): both capsules are the same width, wide enough for the minimum, the maximum and both thumb values, and the height is `range_span_value_thumb_height(size, typography, spacing)`. When vertical, the default control width is the capsule plus the focus ring, the track inset becomes half the capsule height plus the focus ring, and anything aligned beside it uses `range_span_value_track_inset(size, typography, spacing)` (take the theme from `cx.theme()` on the paint context). The live indicator is drawn below the capsule and never covers the digits.

Pressing on a capsule and moving no more than a short distance before releasing is a click: the thumb does not move, a number input replaces the capsule in place, selects the current value and takes focus; moving further is a drag. With focus on the capsule, Enter, Space or an accessibility press do the same. Enter commits, Esc restores, and afterwards focus returns to that thumb; focus leaving the control also ends entry. On commit it first emits `RangeSpanEntered { thumb, value }`: which thumb was entered and with what value (clamped into range, snapped to the step, but not held at the other thumb), then emits `RangeSpanInput` / `RangeSpanChanged` with the pair of that value and the other thumb, swapping the two when it crosses the other thumb. When the application saves this pair in some other orientation (allowing the pair to be inverted, say), listen to `RangeSpanEntered` to know which end changed.

## Live indicator

`indicator` is a dot on the track; it does not snap to the step, does not respond to input, and is painted above the thumbs. A value outside the range is drawn at the nearer end, and a non-finite value is not drawn. Changing only it neither lays out nor touches any other node: the whole track of the control is one self-drawn node, and only that node's paint key changes. From the program, `cx.set_range_span_indicator(field, Some(v))` does the same.

## Visible values and commits

`RangeSpanInput { low, high }` is every visible value, including the preview during a drag. `RangeSpanChanged { low, high }` is a commit: the pointer released with a thumb moved, a keyboard step, an accessibility increment, decrement or set value, or `set_range_span` / `set_range_span_thumb`. A canceled drag does not commit and restores the pair from before the press. Drag start and end have their own `RangeSpanDragging { dragging }`; keyboard and accessibility steps do not count as dragging. The three correspond one to one with `RangeInput` / `RangeChanged` / `RangeDragging` on [RangeField](https://github.com/sena-nana/NanaUI/blob/main/docs/components/range-field.md).

## Pointer

Pressing on a thumb grabs that thumb; it does not jump under the pointer. Pressing on the track moves the nearer thumb to the pointer (exactly halfway takes the low bound). When both thumbs sit on the same value, whichever way the first move goes decides which thumb follows; on the minimum only the high bound can move, on the maximum only the low bound. During a drag the control captures the pointer and the moving thumb takes focus. The thumbs never cross each other: dragging past the other stops there.

## Keyboard and screen reader

Each thumb is a focus stop, and Tab reaches the low bound then the high bound. Arrow keys step by `step`, PageUp / PageDown by `page_step` (ten times `step` by default), and Home / End go to the ends the thumb can reach — End on the low bound stops at the high bound's value. While focus is visible there is a focus ring around the thumb.

In a screen reader each thumb is a `Slider` named `low_label` / `high_label`, or `label` when those are absent. The value range follows the multi-thumb convention: the low bound's maximum is the current high bound, and the high bound's minimum is the current low bound. `Increment`, `Decrement` and `SetValue` are supported. The whole control is a group with `label`. Reach a thumb node with `thumb_node(RangeSpanThumb::Low)`.

## Bounds

`RangeSpanField::new` repairs inconsistent bounds and steps the way `RangeField::new` does. A non-finite low or high bound becomes the minimum or maximum; both are clamped, snapped to the step, and swapped if reversed. With a short decimal step, values land exactly on the grid (`0.6`, not `0.6000000000000001`).

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `low` / `high` | `f64` | The two thumbs. Writing one side from the template pushes the other along |
| `span` | `(f64, f64)` | Template field that writes both sides at once and sorts them if reversed. `v-model` binds it, sourced from `RangeSpanInput` |
| `min` / `max` / `step` | `f64` | The three arguments of `range_span` |
| `indicator` | `Option<f64>` | Live value. No snapping, no response to input; changing only it is one paint update |
| `orientation` | `RangeSpanOrientation` | `Horizontal` (default) or `Vertical` (minimum at the bottom) |
| `label` | `Option<Arc<str>>` | Group name, and the name of any thumb without a name of its own |
| `low_label` / `high_label` | `Option<Arc<str>>` | Screen reader names of the two thumbs |
| `disabled` | `bool` | The thumbs cannot take focus or the pointer; the track and thumbs turn the disabled color |
| `value_thumbs` | `bool` | Draw the thumbs as value capsules that type in place; see "Value thumbs" |
| `page_step` | — | Not in the template field table. Page step, ten times `step` by default |
| `size` | — | Not in the template field table. `ControlSize`, which sets the thumb diameter and the inset |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| `on_input`, template `on:RangeSpanInput` or `@input` | `&RangeSpanInput` | Every visible value, including the preview during a drag |
| `on_change`, template `on:RangeSpanChanged` or `@change` | `&RangeSpanChanged` | Commit: released after a move, keyboard step, accessibility increment, decrement and set, programmatic set. A canceled drag does not commit |
| `.on` listening to `RangeSpanDragging` | `&RangeSpanDragging` | `{ dragging }`. Drag start and end |
| `on_enter`, template `on:RangeSpanEntered` | `&RangeSpanEntered` | `{ thumb, value }`. The thumb and value entered and committed in a value capsule, before the `RangeSpanChanged` it produces |

## Program interface

| Method | Description |
| --- | --- |
| `set_range_span(field, low, high)` | Commit a pair of values; sorts them if reversed |
| `set_range_span_thumb(field, thumb, value)` | Commit one thumb, stopping at the other thumb |
| `adjust_range_span(field, thumb, RangeAdjustment)` | Step one thumb as a key press would |
| `set_range_span_indicator(field, Option<f64>)` | Move only the live indicator |
| `begin_range_span_edit(field, thumb)` | Start entry on this value capsule; does nothing when there are no value capsules or the control is disabled |

## Slots

| Slot | Description |
| --- | --- |
| None | The control assembles the two thumb nodes itself (plus the input field when there are value capsules) |

The control is not registered in the component registry and has no tag of its own; use it only from Rust and `view!`.

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [RangeField](https://github.com/sena-nana/NanaUI/blob/main/docs/components/range-field.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
