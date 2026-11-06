# Avatar

`avatar(size)` creates a round avatar. `size` is an `f32` side length in logical pixels. The template tag is `<Avatar>`.

## Basic usage

Internally it is `Avatar::new("").size(size)`, and the resource starts out empty.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Avatar size=32 resource={photo} label="Avatar" />
}
```

```rust rust
use nana_ui::runtime::view::avatar;

avatar(32_f32).resource(photo).label("Avatar")
```

:::

`size=32` in a template expands to `32_f32`. When written by hand the argument is an `f32`, so integer literals need the suffix. `size` is also a field and can be bound later.

## Sampling

Sampling is Cover, clipped to a circle. An empty `resource`, a slot the host cleared, and a load failure all show the Subtle placeholder; no initials are drawn. On a load failure, the host clears `resource` to empty. This does not change the convention of rejecting the frame for a missing slot.

By default only level 0 is sampled. When the texture has a mip chain, switch to trilinear with `.sampling(ImageSampling::Mipmap)` on the component. This is not a field.

## Changing the image

Changing the image happens only on the host side: register a texture into the same slot again (new pixels, a different view, a re-upload after the device is rebuilt), and every view showing that slot samples the new texture on the next frame without touching the component. The component has no revision number, so one slot can serve several views at once.

## Accessibility

With a `label`, it is a named Image for accessibility. With an empty name it is an Image with no name. The difference from `Thumbnail`: Cover, round, fixed side length, rather than a box that follows the aspect ratio.

## Size

A non-finite or non-positive `size` falls back to the default side length, which is the medium control height. The corner radius is half the side length and overflow is clipped, so it is always round.

## Hit testing

By default it does not take part in hit testing and is not focusable.

To let the avatar receive clicks, use `pointer_events(true)` on the component. It defaults to false, and even then it is still not focusable. This is not a field.

## Properties

The fields are `resource: TextureSlotName`, `size: f32` and `label: Arc<str>`. There are no events and no `model`. By default it does not take part in hit testing and is not focusable.

| Property | Type | Description |
| --- | --- | --- |
| `resource` | `TextureSlotName` | The resource starts out empty. An empty `resource`, a slot the host cleared, and a load failure all show the Subtle placeholder |
| `size` | `f32` | Side length in logical pixels. Also a field and can be bound later. Non-finite or non-positive values fall back to the default side length, which is the medium control height |
| `label` | `Arc<str>` | With a name, it is a named Image for accessibility. With an empty name it is an Image with no name |
| `.sampling` | — | Not a field. By default only level 0 is sampled. `.sampling(ImageSampling::Mipmap)` switches to trilinear |
| `pointer_events` | — | Not a field. Only `pointer_events(true)` receives clicks. Defaults to false, and even then it is still not focusable |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| None | None | There are no events and no `model` |

## Slots

| Slot | Description |
| --- | --- |
| None | None |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
