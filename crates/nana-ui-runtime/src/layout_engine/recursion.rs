//! One level of a layout recursion, on a stack that cannot run out.
//!
//! Measure recurses once per tree level: a size comes up from the children a
//! constraint goes down to. Each level first reads how much stack it has
//! left: below [`RED_ZONE`] it carries on in a fresh [`SEGMENT`]
//! (`stacker::grow`), so no tree is too deep for the thread laying it out,
//! whatever the build and whatever that thread's stack (see `crate::stack`).
//!
//! Placement needs no such guard. Once measured, every box's size is known,
//! so any node can be a boundary: a level [`SPAN`] below the outermost writes
//! its own box and queues the rest of its subtree as a [`Boundary`], and the
//! outermost call places the queue in order before it returns. A subtree's
//! placement reads only its own inputs (where it stands, its size, its
//! containing block, its font and the viewport), never a sibling subtree's
//! output, so placing it later places it the same. The recursion is never
//! deeper than [`SPAN`] levels, whatever the tree; the stack it takes is
//! bounded by the span, not the depth.
//!
//! The same reading prices a level: the stack its entry has less than its
//! parent's entry had. The pass keeps the largest, per recursion
//! (`LayoutExecutionStats::{measure,placement}_stack_per_level`, published as
//! `WorkCounters::layout_{measure,placement}_stack_per_level`), and the
//! `stack_budget` tests hold it to a budget: the guard keeps a deep tree from
//! crashing, the budget keeps each level from growing back.

use super::grid::InheritedGridTracks;
use super::{PassIntrinsicCache, Size};
use crate::StableNodeId;
use crate::stack::{RED_ZONE, SEGMENT};

/// Placement levels one call descends before the rest of a subtree waits in
/// the pass's queue: about 90 KiB of placement stack in either build, an
/// unoptimized level costing three times an optimized one. Tests run with
/// the smallest span, so every one of them places through boundaries.
pub(super) const SPAN: usize = if cfg!(test) {
    2
} else if cfg!(debug_assertions) {
    8
} else {
    24
};

/// A subtree whose root has its box and whose children wait to be placed:
/// where `place_node_body` resumes.
pub(super) struct Boundary {
    pub(super) id: StableNodeId,
    pub(super) size: Size,
    pub(super) containing: Size,
    pub(super) parent_font_px: f32,
    pub(super) inherited: Option<InheritedGridTracks>,
}

#[derive(Clone, Copy)]
pub(super) enum Recursion {
    Measure,
    Placement,
}

/// The open levels of one recursion in a pass.
#[derive(Default)]
pub(super) struct LevelStack {
    /// Stack left at each open level's entry. `None` where the platform
    /// cannot tell, and for a level that moved: its children run on another
    /// segment and are not priced against it.
    open: Vec<Option<usize>>,
    /// Most stack one level took this pass, in bytes.
    pub(super) per_level: usize,
}

impl LevelStack {
    /// Levels open now.
    pub(super) fn depth(&self) -> usize {
        self.open.len()
    }

    fn enter(&mut self, remaining: Option<usize>, moves: bool) {
        if let (Some(Some(parent)), Some(here)) = (self.open.last(), remaining)
            && let Some(level) = parent.checked_sub(here)
        {
            self.per_level = self.per_level.max(level);
        }
        self.open.push(if moves { None } else { remaining });
    }

    fn exit(&mut self) {
        self.open.pop();
    }
}

/// Runs one level of `recursion` and prices it. A measure level moves to a
/// new segment when this stack is nearly used up; a placement level always
/// runs here, its depth bounded by [`SPAN`].
pub(super) fn level<T>(
    cache: &mut PassIntrinsicCache,
    recursion: Recursion,
    run: impl FnOnce(&mut PassIntrinsicCache) -> T,
) -> T {
    let remaining = crate::stack::remaining();
    let moves =
        matches!(recursion, Recursion::Measure) && remaining.is_some_and(|left| left < RED_ZONE);
    cache.levels(recursion).enter(remaining, moves);
    let out = if moves {
        stacker::grow(SEGMENT, || run(cache))
    } else {
        run(cache)
    };
    cache.levels(recursion).exit();
    out
}
