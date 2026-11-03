//! The least thread stack a layout needs at a tree depth, and what one takes.
//!
//! ```text
//! cargo run --release -- search --engine nana-frame --depth 500
//! cargo run --release -- report            # every engine at depths 64..500
//! cargo run --release -- wide              # a wide page: time, heap, allocations per box
//! cargo run --release -- wide-loop         # that page over and over, for a profiler
//! /usr/bin/time -l target/release/nana-stack-depth reps 3200 10 nana
//!                                          # a fixed run: instructions and cycles per box;
//!                                          # SKIP_LAYOUT=1 builds the page only
//! FRAME_PATH=1 cargo run --release -- wide  # the product frame's first layout
//!                                          # (`layout_document_for_viewport`), not a full pass
//! CONTROLS=1 cargo run --release -- wide    # a page of labels and buttons instead of bare boxes
//! cargo run --release -- edits 200,2000   # one edit a product frame: layout and frame time
//! cargo run --release -- stages 250,500    # a deep frame's least time per stage
//! ```
//!
//! A probe runs the work once on a thread of the stack it is given; a stack
//! overflow aborts the process, so `search` runs each probe as a child
//! process and bisects. `nana-layout` and `taffy` lay out the same nested
//! flex columns (a quarter pixel of padding each); `nana-frame` takes a whole
//! NanaUI frame through the same tree: mount, layout, scene extraction, hit
//! testing, accessibility, an update and teardown.

use std::alloc::{GlobalAlloc, Layout, System};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::Instant;

use nana_ui_core::{LayoutStyle, LengthSpec};
use nana_ui_runtime::view::{IntoView, text, widget};
use nana_ui_runtime::{
    AppContext, Button, ComputedStyle, DocumentId, FrameStage, LayoutViewport, MutationQueue,
    StableNodeId, Stack, Text, TextContent, TextMetrics, TextShapeConstraints, TextShaper,
};
use nana_ui_scene::RuntimeDocument;

const ENGINES: [&str; 3] = ["nana-layout", "nana-frame", "taffy"];

/// Live heap bytes: allocations minus frees.
static LIVE: AtomicIsize = AtomicIsize::new(0);
/// Allocation calls (`alloc`, `alloc_zeroed`, `realloc`).
static CALLS: AtomicIsize = AtomicIsize::new(0);

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        CALLS.fetch_add(1, Ordering::Relaxed);
        LIVE.fetch_add(layout.size() as isize, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        CALLS.fetch_add(1, Ordering::Relaxed);
        LIVE.fetch_add(layout.size() as isize, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        CALLS.fetch_add(1, Ordering::Relaxed);
        LIVE.fetch_add(
            new_size as isize - layout.size() as isize,
            Ordering::Relaxed,
        );
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size() as isize, Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// The wide page: a 1280 px column of `rows` rows of `cells` cells. Returns
/// the middle row's sixth cell's leaf (its label on a controls page), which
/// `edits` changes.
fn nana_page(cx: &mut AppContext, document: DocumentId, rows: usize, cells: usize) -> StableNodeId {
    let mut target = None;
    let root = cx
        .create_component(document, Stack::column(0.0).width(LengthSpec::Px(1280.0)))
        .unwrap();
    // `CONTROLS=1`: a page of controls, not bare boxes -- every cell holds a
    // label and a button, so the passes meet visuals, text and their tables.
    let controls = std::env::var_os("CONTROLS").is_some();
    for r in 0..rows {
        let row = cx
            .create_detached_component(document, Stack::row(4.0))
            .unwrap();
        cx.append_child(root, row).unwrap();
        for c in 0..cells {
            if controls {
                let cell = cx
                    .create_detached_component(document, Stack::column(2.0))
                    .unwrap();
                cx.append_child(row, cell).unwrap();
                let label = cx
                    .create_detached_component(document, Text::new("Label"))
                    .unwrap();
                cx.append_child(cell, label).unwrap();
                if (r, c) == (rows / 2, cells / 2) {
                    target = Some(label.stable_id());
                }
                let button = cx
                    .create_detached_component(document, Button::new("Go"))
                    .unwrap();
                cx.append_child(cell, button).unwrap();
                continue;
            }
            let cell = cx
                .create_detached_component(
                    document,
                    Stack::column(0.0).with_layout(|l| *l = padded()),
                )
                .unwrap();
            cx.append_child(row, cell).unwrap();
            let leaf = cx
                .create_detached_component(
                    document,
                    Stack::row(0.0)
                        .width(LengthSpec::Px(24.0))
                        .min_height(LengthSpec::Px(16.0)),
                )
                .unwrap();
            cx.append_child(cell, leaf).unwrap();
            if (r, c) == (rows / 2, cells / 2) {
                target = Some(leaf.stable_id());
            }
        }
    }
    target.unwrap()
}

/// A page-shaped tree: `rows` rows of `cells` cells under one column, each
/// cell a column holding a row (four levels, `1 + rows * (1 + 2 * cells)`
/// boxes). Returns the microseconds the layout took and those the build and
/// layout took together, the live heap bytes
/// the laid-out tree holds and the allocation calls the layout made, the
/// first and last without building it.
fn nana_wide(rows: usize, cells: usize) -> (f64, f64, isize, isize) {
    let document = DocumentId::new(1).unwrap();
    let before = LIVE.load(Ordering::Relaxed);
    let built = Instant::now();
    let mut cx = AppContext::new();
    nana_page(&mut cx, document, rows, cells);
    let calls = CALLS.load(Ordering::Relaxed);
    let start = Instant::now();
    // `SKIP_LAYOUT` leaves the build alone in a `reps` run: subtracting it
    // gives the layout's own instructions and cycles.
    if std::env::var_os("SKIP_LAYOUT").is_some() {
    } else if std::env::var_os("FRAME_PATH").is_some() {
        // The product frame's first layout: a viewport pass over a document
        // laid out for the first time.
        cx.layout_document_for_viewport(document, LayoutViewport::new(1280.0, 800.0), &[])
            .unwrap();
    } else {
        cx.layout_document(document, LayoutViewport::new(1280.0, 800.0))
            .unwrap();
    }
    let took = start.elapsed().as_secs_f64() * 1e6;
    let total = built.elapsed().as_secs_f64() * 1e6;
    let calls = CALLS.load(Ordering::Relaxed) - calls;
    let held = LIVE.load(Ordering::Relaxed) - before;
    drop(cx);
    (took, total, held, calls)
}

/// The same page laid out by Taffy.
/// [`nana_page`] in Taffy: the tree, its root and the leaf `edits` changes.
fn taffy_page(rows: usize, cells: usize) -> (taffy::TaffyTree<()>, taffy::NodeId, taffy::NodeId) {
    use taffy::prelude::*;
    let mut tree: TaffyTree<()> = TaffyTree::new();
    let mut target = None;
    let padding = Rect {
        left: length(0.25_f32),
        right: length(0.25_f32),
        top: length(0.25_f32),
        bottom: length(0.25_f32),
    };
    let mut row_nodes = Vec::with_capacity(rows);
    for r in 0..rows {
        let mut cell_nodes = Vec::with_capacity(cells);
        for c in 0..cells {
            let leaf = tree
                .new_leaf(Style {
                    size: Size {
                        width: length(24.0_f32),
                        height: auto(),
                    },
                    min_size: Size {
                        width: auto(),
                        height: length(16.0_f32),
                    },
                    ..Default::default()
                })
                .unwrap();
            if (r, c) == (rows / 2, cells / 2) {
                target = Some(leaf);
            }
            let cell = tree
                .new_with_children(
                    Style {
                        flex_direction: FlexDirection::Column,
                        padding,
                        ..Default::default()
                    },
                    &[leaf],
                )
                .unwrap();
            cell_nodes.push(cell);
        }
        let row = tree
            .new_with_children(
                Style {
                    gap: Size {
                        width: length(4.0_f32),
                        height: length(4.0_f32),
                    },
                    ..Default::default()
                },
                &cell_nodes,
            )
            .unwrap();
        row_nodes.push(row);
    }
    let root = tree
        .new_with_children(
            Style {
                flex_direction: FlexDirection::Column,
                size: Size {
                    width: length(1280.0_f32),
                    height: auto(),
                },
                ..Default::default()
            },
            &row_nodes,
        )
        .unwrap();
    (tree, root, target.unwrap())
}

fn taffy_wide(rows: usize, cells: usize) -> (f64, f64, isize, isize) {
    use taffy::prelude::*;
    let before = LIVE.load(Ordering::Relaxed);
    let built = Instant::now();
    let (mut tree, root, _) = taffy_page(rows, cells);
    let calls = CALLS.load(Ordering::Relaxed);
    let start = Instant::now();
    tree.compute_layout(
        root,
        Size {
            width: AvailableSpace::Definite(1280.0),
            height: AvailableSpace::Definite(800.0),
        },
    )
    .unwrap();
    let took = start.elapsed().as_secs_f64() * 1e6;
    let total = built.elapsed().as_secs_f64() * 1e6;
    let calls = CALLS.load(Ordering::Relaxed) - calls;
    let held = LIVE.load(Ordering::Relaxed) - before;
    drop(tree);
    (took, total, held, calls)
}

/// One edit to the [`nana_page`] at a time, each in a product frame
/// (`RuntimeDocument::flush`) after the first: the least layout stage and
/// the least whole flush over `runs` edits, in microseconds. `width` and
/// `height` toggle the target leaf's width or minimum height (the second
/// moves every row below it); `text` toggles a controls page's label.
fn nana_edits(rows: usize, cells: usize, edit: &str, runs: usize) -> (f64, f64) {
    let document = DocumentId::new(1).unwrap();
    let mut runtime = RuntimeDocument::new(document);
    let target = nana_page(runtime.context_mut(), document, rows, cells);
    let viewport = LayoutViewport::new(1280.0, 800.0);
    runtime.flush(viewport, &mut FixedShaper).unwrap();
    let (mut layout, mut whole) = (f64::INFINITY, f64::INFINITY);
    for run in 0..runs {
        let on = run % 2 == 0;
        let cx = runtime.context_mut();
        let mut queue = MutationQueue::new();
        if edit == "text" {
            let value = if on { "Label two" } else { "Label" };
            queue.set_text(
                target,
                TextContent {
                    value: value.into(),
                },
            );
        } else {
            let mut style = cx.world().node_style(target).unwrap().clone();
            let spec = std::sync::Arc::make_mut(&mut style.layout);
            match edit {
                "width" => spec.width = Some(LengthSpec::Px(if on { 32.0 } else { 24.0 })),
                "height" => spec.min_height = Some(LengthSpec::Px(if on { 24.0 } else { 16.0 })),
                other => panic!("unknown edit {other}"),
            }
            queue.set_style(target, style);
        }
        cx.commit_mutations(queue).unwrap();
        let start = Instant::now();
        runtime.flush(viewport, &mut FixedShaper).unwrap();
        whole = whole.min(start.elapsed().as_secs_f64() * 1e6);
        let stage: f64 = runtime
            .context()
            .last_frame_profile()
            .stages
            .iter()
            .filter(|timing| timing.stage == FrameStage::Layout)
            .map(|timing| timing.duration.as_secs_f64() * 1e6)
            .sum();
        layout = layout.min(stage);
        if run + 1 == runs && std::env::var_os("PRINT_STAGES").is_some() {
            let stages: Vec<String> = runtime
                .context()
                .last_frame_profile()
                .stages
                .iter()
                .filter(|timing| !timing.duration.is_zero())
                .map(|timing| {
                    format!(
                        "{:?} {:.0}",
                        timing.stage,
                        timing.duration.as_secs_f64() * 1e6
                    )
                })
                .collect();
            eprintln!("{edit}: {} (µs)", stages.join(", "));
        }
        if run + 1 == runs && std::env::var_os("PRINT_COUNTERS").is_some() {
            let w = runtime.context().last_work_counters();
            eprintln!(
                "{edit}: seeds {} full {} local {} measure {} hits {} misses {} place {} reused {} changed {} origin {} commits {} children {} plans {}/{} suffix {} translations {}/{}",
                w.layout_frontier_seeds,
                w.layout_full_document_fallbacks,
                w.layout_local_subtree_fallbacks,
                w.layout_measure_nodes,
                w.layout_measure_cache_hits,
                w.layout_measure_cache_misses,
                w.layout_placement_nodes,
                w.layout_result_reused,
                w.layout_result_changed,
                w.layout_origin_only_updates,
                w.layout_delta_commits,
                w.layout_result_children_visited,
                w.layout_placement_plans_reused,
                w.layout_measure_plans_reused,
                w.layout_suffixes_replayed,
                w.layout_translations,
                w.layout_translated_nodes,
            );
        }
    }
    (layout, whole)
}

/// [`nana_edits`] in Taffy: the least `compute_layout` after each edit.
fn taffy_edits(rows: usize, cells: usize, edit: &str, runs: usize) -> f64 {
    use taffy::prelude::*;
    let (mut tree, root, target) = taffy_page(rows, cells);
    let available = Size {
        width: AvailableSpace::Definite(1280.0),
        height: AvailableSpace::Definite(800.0),
    };
    tree.compute_layout(root, available).unwrap();
    let mut least = f64::INFINITY;
    for run in 0..runs {
        let on = run % 2 == 0;
        let mut style = tree.style(target).unwrap().clone();
        match edit {
            "width" => style.size.width = length(if on { 32.0_f32 } else { 24.0 }),
            "height" => style.min_size.height = length(if on { 24.0_f32 } else { 16.0 }),
            other => panic!("unknown edit {other}"),
        }
        tree.set_style(target, style).unwrap();
        let start = Instant::now();
        tree.compute_layout(root, available).unwrap();
        least = least.min(start.elapsed().as_secs_f64() * 1e6);
    }
    least
}

struct FixedShaper;

impl TextShaper for FixedShaper {
    fn shape(
        &mut self,
        _id: StableNodeId,
        text: &TextContent,
        _style: &ComputedStyle,
        _constraints: TextShapeConstraints,
    ) -> TextMetrics {
        TextMetrics {
            width: text.value.len() as f32 * 8.0,
            height: 18.0,
            ascent: None,
        }
    }
}

fn padded() -> LayoutStyle {
    LayoutStyle {
        padding: Some(LengthSpec::Px(0.25)),
        ..Default::default()
    }
}

/// `depth` nested columns laid out once by NanaUI's layout engine; returns
/// the microseconds the layout took, without building the tree.
fn nana_layout(depth: usize) -> f64 {
    let document = DocumentId::new(1).unwrap();
    let mut cx = AppContext::new();
    let mut parent = cx
        .create_component(document, Stack::column(0.0).width(LengthSpec::Px(800.0)))
        .unwrap();
    for _ in 0..depth {
        let child = cx
            .create_detached_component(document, Stack::column(0.0).with_layout(|l| *l = padded()))
            .unwrap();
        cx.append_child(parent, child).unwrap();
        parent = child;
    }
    let start = Instant::now();
    cx.layout_document(document, LayoutViewport::new(800.0, 600.0))
        .unwrap();
    start.elapsed().as_secs_f64() * 1e6
}

/// A whole frame through `depth` nested columns; returns the microseconds
/// from mounting to the frame's scene, hit index and accessibility tree.
fn nana_frame(depth: usize) -> f64 {
    let start = Instant::now();
    let document = DocumentId::new(1).unwrap();
    let mut runtime = RuntimeDocument::new(document);
    runtime
        .context_mut()
        .mount_view_root(document, move || {
            let mut view = text("leaf").into_any();
            for _ in 0..depth {
                view = widget(Stack::column(0.0).with_layout(|l| *l = padded()))
                    .children((view,))
                    .into_any();
            }
            view
        })
        .unwrap();
    let viewport = LayoutViewport::new(800.0, 600.0);
    runtime.flush(viewport, &mut FixedShaper).unwrap();
    let cx = runtime.context();
    let _ = cx.world().hit_test(document, 1.0, 1.0);
    let _ = cx.world().project_accessibility(document);
    let took = start.elapsed().as_secs_f64() * 1e6;
    drop(runtime);
    took
}

/// One frame through `depth` nested columns, per stage: the least each named
/// stage took over `runs` frames, in microseconds.
fn nana_frame_stages(depth: usize, runs: usize) -> Vec<(String, f64)> {
    let mut least: Vec<(String, f64)> = Vec::new();
    for _ in 0..runs {
        let document = DocumentId::new(1).unwrap();
        let mut runtime = RuntimeDocument::new(document);
        let mount = Instant::now();
        runtime
            .context_mut()
            .mount_view_root(document, move || {
                let mut view = text("leaf").into_any();
                for _ in 0..depth {
                    view = widget(Stack::column(0.0).with_layout(|l| *l = padded()))
                        .children((view,))
                        .into_any();
                }
                view
            })
            .unwrap();
        let mounted = ("Mount".to_owned(), mount.elapsed());
        runtime
            .flush(LayoutViewport::new(800.0, 600.0), &mut FixedShaper)
            .unwrap();
        let profile = runtime.context().last_frame_profile().clone();
        let stages = std::iter::once(mounted).chain(
            profile
                .stages
                .into_iter()
                .map(|timing| (format!("{:?}", timing.stage), timing.duration)),
        );
        for (name, duration) in stages {
            let us = duration.as_secs_f64() * 1e6;
            match least.iter_mut().find(|(n, _)| *n == name) {
                Some((_, held)) => *held = held.min(us),
                None => least.push((name, us)),
            }
        }
    }
    least.retain(|(_, us)| *us > 0.0);
    least
}

/// The same nested columns laid out once by Taffy; returns the microseconds
/// the layout took, without building the tree.
fn taffy(depth: usize) -> f64 {
    use taffy::prelude::*;
    let mut tree: TaffyTree<()> = TaffyTree::new();
    let padding = Rect {
        left: length(0.25_f32),
        right: length(0.25_f32),
        top: length(0.25_f32),
        bottom: length(0.25_f32),
    };
    let column = Style {
        flex_direction: FlexDirection::Column,
        padding,
        ..Default::default()
    };
    let mut node = tree.new_leaf(column.clone()).unwrap();
    for _ in 0..depth {
        node = tree.new_with_children(column.clone(), &[node]).unwrap();
    }
    let root = tree
        .new_with_children(
            Style {
                flex_direction: FlexDirection::Column,
                size: Size {
                    width: length(800.0_f32),
                    height: auto(),
                },
                ..Default::default()
            },
            &[node],
        )
        .unwrap();
    let start = Instant::now();
    tree.compute_layout(
        root,
        Size {
            width: AvailableSpace::Definite(800.0),
            height: AvailableSpace::Definite(600.0),
        },
    )
    .unwrap();
    start.elapsed().as_secs_f64() * 1e6
}

fn run(engine: &str, depth: usize) -> f64 {
    match engine {
        "nana-layout" => nana_layout(depth),
        "nana-frame" => nana_frame(depth),
        "taffy" => taffy(depth),
        other => panic!("unknown engine {other}"),
    }
}

/// Runs `engine` once on a thread of `stack` bytes.
fn probe(engine: String, depth: usize, stack: usize) {
    std::thread::Builder::new()
        .stack_size(stack)
        .spawn(move || {
            run(&engine, depth);
        })
        .unwrap()
        .join()
        .unwrap();
}

/// Whether a probe of `stack` bytes finishes, in a child process.
fn fits(engine: &str, depth: usize, stack: usize) -> bool {
    Command::new(std::env::current_exe().unwrap())
        .args(["probe", engine, &depth.to_string(), &stack.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// The least stack, to 4 KiB, a probe finishes on.
fn least_stack(engine: &str, depth: usize) -> Option<usize> {
    const PAGE: usize = 4 << 10;
    let (mut low, mut high) = (PAGE, 256 << 20);
    if !fits(engine, depth, high) {
        return None;
    }
    while high - low > PAGE {
        let mid = (low + high) / 2 / PAGE * PAGE;
        if fits(engine, depth, mid) {
            high = mid;
        } else {
            low = mid;
        }
    }
    Some(high)
}

/// Fastest of `rounds` runs on an ample stack, in microseconds.
fn fastest(engine: &str, depth: usize, rounds: usize) -> f64 {
    let engine = engine.to_owned();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            (0..rounds)
                .map(|_| run(&engine, depth))
                .fold(f64::INFINITY, f64::min)
        })
        .unwrap()
        .join()
        .unwrap()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize| args.get(i).map(String::as_str);
    match arg(0) {
        Some("probe") => probe(
            args[1].clone(),
            args[2].parse().unwrap(),
            args[3].parse().unwrap(),
        ),
        Some("search") => {
            let engine = args
                .iter()
                .position(|a| a == "--engine")
                .map_or("nana-layout", |i| args[i + 1].as_str());
            let depth = args
                .iter()
                .position(|a| a == "--depth")
                .map_or(500, |i| args[i + 1].parse().unwrap());
            match least_stack(engine, depth) {
                Some(bytes) => println!("{engine} depth {depth}: {} KiB", bytes >> 10),
                None => println!("{engine} depth {depth}: fails at 256 MiB"),
            }
        }
        Some("report") | None => {
            let build = if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            };
            println!("| engine ({build}) | depth | least stack | fastest run |");
            println!("| --- | ---: | ---: | ---: |");
            let depths: Vec<usize> = match arg(1) {
                Some(list) => list.split(',').map(|d| d.parse().unwrap()).collect(),
                None => vec![64, 128, 256, 500],
            };
            for depth in depths {
                for engine in ENGINES {
                    let stack = (arg(2) != Some("time"))
                        .then(|| least_stack(engine, depth))
                        .flatten()
                        .map_or("-".to_owned(), |b| format!("{} KiB", b >> 10));
                    let time = fastest(engine, depth, 20);
                    println!("| {engine} | {depth} | {stack} | {time:.0} µs |");
                }
            }
        }
        Some("wide") => {
            // A page of rows and cells: layout time and held heap per box.
            let build = if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            };
            println!(
                "| engine ({build}) | boxes | fastest layout | per box | build + layout per box | heap per box | allocations per box |"
            );
            println!("| --- | ---: | ---: | ---: | ---: | ---: | ---: |");
            // `wide 50,200,800` picks the row counts (10 cells a row).
            let rows: Vec<usize> = match arg(1) {
                Some(list) => list.split(',').map(|r| r.parse().unwrap()).collect(),
                None => vec![100, 500, 1000],
            };
            for (rows, cells) in rows.into_iter().map(|rows| (rows, 10)) {
                // A controls page's cell holds a label and a button.
                let per_cell = if std::env::var_os("CONTROLS").is_some() {
                    3
                } else {
                    2
                };
                let boxes = 1 + rows * (1 + per_cell * cells);
                for (name, run) in [
                    (
                        "nana-layout",
                        nana_wide as fn(usize, usize) -> (f64, f64, isize, isize),
                    ),
                    ("taffy", taffy_wide),
                ] {
                    // Taffy lays out the bare-box page only.
                    if name == "taffy" && std::env::var_os("CONTROLS").is_some() {
                        continue;
                    }
                    let (mut fastest, mut whole) = (f64::INFINITY, f64::INFINITY);
                    let (mut held, mut calls) = (0, 0);
                    for _ in 0..15 {
                        let (took, total, bytes, made) = run(rows, cells);
                        fastest = fastest.min(took);
                        whole = whole.min(total);
                        (held, calls) = (bytes, made);
                    }
                    println!(
                        "| {name} | {boxes} | {fastest:.0} µs | {:.2} µs | {:.2} µs | {} B | {:.1} |",
                        fastest / boxes as f64,
                        whole / boxes as f64,
                        held / boxes as isize,
                        calls as f64 / boxes as f64
                    );
                }
            }
        }
        Some("edits") => {
            // One edit a frame on the wide page: `edits 200,2000 [width,height]`.
            // A controls page (`CONTROLS=1`) edits its label's text.
            let rows: Vec<usize> = match arg(1) {
                Some(list) => list.split(',').map(|r| r.parse().unwrap()).collect(),
                None => vec![200, 2000, 6400],
            };
            let controls = std::env::var_os("CONTROLS").is_some();
            let edits: Vec<&str> = match arg(2) {
                Some(list) => list.split(',').collect(),
                None if controls => vec!["text"],
                None => vec!["width", "height"],
            };
            println!("| engine | boxes | edit | layout | whole frame | taffy layout |");
            println!("| --- | ---: | --- | ---: | ---: | ---: |");
            for rows in rows {
                let boxes = 1 + rows * (1 + if controls { 3 } else { 2 } * 10);
                for edit in &edits {
                    let (layout, whole) = nana_edits(rows, 10, edit, 40);
                    let taffy = if controls {
                        "-".to_owned()
                    } else {
                        format!("{:.1} µs", taffy_edits(rows, 10, edit, 40))
                    };
                    println!(
                        "| nana | {boxes} | {edit} | {layout:.1} µs | {whole:.1} µs | {taffy} |"
                    );
                }
            }
        }
        Some("reps") => {
            // A fixed amount of work for `time -l`: `reps <rows> <runs> [taffy]`.
            let rows: usize = arg(1).unwrap().parse().unwrap();
            let runs: usize = arg(2).unwrap().parse().unwrap();
            let taffy = arg(3) == Some("taffy");
            for _ in 0..runs {
                if taffy {
                    taffy_wide(rows, 10);
                } else {
                    nana_wide(rows, 10);
                }
            }
        }
        Some("wide-loop") => {
            // For a profiler: lay the 10 501-box page out over and over.
            let rows = arg(1).map_or(500, |rows| rows.parse().unwrap());
            loop {
                nana_wide(rows, 10);
            }
        }
        Some("stages") => {
            // Where a deep frame's time goes: the least per stage over runs.
            // The runtime refuses a tree deeper than 512.
            let depths: Vec<usize> = match arg(1) {
                Some(list) => list.split(',').map(|d| d.parse().unwrap()).collect(),
                None => vec![125, 250, 500],
            };
            for depth in depths {
                let stages = nana_frame_stages(depth, 15);
                let line: Vec<String> = stages
                    .iter()
                    .map(|(name, us)| format!("{name} {us:.0}"))
                    .collect();
                println!("depth {depth}: {} (µs)", line.join(", "));
            }
        }
        Some("loop") => {
            // For a profiler: run one engine over and over.
            let depth = args[2].parse().unwrap();
            loop {
                fastest(&args[1], depth, 1);
            }
        }
        Some(other) => panic!("unknown command {other}"),
    }
}
