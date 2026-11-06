# Divider

`divider()` creates a horizontal divider. The template tag is `<Divider>`. A group title is still a nearby `Text`; do not put the title into the line.

## Basic usage

It takes no constructor arguments and has no fields. Internally it is `Divider::horizontal()`. There are no events and no `model`.

A divider carries no label and takes no part in actions.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Column gap=8>
        <Text>"Previous section"</Text>
        <Divider />
        <Text>"Next section"</Text>
    </Column>
}
```

```rust rust
use nana_ui::runtime::view::{column, divider, text};

column().gap(8).children((
    text("Previous section"),
    divider(),
    text("Next section"),
))
```

:::

## Size

By default it is `Fill` along the cross axis with `align_self: Stretch`. Inside a column with `align_items: Start` it is still visible and does not shrink to a point. The horizontal line spans the parent's width and its height is the thickness.

## Accessibility

It does not receive the pointer and is not focusable. The accessibility role is separator. Without another background, the colour is the theme's `BorderSoft`.

## Vertical lines

A vertical line is not in this function. On the component it is `Divider::vertical()`: the height spans the parent and the width is the thickness. Views have only the `<Divider>` tag, with no second tag.

## Thickness

Thickness and the inset at both ends are on the component: `thickness`, `inset`. A non-finite or non-positive value falls back to the default (thickness 1, inset 0). They are not in the field table, and `divider()` has no methods of the same name.

To align the inset with content that is indented by step, use `inset_step(SpacingStep)`: the pixels come from the installed theme's spacing, so the hairline stays on the same line as the content after a theme change. `inset(px)` is only a number under the default metrics; an `inset(px)` written later replaces `inset_step`.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `thickness` | — | Thickness, on the component. A non-finite or non-positive value falls back to the default (thickness 1). Not in the field table, and `divider()` has no method of the same name |
| `inset` | — | The inset at both ends, on the component. A non-finite or non-positive value falls back to the default (inset 0). Not in the field table, and `divider()` has no method of the same name |
| `inset_step` | `SpacingStep` | Inset at both ends by spacing step, following the theme. On the component |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| None | None | There are no events and no `model` |

## Slots

| Slot | Description |
| --- | --- |
| None | A divider carries no label and takes no part in actions |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
