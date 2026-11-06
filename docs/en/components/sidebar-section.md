# SidebarSection

`SidebarSection` is one section in a sidebar: a title, an optional count, tools that appear on hover, and a body. You supply the rows in the body.

## Basic usage

There is no `<SidebarSection>` in the control table.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::{Icon, SidebarSection};

view! {
    <Widget
        of={SidebarSection::new("Resources")
            .count(2)
            .collapsible(true)
            .empty_text("No resources yet")}
    >
        <template #tools>
            <IconButton icon={Icon::Add}>"New"</IconButton>
        </template>
    </Widget>
}
```

```rust rust
use nana_ui::runtime::view::{icon_button, widget};
use nana_ui::runtime::{Icon, SidebarSection};

widget(
    SidebarSection::new("Resources")
        .count(2)
        .collapsible(true)
        .empty_text("No resources yet"),
)
.tools(icon_button(Icon::Add, "New"))
```

:::

## Expanding and collapsing

`SidebarSection::new(title)` starts expanded, is not collapsible, and is small. `.collapsible(true)` allows collapsing. `.expanded` writes the open state directly and sets the animation progress to 0 or 1.

Expanding on a title click is handled by `activate_sidebar_section`.

## Count and empty text

`.count` shows the number next to the title. `.empty_text` is the note shown when the section has no rows. The empty text appears only when there are no rows.

## Disabling

`.disabled` turns the section off.

## Tools

`.tools(id)` is the tool block that replaces the count while the title bar is hovered. In views, `.tools(view)` goes into the title and child nodes go into the body.

## Assembly

`assemble_sidebar_section` is a `slot_assembler`. It builds the title, count, disclosure button and body opening, and moves child nodes into the body. It runs when the view commits. After placing things by hand, call it once yourself. Changing the title does not reassemble automatically.

## Row selection

Row selection is not on this section; it is written on the `SidebarRow` below.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `title` | — | `SidebarSection::new(title)`. Starts expanded, is not collapsible, and is small. Changing the title does not reassemble automatically |
| `.count` | — | Shows the number next to the title. Hovering tools replace the count |
| `.collapsible` | — | `true` allows collapsing. Not collapsible by default |
| `.expanded` | — | Writes the open state directly and sets the animation progress to 0 or 1 |
| `.empty_text` | — | The note shown when the section has no rows. The empty text appears only when there are no rows |
| `.disabled` | — | Turns the section off |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| None | None | Expanding on a title click is handled by `activate_sidebar_section`. Row selection is written on `SidebarRow` |

## Slots

| Slot | Description |
| --- | --- |
| `.tools` | Replaces the count while the title bar is hovered. Goes into the title in views. Template `#tools` |
| Body | Child nodes go into the body. Assembly builds the body opening and moves them in |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md) · [Workspace](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/workspace.md)
