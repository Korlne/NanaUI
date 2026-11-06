# callout

`callout(icon, tone, text)` is a notice strip colored by `StatusTone`: one icon, a piece of text, and an optional trailing action. It is a view in `view::marks`, not a registered control.

## Basic usage

```rust
use nana_ui::runtime::view::callout;
use nana_ui_core::{Icon, StatusTone};

callout(Icon::AlertTriangle, StatusTone::Warning, "The model is not optimized yet, so the first load is slower.")
```

- The caller supplies the icon; the color comes from the tone's role (`tone_role(tone)`).
- `text` can be bound; when it is empty the whole strip is hidden. `.visible(cond)` applies together with "hide when the text is empty".
- By default it has a light tone background and its own padding, the text starts at the top, and long text wraps.

## Two strengths

| Form | Appearance | Use for |
| --- | --- | --- |
| `callout(..)` | Light tone background, its own padding, body text size | Needs attention but does not interrupt work: incomplete configuration, limited functionality |
| `callout(..).quiet()` | No background, no horizontal padding, text one size smaller and muted, centered with the icon | A line of explanation inside a block already on the content line |

`.quiet()` has no horizontal padding because the block it goes into already has a content line (see [Layout and composition](https://github.com/sena-nana/NanaUI/blob/main/docs/guide/composition.md#内容线)). Put it in a container without padding and it hugs the edge.

## Trailing action

`.trailing(view)` puts a link or button at the end of the row: the trailing item does not shrink, and the text wraps first. When a one-line notice is paired with a taller button, write `.css(css! { align-items: center; })` to center them.

## Place in the status hierarchy

Inline explanation (`.quiet()`) → notice strip (`callout`) → notification (`Toast`) → dialog (`ConfirmDialog`). Use only one level for one thing; operation results go to a notification, so do not add a notice strip as well. Use `EmptyState` for empty lists.

## API

| API | Description |
| --- | --- |
| `callout(icon, tone, text)` | Notice strip |
| `Callout::quiet()` | One line of explanation without a background |
| `Callout::trailing(view)` | Trailing action |
| `.class` / `.css` / `.visible` | Apply to the notice strip's row |

## See also

[Layout and composition](https://github.com/sena-nana/NanaUI/blob/main/docs/guide/composition.md) · [Toast](https://github.com/sena-nana/NanaUI/blob/main/docs/components/toast.md) · [EmptyState](https://github.com/sena-nana/NanaUI/blob/main/docs/components/empty-state.md) · [StatusBadge](https://github.com/sena-nana/NanaUI/blob/main/docs/components/status-badge.md)
