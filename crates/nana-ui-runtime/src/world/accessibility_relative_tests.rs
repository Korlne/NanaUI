//! Accessible bounds relative to the parent's: a box layout moved with its
//! parent keeps them, so a delta leaves it, and what it carries, alone.

use std::collections::BTreeMap;

use nana_ui_core::{
    FlexDirection, LayoutStyle, LengthSpec, OverflowSpec, PaintTransform, PositionSpec,
};

use super::super::reflow_oracle::{
    Builder, accessible_frame, bundled_face_shaper, column, fixed, product_frame, styled,
};
use crate::{
    AccessibilityNode, AppContext, DocumentId, LayoutViewport, MutationQueue, ScrollOffset,
    StableNodeId, StandardVisual,
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
    /// for.
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

/// A fixed box the viewport holds still while the cell it is in moves, so
/// its offset from the cell changes, and an absolute one that moves with the
/// cell and keeps it.
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
        assert!(!updated.contains(&absolute), "{at}: its offset held");
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

/// Boxes under moved rows that do not keep their offset from their parent
/// to the bit -- a transform, a sticky offset, a clip, a box measured from
/// the column above the row, a fixed and an absolute one -- each with a
/// plain box inside, on a page with a visual (so the projection takes the
/// path visuals do). The delta finds them from the pins alone and matches a
/// full projection field by field; the plain boxes around them stay put.
#[test]
fn pinned_boxes_under_moved_rows_match_a_full_projection() {
    let (mut b, root) = Builder::page(document(), 1, 400.0);
    let list = b.element(
        root,
        LayoutStyle {
            position: PositionSpec::Relative,
            ..column(Some(300.0))
        },
    );
    let mut rows = Vec::new();
    for _ in 0..12 {
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
        rows.push([row, cell, leaf]);
    }
    let placed = |position, left: f32, top: f32| LayoutStyle {
        position,
        offset_left: Some(LengthSpec::Px(left)),
        offset_top: Some(LengthSpec::Px(top)),
        ..fixed(10.0, 10.0)
    };
    let pins = [
        LayoutStyle {
            transform: Some(PaintTransform {
                e: 3.25,
                f: 1.5,
                ..Default::default()
            }),
            ..fixed(40.0, 8.0)
        },
        LayoutStyle {
            position: PositionSpec::Sticky,
            offset_top: Some(LengthSpec::Px(0.0)),
            ..fixed(40.0, 8.0)
        },
        LayoutStyle {
            overflow_x: OverflowSpec::Hidden,
            overflow_y: OverflowSpec::Hidden,
            ..fixed(40.0, 8.0)
        },
        // Measured from the column, above the row.
        placed(PositionSpec::Absolute, 210.0, 3.0),
        placed(PositionSpec::Fixed, 320.0, 10.0),
        // Measured from the row's own cell.
        placed(PositionSpec::Absolute, 60.0, 2.0),
    ];
    let mut pinned = Vec::new();
    for (index, style) in pins.into_iter().enumerate() {
        let [row, cell, _] = rows[5 + index];
        let parent = if index == 3 { row } else { cell };
        let pin = b.element(parent, style);
        let inside = b.element(pin, fixed(4.0, 3.0));
        pinned.push((pin, inside));
    }
    b.queue
        .set_standard_visual(rows[11][2], Some(StandardVisual::KeymapLayer));
    let mut context = AppContext::new();
    context.commit_mutations(b.queue).unwrap();
    product_frame(
        &mut context,
        document(),
        viewport(),
        &mut bundled_face_shaper(),
    );
    let mut page = Page {
        context,
        rows,
        hanging: None,
        list,
    };
    let mut retained = snapshot(&page.context);
    for (height, at) in [(26.25, "grown"), (20.0, "back"), (33.5, "grown more")] {
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
        let (absolute, inside) = pinned[5];
        assert!(
            !updated.contains(&absolute) && !updated.contains(&inside),
            "{at}: a box measured from the moved cell kept its offset"
        );
        for &(pin, _) in &pinned[3..5] {
            assert!(updated.contains(&pin), "{at}: its frame did not move");
        }
    }
}

/// A box layout places from the page while its own style is in flow (an
/// open triggered menu's item) is noted as a pin when the frame is written,
/// and stops being one when it is placed from its parent again.
#[test]
fn a_box_placed_from_another_frame_is_noted_as_a_pin() {
    let mut page = page(2, false, false);
    let leaf = page.rows[1][2];
    let pinned = |context: &AppContext| {
        context
            .world()
            .nodes
            .accessibility_pins()
            .any(|id| id == leaf)
    };
    assert!(!pinned(&page.context), "an in-flow box pins nothing");
    let local = page.context.world().local_layout_box(leaf).unwrap();
    for (frame, expected) in [
        (crate::LayoutFrame::Page, true),
        (crate::LayoutFrame::Parent, false),
    ] {
        let mut queue = MutationQueue::new();
        queue.place_layout(leaf, local, frame);
        page.context.commit_mutations(queue).unwrap();
        assert_eq!(pinned(&page.context), expected, "{frame:?}");
    }
}
