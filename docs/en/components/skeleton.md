# Skeleton

`Skeleton` is a placeholder surface that takes no interaction. Use it to hold space while content has not arrived. It does not receive the pointer and is not focusable.

## Basic usage

There is no `<Skeleton>` in the control table.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::{LengthSpec, Skeleton};

view! {
    <Widget of={Skeleton::new(LengthSpec::Px(160.0), 12.0)} />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{LengthSpec, Skeleton};

widget(Skeleton::new(LengthSpec::Px(160.0), 12.0))
```

:::

For one title line plus two body lines, stack skeletons of different heights in a `Stack::column`. Swap in real text once the data arrives.

## Size

In `Skeleton::new(width, height)` the width is a `LengthSpec` and the height is logical pixels. `Skeleton::fill_width(height)` is the same as a width of `LengthSpec::Fill`. `.width` and `.height` can change them again. Invalid sizes are clamped to usable values.

## Appearance

The background is `Subtle`, the corner radius uses the theme's `radius_sm`, and there is no border.

## Pulse

Once mounted, an infinite Motion IR timeline moves opacity back and forth between 1 and 0.48. A timeline already running does not restart on reprojection. Parked nodes skip the start and begin when mounted. Opacity lives on the compositor layer and is not written into logical layout. To stop it, unmount or park the node.

A skeleton's interaction is off, and it stays out of the Tab order. The pulse goes from opacity 1 to `1 - 0.52`, that is 0.48, bouncing in both directions without end. The timeline id is derived from the node; when it comes out as 0 this pulse does not start. When reprojection finds the timeline still there, it does not set it back to the start.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `width` | `LengthSpec` | The width of `Skeleton::new`. `.width` can change it again. `fill_width` is the same as `LengthSpec::Fill`. Invalid sizes are clamped to usable values |
| `height` | — | Logical pixels. `.height` can change it again. Invalid sizes are clamped to usable values |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| None | None | It does not receive the pointer and is not focusable, and stays out of the Tab order |

## Slots

| Slot | Description |
| --- | --- |
| None | None |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
