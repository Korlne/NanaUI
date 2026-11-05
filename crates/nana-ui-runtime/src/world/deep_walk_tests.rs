//! A subtree mounted deep in a laid-out page: the frame after it reads up
//! the chain above every node it touches -- page origins in extraction and
//! accessibility, the hit index's shifts, depths and pointer events, layout's
//! islands, accessibility's chain answers. Each pass walks that chain once,
//! not once per node: its steps grow with the nodes plus the depth, not
//! their product. What the passes produce is checked against the full walks
//! as they run (the remembered answers assert the bit a walk gives).
#![cfg(test)]

use nana_ui_core::{LayoutStyle, LengthSpec};

use super::page_origins::steps::{self, Walk};
use super::reflow_oracle::{
    Builder, accessible_frame, bundled_face_shaper, column, fixed, node, product_frame, styled,
};
use super::{DocumentId, NodeKind, StableNodeId};
use crate::{AppContext, LayoutViewport, MutationQueue};

/// Columns in the chain under the page.
const DEPTH: u64 = 400;
/// Leaves mounted at the chain's bottom.
const LEAVES: u64 = 64;
/// Ids from here on are the leaves.
const FIRST_LEAF: u64 = 10_000;

fn document() -> DocumentId {
    DocumentId::new(1).unwrap()
}

fn viewport() -> LayoutViewport {
    LayoutViewport::new(800.0, 600.0)
}

/// A column as wide as what it holds, padded by an uneven step so the page
/// origins down the chain are sums that round.
fn padded() -> LayoutStyle {
    LayoutStyle {
        padding: Some(LengthSpec::Px(0.37)),
        ..column(None)
    }
}

/// The page with the chain under it, laid out and hit-indexed; with
/// `scrolled`, the chain stands in a scroller a hundred px tall, scrolled
/// and its hit entries shifted as a host shifts them. Returns the page and
/// the chain from the top.
fn page(scrolled: bool) -> (AppContext, Vec<StableNodeId>) {
    let mut cx = AppContext::new();
    let (mut builder, mut parent) = Builder::page(document(), 1, 400.0);
    let scroller = scrolled.then(|| {
        parent = builder.element(
            parent,
            LayoutStyle {
                height: Some(LengthSpec::Px(100.0)),
                overflow_y: nana_ui_core::OverflowSpec::Auto,
                ..column(None)
            },
        );
        parent
    });
    let mut chain = Vec::new();
    for _ in 0..DEPTH {
        parent = builder.element(parent, padded());
        chain.push(parent);
    }
    cx.commit_mutations(builder.queue).unwrap();
    product_frame(&mut cx, document(), viewport(), &mut bundled_face_shaper());
    cx.compat_world_mut().rebuild_hit_test(document());
    if let Some(scroller) = scroller {
        let mut queue = MutationQueue::new();
        queue.set_scroll_offset(scroller, crate::ScrollOffset { x: 0.0, y: 7.3 });
        cx.commit_mutations(queue).unwrap();
        product_frame(&mut cx, document(), viewport(), &mut bundled_face_shaper());
        let updates = cx.take_scroll_hit_updates();
        assert!(!updates.is_empty(), "the scroller scrolled");
        for (scroller, delta) in updates {
            cx.update_hit_test_scroll(document(), scroller, delta);
        }
    }
    (cx, chain)
}

/// The steps each walk took in each pass of the frame after mounting
/// [`LEAVES`] leaves at the chain's bottom ([`steps::take_all`]), and the
/// nodes the frame touched.
struct Walked {
    accessibility: [u64; steps::WALKS],
    extraction: [u64; steps::WALKS],
    touched: u64,
}

fn mount_leaves(scrolled: bool) -> Walked {
    let (mut cx, chain) = page(scrolled);
    let bottom = *chain.last().unwrap();
    let mut queue = MutationQueue::new();
    let mut leaves = Vec::new();
    for index in 0..LEAVES {
        let id = node(FIRST_LEAF + index);
        queue.create(id, document(), NodeKind::Element { tag: "div".into() });
        // Each leaf makes the chain taller: every box above it resizes.
        queue.set_style(id, styled(fixed(12.0, 3.0)));
        queue.insert(bottom, id, None);
        leaves.push(id);
    }
    cx.commit_mutations(queue).unwrap();
    let work = accessible_frame(
        &mut cx,
        document(),
        viewport(),
        &mut bundled_face_shaper(),
        false,
    );
    steps::take_all();
    assert!(
        cx.compat_world_mut()
            .rebuild_hit_test_scoped(document(), &work.input_hit_test),
        "the hit index takes the frame in place"
    );
    let delta = cx.world().project_accessibility_delta(&work);
    assert!(
        delta.updated.len() as u64 >= LEAVES,
        "the leaves and the chain project again"
    );
    let accessibility = steps::take_all();
    let mut extracted = chain.clone();
    extracted.extend(&leaves);
    let (records, _) = cx.world().extract_nodes_reusing(&extracted, |_| None);
    assert_eq!(records.len(), extracted.len());
    let extraction = steps::take_all();
    Walked {
        accessibility,
        extraction,
        touched: DEPTH + LEAVES,
    }
}

/// Steps per node touched that a walk of the chain per node would far pass:
/// one down a chain [`DEPTH`] deep is a hundred or more for every node. A
/// walk takes up to [`super::page_origins::DIRECT_DEPTH`] steps before it
/// looks for what it remembers.
const PER_NODE: u64 = 48;

fn assert_linear(what: &str, steps: u64, touched: u64) {
    assert!(
        steps <= PER_NODE * touched,
        "{what}: {steps} steps for {touched} nodes under a chain {DEPTH} deep"
    );
}

#[test]
fn page_origins_are_summed_once_down_a_deep_chain() {
    let walked = mount_leaves(false);
    let origin = Walk::PageOrigin as usize;
    assert_linear(
        "accessibility's page origins",
        walked.accessibility[origin],
        walked.touched,
    );
    assert_linear(
        "extraction's page origins",
        walked.extraction[origin],
        walked.touched,
    );
}
