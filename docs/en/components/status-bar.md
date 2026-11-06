# StatusBar

`StatusBar` is a horizontal bar below the content. It announces status as a live status, so assistive technology hears changes before focus moves there. An ordinary layout box cannot express this role.

## Basic usage

`StatusBar::new()` paints a surface by default, with a hairline border along the top edge facing the content. There is no `<StatusBar>` in the control table. There is no assembly function; you put readings such as branch, caret position and encoding into child nodes.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::StatusBar;

view! {
    <Widget of={StatusBar::new().label("Status")}>
        <Text>"Line 1"</Text>
    </Widget>
}
```

```rust rust
use nana_ui::runtime::view::{text, widget};
use nana_ui::runtime::StatusBar;

widget(StatusBar::new().label("Status")).children(text("Line 1"))
```

:::

When a reading changes, change the child's text. Do not reinstall the shell just to refresh status.

## Surface

`.label` is the accessible name. `.chrome(false)` is for when the parent already has a surface. The bar itself does not take the pointer.

The bar lays out horizontally with children centered. With `chrome` on, the background is `Surface`, and the hairline is drawn on the top edge facing the content above, in `BorderSoft`. Horizontal padding is also `space::MD`. Branch name, caret and encoding go into this bar as separate text nodes.

## With toasts

The `.status` slot of `DesktopShell` is the overlay host for toasts, not the same thing as this status bar. Toasts go through the `status` host; this bar goes at the bottom of the workspace or below your own content.

## Role

The role is `Status`; when the text changes, assistive technology can hear it while focus stays in the editor.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `.label` | — | Accessible name |
| `.chrome` | — | `new()` paints a surface by default, with a hairline border along the top edge facing the content. `false` is for when the parent already has a surface. When on, the background is `Surface`, the hairline is on the top edge in `BorderSoft`, and horizontal padding is `space::MD` |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| None | None | None |

## Slots

| Slot | Description |
| --- | --- |
| Child nodes | Readings such as branch, caret position and encoding. There is no assembly function. The bar lays out horizontally with children centered |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md) · [Workspace](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/workspace.md)
