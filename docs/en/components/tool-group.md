# ToolGroup

`ToolGroup` puts a set of tool buttons on one shared well: a group of tools in a drawing toolbar, a canvas's display toggles, a panel's own few actions. You place the buttons, usually `IconButton`s; which one is selected is still decided by your state, just as with standalone icon buttons. Compared with `Stack::row`, it adds two things: a well that reads the buttons as one group, and an accessibility role that has the screen reader announce the buttons as a group.

## Basic usage

`ToolGroup::new()` is horizontal; `ToolGroup::vertical()` is vertical, for example a tool rack at the side of a canvas. `.label` is the group's accessible name, for example "Selection tools". `.mode` describes how the buttons relate.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::{Icon, ToolGroup};

view! {
    <Widget of={ToolGroup::new().label("Layer actions")}>
        <IconButton icon={Icon::Add}>"New layer"</IconButton>
        <IconButton icon={Icon::Settings}>"Layer settings"</IconButton>
        <IconButton icon={Icon::Close}>"Delete layer"</IconButton>
    </Widget>
}
```

```rust rust
use nana_ui::runtime::view::{icon_button, widget};
use nana_ui::runtime::{Icon, ToolGroup};

widget(ToolGroup::new().label("Layer actions")).children((
    icon_button(Icon::Add, "New layer"),
    icon_button(Icon::Settings, "Layer settings"),
    icon_button(Icon::Close, "Delete layer"),
))
```

:::

A vertical current-tool group is written `ToolGroup::vertical().mode(ToolGroupMode::Exclusive)`, with the current button marked `.selected(true)`.

The group itself does not receive pointer input; clicks land on the buttons. There is no assembly function; the children are the content. The control table has no `<ToolGroup>`.

## Modes

| Mode | Use | Screen reader |
|---|---|---|
| `Actions` (default) | Each press does one thing, nothing stays lit: new, delete, move up | Toolbar |
| `Exclusive` | One button is the current tool, marked `selected` by you | Radio group |
| `Toggles` | Each button is on or off on its own, for example display toggles | Toolbar; each button reports its own `selected` |

The mode does not change the buttons' appearance; each button paints by its own `selected`. When several groups share one current tool (for example a toolbar split into select, transform, and create groups), mark `selected` in the group that holds it and in no other group.

## Well and arrangement

The well uses the `Subtle` background and `Md` radius, with `space::XXS` between buttons and to the well's edge, so the buttons' own `Sm` radius nests exactly inside. Direction follows `orientation`: a row when horizontal, a column when vertical, children centered and not stretched.

Leave spacing between adjacent groups; do not add a `Divider`: each well already closes off its own group.

To move between buttons with arrow keys, add `.roving_focus(RovingFocusGroup::vertical())` to the group (`horizontal()` for a horizontal one). See the arrow-key focus groups in [Controls](../reference/components.md).

## Properties

| Property | Type | Description |
|---|---|---|
| `label` | `Option<Arc<str>>` | Accessible name of the group |
| `orientation` | `SelectionOrientation` | `Horizontal` (default) or `Vertical`; decides the arrangement direction and the direction the screen reader reports |
| `mode` | `ToolGroupMode` | How the buttons relate; see the table above |
| `style` | `NodeStyle` | Replaces the whole style; direction is still decided by `orientation` |
