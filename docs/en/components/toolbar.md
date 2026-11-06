# Toolbar

`Toolbar` is a horizontal bar above content. You place the buttons inside. Compared with `Stack::bar`, it adds a shell surface and the accessibility role `Toolbar`: the screen reader announces the controls inside as one group.

## Basic usage

`Toolbar::new()` paints a surface by default, with a hairline border along the bottom edge. `.label` is the accessible name; it tells toolbars apart when a window has several. `.chrome(false)` turns off the surface and border, for when the parent already has a surface.

The bar itself does not receive pointer input; clicks land on the children. There is no assembly function; the children are the content. The control table has no `<Toolbar>`.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::Toolbar;

view! {
    <Widget of={Toolbar::new().label("Main toolbar")}>
        <Button>"Save"</Button>
        <Button>"Run"</Button>
    </Widget>
}
```

```rust rust
use nana_ui::runtime::view::{button, widget};
use nana_ui::runtime::Toolbar;

widget(Toolbar::new().label("Main toolbar")).children((button("Save"), button("Run")))
```

:::

Actions such as save and run are buttons you put in. The bar does not interpret what the buttons mean; it only puts them in one accessible toolbar.

To move between buttons with ArrowLeft / ArrowRight like a native toolbar, add `.roving_focus(RovingFocusGroup::horizontal())` to the bar. Home / End go to the first and last button, and all buttons stay in the Tab order. See the arrow-key focus groups in [Controls](../reference/components.md).

## Surface and arrangement

The bar is laid out horizontally with children centered. Spacing is `space::SM`, horizontal padding is `space::MD`, vertical padding is `space::XS`. With `chrome` on, the background is `Surface` and only the bottom edge draws a hairline, in `BorderSoft`.

The bar's height follows its content and padding; it does not compete with the title bar for `TITLE_BAR_HEIGHT`.

## Relation to the shell

Do not put inspector tabs in the main toolbar area. In `DesktopShell`, document and editing actions can go in `RegionId::PrimaryToolbar`, and the inspector goes through `.inspector`.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `.label` | — | Accessible name. Tells toolbars apart when a window has several |
| `.chrome` | — | `Toolbar::new()` paints a surface by default, with a hairline border along the bottom edge. `false` turns off the surface and border. When on, the background is `Surface` and only the bottom edge draws a hairline, in `BorderSoft` |

## Events

No events of its own. The bar itself does not receive pointer input; clicks land on the children.

## Slots

No named slots. Buttons are children.

## See also

[Overview](index.md) · [Controls](../reference/components.md) · [Workspace](../reference/workspace.md)
