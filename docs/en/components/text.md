# Text

`text(value)` draws a run of text. The template tag is `<Text>`.

## Basic usage

The argument is also the field: child text and `value` both write it. There are no events and no `v-model`.

Child text containing `{…}` expands to `text!("…")` and is recomputed when a signal it reads changes. A string with no interpolation expands to `text("…")`. For hand-written interpolation use `text!` from the same module.

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Text>"Total {count} items"</Text>
}
```

```rust rust
use nana_ui::runtime::view::text;

text!("Total {count} items")
```

:::

`text!` goes through `format!` on every evaluation. Calling `count.get()` directly at construction yields a constant from that moment and no longer follows changes. To follow changes, use `text!`, or pass `move || format!(…)` to `text`.

`value` can be a constant, a `Signal<String>` or a closure. A constant is written when the node is built. A signal is bound directly. A closure is boxed once.

When child text and `value="…"` are both present, the attribute wins and the child text is no longer used as `value`. Two consecutive left braces in a template string mean one literal brace, not interpolation.

## Selection

This is plain text, not another `TextInput`. The default is `user-select: auto`, which is not selectable. Written as `text`, `all` or `contain`, it can be copied, and the selection is document-wide. `text` selects by dragging, `all` selects the whole node on one click, and `contain` does not extend into neighbours. An empty selection does not clear the clipboard, and cut does not delete this read-only text. `none` stays out of selection.

When `::selection` is not written, the selection background uses the theme's `accent_soft`.

## Typography

Font size, weight, single-line and elision are on the `Text` component: `font_size`, `font_weight`, `nowrap`, `ellipsis`. `font_size` also sets the line box to the nearest control line height, so glyphs are not packed into a shorter box. Single-line elision is `nowrap(true)` plus `ellipsis(true)`. These are not methods on `text()` and are not in the `<Text>` field table.

`Text::line_height` overrides the line box derived from `font_size`. `width` and `max_width` are on the component as well.

The colour in views that can follow the theme is `.foreground(…)`. The role is resolved at paint time. Colours in a stylesheet are fixed when that sheet is compiled. `.foreground` is on every control that accepts a style, not just text. `color` writes a semantic foreground, not a business colour.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `value` | `String` | Child text and `value` both write it. Can be a constant, a `Signal<String>` or a closure. A constant is written when the node is built. A signal is bound directly. A closure is boxed once. When it appears together with `value="…"`, the attribute wins |
| `decorative` | `bool` | Still painted and laid out, but removed from the accessibility tree. For a leading letter or decorative glyph that repeats an ancestor's name; equivalent to `aria-hidden="true"` |

## Events

There are no events and no `v-model`.

## Slots

There are no named slots. Child text writes `value`.

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
