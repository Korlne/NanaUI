# Text Measurement, Shaping and Painting

NanaUI's text measurement, OpenType shaping and retained text layout have a single authority, `nana-text`. Glyph rasterization, atlas packing and GPU painting are done by `SceneWgpuPainter::text` in `nana-ui` (`scene_paint/text/`). Measurement and painting share one process-wide engine instance (`text_engine.rs`), and the `NanaTextShaper` that `RuntimeDocument::flush` calls is its measurement entry point.

```text
Text node in UiWorld
    │  NanaTextShaper (flush phase)
    ▼
nana-text               font matching → OpenType shaping (ShapedText) → line layout (TextLayout)
    │  TextLayoutId generational handle stays on the node
    ▼
UiScene                 ScenePrimitiveKind::Text (carries the retained TextLayout directly)
    │
    ▼
SceneWgpuPainter::text  glyph rasterization, multi-page atlas upload, painted into the current frame in document order
```

For the full details on cache keys, CJK punctuation compression and the evaluation corpus, see the maintainer documents [Text engine](../reference/text-engine.md) and [Learn the architecture: text engine](/learn/text-engine).

## Engine responsibilities and the three-stage pipeline

`nana-text` owns the text intermediate representation (IR), generational handles (`{ index, generation }`; the generation increments when a slot is recycled, and old handles are rejected outright), and the scheduling logic for "when to reshape and how to break and lay out lines". The underlying standard algorithms (HarfRust OpenType shaping, skrifa font table reading, Unicode BiDi and UAX #14 line-break tables) are wrapped in private modules. Layout enums (variation axes, letter spacing, line breaking, alignment, direction, writing mode, line height) reuse the definitions from `nana-ui-core` directly.

A run of text goes from style to layout through three strictly decoupled stages:

```text
1. Font matching (FontSystem)  : TextStyle → FontQuery → FontSelection → FontAssignment
2. Text shaping (Shaper)       : segment by grapheme cluster / Script / BiDi → HarfRust shaping → immutable ShapedText
3. Paragraph layout (Layouter) : ShapedText + TextConstraints → single-line fast path or UAX #14 wrapping → immutable TextLayout
```

- **Stage 1: font selection and coverage fallback**. The font layer reduces the style to one query, selects a primary font and a fallback chain, and assigns text ranges to concrete faces by `cmap` coverage. Variation axes the font does not have are skipped safely, never misreported or rewritten as `wght`. If the face chosen by coverage still produces `.notdef` missing glyphs after shaping, the shaper automatically retries with a candidate font.
- **Stage 2: OpenType shaping**. Converts the character sequence into immutable `ShapedRun`s. This stage does not wrap lines or compute caret positions. Segmentation strictly respects grapheme cluster, script and BiDi boundaries (style span boundaries never split a grapheme cluster; paragraph direction comes from the style's `direction`, not from guessing the first character). **Color, opacity, geometric transforms, maximum width and wrapping mode do not enter the shaping cache key**: labels with the same text and shaping style share one `ShapedText` across the whole process.
- **Stage 3: line layout**. Combines `ShapedText` with the container's width/line-count constraints to produce an immutable `TextLayout`. Ordinary single-line, non-wrapping labels with no height limit take the single-line fast path. Multi-line paragraphs wrap at UAX #14 break opportunities using the exact shaped glyph advances (never a rough estimate by character count). **Changing container width, wrapping mode, line height or alignment only reruns step 3 layout and never triggers reshaping**.

## Retained handles and tiered invalidation

The Runtime node (`TextNodeState`) stores only the logical text, revision numbers and a `TextLayoutId` handle; the actual shaping and layout caches are held by the process-wide engine. When a node changes, Runtime routes the work precisely at the write site according to the level actually affected:

| What changed | Text work triggered |
| --- | --- |
| Text content, font family, font size, font weight, letter spacing, variation axes (including the matching rich-text spans) | Reshaping + relayout |
| Container width, wrapping policy, line height, alignment, inline object (sticker/image) size | **Relayout only** (reuses the existing `ShapedText`) |
| Plain text color, inline object display content | **Zero text engine work** (no shaping, no layout, repaint only) |
| Color, stroke, shadow, decoration lines of rich-text spans | **Zero text engine work** (the painter only rebuilds glyph draw instances for the affected range) |
| Node `transform` / `opacity`, per-glyph reveal effects (`GLYPH_PRESENTATION`) | **Does not touch the text layer at all** (evaluated only in the compositor) |

When the node's three revision numbers and the font generation are all unchanged, the work is skipped with an $O(1)$ integer comparison before any string is copied or any cache is queried, at a cost independent of text length.

## Editable text and IME preedit

Static labels do not allocate an editing session. Only editable text controls mount the editing layer:

- **Layout split by paragraph (newline)**: each paragraph holds its own `TextLayout`, so typing a character only relays out the paragraph it is in. Merely moving the caret or changing the selection neither reshapes nor relays out.
- **Preedit is a transient overlay**: during IME composition the committed underlying text stays unchanged, and the screen lays out "committed text + preedit overlay". On blur, unfinished preedit is discarded automatically rather than committed into business text and then undone. The caret, pointer hits and selection rectangles are all derived directly from the retained `TextLayout`.

## Painting: consuming the retained layout directly

`SceneWgpuPainter::text` paints directly with the `TextLayout` handle Runtime left on the node and does not lay the text out again from the scene description (only built-in hints without a handle, or special decoration nodes whose alignment box was rewritten, fall back to layout in the painter).

The glyph raster cache key excludes color, opacity and scene transforms, so the same glyph in different colors shares one atlas bitmap. When layout, subpixel phase and font generation are all unchanged, the painter reuses the already generated glyph instance buffer. If the GPU device is lost and rebuilt, the old atlas is released with the old painter, and the first frame on the new device automatically re-rasterizes and uploads the glyphs it uses, while paragraph layout itself still reuses the `TextLayout` Runtime retained.

## Read next

<div class="next-steps">
  <a class="next-step" href="/architecture/window">
    <p class="next-step-link">Windows</p>
    <p class="next-step-caption">Native windows, custom title bars and the system material boundary.</p>
  </a>
  <a class="next-step" href="/reference/text-engine">
    <p class="next-step-link">Text engine reference contract</p>
    <p class="next-step-caption">The full specification for font fallback, the shaping cache, CJK layout and the editable path.</p>
  </a>
  <a class="next-step" href="/architecture/frame">
    <p class="next-step-link">A frame</p>
    <p class="next-step-caption">When text is measured in the flush transaction and how it converges with layout feedback.</p>
  </a>
</div>
