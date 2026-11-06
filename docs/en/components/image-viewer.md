# ImageViewer

`ImageViewer` is a full-window large-image overlay. It draws the scrim, surface, stage, and caption row. You supply the image.

There is no `<ImageViewer>` in the control table.

## Basic usage

The content of `ImageViewer::new(content)` is an `ImageViewerContent`: `None`, `Child(id)`, `HostTexture(slot name)`, or `CustomRender`.

The convenience constructors are `ImageViewerContent::host_texture(slot)` and `ImageViewerContent::child(id)`.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::{ImageViewer, ImageViewerContent};

view! {
    <Widget
        of={ImageViewer::new(ImageViewerContent::host_texture("shot"))
            .name("Screenshot")
            .gallery(2, 9)
            .close_label("Close")}
    />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{ImageViewer, ImageViewerContent};

widget(
    ImageViewer::new(ImageViewerContent::host_texture("shot"))
        .name("Screenshot")
        .gallery(2, 9)
        .close_label("Close"),
)
```

:::

## Feature

This type is behind the Cargo feature `image-viewer`.

`components` turns it on.

## Scrim

The scrim uses the theme's `EffectTokens::media_scrim`. It is dark even in light themes, so images are always viewed on a dark background; the surface and caption row still follow the theme.

## Caption and original size

`.name` and `.metadata` appear in the caption row below the stage.

`.intrinsic_size(width, height)` records the original image's pixel size when both sides are greater than 0; it is used to fit the image into the stage.

## Gallery

The `index` in `.gallery(index, count)` starts at 0.

With more than one image, "Previous", "Next", and the position "3 / 9" appear at the bottom of the stage; the button on the side that has reached the end is disabled.

With only one image, or without `gallery`, no navigation is shown.

You switch images: on `Previous` or `Next`, replace the content and write the new position back to `gallery`.

Whether zoom and pan reset is also up to you.

While focus is on the viewer or one of its controls, the left and right arrow keys emit the same requests; at the ends, the arrow keys are not consumed.

## Close button

The close button is built by `assemble_image_viewer` as an `IconButton`. It is focusable, its default name is "Close", and `.close_label` replaces it.

The default names of the previous and next buttons are "Previous" and "Next".

These controls are always ordered after the other child nodes, so even a `Child` that fills the frame cannot cover them.

When content is added after the controls, the viewer reassembles and moves the controls back to the end.

## Assembly

It is a leaf compound: `assemble_image_viewer` runs when the view is built and after every write.

A viewer created by hand with `create_component` calls it once itself.

## Overlay

The `close` of `ImageViewer::geometry` gives the close button's position, so your content can leave room for it.

Escape goes through the shared overlay dismissal.

Open and close it with `activate_overlay` and `dismiss_overlay` on the host; the viewer must be a direct child of an `OverlayHost`.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `content` | `ImageViewerContent` | `None`, `Child(id)`, `HostTexture(slot name)`, or `CustomRender`. The convenience constructors are `host_texture(slot)` and `child(id)` |
| `.name` | — | Appears in the caption row below the stage |
| `.metadata` | — | Appears in the caption row below the stage |
| `.intrinsic_size` | — | `(width, height)` records the original pixel size when both sides are greater than 0; used to fit the image into the stage |
| `.gallery` | — | `(index, count)`; `index` starts at 0 |
| `.close_label` | — | Replaces the close button's default name "Close" |

## Events

The event type is `ImageViewerEvent`: `Close`, `Outside`, `Interaction`, `Previous`, `Next`.

| Event | Payload | Description |
| --- | --- | --- |
| `ImageViewerEvent::Close` | — | `ImageViewerEvent` |
| `ImageViewerEvent::Outside` | — | `ImageViewerEvent` |
| `ImageViewerEvent::Interaction` | — | `ImageViewerEvent` |
| `ImageViewerEvent::Previous` | — | You switch images: on receipt, replace the content and write the new position back to `gallery` |
| `ImageViewerEvent::Next` | — | You switch images: on receipt, replace the content and write the new position back to `gallery` |

## Slots

| Slot | Description |
| --- | --- |
| None | None |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
