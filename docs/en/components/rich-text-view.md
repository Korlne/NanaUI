# RichTextView

`RichTextView` draws text with styled ranges. Its value is a `RichText`: a string plus styles attached to byte ranges. The application owns the value; the control takes only a clone to project.

There is no `<RichTextView>` in the control table. The tag `rich-text` only creates unstyled plain text; styled values come in from Rust.

## Basic usage

`RichText::builder()` builds the text one run at a time. `plain` uses the node's own style, and `push` attaches a style to that run. Fields left unset inherit the node's computed style.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::RichTextView;
use nana_ui::runtime::rich::{PaintColor, RichSpanStyle, RichText};

let line = RichText::builder()
    .plain("Great work ")
    .push("today", RichSpanStyle::new().bold().color(PaintColor::srgb([1.0, 0.4, 0.5, 1.0])))
    .build();

view! {
    <Widget of={RichTextView::new(line).font_size(20.0)} />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::RichTextView;
use nana_ui::runtime::rich::{PaintColor, RichSpanStyle, RichText};

let line = RichText::builder()
    .plain("Great work ")
    .push("today", RichSpanStyle::new().bold().color(PaintColor::srgb([1.0, 0.4, 0.5, 1.0])))
    .build();

widget(RichTextView::new(line).font_size(20.0))
```

:::

Change the value with `set_component(entity, RichTextView::new(next))`. A value equal to the one already on the node commits nothing. Without a component, `MutationQueue::set_rich_text(id, rich)` works directly on any text node.

## Four style tiers

The fields of `RichSpanStyle` are grouped into four tiers by how much a change costs. A change is priced by the tiers it touches:

| Tier | Fields | After a change |
| --- | --- | --- |
| Shaping `shape` | `family`, `size_px`, `weight`, `stretch_percent`, `italic`, `letter_spacing_px`, `features` | Reshape and relayout; line boxes grow with it |
| Emphasis `emphasis` | `style`, `position` (`emphasis(...)`, `emphasis_position(...)`) | Relayout only: glyphs are not reshaped; line boxes grow by the mark height |
| Paint `paint` | `color`, `background_color`, `decoration`, `decoration_color`, `emphasis_color`, `stroke`, `shadows` | Repaint only: no shaping, no layout |
| Presentation | `effect` | Does not touch the text; handed to the compositor |

A run whose `size` is larger than the node's font size makes its line taller in proportion to the node's own line height. When the node uses an absolute line height, the proportion is the line height divided by the node's font size.

`stretch(percent)` is CSS `font-stretch`, a percentage of normal width (50–200). When the family has static condensed / expanded faces, it picks one; when the face has a `wdth` axis, it sets the axis to this value. `nana-text` makes the choice; see the rules in [Text engine](../reference/text-engine.md#matching).

`background_color(color)` fills color behind the run, like CSS `background-color` on an inline element: across lines, each line fills its own segment of the run. The rectangles come from the layout itself (the same algorithm as selection), so bidirectional text, vertical text, and ruby all line up. The background sits below the document selection, the editor selection, and the glyphs. Changing only the background color is a paint-tier change: no reshape, no relayout.

## Stroke, shadow, and decoration lines

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::RichTextView;
use nana_ui::runtime::rich::{PaintColor, RichSpanStyle, RichText, RichTextShadow, RichTextStroke};

let black = PaintColor::srgb([0.0, 0.0, 0.0, 1.0]);
let line = RichText::new("Look here").with_span(
    0..9,
    RichSpanStyle::new()
        .stroke(RichTextStroke::new(3.0, black))
        .shadow(RichTextShadow::new([2.0, 2.0], 6.0, black))
        .underline(),
);

view! {
    <Widget of={RichTextView::new(line)} />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::RichTextView;
use nana_ui::runtime::rich::{PaintColor, RichSpanStyle, RichText, RichTextShadow, RichTextStroke};

let black = PaintColor::srgb([0.0, 0.0, 0.0, 1.0]);
let line = RichText::new("Look here").with_span(
    0..9,
    RichSpanStyle::new()
        .stroke(RichTextStroke::new(3.0, black))
        .shadow(RichTextShadow::new([2.0, 2.0], 6.0, black))
        .underline(),
);

widget(RichTextView::new(line))
```

:::

- By default the stroke is drawn under the fill (`TextStrokePlacement::Under`): the glyph itself stays intact and a ring grows outside it. `Over` centers the stroke on the outline, matching the default of CSS `-webkit-text-stroke`.
- Up to four shadow layers (`MAX_TEXT_SHADOWS`), with the first on top. `blur_px` is the CSS blur radius; values above 24 logical pixels are drawn as 24. `spread_px` expands the coverage before blurring.
- Underline and line-through are drawn per line and per run, with position and thickness from the font's `post` / `OS/2` tables. Without `decoration_color` they follow the fill color of those characters.

Drawing order: shadows, stroke under the fill, underline, fill, stroke over the fill, line-through. All are glyph instances of the same paragraph, drawn in one draw.

The node's CSS (`text-decoration`, multi-layer `text-shadow`, `-webkit-text-stroke`, `paint-order`) is the base, and the span's paint tier sits on top. Fields the span does not set keep the node's values.

## Inline stickers

Stickers and emoji are objects in the text and wrap together with the characters. `RichTextBuilder::object` inserts a `RichObject`. It stands on the baseline; `descent` sinks it a little below. Changing its size only relays out; it does not reshape.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::RichTextView;
use nana_ui::runtime::rich::{RichObject, RichText};

let line = RichText::builder()
    .plain("Good morning")
    .object(RichObject::image(1, "file:///stickers/wave.png", 28.0, 28.0).descent(4.0))
    .plain("Have a great day")
    .build();

view! {
    <Widget of={RichTextView::new(line)} />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::RichTextView;
use nana_ui::runtime::rich::{RichObject, RichText};

let line = RichText::builder()
    .plain("Good morning")
    .object(RichObject::image(1, "file:///stickers/wave.png", 28.0, 28.0).descent(4.0))
    .plain("Have a great day")
    .build();

widget(RichTextView::new(line))
```

:::

- `RichObject::image(id, url, w, h)`: an image, with the same origin rules as CSS `url()`.
- `RichObject::texture(id, slot, w, h)`: a host texture slot. For animated images, the application decodes frames and writes them into this slot, and the host texture renderer draws it.
- `RichObject::chip(id, label, kind)`: a marker in the editor. It takes no width and the view does not draw it, so the editor and the view break lines the same way.

## Block images

`.flow(RichObjectFlow::Block)` puts the object on a line of its own, like a picture between two paragraphs: the text before it ends its line first, and the text after it starts on the next line. The text does not change and no line break needs to be inserted; the object is still one character, and the caret steps over it in one move. `.fit_width(true)` scales an object wider than the line down to the line width, like CSS `max-width: 100%`: the container can be narrower than the object and the object shrinks with it; it is never enlarged, and the `RichObject` still records the original size. The two can be used separately: an inline sticker can enable only `fit_width`.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::RichTextView;
use nana_ui::runtime::rich::{RichObject, RichObjectFlow, RichText};

let post = RichText::builder()
    .plain("The sea today")
    .object(
        RichObject::image(1, "file:///photos/beach.png", 1200.0, 800.0)
            .flow(RichObjectFlow::Block)
            .fit_width(true),
    )
    .plain("Very windy")
    .build();

view! {
    <Widget of={RichTextView::new(post)} />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::RichTextView;
use nana_ui::runtime::rich::{RichObject, RichObjectFlow, RichText};

let post = RichText::builder()
    .plain("The sea today")
    .object(
        RichObject::image(1, "file:///photos/beach.png", 1200.0, 800.0)
            .flow(RichObjectFlow::Block)
            .fit_width(true),
    )
    .plain("Very windy")
    .build();

widget(RichTextView::new(post))
```

:::

Changing the flow or `fit_width` only relays out; it does not reshape. Label objects (`tag`, `label`) are sized by their label and are not scaled.

## Ruby

`RichTextBuilder::ruby(base, annotation)` places a line of half-size annotation (kana, pinyin) above the base text. The base text never splits across two lines; when the annotation is wider than the base, the base is spread out, and the line grows by the annotation's height. Horizontal text only; in vertical text the ruby is not drawn.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::RichTextView;
use nana_ui::runtime::rich::RichText;

let line = RichText::builder()
    .plain("Today's word: ")
    .ruby("漢字", "かんじ")
    .build();

view! {
    <Widget of={RichTextView::new(line)} />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::RichTextView;
use nana_ui::runtime::rich::RichText;

let line = RichText::builder()
    .plain("Today's word: ")
    .ruby("漢字", "かんじ")
    .build();

widget(RichTextView::new(line))
```

:::

- The ruby uses the base text's style (font, weight, color, stroke, shadow) at half the font size and is revealed together with the base.
- `RichText::set_ruby(range, text)` / `clear_ruby(range)` change an existing document; an edit that reaches inside the base text drops that ruby.

## Per-glyph effects and typewriter reveal

`RichSpanStyle::effect(i)` makes a run play effect `i` of the effect table; `cx.set_rich_presentation(view, effects, reveal)` supplies that table and the reveal schedule. These are presentation only: no reshaping, no relayout, and no glyph rebuild. The text shader computes each glyph's position and opacity from the motion clock. Frames are requested only while something is still moving.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::RichTextView;
use nana_ui::runtime::rich::{GlyphEffect, GlyphIntro, RevealSchedule, RichSpanStyle, RichText};

let line = RichText::builder()
    .plain("Hi ")
    .push("all!", RichSpanStyle::new().effect(0))
    .build();
let view = cx.create_component(document, RichTextView::new(line))?;
let reveal = RevealSchedule::uniform(cx.animation_now(), 7, 0.08).intro(GlyphIntro::pop(0.2));
cx.set_rich_presentation(view, vec![GlyphEffect::wave(3.0)], Some(reveal))?;

view! {
    <Widget of={RichTextView::new(RichText::new("Hi all!"))} />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::RichTextView;
use nana_ui::runtime::rich::{GlyphEffect, GlyphIntro, RevealSchedule, RichSpanStyle, RichText};

let line = RichText::builder()
    .plain("Hi ")
    .push("all!", RichSpanStyle::new().effect(0))
    .build();
let view = cx.create_component(document, RichTextView::new(line))?;
let reveal = RevealSchedule::uniform(cx.animation_now(), 7, 0.08).intro(GlyphIntro::pop(0.2));
cx.set_rich_presentation(view, vec![GlyphEffect::wave(3.0)], Some(reveal))?;

widget(RichTextView::new(RichText::new("Hi all!")))
```

:::

- Effects: `shake`, `wave`, `jump`, `rainbow`, `pulse` (breathing scale), `flicker`; `stagger` is the phase offset between neighboring characters.
- Reveal: `at_s[i]` is when grapheme `i` starts its entrance (seconds, relative to `start`); `limit` stops the reveal before a given grapheme (a pause marker). Use `cx.animation_now()` for `start`, the same clock as the compositor.
- Entrance: `GlyphIntro::fade` / `pop` / `rise`.
- In vertical text (`vertical-rl` / `vertical-lr`), `wave`, `jump`, and `rise` automatically switch to the column's cross axis: "up" is to the right, and `rise` enters from the left. The direction comes from the node's `writing-mode`; nothing else needs to be set.
- Stickers are revealed with the grapheme they sit in and move with its effect.
- Cost: while playing, the host presents this window at 120Hz by default; a program caps it with `RuntimeProgram::max_frame_rate` (see [Application API](../reference/application-api.md#runtimeprogram)). When only per-glyph presentation is moving and nothing else has changed, from the second frame on the painter keeps what was drawn before the first text (backgrounds, panels, and so on) as a snapshot, then each frame copies the snapshot back and redraws only the text, without drawing geometry again. When the frame has other compositor animations, chart transitions, or a HostTexture / GPU node before the first text, the whole frame is redrawn as usual. The snapshot is released once per-glyph presentation stops.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `value` | `RichText` | Text and style ranges. Ranges are in bytes; an endpoint inside a character snaps back to the character boundary |
| `decorative` | `bool` | Still drawn and laid out, but removed from the accessibility tree |
| `font_size` | `f32` | Node font size, used by spans that set no size. Does not pin line boxes to an absolute height |
| `font_family` | `String` | Node font family, in CSS `font-family` syntax |
| `line_height` | `f32` | Line height, as a multiple of each run's own font size |
| `color` | `SemanticColorRole` | Semantic foreground for uncolored text |
| `width` / `max_width` / `nowrap` | | Same as [Text](text.md) |

## Events

None.

## See also

[Text](text.md) · [Overview](index.md) · [Text engine](../reference/text-engine.md#rich-text-spans)


### Combined effects

`GlyphEffectChain::new(vec![GlyphEffect::wave(3.0), GlyphEffect::rainbow()])` combines continuous effects on a run of text. Use `RichTextView::presentation_chains(chains, reveal)` or `AppContext::set_rich_presentation_chains(node, chains, reveal)`; the span's `effect` index then points into the chain table. The original single-effect `presentation` / `set_rich_presentation` APIs remain available.

Offsets add, scale and opacity multiply, and the last color effect supplies the text color. CPU evaluation of inline objects uses `evaluate_glyph_chain`, the same as the text shader. An empty chain, zero amplitude, or non-finite parameters do not form a continuous animation; zero frequency keeps the static pose but requests no continuous frames; with only a reveal, frame requests stop after the finite entrance ends. `GlyphEffect::flicker()` keeps the original brightness, and a non-default amplitude controls the flicker strength.
