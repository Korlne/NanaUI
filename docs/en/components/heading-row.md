# heading_row

`heading_row(title, tools)` is the heading row of a group or card: the title on the left and a set of heading tools on the right. It is a view in `view::marks`, built from `Stack`; it is not a registered control and has no template tag.

## Basic usage

```rust
use nana_ui::runtime::view::{heading_row, heading_tools, text, widget};
use nana_ui::runtime::{Icon, IconButton};
use nana_ui::theme::TypeRole;

heading_row(
    text("Models").role(TypeRole::Meta),
    heading_tools((
        widget(IconButton::tool(Icon::Plus, "Import")),
        widget(IconButton::tool(Icon::Refresh, "Refresh")),
    )),
)
```

- The title takes the remaining width and yields first, with ellipsis, when space runs out; the tools keep their own width.
- `.lead(view)` puts a leading element before the title (a card icon, a back arrow). It keeps its width, sits `SpacingStep::Sm` from the title, and is vertically centered in the heading row. Apart from the title text, the layout is the same with or without a leading element.

## Tool overhang

`heading_tools(tools)` is a toolbar with no appearance and no vertical padding. Tools use `IconButton::tool(icon, label)` (compact, target `COMPACT_TARGET`, 24×24 by default, with the label doubling as the tooltip) or a compact menu.

The compact target is taller than the title's line box. By default the toolbar **overhangs**: it takes a negative margin of `heading_overhang()` (`(COMPACT_TARGET − line box) / 2`, 4px by default) above and below, so it does not make the heading row taller. As a result:

- Heading rows with and without tools are the same height, and headings on the same page line up with the content below;
- When the heading is at the top of a scroll area, the overhanging top half is clipped by the scroll area, so the scroll area must use `ScrollView::heading_room()` to leave the same distance at the top.

Content on a heading row that is not a tool but is as tall as one (a status pill next to a name, a compact button) uses `heading_hang(view)`: it overhangs above and below in the same way, keeps its width, and is centered in the row.

The overhang is computed from the theme's line box (`LineRole::Line`), matching titles from `Meta` to `Body` size. For rows shorter than the line box (the name row of a settings row, whose line height is the body size, 13), use `line_hang(view)`: content as tall as a tool (`COMPACT_TARGET`) overhangs by half above and below and takes no row height at all, so it never makes its row taller, whatever that row's height; the row needs padding above and below to fit the overhanging part. When the title itself is taller than the target (`Heading`, `Title`), write `heading_tools(..).inline()`: the tools stay inline and the row height is the larger of the two.

## Styles

`heading_row` and `heading_tools` both take `.class` / `.css` / `.visible`, applied to their own row or toolbar. Align a multi-line title to the top with `heading_row(..).css(css! { align-items: flex-start; })`.

## When not to use it

- Actions that move a task forward (Save, Next) use text buttons placed below the content, not heading tools.
- Actions on a single row go at the end of that row, not into heading tools.

## API

| API | Description |
| --- | --- |
| `heading_row(title, tools)` | Heading row |
| `HeadingRow::lead(view)` | Leading element before the title |
| `heading_tools(tools)` | Heading toolbar; overhangs by default |
| `HeadingTools::inline()` | No overhang; row height follows the tools |
| `heading_hang(view)` | Hangs non-tool content on a heading row, with the same overhang |
| `line_hang(view)` | Hangs content on a row of any height, taking no row height (use it on rows shorter than the heading line box) |
| `COMPACT_TARGET` | Target step for compact tools (`SpacingStep::Page`) |
| `heading_overhang()` | Overhang amount (theme expression) |

## See also

[Layout and composition](../guide/composition.md) · [ScrollView](scroll-view.md) · [IconButton](icon-button.md) · [Overview](index.md)
