# CurveField

A polyline inside a unit square; the points on the line keep a fixed horizontal position and can only be dragged up and down to change their height. Use it to edit shapes such as mapping curves and response curves, where an input of 0–1 corresponds to an output of 0–1. The template tag is `<Curve>`, the function is `curve(step)`, and the type is `CurveField`. `step` is `f64`, and heights land on its grid.

## Basic usage

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Curve step=0.001 ends={Some((0.0, 1.0))} label="Mapping curve"
        point_labels={names} v-model={heights} />
}
```

```rust rust
use nana_ui::runtime::view::curve;

curve(0.001)
    .ends(Some((0.0, 1.0)))
    .label("Mapping curve")
    .point_labels(names)
    .model(heights)
```

:::

`heights` is a `Signal<Vec<f64>>` with one height per point, left to right, all within `0..=1`. The number of points is its length: write a group with a different length and the points are assembled again. `v-model={heights}` expands to `.model(heights)`, listens to `CurveInput`, and also writes back the preview during a drag. `names` is `Vec<Arc<str>>`, the screen reader names of the points.

To build it without a template: `CurveField::new(vec![0.2, 0.4, 0.6], 0.001).ends(0.0, 1.0)`.

## Point positions

Of `n` points, the `k`-th (counting from 0) sits at `x = (k + 1) / (n + 1)`, spread evenly between the two ends; `position(k)` returns this value. Points move only up and down, so drifting sideways during a drag does not change point.

`ends` is the fixed height of the curve at `x = 0` and `x = 1`: the drawn line starts at the left end, passes through every point, and stops at the right end. The ends draw no handle and take no input. Without `ends` the line runs from the first point to the last.

## Size and inset

The width fills by default, and with no height the square follows a 1:1 aspect ratio; for another ratio, give a height or an `aspect_ratio` yourself. The plot square is not at the edges of the node: all four sides are inset by `curve_plot_inset(size)` (on an instance, `plot_inset()`): half a point plus the focus ring, so the ring still sits inside the node when a point rests in a corner. To draw ticks or live values beside it, align to the same formula: a point `(x, y)` sits in the node at `(inset + x * (width - 2 * inset), inset + (1 - y) * (height - 2 * inset))`.

## Look

The square has a thin `Border` frame and `BorderSoft` grid lines dividing it into quarters, and the curve is an accent-colored (`Accent`) polyline with rounded joins. Points match the thumbs of [RangeField](https://github.com/sena-nana/NanaUI/blob/main/docs/components/range-field.md), one step smaller in diameter (`curve_point_extent(size)`). When disabled the curve turns `Faint` and the points turn the disabled color. Moving a point changes only the paint key of the square node and that point's position.

## Visible values and commits

`CurveInput { point, values }` is every visible value, including the preview during a drag. `CurveChanged { point, values }` is a commit: the pointer released with a point moved, a keyboard step, an accessibility increment, decrement or set value, or `set_curve_point` / `set_curve_values`. `point` is the point that moved; it is `None` when the whole group was replaced. A canceled drag does not commit and restores the height from before the press. Drag start and end have their own `CurveDragging { dragging }`; keyboard and accessibility steps do not count as dragging. The three correspond one to one with `RangeSpanInput` / `RangeSpanChanged` / `RangeSpanDragging` on [RangeSpanField](https://github.com/sena-nana/NanaUI/blob/main/docs/components/range-span-field.md).

## Pointer

Pressing on a point grabs that point; it does not jump under the pointer. Pressing elsewhere in the square moves the nearest point horizontally to the pointer's height (exactly halfway between two points takes the left one). During a drag the control captures the pointer and the moving point takes focus. Heights are clamped to `0..=1`.

## Keyboard and screen reader

Each point is a focus stop, and Tab walks from left to right. With focus on a point, Left / Right move to the neighbouring point (stopping in place at either end, and the control still consumes the press); Up / Down change the height by `step`, PageUp / PageDown by `page_step` (ten times `step` by default), and Home / End go to 0 and 1. While focus is visible there is a focus ring around the point.

In a screen reader each point is a vertical `Slider`: the name is the matching entry in `point_labels`, or its ordinal number ("1", "2", …) when there is none; the value is read as a percentage (`35%`, with the decimals following the step), and the value range is 0 to 1. `Increment`, `Decrement` and `SetValue` are supported, and `SetValue` accepts both `"35%"` and `"0.35"`. The whole control is a group with `label`. Reach a point node with `point_node(k)`.

## Bounds

`CurveField::new` repairs inconsistent input: a non-finite or non-positive step becomes `0.01`, and a step above 1 is treated as 1; every height is clamped into `0..=1` and snapped to the step, and a non-finite height is 0. With a short decimal step, values land exactly on the grid (`0.6`, not `0.6000000000000001`). `set_curve_values` / `set_curve_point` return an error for a non-finite value.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `values` | `Vec<f64>` | The height of each point, left to right. Written from the template it is clamped and snapped to the step, and its length sets the point count. `v-model` binds it, sourced from `CurveInput` |
| `step` | `f64` | The argument of `curve` |
| `ends` | `Option<(f64, f64)>` | Fixed height of the curve at the two ends |
| `label` | `Option<Arc<str>>` | Group name |
| `point_labels` | `Vec<Arc<str>>` | Screen reader names of the points |
| `disabled` | `bool` | The points cannot take focus or the pointer; the curve and points turn the disabled color |
| `page_step` | — | Not in the template field table. Page step, ten times `step` by default |
| `size` | — | Not in the template field table. `ControlSize`, which sets the point diameter and the inset |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| `on_input`, template `on:CurveInput` or `@input` | `&CurveInput` | Every visible value, including the preview during a drag |
| `on_change`, template `on:CurveChanged` or `@change` | `&CurveChanged` | Commit: released after a move, keyboard step, accessibility increment, decrement and set, programmatic set. A canceled drag does not commit |
| `.on` listening to `CurveDragging` | `&CurveDragging` | `{ dragging }`. Drag start and end |

## Program interface

| Method | Description |
| --- | --- |
| `set_curve_values(field, values)` | Commit the whole group of heights; the length may change |
| `set_curve_point(field, k, value)` | Commit the height of one point |
| `adjust_curve_point(field, k, RangeAdjustment)` | Step one point as a key press would |

## Slots

| Slot | Description |
| --- | --- |
| None | The control assembles the point nodes itself |

The control is not registered in the component registry; use it from Rust and `view!`.

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [RangeSpanField](https://github.com/sena-nana/NanaUI/blob/main/docs/components/range-span-field.md) · [RangeField](https://github.com/sena-nana/NanaUI/blob/main/docs/components/range-field.md) · [TimelineEditor](https://github.com/sena-nana/NanaUI/blob/main/docs/components/timeline-editor.md)
