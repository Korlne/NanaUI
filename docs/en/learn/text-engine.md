---
title: "Text Engine: nana-text"
---

# Text engine: `nana-text`

This chapter goes deep into `nana-text`: the framework's only authority for text measurement, OpenType shaping and retained text layout. By the end you can answer: within one `TextEngine::layout` call, what each of the three stages does ("font selection → text shaping → line layout") and where the cache boundaries sit; how the font layer uses generations (`FontGeneration`) and `cmap` coverage to decide "which font face each grapheme cluster uses"; why decoupling line layout from shaping completely makes continuous window resizing cost only line breaking rather than a shaping rerun; and how the Runtime keeps text layouts across frames through `TextLayoutStore` generation handles.

## Responsibilities and boundaries

`crates/nana-text/src/lib.rs` (`:7-21`) defines the territory of the text stack precisely:

- **Stage one: the font layer (#90)**: font registration, generation management, font face matching, variable-font axis coordinate resolution and missing-glyph fallback based on `cmap` coverage.
- **Stage two: OpenType shaping (#91)**: grapheme cluster / script / bidirectional text (BiDi) segmentation, HarfRust shaping and a bounded shaping cache (`ShapeCache`).
- **Stage three: paragraph line layout (#92)**: the single-line label fast path, greedy wrapping over UAX #14 break opportunities, per-line BiDi visual reordering, line box metrics, alignment, ellipsis truncation and a bounded layout cache (`LayoutCache`). `NativeTextEngine` gathers these three stages behind the single entry point `TextEngine::layout`.
- **Stages four to eight: retained handles and the single product authority (#95 / #99)**: text nodes in `UiWorld` keep their layout results through `TextLayoutStore` generation handles, Rust L3 and `.vue` both measure through the same process-level engine, and `SceneWgpuPainter::text` in `nana-ui` draws the retained `TextLayout` directly.
- **Low-level algorithm encapsulation and vocabulary borrowing**: `cosmic-text` and `cryoglyph` have been removed from the dependency graph entirely (guarded by `scripts/check-engine-boundary.py`); low-level standard libraries (`fontdb`, `skrifa`, `harfrust`, `unicode-bidi`, `unicode-linebreak`) stay strictly sealed in their own private submodules and never leak into public signatures; the 12 typography enums (such as `FontVariationSetting`, `LineBreakSpec`, `WritingModeSpec`, `lib.rs:30-37`) are imported directly from the `nana-ui-core` allowlist rather than redefined.
- **Cursor and editing are pure derivations over the IR**: cursor placement, hit testing and selection rectangles are pure functions over a completed `TextLayout` (`crates/nana-text/src/edit.rs`) and do not pollute engine cache state.

## Dependencies and neighbors

`crates/nana-text/Cargo.toml` (`:16-37`) specifies the single private file each external algorithm library is allowed to appear in:

| Dependency / neighbor crate | The only allowed module / collaboration |
| --- | --- |
| `nana-ui-core` | Provides the 12 typography enum vocabulary items and `WritingContext` |
| `fontdb 0.24` | System font directory scan, limited to `font/discovery.rs` |
| `skrifa 0.44` / `read-fonts` | Variable-font axes, named instances, color glyph tables and `cmap`, limited to `font/face.rs` |
| `icu_properties 2.3` | Unicode Script and `Emoji_Presentation` properties, limited to `font/unicode.rs` |
| `harfrust 0.12` | OpenType shaping (GSUB / GPOS), limited to `shaping/opentype.rs` |
| `unicode-bidi 0.3.18` | Unicode Bidirectional Algorithm (UBA), limited to `shaping/bidi.rs` |
| `unicode-linebreak 0.1.5` | UAX #14 break opportunity table, limited to `layout/breaks.rs` |
| `nana-ui-runtime` | The largest consumer: measures through `SharedTextEngine` and retains handles through `TextLayoutStore` |
| `nana-ui` | `SceneWgpuPainter::text` draws the `TextLayout` retained by the Runtime directly |

## Key types and data

| Type | File and line | Purpose |
| --- | --- | --- |
| `TextEngine` | `crates/nana-text/src/engine.rs:25` | The overall three-stage pipeline interface: only `font_generation` and `layout` |
| `NativeTextEngine` | `crates/nana-text/src/engine.rs:79` | `FontSystem` + `Shaper` + `Layouter` + ellipsis source + language hint + strut cache |
| `SharedTextEngine` | `crates/nana-text/src/engine.rs:50` | `Arc<Mutex<NativeTextEngine>>`: every window and painter in the process shares one instance |
| `TextSource` / `TextStyle` / `TextKind` | `crates/nana-text/src/source.rs` / `style.rs:28` / `:16` | Text source with rich-text spans and inline objects, base style, text kind (`Label` / `Paragraph` / `Editable`) |
| `TextConstraints` | `crates/nana-text/src/constraints.rs:30` | Container constraints: max width/height, wrap strategy, `max_lines`, ellipsis, writing mode, alignment, tab width |
| `ShapedText` / `ShapedRun` | `crates/nana-text/src/shaping/` / `shape.rs` | The shaping product: an immutable sequence of glyph runs (carrying direction, script and upright/sideways orientation) |
| `LayoutRequest` | `crates/nana-text/src/layout/engine.rs:33` | Input to one line-layout pass: a read-only `ShapedText`, container constraints, ellipsis, strut metrics, ruby/labels |
| `TextLayout` | `crates/nana-text/src/layout/ir.rs:182` | The retained layout IR: `runs` / `lines` / `bounds` / `overflow` / `objects` / `rubies` / `labels` |
| `LineBox` / `LineMetrics` | `crates/nana-text/src/layout/ir.rs` / `metrics.rs` | One line's run ranges and metrics (line width, line height, baseline offset) |
| `TextLayoutStore` / `TextLayoutId` | `crates/nana-text/src/layout/store.rs:54` | Retained layout repository of generational handles |
| `FontSystem` | `crates/nana-text/src/font/system.rs` | Font database: registration, generation increment, matching, instances and `cmap` coverage fallback |
| `FontGeneration` / `TextRevision` | `crates/nana-text/src/id.rs` | The two monotonic integers for $O(1)$ staleness decisions: font library generation and source text revision |
| `IntrinsicWidths` | `crates/nana-text/src/layout/engine.rs:109` | The `min-content` and `max-content` intrinsic width measurements |

## Internal structure

### 1. One call threading three stages: `NativeTextEngine::layout`

`TextEngine::layout(kind, source, base, constraints, counters) -> Arc<TextLayout>` (`crates/nana-text/src/engine.rs:36-43`) returns an immutable shared `Arc` — the same text with the same constraints hits the cache on later frames and receives the same pointer. Its internal implementation (`engine.rs:294-378`) advances through a fixed six steps:

1. **`fold_lines` (`:192`)**: under `white-space: normal`, newlines in the source are folded into spaces — the shaper and the line breaker must see exactly the same byte stream, so folding happens **before** shaping;
2. **`shape` (`:178`)**: builds a `ShapeRequest` from `base.language` or the engine default language and hands it to the `Shaper`, producing an `Arc<ShapedText>`;
3. **ellipsis shaping (`:203`)**: runs `const ELLIPSIS: &str = "…"` (`:23`) through the standard shaping path so that every truncated node in the same style shares one ellipsis cache entry;
4. **`strut_metrics` (`:218`)**: computes the strut metrics of the base style's own font face, used to lock the minimum line box height and baseline so a taller fallback glyph on the same line cannot tilt the baseline (cached by `(FontInstanceKey, canonical_f32_bits(size))` in a `BoundedCache` with capacity `1024`, `:76`);
5. **ruby and inline object label shaping (`:305-341`)**: each shaped at its proportional font size;
6. **line layout and counter diffing (`:343-376`)**: assembles the `LayoutRequest` and hands it to `Layouter::layout`, then writes this pass's cache hit/miss, resolved glyph and break opportunity deltas into `TextWorkCounters`.

### 2. The font layer: registration, generations and `cmap` coverage fallback

`FontSystem` (`crates/nana-text/src/font/system.rs:1-31`) maintains three core guarantees:

- **Byte lifetime decoupled from handle retirement**: font bytes are wrapped in `FontBlob` (`Arc`); unregistering a face only retires its `FontId` handle and never takes away bytes that background readers are still using (`:4-12`).
- **Bidirectionally precise generation invalidation (`FontGeneration`)**: every registration, unregistration, system font load or fallback policy switch increments `FontGeneration` (`:18-31`). The selection cache is cleared wholesale (a newly registered font may change the winner for any family list), while the `cmap` coverage cache removes only the exact faces that were unloaded; slots of retired `FontId`s are never reused by generation.
- **Deterministic selection and explainable diagnostics**: `FontQuery` (family list, weight, width, slant, language) combined with `FontOrigin` priority (explicit registration beats the system font scan, `:61-79`) and `FaceDescriptor` (`@font-face` declared family names override the names inside the font's own tables, `:82-90`) selects a face and records `FontChoiceReason` and `FontAssignment`, so diagnostic tools can answer precisely "why did this piece of text land on this fallback font".

### 3. The shaping layer: segmentation, BiDi, HarfRust and a bounded fallback budget

`Shaper` (`crates/nana-text/src/shaping/shaper.rs:27-34`) first splits the text source into **style segments (`StyleSegment:43`)**, then splits those into **items (`Item:51`)** by same style, same BiDi embedding level, same Unicode Script and same upright/sideways orientation, and splits items into **pieces (`Piece:64`)** by font face `cmap` coverage (the `U+FFFC` inline object placeholder never enters the shaper, `:58`).

To keep a stretch of rare or corrupt characters from exhausting every system font, fallback has two hard budgets: `MAX_FALLBACK_CANDIDATES_PER_RANGE = 8` (at most 8 candidate faces tried per missing-glyph range) and `MAX_FALLBACK_RETRIES_PER_ITEM = 32` (at most 32 retries per item, `shaper.rs:20-24`). Vertical text is split at shaping time under UAX #50 into upright runs (using the font's vertical metrics `vert` / `vrt2`) and sideways runs (shaped horizontally, rotated 90° clockwise when drawn).

### 4. The line layout layer: decoupled from shaping, UAX #14 wrapping and the `TextLayout` IR

The core design of `layout/mod.rs` (`crates/nana-text/src/layout/mod.rs:4-17`) is **fully decoupling line layout from OpenType shaping**: the `Layouter` only reads an already-shaped `ShapedText` and slices it into lines, never calling HarfRust again. So **when window drag-resizing changes the container width, the wrap mode is switched, `max_lines` is modified or horizontal alignment changes, only the lightweight line layout module reruns and the existing shaping cache is fully reused**.

Each line's termination reason is recorded precisely by `LineBreakCause` (`ir.rs:110-121`):

| Variant | Meaning |
| --- | --- |
| `Explicit` | An explicit newline `\n` in the source text |
| `Wrap` | Automatic wrapping triggered by exhausting the line width budget |
| `MaxLines` | Truncated by reaching the `max_lines` limit |
| `MaxHeight` | Truncated because the next line does not fit the `max_height_px` height budget |
| `EndOfText` | The natural end of the text |

Under vertical writing (`vertical-rl` / `vertical-lr`) line layout reuses the same code as horizontal in "line-relative coordinates"; only three places in the whole engine know the page turned: the line budget reads the container height instead of its width (`TextConstraints::inline_budget_px`), line boxes use the central baseline instead of the alphabetic baseline, and the final physical coordinate mapping (`physical_x_of_block` / `physical_size`, `ir.rs:170-176, :240`). Staleness costs one $O(1)$ two-integer comparison: `layout.is_stale(revision, font_generation)` (`:310-312`).

### 5. `TextLayoutStore`: retained handles versus the cache boundary

An ordinary LRU/FIFO cache evicts entries whenever it likes, but a layout attached to a retained tree node needs a handle that "lives and dies with the node and explicitly rejects access across worlds or from an older generation" — that is the job of `TextLayoutStore` (`crates/nana-text/src/layout/store.rs:54`). Slot generations come from the process-wide monotonic sequence `NEXT_GENERATION` (`:9-15`), and `resolve` checks both the slot generation and `FontGeneration` (returning `StaleLayout::Retired` or `StaleLayout::FontGeneration` when stale).

## Collaboration and data flow

`UiWorld` holds the process-level shared engine through `paint_text_engine: Option<nana_text::SharedTextEngine>` (`crates/nana-ui-runtime/src/world.rs:677`) and keeps `text_nodes: HashMap<StableNodeId, TextNodeState>` in a separate table inside `NodeStore` (`store.rs:436-444`), so large layouts can skip nodes with no text change. In `resolve_plain_text` (`world/text.rs:4197-4251`), if the truncated layout carries `OverflowFlags::ELLIPSIZED` and the parent box needs a natural width, the Runtime queries a natural layout once more with unbounded constraints to obtain the true maximum width before wrapping. The `TextLayoutId` finally retained on the node travels with `ExtractedNode` to `UiScene`, and `SceneWgpuPainter::text` reads the glyph runs directly, rasterizes and presents them.

## Code map

| File path | Core content |
| --- | --- |
| `crates/nana-text/src/lib.rs` | Facade exports, stage evolution documentation and the 12-item `nana-ui-core` typography vocabulary allowlist |
| `crates/nana-text/src/engine.rs` | `TextEngine` / `NativeTextEngine` / `SharedTextEngine`: the three-stage pipeline entry |
| `crates/nana-text/src/font/system.rs` | `FontSystem`: font registration, generation increments and precise cache invalidation |
| `crates/nana-text/src/font/{matching,fallback,variations,coverage,face,discovery,unicode,query}.rs` | Face selection, `cmap` coverage fallback, variable-font axes, system font scan and Unicode properties |
| `crates/nana-text/src/shaping/{shaper,bidi,opentype,cache,key}.rs` | Shaping segmentation, BiDi levels, the HarfRust wrapper, `ShapeCache` and cache keys |
| `crates/nana-text/src/layout/{engine,lines,breaks,ir,store,cache,key}.rs` | Line layout solving, UAX #14 break points, the `TextLayout` IR and `TextLayoutStore` |
| `crates/nana-text/src/edit.rs` / `editable/` | Cursor placement, hit-test derivation and editable segmented sessions |

## Further reading

- [Core contracts](/learn/core-contracts): where the typography vocabulary this crate borrows is defined in `nana-ui-core`
- [Layout engine](/learn/runtime-layout): how intrinsic text measurement converges with box-level layout feedback
- [Retained tree: UiWorld](/learn/runtime-tree): how `TextNodeState` and the sparse side tables of `NodeStore` are organized
- [Architecture: text](/architecture/text) · [Text engine reference](/reference/text-engine): the text specification for applications and maintainers
