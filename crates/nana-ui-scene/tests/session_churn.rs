//! What a session holds follows what is on screen, not how long it has run.
//!
//! Node ids are handed out in order and never reused, and several things
//! grew with them: every node ever removed left a layout cause queued that
//! each frame looked at again (70,000 removals made every frame 3 ms
//! slower); the tables indexed by id kept a slot per id ever issued (about
//! half a kilobyte each, 33 MB by the time the ids passed 65,000); and the
//! set of nodes whose layout result is withheld and the memo of which boxes
//! hold a line box each kept an entry per removed node.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, Ordering};

use nana_ui_runtime::{Button, DocumentId, Entity, LayoutViewport, MeasureTextShaper, Stack, Text};
use nana_ui_scene::RuntimeDocument;

struct Counted;

/// Bytes allocated and not yet freed.
static LIVE: AtomicIsize = AtomicIsize::new(0);

// SAFETY: the system allocator receives every original pointer and layout;
// the counter neither allocates nor changes what is asked for.
unsafe impl GlobalAlloc for Counted {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LIVE.fetch_add(layout.size() as isize, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        LIVE.fetch_add(layout.size() as isize, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size() as isize, Ordering::Relaxed);
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        LIVE.fetch_add(size as isize - layout.size() as isize, Ordering::Relaxed);
        unsafe { System.realloc(pointer, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counted = Counted;

fn frame(runtime: &mut RuntimeDocument) {
    let viewport = LayoutViewport::new(400.0, 600.0);
    for _ in 0..2 {
        runtime.flush(viewport, &mut MeasureTextShaper).unwrap();
    }
}

/// A row of a label and a button, last in `column`.
fn add_row(runtime: &mut RuntimeDocument, column: Entity<Stack>, index: usize) -> Entity<Stack> {
    let document = DocumentId::new(1).unwrap();
    let cx = runtime.context_mut();
    let row = cx
        .create_detached_component(document, Stack::row(4.0))
        .unwrap();
    cx.append_child(column, row).unwrap();
    let label = cx
        .create_detached_component(document, Text::new(format!("row {index}")))
        .unwrap();
    cx.append_child(row, label).unwrap();
    let button = cx
        .create_detached_component(document, Button::new("Open"))
        .unwrap();
    cx.append_child(row, button).unwrap();
    row
}

/// Ten rows shown for a frame and removed again.
fn come_and_go(runtime: &mut RuntimeDocument, column: Entity<Stack>) {
    let rows: Vec<_> = (0..10)
        .map(|index| add_row(runtime, column, index))
        .collect();
    frame(runtime);
    for row in rows {
        runtime.context_mut().remove_view(row).unwrap();
    }
    frame(runtime);
}

#[test]
fn rows_coming_and_going_leave_nothing_behind() {
    let document = DocumentId::new(1).unwrap();
    let mut runtime = RuntimeDocument::new(document);
    let column = runtime
        .context_mut()
        .create_component(document, Stack::column(2.0))
        .unwrap();
    for index in 0..40 {
        add_row(&mut runtime, column, index);
    }
    frame(&mut runtime);

    // Tables reach the capacity ten more rows need.
    for _ in 0..100 {
        come_and_go(&mut runtime, column);
    }
    let settled = LIVE.load(Ordering::Relaxed);
    let nodes = runtime.context().world().len();

    // Eighteen thousand more nodes created and removed.
    const CYCLES: isize = 600;
    for _ in 0..CYCLES {
        come_and_go(&mut runtime, column);
    }
    assert_eq!(runtime.context().world().len(), nodes);
    let grown = LIVE.load(Ordering::Relaxed) - settled;
    // What is left to grow is the directory of pages of each table indexed
    // by id: eight bytes per sixty-four ids between the oldest node alive
    // and the newest, under two bytes a node over all of them. A table
    // that keeps anything per removed node keeps eight bytes or more.
    let created = CYCLES * 30;
    assert!(
        grown < created * 4,
        "{grown} bytes more are held after {created} nodes came and went: {} a node",
        grown / created
    );
}
