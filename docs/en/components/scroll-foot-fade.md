# scroll_foot_fade

`scroll_foot_fade(height, surface)` is a fade laid over the bottom edge of a scroll area: it goes from transparent to `surface`, telling the user the list has not reached its end instead of cutting off the last row hard. It is a view in `view::marks`, not a registered control.

## Basic usage

```rust
use nana_ui::runtime::view::{scroll_foot_fade, widget};
use nana_ui::runtime::{PositionSpec, ScrollAxes, ScrollView, SemanticColorRole, Stack};
use nana_ui::theme::ThemeToken;

let gutter = ThemeToken::PanelPaddingX; // the card's own horizontal padding; follows the theme
// This column sits in the card's content box: its left and right edges are the content lines.
widget(Stack::column(0.0).with_layout(|l| l.position = PositionSpec::Relative))
    .children((
        widget(ScrollView::new(ScrollAxes::Vertical).gutter_theme(gutter)).children(list),
        scroll_foot_fade(24.0, SemanticColorRole::Surface).clear_of_theme(gutter),
    ))
```

- The fade is absolutely positioned in the positioned container that holds the scroll area, against its bottom edge, spanning its full width.
- It takes no input: wheel and pointer events fall through to the list below.
- The end of the list leaves `height` of blank space so the last row can scroll out of the fade.
- Use the surface role behind the scroll area for `surface`, so the fade has no visible edge.
- When what is behind the scroll area is transparent or not a solid color (for example the gaps between floating cards on a stage), the surface color covers whatever is behind it. In that case use [`ScrollView::edge_fade`](scroll-view.md#edge-fade) to fade the content itself.

## Staying clear of the scrollbar

When the scroll area uses `ScrollView::gutter_theme(token)` to extend into the container's padding and draw the scrollbar there, give the fade `.clear_of_theme(token)` (the same value) and put it in the same positioned container as the scroll area (the column in the card's content box in the example above). The scroll area extends this distance beyond the container on the left and right, and the scrollbar lane is the part extending on the right; the fade extends with the scroll area on the left and stops at the container's right edge, where the lane begins. The fade therefore covers the whole content line and the left padding, the scrollbar is not dimmed, and the fade's bottom edge is the scroll area's bottom edge.

In a card, use `ThemeToken::PanelPaddingX`, the card's own horizontal padding: 16 in the default theme and 10 under `COMPACT_METRICS`, and the scroll area and the fade both follow it. The pixel versions `.gutter(px)` / `.clear_of(px)` hold only under the theme they were written for; for example, `gutter(16.0)` under COMPACT extends 6 beyond the card padding.

## API

| API | Description |
| --- | --- |
| `scroll_foot_fade(height, surface)` | Bottom fade |
| `ScrollFootFade::clear_of_theme(gutter)` | Pairs with a scroll area using `gutter_theme(gutter)` in the same container: covers the extent of the scroll area, leaves the scrollbar lane clear, and follows the theme |
| `ScrollFootFade::clear_of(px)` | Same, pixel version, pairs with `gutter(px)`; holds only under one theme |

## See also

[ScrollView](scroll-view.md) · [Layout and composition](../guide/composition.md#scroll-areas)
