# StatusBadge

`status_badge(label)` creates a status badge. `label` is the constructor argument, with field type `Arc<str>`; the template tag is `<StatusBadge>`, and child text and `label` both write this one field. It only describes state and owns no action.

## Basic usage

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::StatusTone;

view! {
    <StatusBadge tone={StatusTone::Success} compact={false}>"Saved"</StatusBadge>
}
```

```rust rust
use nana_ui::runtime::view::status_badge;
use nana_ui::runtime::StatusTone;

status_badge("Saved").tone(StatusTone::Success).compact(false)
```

:::

The `StatusTone` variants are `Neutral`, `Info`, `Success`, `Warning`, `Danger`. When `tone` is not written, the function uses `StatusTone::Neutral` internally. `compact` defaults to true when the component is constructed; the example above turns it off, so padding and font size take the non-compact tier.

## Tone

`tone` and `compact` can be constants, signals or closures. A tone first maps to a semantic role, and is matched to the current theme at paint time; no concrete color values are written here. On the background, Neutral, Success and Danger use Subtle, Info uses AccentSoft, and Warning uses WarningSoft. The foreground takes a role from the status recipe.

The default variant of `StatusTone` is `Neutral`. The view function also uses it explicitly when you do not write `tone`. `tone` can be a `StatusTone` or a `Signal<StatusTone>`. Do not write color values into `label`.

## Compact

Compact is the default: hint font size, smaller padding. `compact(false)` switches to the meta font size. The corners are a fully rounded pill. `compact` does not change the text, only padding and font size. The label is still that one `label` field.

## Layout

The dot and the label are parts published by layout: the dot is centered in its cell (`LeadingContent`), the label follows after one step of spacing (`TextContent`) and takes the remaining width. When compact, that cell and the gap are `sm` and `xs`, otherwise `md` and `sm`, all read from the current theme's spacing. Padding is only the capsule's left and right tier; the dot's position is no longer counted into the left padding, and measurement, wrap width and painting all read the same parts. Changing `tone` changes only colors and does not re-layout. Under a right-to-left direction the dot stays on the left.

## Accessibility

Accessibility is text, and the name is `label`. There is no pressed, no selected and no close. For a selectable, dismissible token use `Chip`. To do something, use `Button`.

## Properties

The fields are `label: Arc<str>`, `tone: StatusTone`, `compact: bool`. There are no events and no `model`.

| Property | Type | Description |
| --- | --- | --- |
| `label` | `Arc<str>` | Constructor argument. Child text writes the same field. The accessible name. Do not write color values here |
| `tone` | `StatusTone` | Can also be a `Signal<StatusTone>`, or a constant, signal or closure. The variants are `Neutral`, `Info`, `Success`, `Warning`, `Danger`. When not written, the function uses `Neutral` internally. The default variant is also `Neutral` |
| `compact` | `bool` | A constant, signal or closure. Defaults to true when the component is constructed. `false` takes non-compact padding and font size and does not change the text |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| None | None | There are no events and no `model` |

## Slots

| Slot | Description |
| --- | --- |
| None | Child text and `label` both write this one field |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
