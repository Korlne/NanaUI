# Thumbnail

`thumbnail()` creates a cover slot for a host texture. The tag in templates is `<Thumbnail>`. It takes no constructor argument; internally it is `Thumbnail::new("")`.

## Basic usage

An empty `resource` is the empty state and samples nothing. The template fields are `resource`, `aspect`, `label`, `decorative`, `pickable` and `intrinsic_size`. When pickable, there is an `@pick` event; there is no `model`.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Thumbnail resource={cover} aspect=1.5 label="Cover" />
}
```

```rust rust
use nana_ui::runtime::view::thumbnail;

thumbnail().resource(cover).aspect(1.5_f32).label("Cover")
```

:::

`aspect=1.5` in a template expands to `1.5_f32`. When written by hand the field is `f32`, so the literal needs the suffix. Without `aspect` it is 1. The box is the control height times this ratio. An explicit width and height, constraints and corner radius from the style win. Only when no radius is written does it take `Xs`. A non-finite or non-positive `aspect` falls back to 1.

The default fit is `ContentFit::Contain`. For cover cropping use `Thumbnail::fit(ContentFit::Cover)` on the component, not a field. Empty, loading, ready and unavailable share this one set of layout dimensions.

## State

The state is `ThumbnailState` on the component: `Empty`, `Loading`, `Ready`, `Unavailable`. It is not in the field table. When `resource` is non-empty after trimming whitespace, it is Ready at construction; otherwise Empty. `loading()` and `unavailable()` mark the other two states. All four share the box, so layout does not jump while loading.

The loading spinner is centered and its edge length is twice that of a compact `Spinner`.

## Swapping the image

Swapping happens only on the host side: register a texture again on the same slot (new pixels, a different view, a re-upload after device rebuild), and every view showing that slot samples the new texture on the next frame without touching the component. The component has no revision number, so one slot can serve several views at once.

By default only layer 0 is sampled, and the host prepares the size from `painted_extent`. With a mip chain, `.sampling(ImageSampling::Mipmap)` on the component switches to trilinear.

## Hitting, badge and name

By default the pointer does not take part in hitting; hitting belongs to the parent row.

## Picking on the image

With `pickable` on, the control takes the pointer itself and reports `ThumbnailPick` in the image's own coordinates: `(0, 0)` is the image's top left, `(1, 1)` the bottom right, independent of box size and fit. The host uses this point to look up its own pixel data (for example an ownership map in image pixels to find the region under the point).

- `Hover(Some(point))`: the pointer moves over the image, reported on every move.
- `Hover(None)`: the pointer leaves the image (entering a Contain letterbox, leaving the box, or a press being cancelled), reported once per leave.
- `Press { point, modifiers }`: the primary button goes down on the image. A press on the letterbox is not reported and does not take the pointer; it goes to the parent.

The Contain letterbox can only be computed with the image's pixel size, so the host writes the slot texture's width and height through `intrinsic_size`. Without it the whole box is treated as the image, which is accurate only for `Fill` or when the box has the image's ratio. Whatever Cover crops away is not pickable.

The control does not take focus. For pointer picking, give a separate keyboard-reachable list (for example per-item toggles).

:::api

```rust view
use nana_ui::runtime::{ThumbnailPick, view};

view! {
    <Thumbnail resource={page} pickable=true intrinsic_size={Some((w, h))}
        label="Texture page" @pick={move |pick: &ThumbnailPick| on_pick(*pick)} />
}
```

```rust rust
use nana_ui::runtime::ThumbnailPick;
use nana_ui::runtime::view::thumbnail;

thumbnail()
    .resource(page)
    .pickable(true)
    .intrinsic_size(Some((w, h)))
    .label("Texture page")
    .on_pick(move |pick: &ThumbnailPick| on_pick(*pick))
```

:::

On the component the form is `Thumbnail::new(page).intrinsic_size(w, h).pickable()`; `Thumbnail::picture_point` provides the same conversion for the host's own tests or other input paths.

For a badge use `Thumbnail::badge()`: bottom right, not hit, solid, clipped inside this box's corner radius. The badge is `Thumbnail::badge()`, which also turns off hitting on itself.

With a `label` the accessibility node is a named Image. An empty name means no name. The difference from `Avatar`: here the default is Contain and the box follows the height and aspect ratio, not a fixed edge length in a circle.

A cover layer that is repeated or decorative can use `Thumbnail::decorative()`, or `decorative=true` in a template. It still paints and takes part in layout, but is removed from the accessibility tree, equivalent to `aria-hidden="true"`; only the frontmost cover layer should keep a `label`.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `resource` | `Arc<str>` | Empty is the empty state and samples nothing. Non-empty after trimming whitespace means Ready at construction, otherwise Empty |
| `aspect` | `f32` | 1 when not written. `aspect=1.5` in a template expands to `1.5_f32`. Non-finite or non-positive falls back to 1. The box is the control height times this ratio |
| `label` | `Arc<str>` | With a `label` the accessibility node is a named Image. An empty name means no name |
| `decorative` | `bool` | Still paints and takes part in layout, but is removed from the accessibility tree; for repeated or decorative layers |
| `pickable` | `bool` | The control takes the pointer itself and reports `@pick` in image coordinates. Off by default, hitting belongs to the parent |
| `intrinsic_size` | `Option<(u32, u32)>` | Pixel size of the slot texture, used to compute the fitted image position. Without it the whole box is the image |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| `@pick` | `ThumbnailPick` | Only emitted when `pickable`: hover, leave and primary-button press on the image, with coordinates in `0..=1` inside the image |

There is no `model`.

## Slots

No slots. The picture comes from the host texture the `resource` points at.

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
