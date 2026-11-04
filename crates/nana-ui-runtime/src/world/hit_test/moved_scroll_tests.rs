//! Rows moved inside a scrolled list: the hit index builds the moved rows'
//! entries again in place, under the scroll shift it holds the list at, and
//! keeps what they hold. Every frame answers the pointer as an index built
//! from scratch does, and the cold check (`check_moved_hit_entries`) runs on
//! every patch.

use nana_ui_core::{FlexDirection, LayoutStyle, LengthSpec, OverflowSpec, PositionSpec};

use super::super::reflow_oracle::{Builder, accessible_frame, bundled_face_shaper, fixed, styled};
use crate::{AppContext, DocumentId, LayoutViewport, MutationQueue, ScrollOffset, StableNodeId};

fn document() -> DocumentId {
    DocumentId::new(1).unwrap()
}

struct Page {
    context: AppContext,
    /// A box above the scroller, which moves it.
    spacer: StableNodeId,
    scroller: StableNodeId,
    /// A scroller in the last row, scrolled too.
    inner: StableNodeId,
    rows: Vec<StableNodeId>,
    /// Every node, for the pointer grid's candidates.
    entries: usize,
}

/// A 300 x 120 scroller holding a column of rows, each a row holding a
/// padded cell holding a leaf; the last row holds a scroller of its own.
fn page() -> Page {
    let (mut b, root) = Builder::page(document(), 1, 400.0);
    let spacer = b.element(root, fixed(300.0, 3.0));
    let scroller = b.element(
        root,
        LayoutStyle {
            overflow_x: OverflowSpec::Auto,
            overflow_y: OverflowSpec::Auto,
            direction: Some(FlexDirection::Column),
            flex_shrink: Some(0.0),
            ..fixed(300.0, 120.0)
        },
    );
    let list = b.element(
        scroller,
        LayoutStyle {
            width: Some(LengthSpec::Px(300.0)),
            direction: Some(FlexDirection::Column),
            flex_shrink: Some(0.0),
            ..LayoutStyle::default()
        },
    );
    let mut rows = Vec::new();
    for _ in 0..12 {
        let row = b.element(
            list,
            LayoutStyle {
                flex_shrink: Some(0.0),
                position: PositionSpec::Relative,
                ..fixed(300.0, 20.0)
            },
        );
        let cell = b.element(
            row,
            LayoutStyle {
                padding_left: Some(LengthSpec::Px(1.3)),
                padding_top: Some(LengthSpec::Px(2.5)),
                ..fixed(100.0, 14.0)
            },
        );
        b.element(cell, fixed(50.0, 5.0));
        rows.push(row);
    }
    let inner = b.element(
        *rows.last().unwrap(),
        LayoutStyle {
            overflow_y: OverflowSpec::Auto,
            direction: Some(FlexDirection::Column),
            ..fixed(120.0, 18.0)
        },
    );
    for _ in 0..3 {
        b.element(
            inner,
            LayoutStyle {
                flex_shrink: Some(0.0),
                ..fixed(110.0, 9.3)
            },
        );
    }
    let mut context = AppContext::new();
    context.commit_mutations(b.queue).unwrap();
    let mut page = Page {
        context,
        spacer,
        scroller,
        inner,
        rows,
        entries: 0,
    };
    frame(&mut page, false);
    page.context.rebuild_hit_test(document());
    page.entries = page.context.world().len();
    page
}

/// A product frame, then the hit index brought up to date as
/// `RuntimeDocument` does it: a pure scroll shifts the entries, anything
/// else patches them -- or, `cold`, builds the whole index again. Returns
/// the entries the frame built.
fn frame(page: &mut Page, cold: bool) -> usize {
    let work = accessible_frame(
        &mut page.context,
        document(),
        LayoutViewport::new(400.0, 600.0),
        &mut bundled_face_shaper(),
        false,
    );
    let updates = page.context.take_scroll_hit_updates();
    let mut dirty = work.input_hit_test.clone();
    dirty.sort_unstable();
    dirty.dedup();
    let built = |context: &AppContext| {
        context
            .world()
            .last_work_counters()
            .hit_test_nodes_rebuilt
            .unwrap_or_default()
    };
    let before = built(&page.context);
    if cold {
        page.context.rebuild_hit_test(document());
    } else if !dirty.is_empty() {
        if page.context.hit_test_work_is_scroll_only(&dirty, &updates) {
            for (scroller, delta) in updates {
                page.context
                    .update_hit_test_scroll(document(), scroller, delta);
            }
        } else {
            page.context.rebuild_hit_test_for(document(), &dirty);
        }
    }
    built(&page.context).saturating_sub(before)
}

/// What the pointer finds on a grid over the scroller and a little past it:
/// the topmost hit and every candidate under it. The points stay off the
/// boxes' edges, which a shift and a fresh build may place an ulp apart.
fn grid(page: &Page) -> Vec<(Option<StableNodeId>, Vec<StableNodeId>)> {
    let world = page.context.world();
    let mut hits = Vec::new();
    for row in 0..30 {
        let y = row as f32 * 4.9 + 0.71;
        for column in 0..12 {
            let x = column as f32 * 27.3 + 0.43;
            hits.push((
                world.hit_test(document(), x, y),
                world.hit_test_candidates(document(), x, y),
            ));
        }
    }
    hits
}

fn grow(page: &mut Page, height: f32) {
    let mut queue = MutationQueue::new();
    queue.set_style(
        page.rows[0],
        styled(LayoutStyle {
            flex_shrink: Some(0.0),
            position: PositionSpec::Relative,
            ..fixed(300.0, height)
        }),
    );
    page.context.commit_mutations(queue).unwrap();
}

fn spacer(page: &mut Page, height: f32) {
    let mut queue = MutationQueue::new();
    queue.set_style(page.spacer, styled(fixed(300.0, height)));
    page.context.commit_mutations(queue).unwrap();
}

fn scroll(page: &mut Page, scroller: StableNodeId, y: f32) {
    let mut queue = MutationQueue::new();
    queue.set_scroll_offset(scroller, ScrollOffset { x: 0.0, y });
    page.context.commit_mutations(queue).unwrap();
}

/// The list scrolled by an uneven amount, its first row grown by an uneven
/// step again and again: the moved rows are built again on their own, what
/// they hold keeps its entries, and the pointer finds what it finds in an
/// index built from scratch -- also after the list scrolls on its own, and in
/// a frame that scrolls it and moves the rows at once, and in one that
/// scrolls it and moves the scroller, where the scroll reaches the index
/// through the patch rather than a shift.
#[test]
fn rows_moved_in_a_scrolled_list_are_built_again_alone() {
    let mut retained = page();
    let mut cold = page();
    let mut list_scroll = 30.37;
    let mut height = 20.0;
    for page in [&mut retained, &mut cold] {
        let (scroller, inner) = (page.scroller, page.inner);
        scroll(page, scroller, list_scroll);
        scroll(page, inner, 4.13);
    }
    assert_eq!(
        frame(&mut retained, false),
        0,
        "a pure scroll shifts entries"
    );
    frame(&mut cold, true);
    assert_eq!(grid(&retained), grid(&cold), "scrolled");
    for round in 0..2 {
        for step in 0..6 {
            height += 1.37;
            let at = format!("round {round}, step {step}");
            grow(&mut retained, height);
            grow(&mut cold, height);
            let built = frame(&mut retained, false);
            frame(&mut cold, true);
            // The rows, the list and the scroller, which the move resizes
            // and gives a new extent: nothing the rows hold.
            assert!(
                built <= retained.rows.len() + 2,
                "{at}: {built} of {} entries built again",
                retained.entries
            );
            assert!(
                retained
                    .context
                    .world()
                    .last_work_counters()
                    .hit_test_entries_moved
                    >= retained.rows.len() - 1,
                "{at}: the moved rows take new origins"
            );
            assert_eq!(grid(&retained), grid(&cold), "{at}");
        }
        // A scroll on its own, then one with a move in the same frame.
        list_scroll += 11.13;
        for page in [&mut retained, &mut cold] {
            let scroller = page.scroller;
            scroll(page, scroller, list_scroll);
        }
        frame(&mut retained, false);
        frame(&mut cold, true);
        assert_eq!(grid(&retained), grid(&cold), "round {round}: scrolled");
        list_scroll -= 5.29;
        height += 1.37;
        for page in [&mut retained, &mut cold] {
            let scroller = page.scroller;
            scroll(page, scroller, list_scroll);
            grow(page, height);
        }
        frame(&mut retained, false);
        frame(&mut cold, true);
        assert_eq!(
            grid(&retained),
            grid(&cold),
            "round {round}: scrolled and moved"
        );
        // The scroller itself moved and scrolled, nothing in it: its entry
        // is built again, and the scroll the index has not taken must
        // reach what it holds.
        list_scroll += 2.71;
        for page in [&mut retained, &mut cold] {
            let scroller = page.scroller;
            scroll(page, scroller, list_scroll);
            spacer(page, 3.0 + 1.37 * (round + 1) as f32);
        }
        frame(&mut retained, false);
        frame(&mut cold, true);
        assert_eq!(
            grid(&retained),
            grid(&cold),
            "round {round}: the scroller scrolled and moved"
        );
    }
}
