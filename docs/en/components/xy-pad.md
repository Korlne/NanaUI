# XYPad

`XYPad` is a two-axis pad. Pointer press and move emit `XYPadEvent::Input`; release and arrow keys under focus emit `XYPadEvent::Change`. The value is `XYPadValue { x, y }`, constructed with `XYPadValue::new(x, y)`.

## Basic usage

`XYPad::new(value)` defaults both axes to `0.0..=1.0`, with step `0` meaning no quantization. `.x_range` and `.y_range` change the ranges; an invalid range is narrowed to a valid interval and the current value is clamped into the new range. `.step` accepts only finite numbers greater than 0; anything else means no quantization. `.size` changes the height: small 40, medium 48, large 64, taller than single-line controls. `.label` is the accessible name. `.disabled`, `.loading`, and `.invalid` work like other fields.

The control table has no `<XYPad>`. The saturation/value pad assembled by the color swatch field is also this type.

## Picture pad

`.surface(XYPadSurface::Picture)` treats the pad's own painting as the picture the value is picked from, such as a color picker's saturation/value square. The default `XYPadSurface::Plain` is a neutral background (`Subtle` when the style specifies no fill), a center crosshair, and an accent-colored dot. A picture pad draws no crosshair and uses the style's background color and background layers (`layout.background`, `layout.paint.background_image`, `background_layers`, painted as usual by ordinary box painting) as the picture; the value is marked with a contrasting ring: a light ring between two thin dark edges, hollow in the middle, visible on both light and dark colors, with colors taken from the theme's media foreground and media scrim colors. Background layers would cover the border stroked beneath the surface, so a picture pad strokes the border again above the layers, keeping hover, focus, and `invalid` visible on all four edges. Drag, keyboard, and screen reader semantics are unchanged.

`.height(pixels)` replaces the height given by `.size`, for large value-picking blocks such as picture pads rather than controls in a row. It is ignored when not a finite positive number.

```rust
use nana_ui::runtime::{XYPad, XYPadSurface, XYPadValue};

let mut pad = XYPad::new(XYPadValue::new(0.5, 0.5))
    .surface(XYPadSurface::Picture)
    .height(128.0);
std::sync::Arc::make_mut(&mut pad.style.layout).background = Some([0.2, 0.6, 0.9, 1.0]);
```

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::{XYPad, XYPadValue};

view! {
    <Widget
        of={XYPad::new(XYPadValue::new(0.2, 0.8))
            .x_range(-1.0, 1.0)
            .y_range(-1.0, 1.0)
            .step(0.05)
            .label("Offset")}
    />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{XYPad, XYPadValue};

widget(
    XYPad::new(XYPadValue::new(0.2, 0.8))
        .x_range(-1.0, 1.0)
        .y_range(-1.0, 1.0)
        .step(0.05)
        .label("Offset"),
)
```

:::

## Dragging and keyboard

`inactive` is true when disabled or loading. The vertical axis increases upward: the top of the pad is `y_max` and the bottom is `y_min`. When dragging with Shift held, the axis that moves first is locked (`|dx| >= |dy|` locks the horizontal axis) and the other axis keeps its value from the press. Keyboard steps are `XYPadAdjustment`'s `Left`, `Right`, `Up`, `Down`; each step commits `Change` directly.

Previews during a drag read `Input`. Only release or arrow keys produce `Change`. A canceled drag does not commit. `value_at` converts pad-local coordinates to a clamped, quantized value, so you can check a hit yourself. Each axis's range clamps separately. After you narrow a range, the current value moves into the new interval. When `step` is `0`, non-finite, or not positive, the pad treats values as continuous. Shift locks whichever axis dominates first in the drag; the other axis stays at its value from the press. Height is decided by `.size`: small 40, medium 48, large 64; `.height` can override it.

The saturation/value picker assembled by `ColorField` is also this pad, with the vertical axis likewise increasing upward.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `value` | `XYPadValue { x, y }` | `XYPad::new(value)`. Constructed with `XYPadValue::new(x, y)` |
| `.x_range` | — | Default `0.0..=1.0`. An invalid range is narrowed to a valid interval and the current value is clamped into the new range |
| `.y_range` | — | Default `0.0..=1.0`. The vertical axis increases upward: top is `y_max`, bottom is `y_min` |
| `.step` | — | Default `0` means no quantization. Accepts only finite numbers greater than 0; anything else means no quantization |
| `.size` | — | Height: small 40, medium 48, large 64, taller than single-line controls |
| `.height` | `f32` | Replaces the height given by `.size`. Ignored when not a finite positive number |
| `.surface` | `XYPadSurface` | Default `Plain`: neutral background, crosshair, accent-colored dot. `Picture`: the pad's own background color and background layers are the picture, no crosshair, the value marked with a contrasting ring, and the border stroked above the picture |
| `.label` | — | Accessible name |
| `.disabled` | — | Like other fields. `inactive` is true when disabled |
| `.loading` | — | Like other fields. `inactive` is true when loading |
| `.invalid` | — | Like other fields |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| `XYPadEvent::Input` | — | Pointer press and move. Previews during a drag read it |
| `XYPadEvent::Change` | — | Release, and arrow keys under focus. Commits directly after a keyboard step. A canceled drag does not commit |

## Slots

No slots.

## See also

[Overview](index.md) · [Controls](../reference/components.md)
