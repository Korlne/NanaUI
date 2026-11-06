# ValidationMessage

`ValidationMessage` shows one validation message on its own. It, `invalid` on a control, and `error` on a `FormField` are three separate displays. When a field is filled in wrong, the control is marked `invalid` and this sentence is responsible for saying why.

## Basic usage

The `intent` of `ValidationMessage::new(message, intent)` is `nana_ui::runtime::ValidationIntent::Warning` or `Danger`. A warning uses the warning color, danger uses the danger color. At construction `compact` is true: the font size uses the hint tier and an indicator bar is reserved on the left. `.compact(false)` switches to a larger font size and a wider indicator. `.intent` can be changed afterward.

The accessibility role is text, the name is the sentence, and it is marked `invalid`. The control table has no `<ValidationMessage>`.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::{ValidationIntent, ValidationMessage};

view! {
    <Widget of={ValidationMessage::new("Display name cannot be empty", ValidationIntent::Danger)} />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::{ValidationIntent, ValidationMessage};

widget(ValidationMessage::new(
    "Display name cannot be empty",
    ValidationIntent::Danger,
))
```

:::

Put it below the field it belongs to. When the field's `error` already shows the same sentence, there is no need to stack another one, unless you want separate emphasis outside the field. `.style` replaces the whole style; the foreground color is still filled in from `intent` during projection.

## Layout

The dot on the left and the message text are parts published by layout: the dot is centered in its cell (`LeadingContent`), followed after one spacing step by the text (`TextContent`), which takes the remaining width. When compact, the cell is `xl` and the gap `xs`; otherwise `xxl` and `sm`, both read from the current theme's spacing. The left padding is 0; the dot's position is no longer written as padding. Measurement, wrapping width, and painting all read the same parts, so overlong text wraps inside the text part and the box is not pushed out of line by the dot. Changing `intent` only changes color and does not relayout. In right-to-left direction the dot stays on the left.

## Whole form

To ask about the whole form before submitting, use `AppContext::validity_of(root)`. It reads the accessibility state controls have already published and returns, in document order, the fields in the subtree that are `invalid` and not disabled. This message itself is not part of that list.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `message` | — | The sentence given to `ValidationMessage::new`. The accessibility role is text, the name is the sentence, and it is marked `invalid` |
| `intent` | `nana_ui::runtime::ValidationIntent` | `Warning` or `Danger`. A warning uses the warning color, danger uses the danger color. `.intent` can be changed afterward |
| `compact` | — | True at construction: the font size uses the hint tier and an indicator bar is reserved on the left. `.compact(false)` switches to a larger font size and a wider indicator |
| `.style` | — | Replaces the whole style; the foreground color is still filled in from `intent` during projection |

## Events

No events.

## Slots

No slots.

## See also

[Overview](index.md) · [Controls](../reference/components.md)
