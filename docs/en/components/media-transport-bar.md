# MediaTransportBar

`MediaTransportBar` is a media transport bar. The framework provides play, VOD or live progress, a volume popup, a settings menu and fullscreen. Scene-specific controls go into the slots.

The control table has no `<MediaTransportBar>`.

## Basic usage

`MediaTransportBar::new()` defaults to regular density, pinned to the parent's bottom edge, volume 100, and a draggable progress range.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::{MediaTransportBar, MediaTransportDensity, MediaTransportPlacement};

view! {
    <Widget
        of={MediaTransportBar::new()
            .density(MediaTransportDensity::Regular)
            .placement(MediaTransportPlacement::Overlay)
            .show_play(true)}
    />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{MediaTransportBar, MediaTransportDensity, MediaTransportPlacement};

widget(
    MediaTransportBar::new()
        .density(MediaTransportDensity::Regular)
        .placement(MediaTransportPlacement::Overlay)
        .show_play(true),
)
```

:::

## Density

`.density` is a `MediaTransportDensity`: `Regular` puts the merged time readout above the progress and may open a second row; `Compact` collapses to one row and hides settings and fullscreen by default; `Stacked` keeps the same buttons as compact, with the current time, progress and total duration laid out in order on one full row above, so the time readout never squeezes the progress range; `Mini` is the compact-window player.

### Mini

`MediaTransportBar::new().mini()` (`Mini` plus `Inline`) is for compact windows; the bar owns the whole layout:

- The progress is a 2 px thin rail along the bar's top edge (`RangeField::rail`), wrapped in a transparent 16 px hit band centered on it. The hit band reaches over the content above the bar (the picture), so pressing and dragging there seeks; the part that reaches into the button row is given up to the buttons. For live playback the thin rail becomes the live progress meter.
- The row below is, in order: play, the mute button, a volume rail with no dot, the `leading` slot, then the merged time readout and the `trailing` slot. There is no volume popup; settings and fullscreen are hidden by default and can be enabled with `show_settings` / `show_fullscreen`.
- Pressing the mute button emits `MediaTransportEvent::Mute(the desired mute state)`, that is `!muted`. The icon switches to the muted icon when `muted` or the volume is 0, and the name says what pressing will do (`mute_label` / `unmute_label`).
- The play overlay on the picture is still the application's own; use `show_play(false)` if you do not want the play button on the bar.

```rust
widget(MediaTransportBar::new().mini().show_play(false).icons(icons))
    .leading(extra_buttons)   // after the volume rail
    .trailing(window_actions) // after the time readout
    .bind(move |bar| playback.with(|p| p.paint(bar)))
    .on(move |event: &MediaTransportEvent| match event {
        MediaTransportEvent::Mute(muted) => set_muted(*muted),
        _ => {}
    });
```

Applications must not move nodes inside the bar, or change their size or opacity: when `density` switches between `Mini` and the other tiers, the bar puts progress, time and volume back in their own places.

## Placement

`.placement` is `MediaTransportPlacement::Overlay` or `Inline`.

Overlay positions the bar absolutely along the containing block's bottom edge, and only the bar receives hits; Inline takes part in the parent's layout and ignores `max_width`. An Overlay bar hangs off the nearest positioned ancestor, so the parent of the picture area must set `position: relative` (see [Layout positioning](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/layout.md#能用的)).

## Visibility

`.show_play(false)` hides the play button; a hidden button takes no space and cannot be focused.

`.show_settings` and `.show_fullscreen` follow the density when `None`.

## Playback state

`.playing`, `.position`, `.duration`, `.volume`, `.live`, `.muted` and `.seekable` are playback state.

After `update_component` writes those fields, the playback state, progress and time readouts are already synced, so a playback tick does not need to call sync again.

The bar's own `hidden` only means transport is unavailable; an idle hide does not write this field.

## Assembly

`assemble_media_transport_bar` is the leaf assembly: when the view is built, and after every write, it rebuilds the slots and wires them.

The second row collapses when `secondary` has no visible children.

Theater, separate window and stop playback in the settings menu go into `.settings`.

## Menus and idle

Opening and closing menus calls `AppContext::sync_overlay_visibility`; an open menu keeps the bar around, and once it closes the idle timer restarts from that moment.

### Auto-hide

`.auto_hide(true)` lets the runtime drive idle hiding itself, so the host need not forward input or schedule timers:

- During playback (`playing` and not `disabled`) the bar hides after `OVERLAY_IDLE` (3 seconds) without activity. Activity means the pointer moving, pressing or releasing on the bar's parent (the picture), or any key press anywhere in the document.
- The pointer leaving the bar's parent (the picture area) or the window hides the bar at once, including while paused; returning to the picture or pressing a key brings it back at once. The picture's letterbox and the controls layered inside it belong to that area; comments, chat and similar content outside it do not.
- Open menus, drags (seek, volume) and keyboard focus inside the bar keep it visible; focus left behind by a click does not count, otherwise the bar would never hide again after a button is clicked once.
- A menu or drag that crosses the picture boundary keeps the bar visible; once the protection ends it hides at once if the pointer is still outside the area. Drag capture does not change the area boundary, which is judged by the actual coordinates.
- Every visibility flip emits `OverlayVisibilityChanged { visible }`. Other chrome on the picture (the title bar) follows it.

Such a bar must not also call `sync_overlay_visibility` / `reveal_overlay`: those use the host's clock, and mixing it with the runtime's clock is meaningless. A manually driven bar also emits `OverlayVisibilityChanged`; `conceal_overlay` is the manual “pointer left”.

A video-only window looks like this:

```rust
let chrome = signal(true);
view! {
    <Widget of={Stack::fill_column(0.0)}>
        <Widget of={GpuTextureView::new("").contain()} />
        <Widget
            of={MediaTransportBar::new().compact_overlay().auto_hide(true)}
            on:OverlayVisibilityChanged={move |e: &OverlayVisibilityChanged| {
                chrome.set_if_changed(e.visible);
            }}
        />
        <Widget of={AppTitleBar::new("").over_media(true)} v-show={chrome.get() && !fullscreen.get()} />
    </Widget>
}
```

With the bar under the same parent as the picture, the pointer counts as “on the picture” anywhere in the window. The title bar uses [`over_media(true)`](https://github.com/sena-nana/NanaUI/blob/main/docs/components/app-title-bar.md#叠在媒体上), so buttons and the title stay legible over the picture in light themes. The window's aspect ratio is left to [`content_aspect_ratio`](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/window.md#内容宽高比).

## Time readout

The time readout format is `media_clock`: `m:ss` or `h:mm:ss`.

`Stacked` places the current time and the total duration at the two ends of the progress range; while dragging, the current time shows the preview value. `Regular` and `Compact` keep the merged `current / total` readout. Live playback has no usable endpoint total duration, so it still shows only the live progress meter.

Live playback uses the meter on the progress bar; VOD uses the draggable range; when `seekable` is false the range is still there, just disabled.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `.density` | `MediaTransportDensity` | `Regular` puts the merged readout above the progress and may open a second row; `Compact` collapses to one row and hides settings and fullscreen by default; `Stacked` puts the current time, progress and total duration on one full row above; `Mini` adds a thin top rail plus one row of controls, see above. `new()` defaults to regular density |
| `.mini()` | — | `Mini` plus `Inline` |
| `.placement` | `MediaTransportPlacement` | `Overlay` or `Inline`. Pinned to the parent's bottom edge by default |
| `.show_play` | — | `show_play(false)` hides the play button; a hidden button takes no space and cannot be focused |
| `.show_settings` | — | Follows the density when `None` |
| `.show_fullscreen` | — | Follows the density when `None` |
| `.playing` | — | Playback state |
| `.position` | — | Playback state |
| `.duration` | — | Playback state |
| `.volume` | — | Playback state. Volume 100 by default |
| `.live` | — | Playback state |
| `.muted` | — | Playback state |
| `.seekable` | — | Playback state. When false the range is still there, just disabled |
| `.hidden` | — | Only means transport is unavailable; an idle hide does not write this field |

## Events

Events are `MediaTransportEvent`: `PlayPause`, `Seek(seconds)`, `SeekStarted`, `SeekEnded`, `Volume` (`0..=100`, including drag preview), `Fullscreen`, `Mute(bool)` (only the `Mini` mute button emits it), `MenuOpened`, `MenuClosed`.

`Seek` is committed only on release or a keyboard step.

| Event | Payload | Description |
| --- | --- | --- |
| `MediaTransportEvent::PlayPause` | — | `MediaTransportEvent` |
| `MediaTransportEvent::Seek` | seconds | Committed only on release or a keyboard step |
| `MediaTransportEvent::SeekStarted` | — | `MediaTransportEvent` |
| `MediaTransportEvent::SeekEnded` | — | `MediaTransportEvent` |
| `MediaTransportEvent::Volume` | `0..=100` | Includes drag preview |
| `MediaTransportEvent::Fullscreen` | — | `MediaTransportEvent` |
| `MediaTransportEvent::Mute` | `bool` | The `Mini` mute button: the desired mute state, that is `!muted` |
| `MediaTransportEvent::MenuOpened` | — | Calls `AppContext::sync_overlay_visibility` when a menu opens; an open menu keeps the bar around |
| `MediaTransportEvent::MenuClosed` | — | Once closed, the idle timer restarts from that moment |

## Slots

The view slots are `.leading`, `.trailing`, `.secondary` and `.settings`.

| Slot | Description |
| --- | --- |
| `.leading` | View slot |
| `.trailing` | View slot |
| `.secondary` | The second row collapses when it has no visible children |
| `.settings` | Theater, separate window and stop playback in the settings menu |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md) · [Workspace](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/workspace.md)
