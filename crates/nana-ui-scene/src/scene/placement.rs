//! Where a retained record stands now.
//!
//! A record keeps the page box it was extracted with ([`ExtractedNode::layout`])
//! and every rectangle derived from it -- primitives, component geometry,
//! clips -- in that space. When layout only moves a subtree, the runtime
//! sends its root's new local box ([`ExtractedMove`]) and nothing for what it
//! holds: each record is placed from its frame's current origin instead, the
//! same sum down the frame chain the runtime takes, and the difference from
//! where it was extracted is a translation its own transform takes on.
use super::*;

impl UiScene {
    /// Where `node`'s border box stands on the page now: its frame's origin
    /// plus its local box, summed down from the top one axis at a time, as
    /// the runtime sums them (`UiWorld::layout_box`). For a record extracted
    /// since the last move above it this is its [`ExtractedNode::layout`]
    /// origin to the bit. A frame the scene does not hold leaves the record
    /// where it was extracted.
    pub(super) fn placed_origin(&self, node: &ExtractedNode) -> [f32; 2] {
        let mut memo = self.placed_origins.lock().expect("scene placements");
        placed_origin_in(&self.nodes, &mut memo, self.placement_epoch, node)
    }

    /// How far `node`'s box stands from where its record was extracted:
    /// `None` when it stands there, which is every record but those under a
    /// kept move.
    pub(super) fn layout_shift(&self, node: &ExtractedNode) -> Option<[f32; 2]> {
        if self.moved_roots.is_empty() {
            return None;
        }
        let [x, y] = self.placed_origin(node);
        let shift = [x - node.layout.x, y - node.layout.y];
        (shift != [0.0, 0.0]).then_some(shift)
    }

    /// `node`'s box where it stands now, at its record's size.
    pub(super) fn placed_layout(&self, node: &ExtractedNode) -> LayoutBox {
        match self.layout_shift(node) {
            Some([x, y]) => LayoutBox {
                x: node.layout.x + x,
                y: node.layout.y + y,
                ..node.layout
            },
            None => node.layout,
        }
    }

    /// `transform`, a page-space transform of `node` (one its children are
    /// handed), for the space `node`'s own record is in: what was extracted
    /// at its old origin moves to the new one first.
    pub(super) fn record_space(
        &self,
        node: &ExtractedNode,
        transform: AffineTransform,
    ) -> AffineTransform {
        match self.layout_shift(node) {
            Some([x, y]) => {
                transform.then(AffineTransform::from_matrix([1.0, 0.0, 0.0, 1.0, x, y]))
            }
            None => transform,
        }
    }

    /// Where `id`'s box stands now, before any scroll or transform: its
    /// [`Self::node_bounds`] moved by what layout moved it since its record
    /// was extracted.
    pub fn node_layout_bounds(&self, id: StableNodeId) -> Option<SceneRect> {
        let node = self.nodes.get(&id)?;
        Some(scene_rect(self.placed_layout(node)))
    }
}

impl UiScene {
    /// Place again, where its frames stand now, every record under `roots`
    /// (moved this delta) that paints, and every record above one up to the
    /// page that a move kept: returns the ones that paint, to be rebuilt.
    ///
    /// A kept record is drawn through a translation, and the painter puts a
    /// pure translation on whole device pixels and keeps the glyph phase of
    /// the box the record was built at (#223): a record drawn that way does
    /// not paint what a cold one does when the move is a fraction of a
    /// pixel. So only what paints nothing -- most of a page's boxes -- stays
    /// where it was extracted. A record placed again is the cold record to
    /// the bit (its box is the frames' sum, and nothing else of it is on the
    /// page: a record with component geometry is extracted again instead),
    /// and so are the primitives built from it once the records above it
    /// are placed too.
    pub(super) fn place_painted_records(&mut self, roots: &[StableNodeId]) -> Vec<StableNodeId> {
        let mut candidates = Vec::new();
        match self.visibility.get() {
            Some(index) => {
                for &root in roots {
                    index.painted_under(root, &mut candidates);
                }
                candidates.sort_unstable();
                candidates.dedup();
            }
            // Primitives are keyed by node first: one run per node.
            None => {
                for id in self.primitives.keys() {
                    if candidates.last() != Some(&id.node) {
                        candidates.push(id.node);
                    }
                }
            }
        }
        let epoch = self.placement_epoch;
        let nodes = &self.nodes;
        let memo = self.placed_origins.get_mut().expect("scene placements");
        let stands = |node: &ExtractedNode, memo: &mut NodeMap<(u64, [f32; 2])>| {
            let origin = placed_origin_in(nodes, memo, epoch, node);
            (origin != [node.layout.x, node.layout.y]).then_some(origin)
        };
        let mut visited = NodeSet::default();
        let mut placed = Vec::new();
        let mut painted = Vec::new();
        for id in candidates {
            let Some(node) = nodes.get(&id) else {
                continue;
            };
            let Some(origin) = stands(node, memo) else {
                continue;
            };
            painted.push(id);
            placed.push((id, origin));
            visited.insert(id);
            let mut cursor = node.parent;
            while let Some(at) = cursor {
                if !visited.insert(at) {
                    break;
                }
                let Some(node) = nodes.get(&at) else {
                    break;
                };
                cursor = node.parent;
                if let Some(origin) = stands(node, memo) {
                    placed.push((at, origin));
                }
            }
        }
        for (id, [x, y]) in placed {
            let held = self.nodes.get_mut(&id).expect("held above");
            debug_assert!(
                held.component_geometry.is_none(),
                "{id:?}: a record with component geometry was kept through a move"
            );
            let node = Arc::make_mut(held);
            node.layout.x = x;
            node.layout.y = y;
        }
        painted
    }
}

/// [`UiScene::placed_origin`] against `memo`, stamped with `epoch`.
fn placed_origin_in(
    nodes: &SceneNodes,
    memo: &mut NodeMap<(u64, [f32; 2])>,
    epoch: u64,
    node: &ExtractedNode,
) -> [f32; 2] {
    if is_document(node) {
        return [0.0, 0.0];
    }
    // The nodes still to place, innermost first.
    let mut chain: Vec<&ExtractedNode> = Vec::new();
    let mut cursor = node;
    let mut origin = loop {
        if let Some(&(stamp, origin)) = memo.get(&cursor.id)
            && stamp == epoch
        {
            break origin;
        }
        let frame = match cursor.layout_frame {
            nana_ui_runtime::LayoutFrame::Parent => cursor.parent,
            nana_ui_runtime::LayoutFrame::Block(block) => Some(block),
            nana_ui_runtime::LayoutFrame::Page => None,
        };
        let top = match frame {
            None => Some([cursor.local_layout.x, cursor.local_layout.y]),
            Some(frame) => match nodes.get(&frame) {
                Some(frame) if is_document(frame) => {
                    Some([cursor.local_layout.x, cursor.local_layout.y])
                }
                Some(frame) => {
                    chain.push(cursor);
                    cursor = frame;
                    None
                }
                None => Some([cursor.layout.x, cursor.layout.y]),
            },
        };
        if let Some(top) = top {
            memo.insert(cursor.id, (epoch, top));
            break top;
        }
    };
    for node in chain.into_iter().rev() {
        origin = [
            origin[0] + node.local_layout.x,
            origin[1] + node.local_layout.y,
        ];
        memo.insert(node.id, (epoch, origin));
    }
    origin
}

fn is_document(node: &ExtractedNode) -> bool {
    matches!(node.kind.as_ref(), nana_ui_runtime::NodeKind::Document)
}
