# SegmentedControl

`SegmentedControl` is a horizontal segmented choice whose accessibility role is a radio group. Its options are its children, and each option has its own label, disabled state, and selection. Arrow keys move only focus, not the selection; the selection follows click, Enter, or Space, which is what screen readers expect of a radiogroup.

## Basic usage

The public view functions are `segmented` and `segmented_option`. `segmented()` is `widget(SegmentedControl::new())`. There is no `<SegmentedControl>` in the control table. Options are still `segmented_option`.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::segmented_option;
use nana_ui::runtime::SegmentedControl;

view! {
    <Widget of={SegmentedControl::new().label("Input method")}>
        {segmented_option("Face tracking").selected(move || mode_is_face()).on_select(face)}
        {segmented_option("Mouse").selected(move || !mode_is_face()).on_select(mouse)}
    </Widget>
}
```

```rust rust
use nana_ui::runtime::view::{segmented, segmented_option};

segmented().label("Input method").children((
    segmented_option("Face tracking").selected(move || mode_is_face()).on_select(face),
    segmented_option("Mouse").selected(move || !mode_is_face()).on_select(mouse),
))
```

:::

`mode_is_face`, `face`, and `mouse` are your own predicate and callbacks.

## Construction

`SegmentedControl::new` takes no arguments and defaults to medium size, not stretched. `.label` gives the whole group an accessible name, `.size` changes the control size, and `.fill(true)` makes the options share the width equally.

A horizontal segmented choice wraps automatically by default (turn it off with `.wrap(false)`): when the width it is given fits one row, it stays on one row at its own width; when it does not fit, it wraps and its height follows its content, so option text is never truncated or overflows the container. On one row the height equals that of the non-wrapping control. Combined with `.fill(true)`, each row holds as many options as fit, in their original order, judging fit by the intrinsic size of the real text, icons, and theme spacing, and then divides that row's width equally; the last row also fills. `.fill(false).wrap(true)` keeps each option's intrinsic width and wraps only when it does not fit. Hidden items take no space; disabled items take part in layout as usual. When a single label is longer than a whole row, it takes a row of its own and wraps to grow taller, without shrinking the font size. Ordinary options keep the current theme's standard height. Radio and standalone Tabs do not use this wrapping policy.

```rust
SegmentedControl::new().size(ControlSize::Small).fill(true)
```

The semantic component entry point accepts `fill` and `wrap` boolean attributes and uses the same layout as the Rust constructor. Without `wrap`, it wraps by default, like `SegmentedControl::new()`; only `wrap="false"` (or `0`, `no`) keeps it on one row.

## Vertical

`radio_group()` uses the same selection but stacks vertically with rings. `radio_group()` suits places where option descriptions are long and need a vertical layout; its selection contract is the same as the horizontal segmented control.

## Options

Options use `segmented_option(label)`. `.selected` tells the control whether this item is the current choice, `.disabled` disables it, and `.on_select` runs when the user selects this item.

After an option is selected, the next binding states `.selected` again as you wrote it; the control does not keep a copy of product state for you.

## Layout

An option is one row: the vertical ring first (`Indicator`, at the fixed control-size dimension, separated by `indicator_gap`), then the icon (`LeadingContent`, 1em of the label font size for segmented items, the control size's icon size for vertical ones, separated by the theme's `xs`), and the label takes the remaining width (`TextContent`). These are parts published by layout, and intrinsic measure, the paragraph budget, placement, and painting all read this one copy; the padding is only the option's own inset and no longer reserves space for the ring and icon. A segmented item's label is centered in its width; a vertical item's label is start-aligned. When the label is not vertically centered, the icon aligns to the center line of the label's first line. Selected, disabled, and hover states change only painting, not layout. In right-to-left direction, the ring and icon stay on the left and are not mirrored.

## Registration

The control registers its children and selection itself after it is built and whenever a binding changes it or one of its items; do not call `set_segmented_options` in the view.

## Hiding an item

When hiding an item, disable it as well; the node remains. When an item is hidden, disable and hide it together, and the node stays in the group.

## Activation

On activation the control changes the selection itself: it first emits `SegmentedSelectionRequested { option }` on the control, then `SegmentedOptionChosen` on the selected item. To veto, write `.selected` back to the item you want.

The `option` in `SegmentedSelectionRequested` is that item's `StableNodeId`. Use this identity to find your own business value. `SegmentedOptionChosen` is emitted on the clicked item, so you can listen to a single segment.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `.label` | — | Accessible name of the whole group |
| `.size` | — | Control size. `new` defaults to medium |
| `.fill` | — | `true` makes the options share the width equally. Not stretched by default |
| `.wrap` | `bool` | Horizontal segmented options wrap when the width is not enough; on by default; combined with fill, each row divides equally |
| Option `label` | — | `segmented_option(label)` |
| `.selected` | — | Whether this item is the current choice. The control does not keep product state for you; the next binding states it again as you wrote it |
| `.disabled` | — | Disables this item. When hiding, also disable it; the node remains |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| `SegmentedSelectionRequested` | `{ option }`, where `option` is a `StableNodeId` | Emitted first on the control during activation. To veto, write `.selected` back to the item you want |
| `SegmentedOptionChosen` | — | Then emitted on the selected item, so you can listen to a single segment |
| `.on_select` | — | Runs when the user selects this item |

## Slots

| Slot | Description |
| --- | --- |
| Options | Children, using `segmented_option`. The control registers children and selection itself; do not call `set_segmented_options` in the view |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
