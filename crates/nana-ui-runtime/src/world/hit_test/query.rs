//! Walking the index for a point: roots, escaped entries, `open_hit` and
//! painter hit tests.

use super::*;

impl HitIndex {
    pub(super) fn viewport_hit_at(&self, x: f32, y: f32) -> bool {
        (!self.viewport_roots.is_empty()
            && self.root_bounds.visit(x, y, &mut |slot| {
                self.roots[slot].is_some_and(|id| self.viewport_roots.contains(&id))
            }))
            || self.escaped_hit_at(x, y)
    }

    /// Whether an entry that hangs from its clip parent covers `(x, y)`. Its
    /// place among its new siblings is not its paint order, so such a point
    /// is ranked by paint order as a viewport root's is.
    fn escaped_hit_at(&self, x: f32, y: f32) -> bool {
        self.escaped.iter().any(|id| {
            self.entries.get(id).is_some_and(|indexed| {
                indexed.entry.escaped
                    && self
                        .reach_point(*id, x, y)
                        .is_some_and(|(x, y)| indexed.bounds.contains(x, y))
            })
        })
    }

    pub(super) fn visit_roots(
        &self,
        x: f32,
        y: f32,
        emit: &mut impl FnMut(StableNodeId) -> bool,
    ) -> bool {
        self.root_bounds.visit(x, y, &mut |slot| {
            self.visit_hits(self.roots[slot].expect("live root bounds"), x, y, emit)
        })
    }

    /// Emits the hits under `x`, `y` in `id`'s subtree, topmost first, until
    /// `emit` says to stop; whether it did. The open levels are kept on the
    /// heap, so a subtree as deep as the tree allows is walked on any stack,
    /// in buffers reused from one query to the next.
    pub(super) fn visit_hits(
        &self,
        id: StableNodeId,
        x: f32,
        y: f32,
        emit: &mut impl FnMut(StableNodeId) -> bool,
    ) -> bool {
        // A query `emit` starts from inside this one walks its own buffers.
        let mut fresh = HitVisitScratch::default();
        let mut held = self.visit_scratch.try_borrow_mut().ok();
        let scratch = held.as_deref_mut().unwrap_or(&mut fresh);
        scratch.open.clear();
        scratch.children.clear();
        let stopped = self.walk_hits(id, x, y, emit, scratch);
        scratch.open.clear();
        scratch.children.clear();
        stopped
    }

    fn walk_hits(
        &self,
        id: StableNodeId,
        x: f32,
        y: f32,
        emit: &mut impl FnMut(StableNodeId) -> bool,
        scratch: &mut HitVisitScratch,
    ) -> bool {
        let HitVisitScratch { open, children } = scratch;
        let Some(root) = self.open_hit(id, x, y, children) else {
            return false;
        };
        open.push(root);
        while let Some(top) = open.last_mut() {
            if top.next < top.children_end {
                let child = children[top.next];
                top.next += 1;
                if !top.emitted_menu && self.entries[&child].entry.z_index <= top.menu_z {
                    top.emitted_menu = true;
                    if emit(top.id) {
                        return true;
                    }
                }
                let (child_x, child_y) = top.child_point;
                if let Some(level) = self.open_hit(child, child_x, child_y, children) {
                    open.push(level);
                }
                continue;
            }
            let done = open.pop().expect("a level is open");
            children.truncate(done.children_start);
            if !done.emitted_menu && emit(done.id) {
                return true;
            }
            let node = &self.entries[&done.id].entry;
            if node.hittable
                && transformed_contains(node.layout, node.transform, node.persp, done.x, done.y)
                && painter_accepts(
                    node,
                    node.layout,
                    node.transform,
                    node.persp,
                    done.x,
                    done.y,
                )
                && emit(done.id)
            {
                return true;
            }
        }
        false
    }

    /// `id` as an open level of [`Self::visit_hits`], its reachable children
    /// pushed onto `children`: `None` when the point misses it or one of its
    /// own clips, so nothing under it is hit.
    fn open_hit(
        &self,
        id: StableNodeId,
        x: f32,
        y: f32,
        children: &mut Vec<StableNodeId>,
    ) -> Option<OpenHit> {
        let indexed = self.entries.get(&id)?;
        if !indexed.bounds.contains(x, y) {
            return None;
        }
        let node = &indexed.entry;
        // Into the entry's own space: everything it holds is measured there.
        let (x, y) = (x - node.origin[0], y - node.origin[1]);
        if !node
            .self_clips
            .iter()
            .all(|(bounds, transform)| transformed_contains(*bounds, *transform, [0.0, 0.0], x, y))
        {
            return None;
        }
        if !path_clips_contain(&node.path_clips, node.transform, node.persp, x, y) {
            return None;
        }
        let menu_hit = node
            .menu
            .is_some_and(|menu| transformed_contains(menu, node.transform, node.persp, x, y));
        let mut children_ok = node
            .child_clips
            .iter()
            .all(|(bounds, transform)| transformed_contains(*bounds, *transform, [0.0, 0.0], x, y));
        // Children held in the entry's own layout coordinates take the
        // point through the inverse of its transform first.
        let child_point = if node.local_children {
            match transformed_point(node.transform, [0.0, 0.0], x, y) {
                Some((x, y)) => (x - indexed.shift[0], y - indexed.shift[1]),
                None => {
                    children_ok = false;
                    (x, y)
                }
            }
        } else {
            (x - indexed.shift[0], y - indexed.shift[1])
        };
        // The children the point can reach, topmost first, as the bounds
        // tree yields them.
        let children_start = children.len();
        if children_ok {
            indexed
                .child_bounds
                .visit(child_point.0, child_point.1, &mut |slot| {
                    children.push(indexed.children[slot].expect("live child bounds"));
                    false
                });
        }
        Some(OpenHit {
            id,
            x,
            y,
            child_point,
            children_start,
            children_end: children.len(),
            next: children_start,
            emitted_menu: !menu_hit,
            menu_z: node.z_index.max(1000),
        })
    }
}

/// A node [`HitIndex::visit_hits`] has entered and not yet finished.
pub(super) struct OpenHit {
    pub(super) id: StableNodeId,
    /// The point in this node's space (past its origin), and in its
    /// children's.
    pub(super) x: f32,
    pub(super) y: f32,
    child_point: (f32, f32),
    /// Its pending children: this range of the walk's shared list.
    children_start: usize,
    children_end: usize,
    /// The next of them to visit.
    pub(super) next: usize,
    /// Whether the node's menu, when the point is on it, is emitted yet.
    pub(super) emitted_menu: bool,
    menu_z: i32,
}

/// Whether a painted node's painter takes a point its box already contains.
pub(super) fn painter_accepts(
    node: &HitEntry,
    layout: LayoutBox,
    transform: [f32; 6],
    persp: [f32; 2],
    x: f32,
    y: f32,
) -> bool {
    let Some(hit) = node.painter.as_deref() else {
        return true;
    };
    let Some((px, py)) = transformed_point(transform, persp, x, y) else {
        return false;
    };
    let local = [px - layout.x, py - layout.y];
    let size = [layout.width, layout.height];
    if let Some(answer) = hit.painter.painter().hit_test(local, size) {
        return answer;
    }
    hit.recording.as_ref().is_none_or(|recording| {
        recording
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains(local, size)
    })
}

impl UiWorld {
    pub(super) fn painter_hit(
        &self,
        id: StableNodeId,
        layout: LayoutBox,
    ) -> Option<Arc<PainterHit>> {
        let painter = self.node_painter(id)?;
        let recording = painter
            .painter()
            .hit_painted_outline()
            .then(|| self.shared_recording(id, painter, layout))
            .flatten();
        Some(Arc::new(PainterHit {
            painter: painter.clone(),
            recording,
        }))
    }
}
