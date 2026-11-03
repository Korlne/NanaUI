//! Accessible bounds relative to the parent's: a box layout moved with its
//! parent keeps them, so a delta leaves it, and what it carries, alone.

use std::collections::BTreeMap;

use nana_ui_core::{FlexDirection, LayoutStyle, LengthSpec, OverflowSpec, PositionSpec};

use super::super::reflow_oracle::{
    Builder, accessible_frame, bundled_face_shaper, column, fixed, product_frame, styled,
};
use crate::{
    AccessibilityNode, AppContext, DocumentId, LayoutViewport, MutationQueue, ScrollOffset,
    StableNodeId,
};

fn document() -> DocumentId {
    DocumentId::new(1).unwrap()
}

fn viewport() -> LayoutViewport {
    LayoutViewport::new(400.0, 600.0)
}

struct Page {
    context: AppContext,
    /// The rows, each a row holding a cell holding a leaf.
    rows: Vec<[StableNodeId; 3]>,
    /// A fixed box, and an absolute one, in the last row's cell, when asked
    /// for. The engine then places the moved rows again instead of
    /// translating them.
    hanging: Option<[StableNodeId; 2]>,
    /// The column the rows stand in.
    list: StableNodeId,
}

/// `rows` rows in a column, the column a scroller when `scroller`.
fn page(rows: usize, scroller: bool, hanging: bool) -> Page {
    let (mut b, page) = Builder::page(document(), 1, 400.0);
    let list = b.element(
        page,
        if scroller {
            LayoutStyle {
                overflow_x: OverflowSpec::Auto,
                overflow_y: OverflowSpec::Auto,
                direction: Some(FlexDirection::Column),
                flex_shrink: Some(0.0),
                ..fixed(300.0, 120.0)
            }
        } else {
            column(Some(300.0))
        },
    );
    let mut ids = Vec::new();
    for _ in 0..rows {
        let row = b.element(
            list,
            LayoutStyle {
                flex_shrink: Some(0.0),
                ..fixed(300.0, 20.0)
            },
        );
        let cell = b.element(
            row,
            LayoutStyle {
                position: PositionSpec::Relative,
                ..fixed(100.0, 10.0)
            },
        );
        let leaf = b.element(cell, fixed(50.0, 5.0));
        ids.push([row, cell, leaf]);
    }
    let cell = ids.last().unwrap()[1];
    let placed = |position, left: f32, top: f32| LayoutStyle {
        position,
        offset_left: Some(LengthSpec::Px(left)),
        offset_top: Some(LengthSpec::Px(top)),
        ..fixed(10.0, 10.0)
    };
    let hanging = hanging.then(|| {
        [
            b.element(cell, placed(PositionSpec::Fixed, 320.0, 10.0)),
            b.element(cell, placed(PositionSpec::Absolute, 60.0, 2.0)),
        ]
    });
    let mut context = AppContext::new();
    context.commit_mutations(b.queue).unwrap();
    product_frame(
        &mut context,
        document(),
        viewport(),
        &mut bundled_face_shaper(),
    );
    Page {
        context,
        rows: ids,
        hanging,
        list,
    }
}

fn snapshot(context: &AppContext) -> BTreeMap<StableNodeId, AccessibilityNode> {
    context
        .world()
        .project_accessibility(document())
        .into_iter()
        .map(|node| (node.id, node))
        .collect()
}

/// Commit `queue`, run the frame and apply its delta to `retained`, which
/// then equals a full projection. Returns the ids the delta updated.
fn edit(
    page: &mut Page,
    retained: &mut BTreeMap<StableNodeId, AccessibilityNode>,
    queue: MutationQueue,
    at: &str,
) -> Vec<StableNodeId> {
    page.context.commit_mutations(queue).unwrap();
    let work = accessible_frame(
        &mut page.context,
        document(),
        viewport(),
        &mut bundled_face_shaper(),
        false,
    );
    let delta = page.context.world().project_accessibility_delta(&work);
    for id in &delta.removed {
        retained.remove(id);
    }
    let updated = delta.updated.iter().map(|node| node.id).collect();
    for node in delta.updated {
        retained.insert(node.id, node);
    }
    assert_eq!(
        *retained,
        snapshot(&page.context),
        "{at}: the delta left the tree unlike a full projection"
    );
    updated
}

fn row_height(queue: &mut MutationQueue, row: StableNodeId, height: f32) {
    queue.set_style(
        row,
        styled(LayoutStyle {
            flex_shrink: Some(0.0),
            ..fixed(300.0, height)
        }),
    );
}

fn viewport_bounds(context: &AppContext, id: StableNodeId) -> crate::LayoutBox {
    context
        .world()
        .project_accessibility_in_viewport(document())
        .into_iter()
        .find(|node| node.id == id)
        .unwrap()
        .bounds
}

#[test]
fn a_grown_row_projects_the_rows_it_moves_but_not_what_they_hold() {
    let mut page = page(12, false, false);
    let mut retained = snapshot(&page.context);
    let leaf = page.rows[5][2];
    let before = viewport_bounds(&page.context, leaf);
    assert_eq!(
        retained[&leaf].bounds,
        crate::LayoutBox {
            x: 0.0,
            y: 0.0,
            width: 50.0,
            height: 5.0
        },
        "a leaf at its cell's corner stands at 0,0 from it"
    );
    for (height, at) in [(26.0, "grown"), (20.0, "back")] {
        let mut queue = MutationQueue::new();
        row_height(&mut queue, page.rows[0][0], height);
        let updated = edit(&mut page, &mut retained, queue, at);
        for [row, cell, leaf] in &page.rows[1..] {
            assert!(updated.contains(row), "{at}: a moved row projects again");
            assert!(
                !updated.contains(cell) && !updated.contains(leaf),
                "{at}: what a moved row holds moved with it"
            );
        }
    }
    let mut queue = MutationQueue::new();
    row_height(&mut queue, page.rows[0][0], 26.0);
    edit(&mut page, &mut retained, queue, "grown again");
    let after = viewport_bounds(&page.context, leaf);
    assert_eq!((after.x, after.y), (before.x, before.y + 6.0));
}

/// A fixed box the viewport holds still while the cell it is in moves, and
/// an absolute one that moves with it.
#[test]
fn boxes_hanging_from_a_moved_row_match_a_full_projection() {
    let mut page = page(12, false, true);
    let mut retained = snapshot(&page.context);
    let [fixed_box, absolute] = page.hanging.unwrap();
    let before = viewport_bounds(&page.context, fixed_box);
    for (height, at) in [(26.0, "grown"), (20.0, "back"), (33.0, "grown more")] {
        let mut queue = MutationQueue::new();
        row_height(&mut queue, page.rows[0][0], height);
        let updated = edit(&mut page, &mut retained, queue, at);
        assert!(updated.contains(&fixed_box), "{at}: its offset changed");
        assert!(updated.contains(&absolute), "{at}");
    }
    assert_eq!(viewport_bounds(&page.context, fixed_box), before);
}

#[test]
fn moved_rows_under_a_scrolled_list_match_a_full_projection() {
    let mut page = page(12, true, true);
    let mut retained = snapshot(&page.context);
    let mut queue = MutationQueue::new();
    queue.set_scroll_offset(page.list, ScrollOffset { x: 0.0, y: 30.0 });
    edit(&mut page, &mut retained, queue, "scrolled");
    for (height, at) in [(26.0, "grown"), (20.0, "back"), (33.0, "grown more")] {
        let mut queue = MutationQueue::new();
        row_height(&mut queue, page.rows[2][0], height);
        edit(&mut page, &mut retained, queue, at);
    }
    // A scroll and a move in one frame.
    let mut queue = MutationQueue::new();
    queue.set_scroll_offset(page.list, ScrollOffset { x: 0.0, y: 10.0 });
    row_height(&mut queue, page.rows[1][0], 24.0);
    edit(&mut page, &mut retained, queue, "scrolled and grown");
}
