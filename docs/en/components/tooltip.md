# Tooltip

`Tooltip` is a small piece of explanation that follows the pointer or is anchored next to its trigger. Its accessibility role is tooltip, and its label is the text it shows.

## Basic usage

`Tooltip::new(label)` uses the defaults of `nana_ui::TooltipConfig`: placement `FollowCursor`, a 350 ms delay, max width 280. `Tooltip::with_config(label, config)` swaps in another configuration.

The control table has no `<Tooltip>`.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::Tooltip;
use nana_ui::{TooltipConfig, TooltipPlacement};

view! {
    <Widget
        of={Tooltip::with_config(
            "Save the current file",
            TooltipConfig {
                placement: TooltipPlacement::Top,
                ..TooltipConfig::default()
            },
        )}
    />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::Tooltip;
use nana_ui::{TooltipConfig, TooltipPlacement};

widget(Tooltip::with_config(
    "Save the current file",
    TooltipConfig {
        placement: TooltipPlacement::Top,
        ..TooltipConfig::default()
    },
))
```

:::

`IconButton::with_tooltip` attaches a tooltip to a button with this default configuration. To change the delay or max width, build your own `TooltipConfig` and hand it to the control that shows it.

Charts also use ordinary tooltip nodes internally; they close on leave, unmount, or parking.

## Placement and surface

`placement` can also be `Top`, `Right`, `Bottom`, or `Left`. Directional placements anchor on the trigger and flip to the opposite side only when the preferred side does not fit.

`gap`, `viewport_padding`, and `max_width` control the spacing, viewport margin, and max width. The surface is fixed-positioned, with background `Surface` and border `BorderSoft`; font size and padding come from constants in `TooltipConfig`.

`.style` replaces the whole node style; builder calls that depend on other properties must come after it.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `label` | — | The text given to `Tooltip::new` or `with_config`; also the accessibility label |
| `placement` | — | Default `FollowCursor`. Can also be `Top`, `Right`, `Bottom`, `Left` |
| `gap` | — | Spacing |
| `viewport_padding` | — | Viewport margin |
| `max_width` | — | Max width. The `Tooltip::new` default is 280 |
| Delay | — | The `Tooltip::new` default is 350 ms |
| `.style` | — | Replaces the whole node style. Builder calls that depend on other properties must come after it |

## Events

No events.

## Slots

No slots. The displayed text is `label`.

## See also

[Overview](index.md) · [Controls](../reference/components.md)
