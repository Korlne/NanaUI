# Texture

`texture()` shows a host texture inside a layout box. The template tag is `<Texture>` and the type is `GpuTextureView`. It takes no constructor arguments.

## Basic usage

Use it for video frames and previews; it does not load web pages. The fields are `resource: TextureSlotName`, `opacity: f32` and `corner_radius: f32`. There are no events and no `model`. `resource` is the slot name the host registered the texture under; a decimal id also works, as long as the host registered it under that key.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Texture resource={frame} corner_radius=8 />
}
```

```rust rust
use nana_ui::runtime::view::texture;

texture().resource(frame).corner_radius(8_f32)
```

:::

`corner_radius=8` in a template expands to `8_f32`. Hand-written `f32` literals need the suffix. `opacity` is also an `f32` and is fully opaque when not written. An empty `resource` mounts no scene node, so a blank identity is not rejected.

## Changing the image

Changing the image happens only on the host side: register a texture into the same slot again (new pixels, a different view, a re-upload after the device is rebuilt), and every view showing that slot samples the new texture on the next frame without touching the component. The component has no revision number, so one slot can serve several views at once.

By default only level 0 is sampled, and the host prepares the size per `painted_extent`. When the texture has a mip chain, `.sampling(ImageSampling::Mipmap)` switches to trilinear.

## Fitting and hit testing

It fills the box by default (`ContentFit::Fill`) and does not take part in hit testing. To receive hits, use `with_pointer_events` on the component. To keep the whole image with margins, use `contain`. Neither is in the field table. The corner radius field applies one radius to all four corners; for a shape with only some corners rounded, use `with_corner_radii` on the component.

During projection opacity is clamped to 0 to 1, and a non-finite value counts as 1. A non-finite or negative corner radius counts as 0. The checkerboard backdrop and zoom are on the component: `checkerboard`, `zoom`. A `zoom` below 1 or non-finite counts as 1, which means fit to size rather than shrink inside the box and leave a ring around it.

## Web pages and the scene key

`GpuTextureView` is not a browser. In-app web pages use `runtime::BrowserView`, which is not an alias of this type. Today only macOS `WKWebView` is available; Windows and Linux are explicitly unsupported.

The scene key is `"nana.host-texture"`, not another painter and not `"gpu-texture-view"`. `Thumbnail` and `Avatar` use the same registration.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `resource` | `TextureSlotName` | The slot name the host registered the texture under; a decimal id also works, as long as the host registered it under that key. An empty one mounts no scene node, so a blank identity is not rejected |
| `opacity` | `f32` | Fully opaque when not written. During projection it is clamped to 0 to 1, and a non-finite value counts as 1 |
| `corner_radius` | `f32` | One radius for all four corners. `corner_radius=8` in a template expands to `8_f32`. A non-finite or negative value counts as 0 |

## Events

There are no events and no `model`.

## Slots

There are no slots. The picture comes from the host texture `resource` points at.

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
