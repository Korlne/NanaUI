# ConfirmDialog

`ConfirmDialog` asks for one confirmation. Its accessibility role is alert dialog. You give the title and message; assembly builds the confirm and cancel buttons and wires them to `ConfirmIntent`.

There is no `<ConfirmDialog>`, and no `open()`.

## Basic usage

`ConfirmDialog::new(title, message)` defaults its button labels to "Confirm" and "Cancel".

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::ConfirmDialog;

view! {
    <Widget
        of={ConfirmDialog::new("Delete project", "This cannot be undone.")
            .confirm_label("Delete")
            .cancel_label("Keep")}
    />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::ConfirmDialog;

widget(
    ConfirmDialog::new("Delete project", "This cannot be undone.")
        .confirm_label("Delete")
        .cancel_label("Keep"),
)
```

:::

## Button labels

`.confirm_label` and `.cancel_label` replace them with your wording. When unset they use `dialog.confirm` / `dialog.cancel` from the framework string table, see [Framework strings](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/framework-strings.md).

## Danger

When `danger` is true (the field, or `.danger(true)`), the confirm button uses the danger color and the title takes a dangerous tone: the danger semantic color from the theme's status recipe. It is the same tone as `.danger` on [Dialog](https://github.com/sena-nana/NanaUI/blob/main/docs/components/dialog.md#口气与标题图标).

## Size and closing

`.size` takes a `nana_ui::DialogSize`: `Compact`, `Default`, `Medium`, `Wide`, `Workspace`, or `Width(length)` / `capped(px, vw)` for a width of your own, see [Dialog](https://github.com/sena-nana/NanaUI/blob/main/docs/components/dialog.md#尺寸). The card's distance from the top and its maximum height belong to the theme's `DialogRecipe`.

`.close_policy` decides whether Escape and a click outside can close it. Both gestures and the close position always emit `DialogCloseRequested` on the confirm dialog itself first; `DialogClosePolicy::requests_only()` leaves every one of them to the application, see [Dialog](https://github.com/sena-nana/NanaUI/blob/main/docs/components/dialog.md#由应用决定开合).

`.initial_focus` takes a `ModalInitialFocus` and lands on the first action by default.

## Assembly

`AppContext::assemble_confirm_dialog` builds the buttons while the confirm or cancel slot is still empty.

If you have already filled the slots with `set_confirm_slots`, it does not overwrite them.

After creating the component by hand with `create_component` and placing your slots, call it once yourself; field writes do not reassemble every time.

## Opening

Open it with `AppContext::activate_overlay(host, dialog)`.

The dialog must be a direct child of this `OverlayHost`.

Close it with `dismiss_overlay(host)`.

You can also declare it: `.open(..)` with `true` opens it after assembly has built the buttons, and `false` closes it; `.model(signal)` also writes the host's open and close back into the signal. See [Dialog](https://github.com/sena-nana/NanaUI/blob/main/docs/components/dialog.md#声明式开合).

## Busy and body

When `busy` is true, the framework treats this confirmation as busy.

Put secondary actions in `.secondary`.

When the body is more than one sentence, put your content in `.body`; assembly does not replace it with the default body.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `ConfirmDialog::new(title, message)` | — | Default button labels are "Confirm" and "Cancel" |
| `.confirm_label` | — | Replaces it with your wording |
| `.cancel_label` | — | Replaces it with your wording |
| `danger` / `.danger` | `bool` | When true, the confirm button uses the danger color and the title takes a dangerous tone |
| `.size` | `nana_ui::DialogSize` | `Compact`, `Default`, `Medium`, `Wide`, `Workspace`, or `Width(length)` / `capped(px, vw)` |
| `.close_policy` | — | Decides whether Escape and a click outside can close it |
| `.initial_focus` | `ModalInitialFocus` | Lands on the first action by default |
| `busy` | — | When true, the framework treats this confirmation as busy |
| `set_confirm_slots` | — | Once you have filled the slots with it, `assemble_confirm_dialog` does not overwrite them |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| `ConfirmIntent` | — | Assembly builds the confirm and cancel buttons and wires them to `ConfirmIntent` |
| `DialogToggled` | `open` | The host opened or closed it. Not emitted when the application writes `open` itself |

## Slots

The slots on the view are `.title_icon`, `.body`, `.footer`, `.close_action`, `.cancel`, `.secondary` and `.confirm`.

This is a `slot_assembler`: it runs when the view commits.

| Slot | Description |
| --- | --- |
| `.title_icon` | The icon before the title; for its arrangement see [Dialog](https://github.com/sena-nana/NanaUI/blob/main/docs/components/dialog.md#口气与标题图标) |
| `.body` | Puts your content when the body is more than one sentence; assembly does not replace it with the default body |
| `.footer` | Content at the left end of the action row, such as a "Do not ask again" switch. One button taller, and as wide as the space up to `action_gap` before the leftmost button; it neither makes the card taller nor moves the buttons. Its tab order comes before the buttons, and focus still lands on the first action when it opens |
| `.close_action` | A slot on the view |
| `.cancel` | A slot on the view. Assembly builds the button only while the confirm or cancel slot is still empty |
| `.secondary` | Puts secondary actions in `.secondary` |
| `.confirm` | A slot on the view. Assembly builds the button only while the confirm or cancel slot is still empty |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
