# SidebarFrame

`SidebarFrame` is the outer frame of a navigation sidebar: a fixed top, a body that scrolls on its own, and a fixed footer. You put links and selected items into the body; NanaUI ships no product navigation.

## Basic usage

All three slots of `SidebarFrame::new()` are empty. There is no `<SidebarFrame>` in the control table.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::SidebarFrame;

view! {
    <Widget of={SidebarFrame::new()}>
        <template #top>
            <Text>"Project"</Text>
        </template>
        <template #body>
            <Text>"Files"</Text>
        </template>
        <template #footer>
            <Text>"Settings"</Text>
        </template>
    </Widget>
}
```

```rust rust
use nana_ui::runtime::view::{text, widget};
use nana_ui::runtime::SidebarFrame;

widget(SidebarFrame::new())
    .top(text("Project"))
    .body(text("Files"))
    .footer(text("Settings"))
```

:::

Put `SidebarSection` and `SidebarRow` inside the body. Which row is selected and where a click goes live in your state.

## Scrolling

In views `.top(view)`, `.body(view)` and `.footer(view)` become child nodes in that order. `.body` first wraps the content in `SidebarFrame::vertical_body_scroll()`, a vertical `ScrollView`.

During projection the body lends out the scroll opening, so no scrollbar is drawn over the frame's own visuals. The frame has no separate `assemble_*`. Top, body and footer are simply its child nodes. The scroll offset stays on that `ScrollView`.

`vertical_body_scroll` is a vertical `ScrollView`. The view's `.body` uses it to wrap what you give it, then puts it in the frame's body slot. When you take `SidebarFrame::scroll_body` yourself, you can pass a `LayoutStyle` you already prepared.

A long list scrolls only the middle; the top and bottom bars stay in the viewport.

## Projection

The frame's projection clears its own text and standard visuals, leaving scrollbars to the body's `ScrollView`. `ALWAYS_REPROJECT` is on because the body node is projected once more by the scroll opening.

## Properties

The frame's spacing field is `gap`.

| Property | Type | Description |
| --- | --- | --- |
| `gap` | — | The frame's spacing |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| None | None | Which row is selected and where a click goes live in your state |

## Slots

| Slot | Description |
| --- | --- |
| `.top` | The fixed top. Empty at `new()`. Template `#top` |
| `.body` | The body that scrolls on its own. Wrapped first in `vertical_body_scroll()`, a vertical `ScrollView`. Template `#body` |
| `.footer` | The fixed footer. Template `#footer` |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md) · [Workspace](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/workspace.md)
