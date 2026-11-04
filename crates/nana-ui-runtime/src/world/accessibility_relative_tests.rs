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
    let full = snapshot(&page.context);
    for (id, node) in &full {
        assert_eq!(
            retained.get(id),
            Some(node),
            "{at}: the delta left {id:?} unlike a full projection"
        );
    }
    // A closed menu's options are no one's children any more: a platform
    // tree drops them with the field's new child list, not by tombstone.
    retained.retain(|id, _| full.contains_key(id) || page.context.world().nodes.contains(*id));
    assert_eq!(
        retained.len(),
        full.len(),
        "{at}: the delta left nodes a full projection does not have"
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

/// Rows grown by an uneven step, again and again, move what they hold by
/// amounts whose page sums round differently each frame. A box that moved
/// with its parent is handed its offset from it, its local layout offset,
/// so the bounds it kept are a cold projection's to the bit -- with a
/// visual on the page, too, so the projection takes the path visuals do.
#[test]
fn boxes_moved_by_an_uneven_step_keep_a_cold_projection_to_the_bit() {
    for visual in [false, true] {
        let (mut b, root) = Builder::page(document(), 1, 400.0);
        let list = b.element(root, column(Some(300.0)));
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
                    padding_left: Some(LengthSpec::Px(1.3)),
                    padding_top: Some(LengthSpec::Px(2.5)),
                    ..fixed(100.0, 14.0)
                },
            );
            let leaf = b.element(cell, fixed(50.0, 5.0));
            rows.push([row, cell, leaf]);
        }
        if visual {
            b.queue
                .set_standard_visual(rows[11][2], Some(StandardVisual::KeymapLayer));
        }
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
        let mut height = 20.0;
        for step in 0..12 {
            height += 1.37;
            let at = format!("visual {visual}, step {step}");
            let mut queue = MutationQueue::new();
            row_height(&mut queue, page.rows[0][0], height);
            let updated = edit(&mut page, &mut retained, queue, &at);
            for [_, cell, leaf] in &page.rows[1..] {
                assert!(
                    !updated.contains(cell) && !updated.contains(leaf),
                    "{at}: what a moved row holds moved with it"
                );
                let local = page.context.world().local_layout_box(*leaf).unwrap();
                let bounds = retained[leaf].bounds;
                assert_eq!(
                    (bounds.x.to_bits(), bounds.y.to_bits()),
                    (local.x.to_bits(), local.y.to_bits()),
                    "{at}: a leaf is handed its offset in its cell"
                );
            }
        }
    }
}

/// `rows` rows standing in `list` under `root`, each a row holding a cell
/// (padded unevenly) holding a leaf; the rows are `relative` when `placed`.
fn uneven_rows(b: &mut Builder, list: StableNodeId, rows: usize) -> Vec<[StableNodeId; 3]> {
    (0..rows)
        .map(|_| {
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
            let leaf = b.element(cell, fixed(50.0, 5.0));
            [row, cell, leaf]
        })
        .collect()
}

fn laid_out(b: Builder, rows: Vec<[StableNodeId; 3]>, list: StableNodeId) -> Page {
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
        rows,
        hanging: None,
        list,
    }
}

/// Grow the first row by an uneven step `steps` times, each a frame whose
/// delta must match a full projection; what the moved rows hold must not
/// project again. `each` sees the page after every step.
fn grow_unevenly(
    page: &mut Page,
    retained: &mut BTreeMap<StableNodeId, AccessibilityNode>,
    steps: usize,
    label: &str,
    mut each: impl FnMut(&Page, &BTreeMap<StableNodeId, AccessibilityNode>, &[StableNodeId], &str),
) {
    let mut height = 20.0;
    for step in 0..steps {
        height += 1.37;
        let at = format!("{label}, step {step}");
        let mut queue = MutationQueue::new();
        row_height(&mut queue, page.rows[0][0], height);
        let updated = edit(page, retained, queue, &at);
        for [row, cell, leaf] in &page.rows[1..] {
            assert!(updated.contains(row), "{at}: a moved row projects again");
            assert!(
                !updated.contains(cell) && !updated.contains(leaf),
                "{at}: what a moved row holds moved with it"
            );
        }
        each(page, retained, &updated, &at);
    }
}

/// Rows in a list scrolled by an uneven amount, grown by an uneven step again
/// and again. Scrolling only translates, so a moved row is handed its offset
/// less the list's scroll and what it holds its offset: the delta projects
/// the moved rows alone, from the pins, and what they kept is a cold
/// projection's to the bit -- with a visual on the page too, and after the
/// list scrolls again in a frame that also moves them.
#[test]
fn boxes_moved_in_a_scrolled_list_keep_a_cold_projection_to_the_bit() {
    for visual in [false, true] {
        let (mut b, root) = Builder::page(document(), 1, 400.0);
        let list = b.element(
            root,
            LayoutStyle {
                overflow_x: OverflowSpec::Auto,
                overflow_y: OverflowSpec::Auto,
                direction: Some(FlexDirection::Column),
                flex_shrink: Some(0.0),
                ..fixed(300.0, 120.0)
            },
        );
        let rows = uneven_rows(&mut b, list, 12);
        if visual {
            b.queue
                .set_standard_visual(rows[11][2], Some(StandardVisual::KeymapLayer));
        }
        let mut page = laid_out(b, rows, list);
        let mut retained = snapshot(&page.context);
        let mut scroll = 30.37;
        let mut queue = MutationQueue::new();
        queue.set_scroll_offset(page.list, ScrollOffset { x: 0.0, y: scroll });
        edit(&mut page, &mut retained, queue, "scrolled");
        for round in 0..2 {
            let label = format!("visual {visual}, round {round}");
            grow_unevenly(
                &mut page,
                &mut retained,
                6,
                &label,
                |page, retained, _, at| {
                    let world = page.context.world();
                    for [row, _, leaf] in &page.rows[1..] {
                        let local = world.local_layout_box(*leaf).unwrap();
                        let bounds = retained[leaf].bounds;
                        assert_eq!(
                            (bounds.x.to_bits(), bounds.y.to_bits()),
                            (local.x.to_bits(), local.y.to_bits()),
                            "{at}: a leaf is handed its offset in its cell"
                        );
                        let local = world.local_layout_box(*row).unwrap();
                        let bounds = retained[row].bounds;
                        assert_eq!(
                            (bounds.y.to_bits(), bounds.height.to_bits()),
                            ((local.y - scroll).to_bits(), local.height.to_bits()),
                            "{at}: a row is handed its offset less the scroll"
                        );
                    }
                },
            );
            // Scroll and move in one frame, then go on from there.
            scroll += 11.13;
            let mut queue = MutationQueue::new();
            queue.set_scroll_offset(page.list, ScrollOffset { x: 0.0, y: scroll });
            row_height(&mut queue, page.rows[0][0], 20.0);
            edit(
                &mut page,
                &mut retained,
                queue,
                &format!("{label}: scrolled and moved"),
            );
        }
    }
}

/// Rows under an ancestor that scales and translates them, moved by an uneven
/// step: a positive scale multiplies every offset by the same factor
/// wherever the boxes stand, so the moved rows' contents keep their bounds
/// to the bit. Under a rotation the bounds are the box around turned
/// corners, which no offset gives: those take the walk, and still match a
/// full projection.
#[test]
fn boxes_moved_under_a_transformed_ancestor_match_a_cold_projection() {
    let (sin, cos) = 0.3f32.sin_cos();
    for (name, transform, offsets) in [
        (
            "scaled",
            PaintTransform {
                a: 1.5,
                d: 1.25,
                e: 3.3,
                f: 7.1,
                ..Default::default()
            },
            true,
        ),
        (
            "rotated",
            PaintTransform {
                a: cos,
                b: sin,
                c: -sin,
                d: cos,
                e: 2.0,
                f: 1.0,
            },
            false,
        ),
    ] {
        let (mut b, root) = Builder::page(document(), 1, 400.0);
        let wrapper = b.element(
            root,
            LayoutStyle {
                transform: Some(transform),
                ..fixed(320.0, 500.0)
            },
        );
        let list = b.element(wrapper, column(Some(300.0)));
        let rows = uneven_rows(&mut b, list, 12);
        let mut page = laid_out(b, rows, list);
        let mut retained = snapshot(&page.context);
        if offsets {
            grow_unevenly(
                &mut page,
                &mut retained,
                12,
                name,
                |page, retained, _, at| {
                    for [_, _, leaf] in &page.rows[1..] {
                        let local = page.context.world().local_layout_box(*leaf).unwrap();
                        let bounds = retained[leaf].bounds;
                        assert_eq!(
                            [bounds.x, bounds.y, bounds.width, bounds.height].map(f32::to_bits),
                            [
                                transform.a * local.x,
                                transform.d * local.y,
                                transform.a * local.width,
                                transform.d * local.height,
                            ]
                            .map(f32::to_bits),
                            "{at}: a leaf is handed its scaled offset"
                        );
                    }
                },
            );
        } else {
            let mut height = 20.0;
            for step in 0..12 {
                height += 1.37;
                let mut queue = MutationQueue::new();
                row_height(&mut queue, page.rows[0][0], height);
                edit(
                    &mut page,
                    &mut retained,
                    queue,
                    &format!("{name}, step {step}"),
                );
            }
        }
    }
}

/// Pins below pinned boxes that still hand their children offsets -- a
/// scaled box, a sticky one, and an absolute box measured from the moved row
/// above its parent -- are found from the pins too: each holds a box that
/// clips (whose subtree projects again) and a plain one (which keeps its
/// bounds).
#[test]
fn pins_under_boxes_that_hand_offsets_are_found_from_the_pins() {
    let (mut b, root) = Builder::page(document(), 1, 400.0);
    let list = b.element(root, column(Some(300.0)));
    let rows = uneven_rows(&mut b, list, 12);
    let holders = [
        LayoutStyle {
            transform: Some(PaintTransform {
                a: 2.0,
                d: 0.5,
                e: 1.25,
                ..Default::default()
            }),
            ..fixed(40.0, 8.0)
        },
        LayoutStyle {
            position: PositionSpec::Sticky,
            offset_top: Some(LengthSpec::Px(0.0)),
            ..fixed(40.0, 8.0)
        },
        // The cell is static, so this one is measured from the row.
        LayoutStyle {
            position: PositionSpec::Absolute,
            offset_left: Some(LengthSpec::Px(60.0)),
            offset_top: Some(LengthSpec::Px(2.0)),
            ..fixed(30.0, 10.0)
        },
    ];
    let mut held = Vec::new();
    for (index, style) in holders.into_iter().enumerate() {
        let cell = rows[4 + index][1];
        let holder = b.element(cell, style);
        let clip = b.element(
            holder,
            LayoutStyle {
                overflow_x: OverflowSpec::Hidden,
                overflow_y: OverflowSpec::Hidden,
                ..fixed(20.0, 6.0)
            },
        );
        let clipped = b.element(clip, fixed(4.0, 3.0));
        let plain = b.element(holder, fixed(5.0, 2.0));
        held.push([holder, clip, clipped, plain]);
    }
    let mut page = laid_out(b, rows, list);
    assert!(matches!(
        page.context.world().layout_frame(held[2][0]),
        Some(crate::LayoutFrame::Block(_))
    ));
    let mut retained = snapshot(&page.context);
    grow_unevenly(&mut page, &mut retained, 6, "held", |_, _, updated, at| {
        for (index, [_, clip, clipped, plain]) in held.iter().enumerate() {
            assert!(
                updated.contains(clip) && updated.contains(clipped),
                "{at}: a moved clip projects what it holds again"
            );
            // A sticky box is marked whenever its scroller's geometry
            // changes, and projects its subtree again.
            if index != 1 {
                assert!(
                    !updated.contains(plain),
                    "{at}: a plain box kept its bounds"
                );
            }
        }
    });
}

/// A closed select projects its field alone, placed as any box is, so it
/// moves with its row and keeps its node; an open one places its list box
/// and options from its geometry, which is on the page, and projects again
/// under every move -- in a scrolled list, by an uneven step.
#[test]
fn a_closed_select_moves_with_its_row_and_an_open_one_projects_again() {
    let mut page = page(12, true, false);
    let mut select = crate::Select::new(Some("a"));
    select.options = vec![
        crate::SelectOption::new("a", "Alpha"),
        crate::SelectOption::new("b", "Beta"),
    ];
    let select = page
        .context
        .create_detached_component(document(), select)
        .unwrap();
    let field = select.stable_id();
    let mut queue = MutationQueue::new();
    queue.insert(page.rows[6][0], field, None);
    queue.set_scroll_offset(page.list, ScrollOffset { x: 0.0, y: 30.37 });
    let mut retained = snapshot(&page.context);
    edit(&mut page, &mut retained, queue, "placed");
    let pinned = |page: &Page| {
        page.context
            .world()
            .nodes
            .accessibility_pins()
            .any(|id| id == field)
    };
    let mut height = 20.0;
    for (opened, at) in [(false, "closed"), (true, "open"), (false, "closed again")] {
        if opened != retained[&field].expanded.unwrap_or(false) {
            page.context.toggle_select(select).unwrap();
            edit(
                &mut page,
                &mut retained,
                MutationQueue::new(),
                &format!("{at}: toggled"),
            );
        }
        assert_eq!(pinned(&page), opened, "{at}: only an open select pins");
        for step in 0..3 {
            height += 1.37;
            let mut queue = MutationQueue::new();
            row_height(&mut queue, page.rows[0][0], height);
            let updated = edit(
                &mut page,
                &mut retained,
                queue,
                &format!("{at}, step {step}"),
            );
            assert_eq!(
                updated.contains(&field),
                opened,
                "{at}, step {step}: a select projects again under a move only while open"
            );
        }
    }
}

/// A box that clips, resized where it stands as the rows in it grow, cuts
/// what it holds by a new box: its whole subtree projects again, though it
/// did not move. (A scroller whose content grew keeps its box, and cuts
/// as before: see the scrolled list above.)
#[test]
fn a_clip_resized_in_place_projects_what_it_holds_again() {
    let (mut b, root) = Builder::page(document(), 1, 400.0);
    let list = b.element(
        root,
        LayoutStyle {
            overflow_x: OverflowSpec::Hidden,
            overflow_y: OverflowSpec::Hidden,
            ..column(Some(300.0))
        },
    );
    let rows = uneven_rows(&mut b, list, 6);
    let mut page = laid_out(b, rows, list);
    let mut retained = snapshot(&page.context);
    let mut height = 20.0;
    for step in 0..4 {
        height += 1.37;
        let at = format!("step {step}");
        let mut queue = MutationQueue::new();
        row_height(&mut queue, page.rows[0][0], height);
        let updated = edit(&mut page, &mut retained, queue, &at);
        for [_, cell, leaf] in &page.rows {
            assert!(
                updated.contains(cell) && updated.contains(leaf),
                "{at}: what a resized clip holds is cut by its new box"
            );
        }
    }
}
