# Taffy reference benchmark

NanaUI's layout performance reference is [Taffy](https://github.com/DioxusLabs/taffy) 0.11 (the Rust layout engine used by Bevy, Dioxus, Zed and others). Taffy keeps a style, a layout cache and the computed box per node and stops there; it is the baseline for how fast "layout and nothing else" can be. `perf/micro/taffy-reference` has both lay out the same boxes on the same machine and answers three questions:

1. **What the reference is**: Taffy 0.11, on the same pages with the same CSS, every box checked to come out the same.
2. **How large the gap is**: cold layout, incremental edits, resident memory and instructions per box for block, flex and grid.
3. **What the extra cost buys**: what NanaUI stores and does per node beyond Taffy, which capability each item serves, and what the capabilities Taffy has no counterpart for cost in themselves.

The short answer: flex and grid cold layout is 1.4–1.6× slower while instructions per box are only 1.1–1.2× higher; by the fourth round's breakdown, turning off the three things kept for incremental edits and public geometry (plans, intrinsic facts, the published `LayoutResult`) lands flex at Taffy's per-box time. The gap is a trade-off in features, not a failure to catch up. Block is the furthest apart (11×) because Taffy's block is a very short algorithm, while NanaUI's block containers go through the same per-node retained work as its flex containers.

This is an observation, not a gate; the numbers move with the machine and its load, so read the ratios and where the cost goes.

## Running

The benchmark is a workspace of its own with its own lockfile, outside the root workspace; Taffy is a dependency of the benchmark only and never of a product crate.

```sh
cd perf/micro/taffy-reference
cargo run --release -- check              # both engines box by box (100 rows by default)
cargo run --release -- check 1000         # 21 001 boxes
cargo run --release -- compare 100,1000   # block / flex / grid, 2 101 and 21 001 boxes
cargo run --release -- features           # NanaUI-only capabilities, 1000 rows by default
ROUNDS=12 cargo run --release -- compare  # rounds to take the least of (8 by default)
SHOW=1 cargo run --release -- features    # also prints counters and stages of the inline edit and the scroll frames
/usr/bin/time -l target/release/nana-taffy-reference reps flex nana 1000 5
                                          # a fixed amount of work, to read instructions retired from outside
```

## Method

- **Pages**: N rows of 10 cells under a 1280 px root, each cell with 0.25 px padding around one 24 × 16 leaf (`1 + N × 21` boxes, the shape of `perf/micro/stack-depth`'s `wide`). Block page: rows and cells are blocks. Flex page: the root is a column, rows are rows with a 4 px gap. Grid page: every row is a grid of ten `1fr` columns with a 4 px column gap.
- **The same CSS**: on the NanaUI side `Stack::from_layout` writes out the same style as Taffy's field by field, one shared layout per role. Blocks are written as in CSS, `display: block` alone: their `auto` width fills the containing block, as in Taffy. Taffy's rounding is off (NanaUI keeps fractional pixels). `check` compares every box's page position and size: 21 001 boxes for each of the three layouts and 2 103 for the flex page with a header, with 0 boxes differing by more than 0.01 px.
- **Cold layout**: for NanaUI, the product frame's first layout (`layout_document_for_viewport`, layout only, no Scene, hit testing or accessibility); for Taffy, `compute_layout`. "Build + layout" starts from an empty tree.
- **Incremental edits**: NanaUI edits in a product frame (`RuntimeDocument::flush`) and reports the layout stage and the whole frame (with the hit index, accessibility and extraction); Taffy does `set_style` (which marks the node and its ancestors dirty as `mark_dirty` does) and `compute_layout`.
- **Timing**: within each round the two engines take turns, swapping order every round, and the least is read; the whole program runs twice (8 and 12 rounds) and the least of both is taken. Release, aarch64 macOS, 2026-11-09, machine load 3–5.
- **Instructions**: read in process from `proc_pid_rusage`'s `ri_instructions`, which is what `/usr/bin/time -l` prints as instructions retired, also the least over the rounds. Cross-checked against `time -l` (`reps flex … 100`, 25 runs less 5, over 20 × 2 101 boxes): NanaUI 39.9k per box (with teardown) against 38.3k for build + layout in process; Taffy 18.0k against 17.8k.
- **Memory**: the heap the laid-out tree holds (allocations less frees), not allocation traffic.
- **Why this differs from Resource budgets**: the 1.24 µs / box, 2 523 B and 23.3k layout instructions at 21 001 boxes in [Resource budgets](/reference/resource-budgets) come from `stack-depth`'s `wide`, whose page uses the `Stack::row` / `column` presets (rows that shrink to content, children centred, and so on), and whose instruction counts subtract whole processes, teardown included. Here the styles are CSS matched one for one with Taffy's, so the absolute values are somewhat lower and the ratios are comparable.

## The gap

21 001 boxes (per box):

| Layout | Engine | Cold layout | Build + layout | Resident | Layout instructions | Build + layout instructions |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| block | NanaUI | 0.99 µs | 2.03 µs | 2 399 B | 17.7k | 37.7k |
| block | Taffy | 0.09 µs | 0.19 µs | 1 252 B | 1.8k | 3.6k |
| block | ratio | 11× | 10.7× | 1.9× | 9.8× | 10.6× |
| flex | NanaUI | 1.00 µs | 2.04 µs | 2 399 B | 17.8k | 37.8k |
| flex | Taffy | 0.71 µs | 0.82 µs | 1 252 B | 15.7k | 17.4k |
| flex | ratio | 1.4× | 2.5× | 1.9× | 1.1× | 2.2× |
| grid | NanaUI | 1.39 µs | 2.43 µs | 2 638 B | 26.0k | 46.0k |
| grid | Taffy | 0.89 µs | 1.00 µs | 1 278 B | 21.3k | 23.0k |
| grid | ratio | 1.6× | 2.4× | 2.1× | 1.2× | 2.0× |

At 2 101 boxes (everything in cache) cold layout is 0.78 against 0.07 µs for block, 0.79 against 0.64 µs for flex (1.24×) and 1.16 against 0.82 µs for grid (1.4×); resident 2 692 / 2 930 B against 1 256 / 1 283 B.

Incremental edits at 21 001 boxes: one leaf in the middle row toggles its `min-height` between 16 and 24 px (that row grows and every row below moves). Per edit:

| Layout | NanaUI layout stage | NanaUI whole frame | NanaUI instructions | Taffy layout | Taffy instructions | Layout ratio | Instruction ratio |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| block | 194 µs | 490 µs | 10.9 M | 37.8 µs | 0.82 M | 5.1× | 13× |
| flex | 206 µs | 513 µs | 11.2 M | 94.2 µs | 2.15 M | 2.2× | 5.2× |
| grid | 459 µs | 758 µs | 16.2 M | 99.2 µs | 2.25 M | 4.6× | 7.2× |

How to read it:

- **Flex and grid**: 10–20% more instructions per box, 40–60% more time. Most of the extra time is the larger number of bytes each node reads and writes (layout is memory-bound on large trees, see the scale curve under "CPU time" in Resource budgets), not more computation.
- **Block**: Taffy's block algorithm is very short (1.8k instructions per box, a ninth of its flex). NanaUI spends almost the same time per box on block containers as on flex ones: the cost is the retained work every node does (next section), not the formatting context's algorithm. A block container also decides how each child takes part (inline-level, float, `display: contents`) and records line-box and baseline answers, which Taffy's block does not support. The absolute gap is 0.9 µs per box.
- **Building**: NanaUI builds at about 1.04 µs per box, Taffy at about 0.1 µs; see the next section.
- **Incremental**: the layout stage is 2–5× slower; the other half of the whole frame is the hit index, accessibility and extraction, which Taffy does not have.

## What the extra cost buys

The flex page at 21 001 boxes, per box. The time shares come from the fourth cold-layout round under "CPU time" in [Resource budgets](/reference/resource-budgets) (turning each item off and measuring again on the `wide` page, for the breakdown only, with wrong results), scaled to the 1.00 µs here; the items do not add up, and all four off is −28%. In the memory column, "tree before layout" and "retained by layout" are measured by this benchmark; the finer items come from the allocation-stack attribution under "Resident memory" in Resource budgets (same page shape, rounds of 2026-11-05 to 11-09) and are approximate.

| Done / stored beyond Taffy | Per box | Capability it buys |
| --- | ---: | --- |
| Measurement plans and container plans (built for every container on every pass; the first layout prepares them for the next frame) | about 17%, ≈ 0.17 µs; ≈ 395 B (measurement plans 244, container plan table 151) | Incremental edits with no measurement: successive size changes within a bucket measure nothing, one row growing among a million measures at most 2, child-list edits replay instead of walking every child (9 `WorkCounters` contracts; not building plans for single-child containers turns them red, see round four in Resource budgets) |
| Intrinsic facts (what each box's used size depended on, recorded while measuring) | about 10%, ≈ 0.10 µs; ≈ 120 B (intrinsic table) | Narrow invalidation: an edit lays out only the boxes it actually affects |
| Publishing `LayoutResult` (a 280 B struct per node, plus slices of fragments, parts, child placements and dependencies) | about 20%, ≈ 0.20 µs; ≈ 614 B | The public geometry contract: fragments, parts, baselines, overflow, scroll extent, containing block; Scene, hit testing, accessibility and IME read it |
| All three above (both plans, intrinsic facts, publishing) off | −28%, ≈ 0.72 µs left | On par with Taffy's 0.71 µs |
| Other retained tables (boxes, placements, padding, contexts, frames, line-box memo, plan entries) | ≈ 230 B | Retained state for relative geometry (`abs = frame + local`) and line-box answers |
| Building: components, node records, interned styles, mount commits | 1.04 µs against Taffy's about 0.1 µs; 1 043 B (Taffy's whole tree is 1 252 B) | The component model and atomic `MutationQueue` commits; the `NodeSpace` classification on `NodeRecord` (2 B) that lets moves, hit testing and accessibility skip subtrees; component views, interaction and accessibility state |
| The product frame's other stages: hit index, accessibility, extraction and Scene | first frame 0.18 + 0.26 + 0.61 µs; +1 897 B (4 296 B resident after the product frame) | Hit testing, the accessibility tree, the GPU scene; this benchmark does not split the 1.9 KB between them |

Together: the 0.29 µs flex cold layout spends beyond Taffy is essentially what is kept for incremental edits and public geometry, and about 1.13 KB of the 1 356 B retained by layout is plans, intrinsic facts and published results. Turning them off gives up the incremental contracts and the `LayoutResult` contract, so they are no longer listed as a gap to close; see "To do" in Resource budgets.

## Capabilities Taffy has no counterpart for

NanaUI only, same machine, also the least over rounds (`features 1000`):

| Capability and scene | Cost | Incremental / query | Where the cost lands |
| --- | --- | --- | --- |
| **Inline text wrapping and baselines**: 1 000 paragraphs, each 400 px wide with 4 text runs (mixed Latin and CJK) and 2 inline-blocks, broken into 3 lines (7 001 boxes) | first product frame 3.98 µs / box (layout stage 1.02), 4 789 B / box, 68.4k instructions / box | one run's text in a middle paragraph changed: layout 1.60 ms, frame 1.90 ms, 44 M instructions | nana-text shaping and line breaking (the TextShape stage), inline records, the line-box memo and baselines; the line count changes and the 500 paragraphs below only move their origin (1 measured, 505 placed) |
| **Float**: every cell `float: left` in block rows, each row an `overflow: hidden` BFC as tall as its floats (16.5 px, 21 001 boxes) | 0.97 µs / box, 16.5k instructions | the same page stacked without floats: 1.02 µs / box, 17.7k | Packing floats costs no more than stacking; the cost of supporting floats is already in every block container's participation check, i.e. in the block gap above |
| **Sticky and fixed**: the flex page in an 800 px scroller, every tenth row `sticky; top: 0`, plus 20 `fixed` badges | first frame 2.88 µs / box (2.85 without them) | one scroll: whole frame 6.32 ms, 94.7 M instructions; 5.48 ms and 85.9 M without them; layout stage 0 | Sticky sticks after scroll projection and the Scene places it, with no layout writeback; scrolling does not lay out. Most of the frame is accessibility (about 6 ms) and hit testing (about 0.8 ms), see "Observations" |
| **Moves under retained relative geometry**: a header grows and the column holding every row moves down | 2 100 boxes in the column: layout 5.9 µs, frame 13.6 µs, 0.25 M instructions; 21 000 boxes: 5.8 µs, 17.9 µs, 0.35 M | at both sizes 1 box measured and 3 placed; Taffy 1.0 µs for the same edit | Boxes are stored as offsets from their frame, so moving a parent changes its one offset and the subtree is not laid out again; hit testing, accessibility and the Scene do not walk it either. The cost does not grow with the boxes moved |
| **Hit testing and the accessibility tree**: the flex page's first product frame (21 001 boxes) | per box: hit index 0.18 µs, accessibility 0.26 µs, extraction 0.61 µs; 4 296 B resident (2 399 B after layout) | one hit query 0.16 µs; projecting the whole accessibility tree 0.17 µs / node | Hit entries and accessibility nodes are kept in local coordinates, so a move changes only the root |
| **Deep trees without stack overflow**: a chain of nested flex columns on a 512 KiB thread | 200 levels: NanaUI whole frame 794 µs, Taffy layout 232 µs | 500 levels: NanaUI whole frame 1 969 µs, Taffy overflows the stack | Placement walks at most `SPAN` levels on the thread stack, and measuring and building switch to a new stack segment when the remaining stack runs low; see "Stack" in Resource budgets |

## Observations

These were measured along the way; they are not gates and are not addressed by this change:

- **An edit in an inline paragraph scales with the paragraphs it moves**: changing one run takes about 0.17 ms of layout on a 100-paragraph page and about 1.6 ms on a 1 000-paragraph page, with 1 box measured and the rest paragraphs that only move their origin; the flex page's row-height edit, which moves 500 rows as well, lays out in about 0.2 ms. Each moved paragraph costs about 3 µs, an order of magnitude more than a row of plain boxes.
- **One scroll of a large scroller is linear**: at 21 001 boxes a scroll frame takes about 5.5 ms, nearly all of it in the accessibility and hit-testing stages (2 100 clip updates); layout and extraction take under 20 µs.
- **A block built through `Stack::from_layout` with no width shrinks to its content** (fixed): a container with an explicit `display: block` / `flow-root` now lays its children out as CSS block layout does, a block-level child with an `auto` width filling the containing block whatever `align_items` says (`LayoutStyle::child_align_items`); this benchmark is back to writing `display: block` alone, and `check` still matches box for box. The same change fixed a block of nothing but floats summing their heights as if stacked: floats no longer count toward an ordinary block's height, a BFC contains the packed floats, and a row of floats needs no fixed height. With the natural writing, layout instructions per box in a same-machine A/B: float page 16.5k → 17.4k (measuring now packs the floats too), stacked page 17.8k → 18.0k; the times in the table above are still from before the change. Since then a container with no `display` lays out as the same block layout too (it used to be a `Stack` flow that shrank its children by `align_items`), so a block gets this box with or without `display: block`; the benchmark still writes `display: block`, one for one with Taffy's `Display::Block`. With the blocks' `display` left unset, `check 1000` still finds 0 boxes that differ (before the change that page had 11,000, the worst 1,255.5 px off). Against the previous engine, 8 alternating rounds, least of each, layout instructions per box: block page 17.9k → 17.9k (the same page with no `display` 18.0k, 21.4k before the change), flex 17.9k → 17.9k, grid 26.0k → 26.1k; a line-height edit 10.92M → 10.94M, all within noise.
