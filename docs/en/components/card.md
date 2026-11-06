# Card

`Card` is a surface that takes no interaction. Actions go on controls inside it; the card itself does not act as a button.

There is no `<Card>` in the control table.

## Basic usage

The kind of `Card::new()` is `nana_ui::CardKind::Surface`.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::Card;
use nana_ui::CardKind;

view! {
    <Widget of={Card::new().title("Project").kind(CardKind::Outlined)}>
        <Text>"main.rs"</Text>
    </Widget>
}
```

```rust rust
use nana_ui::runtime::view::{text, widget};
use nana_ui::runtime::Card;
use nana_ui::CardKind;

widget(Card::new().title("Project").kind(CardKind::Outlined)).children(text("main.rs"))
```

:::

Real operations, such as open and delete, go into buttons among the children.

## Kinds

`.kind` decides the surface:

- `Surface` and `Raised` use the surface color, with no border.
- `Outlined` draws only a border.
- `Flat` has no background and no border.
- `Selected` uses the selected background with a soft border.

When your style already sets background, border, corner radius or padding, the kind does not override them.

## Style

`.title` and `.label` write the same title. The title becomes the accessible name and reserves a title band above the content.

The default corner radius is `RadiusTier::Md` and the padding is the panel step; the values come from the current theme.

`.padding` switches to the same physical padding on all four sides and clears the panel step.

`.height` fixes the height.

## Loading

`.loading(true)` marks the accessibility state as busy, and the card advances the phase itself.

A loading card is still a surface.

## Painting

`.painter` can custom-draw the appearance.

To keep the built-in card and layer decoration on top, call `cx.draw_default()` in `paint`.

## Accessibility

Without a title the node text is an empty string, and screen readers do not use the title as the name.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `Card::new()` | — | Kind is `nana_ui::CardKind::Surface` |
| `.title` | — | Writes the same title as `.label`. The title becomes the accessible name and reserves a title band above the content |
| `.label` | — | Writes the same title as `.title` |
| `.kind` | `nana_ui::CardKind` | `Surface` and `Raised` use the surface color, with no border. `Outlined` draws only a border. `Flat` has no background and no border. `Selected` uses the selected background with a soft border. When the style already sets background, border, corner radius or padding, the kind does not override them |
| `.padding` | — | Switches to the same physical padding on all four sides and clears the panel step |
| `.height` | — | Fixes the height |
| `.loading` | — | `.loading(true)` marks the accessibility state as busy, and the card advances the phase itself. A loading card is still a surface |
| `.painter` | — | Can custom-draw the appearance. To keep the built-in card and layer decoration on top, call `cx.draw_default()` in `paint` |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| None | None | Actions go on controls inside it; the card itself does not act as a button |

## Slots

| Slot | Description |
| --- | --- |
| None | Real operations, such as open and delete, go into buttons among the children |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
