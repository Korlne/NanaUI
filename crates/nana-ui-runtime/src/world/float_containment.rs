//! Floats leave the flow (CSS 2.1 §9.5): a block container's `auto` height
//! is its in-flow content's, and the floats add to it only when the container
//! establishes a block formatting context (`overflow` other than `visible`,
//! `display: flow-root`, a float, an inline-block, a flex or grid item; CSS
//! 2.1 §10.6.7). A container of nothing but floats is 0 tall; the same
//! container with `overflow: hidden` is as tall as its packed floats, not as
//! the floats stacked one under another.
//!
//! The gate lays out a 400 px row of ten 30 px floats and an optional block
//! after them, each step against the expected height and a cold layout of
//! the same inputs: the row starting and stopping a block formatting context,
//! the block coming and going and clearing the floats, a float growing, and
//! the floats sinking into the flow and floating again.

#![cfg(test)]

use nana_ui_core::{ClearSpec, DisplaySpec, FloatSpec, LayoutStyle, LengthSpec, OverflowSpec};

use super::reflow_oracle::{
    Builder, assert_matches_cold, bundled_face_shaper, product_frame, styled,
};
use crate::{AppContext, DocumentId, LayoutViewport, MutationQueue, StableNodeId};

const CELLS: usize = 10;

#[derive(Clone, Copy, Debug)]
struct Spec {
    display: DisplaySpec,
    overflow: OverflowSpec,
    floated: bool,
    cell_height: f32,
    /// The block after the floats, and the side it clears.
    tail: Option<ClearSpec>,
}

impl Spec {
    /// The row's height CSS gives it.
    fn expected_height(self) -> f32 {
        let tail = if self.tail.is_some() { 10.0 } else { 0.0 };
        if !self.floated {
            return CELLS as f32 * self.cell_height + tail;
        }
        let clear = matches!(self.tail, Some(ClearSpec::Left | ClearSpec::Both));
        let in_flow = if clear { self.cell_height + tail } else { tail };
        let contains =
            self.overflow != OverflowSpec::Visible || self.display == DisplaySpec::FlowRoot;
        if contains {
            in_flow.max(self.cell_height)
        } else {
            in_flow
        }
    }
}

fn viewport() -> LayoutViewport {
    LayoutViewport::new(800.0, 600.0)
}

fn row_style(spec: Spec) -> LayoutStyle {
    LayoutStyle {
        display: Some(spec.display),
        width: Some(LengthSpec::Px(400.0)),
        overflow_x: spec.overflow,
        overflow_y: spec.overflow,
        ..LayoutStyle::default()
    }
}

fn cell_style(spec: Spec) -> LayoutStyle {
    LayoutStyle {
        width: Some(LengthSpec::Px(30.0)),
        height: Some(LengthSpec::Px(spec.cell_height)),
        float: if spec.floated {
            FloatSpec::Left
        } else {
            FloatSpec::None
        },
        ..LayoutStyle::default()
    }
}

fn tail_style(spec: Spec) -> LayoutStyle {
    LayoutStyle {
        display: Some(if spec.tail.is_some() {
            DisplaySpec::Block
        } else {
            DisplaySpec::None
        }),
        width: Some(LengthSpec::Px(400.0)),
        height: Some(LengthSpec::Px(10.0)),
        clear: spec.tail.unwrap_or(ClearSpec::None),
        ..LayoutStyle::default()
    }
}

struct Page {
    context: AppContext,
    row: StableNodeId,
    cells: Vec<StableNodeId>,
    tail: StableNodeId,
    next: StableNodeId,
}

fn page(spec: Spec) -> Page {
    let document = DocumentId::new(1).unwrap();
    let (mut b, page) = Builder::page(document, 1, 800.0);
    let row = b.element(page, row_style(spec));
    let cells = (0..CELLS)
        .map(|_| b.element(row, cell_style(spec)))
        .collect();
    let tail = b.element(row, tail_style(spec));
    let next = b.element(
        page,
        LayoutStyle {
            width: Some(LengthSpec::Px(400.0)),
            height: Some(LengthSpec::Px(10.0)),
            ..LayoutStyle::default()
        },
    );
    let mut context = AppContext::new();
    context.commit_mutations(b.queue).unwrap();
    product_frame(
        &mut context,
        document,
        viewport(),
        &mut bundled_face_shaper(),
    );
    Page {
        context,
        row,
        cells,
        tail,
        next,
    }
}

fn assert_row(page: &Page, spec: Spec) {
    let world = page.context.world();
    let row = world.layout_box(page.row).unwrap();
    assert!(
        (row.height - spec.expected_height()).abs() < 0.01,
        "{spec:?}: the row is {row:?}"
    );
    // What follows the row starts at its bottom.
    let next = world.layout_box(page.next).unwrap();
    assert!(
        (next.y - (row.y + row.height)).abs() < 0.01,
        "{spec:?}: {next:?} after {row:?}"
    );
    if spec.floated {
        // Side by side on one line at the row's top.
        for (index, &cell) in page.cells.iter().enumerate() {
            let cell = world.layout_box(cell).unwrap();
            assert!(
                (cell.x - (row.x + 30.0 * index as f32)).abs() < 0.01
                    && (cell.y - row.y).abs() < 0.01,
                "{spec:?}: float {index} is {cell:?} in {row:?}"
            );
        }
    }
}

#[test]
fn float_containment_follows_the_block_formatting_context() {
    let document = DocumentId::new(1).unwrap();
    let mut spec = Spec {
        display: DisplaySpec::Block,
        overflow: OverflowSpec::Visible,
        floated: true,
        cell_height: 20.0,
        tail: None,
    };
    let mut warm = page(spec);
    assert_row(&warm, spec);
    let mut shaper = bundled_face_shaper();
    let steps: [fn(&mut Spec); 9] = [
        |spec| spec.overflow = OverflowSpec::Hidden,
        |spec| spec.overflow = OverflowSpec::Visible,
        |spec| spec.display = DisplaySpec::FlowRoot,
        |spec| spec.display = DisplaySpec::Block,
        |spec| spec.tail = Some(ClearSpec::None),
        |spec| spec.tail = Some(ClearSpec::Left),
        |spec| spec.cell_height = 25.0,
        |spec| spec.floated = false,
        |spec| spec.floated = true,
    ];
    for step in steps {
        step(&mut spec);
        let mut queue = MutationQueue::new();
        queue.set_style(warm.row, styled(row_style(spec)));
        for &cell in &warm.cells {
            queue.set_style(cell, styled(cell_style(spec)));
        }
        queue.set_style(warm.tail, styled(tail_style(spec)));
        warm.context.commit_mutations(queue).unwrap();
        product_frame(&mut warm.context, document, viewport(), &mut shaper);
        assert_row(&warm, spec);
        let mut cold = page(spec);
        assert_row(&cold, spec);
        assert_matches_cold(&mut warm.context, &mut cold.context, document);
    }
}
