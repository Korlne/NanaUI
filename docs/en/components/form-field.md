# FormField

`FormField` lays out a label, a control, a hint, and an error as one field. The control is your child node. The field itself does not take pointer input and cannot be focused.

There is no `<FormField>` in the control table.

## Basic usage

The label in `FormField::new(label)` becomes the accessible name.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::FormField;

view! {
    <Widget of={FormField::new("Display name").hint("Shown in the title bar")}>
        <template #control>
            <TextInput />
        </template>
    </Widget>
}
```

```rust rust
use nana_ui::runtime::view::{text_input, widget};
use nana_ui::runtime::FormField;

widget(FormField::new("Display name").hint("Shown in the title bar")).control(text_input())
```

:::

To mark an error, set `.error` to the message and make the inner control's own `invalid` true as well.

## Hints and errors

`.hint` is supplementary text; `.error` is the error message.

When both are present, the error is used for display and for the accessible value.

When there is an error, the accessibility state is marked `invalid`.

The display layer has three separate things: the control's own `invalid`, the field's `error`, and a standalone `ValidationMessage`.

## Whole form

To ask before submitting whether anything in the whole form is still wrong, use `AppContext::validity_of(root)`.

It returns, in document order, the fields in the subtree that are marked `invalid` and not disabled.

## Layout

A field lays out vertically at 100% width.

The label row height is reserved by `ControlSize`, so a wrapped label does not cover the control.

## Properties

`.size` defaults to medium and determines the label font size and spacing.

`.style` replaces the whole style.

| Property | Type | Description |
| --- | --- | --- |
| `label` | — | The label in `FormField::new(label)` becomes the accessible name |
| `.hint` | — | Supplementary text. When `.error` is also present, the error is used for display and the accessible value |
| `.error` | — | Error message. When there is an error, the accessibility state is marked `invalid` |
| `.size` | — | Defaults to medium; determines the label font size and spacing |
| `.style` | — | Replaces the whole style |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| None | None | None |

## Slots

In a view, `.control(view)` puts the control below the label and above the description.

This slot is on `El<FormField>` and is written as `control_child` on commit.

| Slot | Description |
| --- | --- |
| `#control` / `.control` | Puts the control below the label and above the description. On `El<FormField>`; written as `control_child` on commit |

## See also

[Overview](index.md) · [Controls](../reference/components.md)
