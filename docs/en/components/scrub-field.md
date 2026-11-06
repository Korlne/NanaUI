# ScrubField

`ScrubField` is the "drag left and right on the number to change it" field found in the property panels of professional tools. DCC, CAD, and audio software property panels all use this kind of field. It has no template tag; put it in a view with `widget(ScrubField::new(value))`.

## Basic usage

```rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{ScrubChanged, ScrubField, ScrubInput};

widget(
    ScrubField::new(1.0)
        .range(0.0, 10.0)
        .step(0.1)
        .precision(2)
        .label("X")
        .unit("px"),
)
.on(|e: &ScrubInput| { /* preview while dragging */ })
.on(|e: &ScrubChanged| { /* commit: write to the undo history */ })
```

The field draws `label` on the left and "value unit" on the right.

## Dragging

- Press and drag left or right to change the value. Each 1 px of drag changes the value by `drag_rate`; when it is unset, by one `step` per pixel.
- Holding Ctrl (Cmd on macOS) multiplies the change by 0.1, and holding Shift multiplies it by 10. You can switch at any time during the drag.
- The value is rounded to `precision` and clamped to the bounds of `range`.
- A pointer movement under 2 px counts as a click, not a drag.

During a drag, each change in value emits `ScrubInput { value }`, and releasing emits `ScrubChanged { value }` once. The application uses this to write only commits to its undo history. The start and end of a drag also emit `ScrubDragging { dragging }`. A canceled press (the system takes the pointer away) reverts the value to what it was before the drag, emits `ScrubInput` once more, and does not emit `ScrubChanged`.

The field leaves the wheel to the panel it is in and does not handle the wheel itself.

## Typing

Releasing without dragging, or pressing Enter or Space, puts the field into typing mode:

- The assembled `NumberInput` shows the current value, selects all of it, and takes focus.
- Enter or losing focus commits and emits `ScrubChanged`.
- Escape restores the value and emits no event.

After typing ends, focus returns to the field. `editing` tells whether the field is in typing mode.

## Keyboard stepping

While the field has focus:

| Key | Effect |
| --- | --- |
| ↑ / → | Add one `step` |
| ↓ / ← | Subtract one `step` |
| PageUp / PageDown | Add / subtract ten `step`s |
| Home / End | Jump to the lower / upper bound (only when bounds are set) |

Each step is one commit. Screen reader `Increment` / `Decrement` go through here as well.

## Mixed values

With a multi-selection whose objects have different values, set `.mixed(true)` or call `set_scrub_field_value(field, value, true)`. While mixed, the field shows "—" and the screen reader gets no value. The next drag, step, or typed entry sets all objects to the same new value and clears the mixed state. A drag starts from `value`, which the application usually sets to the active object's value.

## Writing the value from the application

`set_scrub_field_value(field, value, mixed)` writes the application-side value and emits no event.

When you create the field by hand with `create_component`, call `assemble_scrub_field` yourself to assemble the label, value text, and input field. When it is placed in a view, the view assembles it.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `ScrubField::new(value)` | `f64` | Initial value |
| `.range(min, max)` | `f64, f64` | Bounds; dragging, typing, and stepping are all clamped to them |
| `.step(step)` | `f64` | Arrow-key step; also the change per pixel when `drag_rate` is unset |
| `.precision(digits)` | `u8` | Decimal places shown and rounded to |
| `.drag_rate(per_pixel)` | `f64` | Change per 1 px of drag |
| `.label(text)` | Text | Label before the value; also the screen reader name |
| `.unit(text)` | Text | Unit after the value |
| `.mixed(bool)` | `bool` | Show a mixed value |
| `.size(size)` | `ControlSize` | Control height |
| `.disabled(bool)` | `bool` | Disabled |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| `ScrubInput` | `value: f64` | Each value change during a drag; on cancel, reverts to the value before the drag |
| `ScrubChanged` | `value: f64` | Commit: release, Enter or focus loss after typing, keyboard step |
| `ScrubDragging` | `dragging: bool` | Drag starts and ends |

## Slots

| Slot | Description |
| --- | --- |
| None | The field assembles the label, value text, and input field itself |

## See also

[NumberInput](https://github.com/sena-nana/NanaUI/blob/main/docs/components/number-input.md) · [RangeField](https://github.com/sena-nana/NanaUI/blob/main/docs/components/range-field.md) · [Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md)
