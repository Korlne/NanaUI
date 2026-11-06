# AppTitleBar

`AppTitleBar` is the title strip at the top of the window. Its height is `TITLE_BAR_HEIGHT`. It owns the title text, the leading, center and trailing slots, and the space for window buttons.

There is no `<AppTitleBar>` in the control table.

## Basic usage

`AppTitleBar::new(title)` is draggable, opaque and shows window buttons by default.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::AppTitleBar;

view! {
    <Widget of={AppTitleBar::new("Nana").drag_enabled(true)}>
        <template #trailing>
            <Button>"Share"</Button>
        </template>
    </Widget>
}
```

```rust rust
use nana_ui::runtime::view::{button, widget};
use nana_ui::runtime::AppTitleBar;

widget(AppTitleBar::new("Nana").drag_enabled(true)).trailing(button("Share"))
```

:::

## Dragging

Window dragging starts from areas of the bar that take no interaction.

`.drag_enabled(false)` turns off window dragging that starts from this bar.

## Window buttons

When the platform defaults to system buttons, `native_controls` is true and the controls slot is an empty space reserved for the system buttons; otherwise the slot holds custom minimize, maximize and close buttons.

`.show_window_controls` and `.native_controls` are separate: the native placeholder is installed even when hidden at first, so there is a box to follow when it is shown later.

`.maximized` is passed to the window buttons' appearance.

`.controls_capsule(true)` puts the window buttons in a capsule: a `Surface` background, a thin `BorderSoft` line, fully rounded ends, and rounded buttons. The capsule sits away from the window edge by the bar's edge inset instead of hugging the edge. Use it on a transparent bar laid over content: bare buttons would land on whatever is behind them, while the application's own controls next to them are capsules too. The native red, yellow and green buttons get the same capsule underneath, as wide as their placeholder. Off by default.

If the host has already bound window buttons, turn off the shell's `title_window_controls` or this bar's `show_window_controls` according to your button scheme, to avoid two sets of buttons.

## Appearance

`.transparent(true)` removes only the title bar background.

`.center_width` is the width of the center title column.

### Over media

`.over_media(true)` is for a title bar laid over video, and includes `.transparent(true)`. The treatment matches viewing an image in the image viewer and does not follow the theme:

- Behind the bar is a top-to-bottom gradient that fades from the theme's `EffectTokens::media_scrim` to transparent.
- The title and custom-drawn window buttons use the theme's `EffectTokens::media_foreground` (role `SemanticColorRole::OnMedia`), which is white in both themes.
- Button hover and press use a translucent background of the same color; close button hover uses a solid `Danger` background with the icon still light.

The same applies in the light theme, so it stays legible on dark and busy images. The application's own controls in the slots are unaffected.

## Assembly

`assemble_app_title_bar` is a `slot_assembler` that puts the three columns and the window buttons back together.

It runs when the view commits.

After building slots by hand, call it once yourself.

A write that changes the title does not reassemble automatically.

## With shells

`DesktopShell::title` builds a bar like this when you do not bring your own title bar.

The shell's `workspace_corners` is not written on the title bar.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `AppTitleBar::new(title)` | — | Draggable, opaque and shows window buttons by default |
| `.transparent` | — | `.transparent(true)` removes only the title bar background |
| `.over_media` | — | Over media: a media scrim gradient underneath, light foreground for the title and window buttons, the same in both themes. Includes `.transparent(true)` |
| `.drag_enabled` | — | `.drag_enabled(false)` turns off window dragging that starts from this bar |
| `.show_window_controls` | — | Separate from `.native_controls`. When the host has already bound window buttons, turn it off according to your button scheme to avoid two sets of buttons |
| `.native_controls` | — | True when the platform defaults to system buttons; the controls slot is then an empty space reserved for the system buttons, otherwise it holds custom minimize, maximize and close. The native placeholder is installed even when hidden at first, so there is a box to follow when it is shown later |
| `.maximized` | — | Passed to the window buttons' appearance |
| `.center_width` | — | Width of the center title column |
| `.edge_inset` | `SpacingStep::Sm` | Distance from the leading slot to the title bar's left edge and from the trailing slot to its right edge. When the title bar holds a card title, give it the card's own padding (for example `Xxxl`) so the title and tools sit on the card's content line, and the application does not have to subtract this step. The native button placeholder still hugs the left edge and custom window buttons still hug the right edge |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| None | None | None |

## Slots

The view slots are `.leading`, `.center` and `.trailing`.

| Slot | Description |
| --- | --- |
| `.leading` | View slot |
| `.center` | View slot. `.center_width` is the width of the center title column |
| `.trailing` | View slot |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md) · [Workspace](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/workspace.md)
