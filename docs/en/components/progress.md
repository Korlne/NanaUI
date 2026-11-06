# Progress

`progress(max)` creates a progress bar: determinate, or indeterminate when you do not know how long is left. `max` is an `f64` given at construction. The function's documentation asks you to bind `.value(…)`.

The tag in templates is `<Progress>`.

## Basic usage

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Progress max=100 value={done} label="Export" />
}
```

```rust rust
use nana_ui::runtime::view::progress;

progress(100.0).value(done).label("Export")
```

:::

`max=100` in a template expands to `100_f64`. When written by hand the argument is `f64`, so write `100.0`. `value` can be a constant, a `Signal<f64>` or a closure.

## Values

The displayed value is clamped between `0` and `max`. A non-finite `value` counts as `0`. When `max` is not a finite positive number, the component collapses it to a tiny positive number instead of dividing by zero. Accessibility reports the percentage, the minimum, the maximum and the current value together.

The accessibility value is the rounded percentage, together with the minimum, the maximum and the current value. The fill ratio is the clamped `value / max`.

`value` can follow a signal. A determinate progress bar shows only the number you write and adds no animation phase of its own.

## Indeterminate progress

When you do not know how long is left, write `indeterminate`. The track no longer has a fill standing for a value; instead a segment about 35% of the track comes in at the start and leaves at the end, pass after pass.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Progress max=1 indeterminate label="Connecting" />
}
```

```rust rust
use nana_ui::runtime::view::progress;

progress(1.0).indeterminate(true).label("Connecting")
```

:::

This sweep is a looping timeline the component starts itself; the host does not need to drive it. One pass reads the installed theme's skeleton pulse (`MotionRole::SkeletonPulse`, 1.4 seconds by default). It stops when hidden, changed back to determinate, or unmounted, and carries on sweeping when mounted again.

With reduced motion on it does not sweep: a very light fill color (35% of the fill color) covers the whole track, so it does not read as "already finished" at a glance. The change starts or stops at the moment the preference switches, without waiting for the next data change.

Accessibility reports no value, and no minimum or maximum either; it only marks the control busy, like `<progress>` without a `value`. Switching `indeterminate` back to `false` brings the percentage and value back.

## Label and track

With a label, the label sits above the track. The label may be omitted. Without a label and without cancellable behaviour, the height is the track itself. The progress bar does not run a clock of its own; the application writes the value.

The track is 6 thick by default with a Subtle background; the fill uses the theme's Indicator recipe, Accent by default. All three can be changed: `thickness` sets the thickness (6 still applies when it is not positive), and `track` and `fill` set semantic colors. The corner radius is always half the thickness.

By default the bar fills the row it is in; a width written in the style (`layout.width` of `style`) is used as written. This is how an assembled toast lets the bar span the text column instead of stretching the toast to the region's full width.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::theme::SemanticColorRole;

view! {
    <Progress max=1 value={ratio} thickness={3.0} track={SemanticColorRole::BorderSoft} fill={SemanticColorRole::Accent} />
}
```

```rust rust
use nana_ui::runtime::view::progress;
use nana_ui::theme::SemanticColorRole;

progress(1.0)
    .value(ratio)
    .thickness(3.0)
    .track(SemanticColorRole::BorderSoft)
    .fill(SemanticColorRole::Accent)
```

:::

`label` has the type `Option<Arc<str>>`. Without it there is no title. With a title the font weight is medium.

## Cancellable

Cancellable is not a field. Only `Progress::cancellable(true)` on the component takes the pointer and is focusable. On activation, if `cancellable` is still true, it emits `ProgressCancelled`; the progress bar itself is neither removed nor stopped. Listen for this event with `.on(|_: &ProgressCancelled| …)`. `<Progress>` has no corresponding `on_` method. The progress control owns no timer.

## Maximum

`max` is only a constructor argument, not a field that can be bound later. The view function has no `.max`. To change the ceiling, use `.bind(|progress| progress.max = next)`, or construct it again.

## Properties

The bindable fields are `value`, `label`, `indeterminate`, `thickness`, `track` and `fill`. There is no `model`.

| Property | Type | Description |
| --- | --- | --- |
| `value` | `f64` | Can be a constant, a `Signal<f64>` or a closure. The display is clamped between `0` and `max`. Non-finite counts as `0`. Can follow a signal |
| `label` | `Option<Arc<str>>` | May be omitted. With a label it sits above the track, in medium weight. Without it there is no title |
| `max` | `f64` | Given at construction, not a field that can be bound later. The view function has no `.max`. When it is not a finite positive number it collapses to a tiny positive number |
| `indeterminate` | `bool` | When you do not know how long is left: a segment sweeps along the track, or under reduced motion the whole track gets a light fill; accessibility only marks busy and reports no value |
| `thickness` | `Option<f32>` | Track thickness. 6 when not written or not positive |
| `track` | `Option<SemanticColorRole>` | Track color. Subtle when not written |
| `fill` | `Option<SemanticColorRole>` | Fill color. When not written, the theme's Indicator recipe applies |

## Events

No events, and no `model`. On activation, if `cancellable` is still true, it emits `ProgressCancelled`.

| Event | Payload | Description |
| --- | --- | --- |
| `.on` | `&ProgressCancelled` | Cancellable is not a field. Only `Progress::cancellable(true)` takes the pointer and is focusable. The progress bar is neither removed nor stopped. `<Progress>` has no corresponding `on_` method. The progress control owns no timer |

## Slots

| Slot | Description |
| --- | --- |
| None | None |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
