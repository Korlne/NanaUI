# IconButton

`icon_button(icon, label)` creates an icon button. `icon` is an `Icon`; `label` is the accessible name, typed `Arc<str>` on the field.

The template tag is `<IconButton>`. `icon` must be written as an attribute. `label` is an attribute or the single child text.

## Basic usage

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::Icon;

view! {
    <IconButton icon={Icon::Search} disabled={pending} @activate={open_search}>
        "Search"
    </IconButton>
}
```

```rust rust
use nana_ui::runtime::view::icon_button;
use nana_ui::runtime::Icon;

icon_button(Icon::Search, "Search")
    .disabled(pending)
    .on_activate(open_search)
```

:::

`@activate={open_search}` passes the function directly. An expression expands to `.on_activate(move || { … })`. When `disabled` is true, `Activate` is not emitted.

## Icons

Icons use catalog constants such as `Icon::Search`, `Icon::Add`, `Icon::Close`, and `Icon::Settings`. `Icon` is a `Copy` identity pointing to static geometry. `Icon::parse_name` only parses shell names (`search`, `settings`, `close`, and their aliases). Catalog constants such as `Icon::Puzzle` return `None` here, so that a name table does not keep unused geometry alive. There is no `from_str` that builds an icon from an arbitrary string.

## Selection

The application writes `selected`. When selected, the glyph and background use the selected state; the control does not select itself because it was clicked.

## Hover tooltip

The hover tooltip is not `label`. The accessible name is `label`. For a tooltip, use `IconButton::with_tooltip` on the component, configured with the default `TooltipConfig`. It is not a method from the field table. Tools in a heading row use `IconButton::tool(icon, label)`: compact size, with the label serving as both accessible name and tooltip.

## Size

It is square by default: the side is the theme's `icon_button_size` (28), and `IconButton::size` switches it to a control height step. This is the square box used when nobody gives a size (`NodeStyle::square`). When the application sets a width or height itself (`style.layout.width` / `height`, or `width` / `height` in template CSS), that axis follows what was written, with no need to clear `square`: a 22×22 close button is 22×22. A button with only a width set keeps the square side as its height. Switching to another theme density changes only the square box, not sizes the application wrote.

The glyph size follows the control size step by default: the body font size of that step, or the medium font size when `compact`. `IconButton::icon_size(px)` sets the glyph side directly (in logical pixels) while the button's square box stays the same. Use it in dense themes when a small button's glyph is too small, to make the glyph stand out without enlarging the button. Non-finite or negative values are ignored, as with `Button::icon_size`.

## Kind

The kind at construction is `ButtonKind::Ghost`. When `selected` is true, or the kind itself is Selected, painting uses the selected color. Change the kind with `IconButton::kind`. `colors_from_style` makes this one button use the colors in its own `style` instead of having them overridden by the kind; it affects only the button that sets it.

The disabled look follows the colors. A button colored by its kind drops to 50% opacity as a whole when disabled: the kind's normal icon color is close to its disabled color, and fading is what shows which button cannot be clicked. A `colors_from_style` button paints `style.interaction.disabled` as written when disabled and is not faded; opacity set on the node is also used as written. If you want it faded, write that there, for example with `SemanticColorMix::alpha` or a `SemanticColorMix::new` pre-mixed against the background. A button that paints its own disabled state through a custom `Painter` using `PaintState::disabled` should also set `colors_from_style`; otherwise the framework fades the already-painted disabled colors once more.

## Properties

The fields are `icon: Icon`, `label: Arc<str>`, `selected: bool`, `disabled: bool`, and `has_popup: bool`. There is no `model`. Activation does not change `selected` on its own.

| Property | Type | Description |
| --- | --- | --- |
| `icon` | `Icon` | Must be written as an attribute. A `Copy` identity pointing to static geometry |
| `label` | `Arc<str>` | Accessible name. An attribute or the single child text |
| `selected` | `bool` | Written by the application. Activation does not change `selected` on its own. When selected, the glyph and background use the selected state |
| `icon_size` | `Option<f32>` | Glyph side (logical pixels); overrides the font size from the size step without changing the square box. Defaults to `None` |
| `disabled` | `bool` | When true, `Activate` is not emitted. A button colored by its kind drops to 50% opacity as a whole; a `colors_from_style` button paints `style.interaction.disabled` without fading. See "Kind" above |
| `has_popup` | `bool` | A button that opens a menu (`aria-haspopup="menu"`): screen readers announce it as a menu button; while it has focus, ArrowUp / ArrowDown emit `SecondaryPress` with `keyboard: true`, like the `ContextMenu` key, and you open the menu. See menu buttons in [Controls](../reference/components.md) |

## Events

The event is `on_activate`, of type `Activate`. The handler takes no arguments.

| Event | Payload | Description |
| --- | --- | --- |
| `on_activate`, template `@activate` | No arguments | Type is `Activate`. Pass a function directly. An expression expands to `.on_activate(move \|\| { … })`. Not emitted when `disabled` is true |

## Slots

| Slot | Description |
| --- | --- |
| None | None |

## See also

[Overview](index.md) · [Controls](../reference/components.md)
