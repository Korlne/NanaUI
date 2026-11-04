//! Accessible projection from retained nodes.

use super::*;

#[derive(Default)]
struct ProjectionMemo {
    transforms: hashbrown::HashMap<StableNodeId, AccessibleTransform>,
    bounds: hashbrown::HashMap<StableNodeId, Option<LayoutBox>>,
    /// Ancestor-hidden answers. A deep tree otherwise rewalks the parent
    /// chain once per node and once per child filter.
    hidden: hashbrown::HashMap<StableNodeId, bool>,
    /// Nodes whose scroll and transform chain is the identity, so viewport
    /// bounds are the layout box.
    aligned: hashbrown::HashMap<StableNodeId, bool>,
    /// Nodes whose chain hands their children offsets
    /// ([`NodeRecord::hands_accessibility_offsets`]).
    offsets: hashbrown::HashMap<StableNodeId, bool>,
    /// What each overflow container of a clip chain leaves its descendants,
    /// with the containers above it, in viewport coordinates (#201).
    clips: hashbrown::HashMap<StableNodeId, LayoutBox>,
    /// How each node's accessible space stands ([`AccessibleSpace`]).
    spaces: hashbrown::HashMap<StableNodeId, AccessibleSpace>,
    chain: Vec<StableNodeId>,
    space_chain: Vec<StableNodeId>,
}

/// How a node's accessible space -- what its children's bounds are offsets
/// in ([`AccessibilityNode::transform`]) -- stands to its own box and to the
/// viewport.
#[derive(Clone, Copy, Debug, PartialEq)]
struct AccessibleSpace {
    /// Placed by its local offset and its own 2D transform in its parent's
    /// space ([`UiWorld::offset_placed`]), not from its viewport box.
    local: bool,
    /// The positive per-axis scale from its own layout px to its space,
    /// where its box stands upright in it: its children placed by their
    /// offsets stand at those offsets less its scroll, scaled by this.
    /// `None` where its box is turned or projected in the viewport and it is
    /// not placed by its offset: its children are placed from their
    /// viewport boxes.
    unit: Option<[f32; 2]>,
    /// Its space is turned from the viewport's axes: a rotation, a skew or
    /// a flip at or above it among boxes placed by their offsets. A box
    /// placed from its viewport box under it is handed the inverse turn.
    turned: bool,
}

#[derive(Clone, Copy)]
struct AccessibleTransform {
    own: ([f32; 6], [f32; 2]),
    children: ([f32; 6], [f32; 2]),
    blocks_3d: bool,
    /// The node's layout origin on the page ([`UiWorld::layout_origin`]):
    /// a child measured from it stands its local offset away, by the same
    /// addition [`UiWorld::record_layout_origin`] makes.
    origin: [f32; 2],
}

/// What a projected node's chain answers for every child it holds, asked
/// once for them all: whether a box above them is hidden, is not
/// axis-aligned ([`UiWorld::axis_aligned`]) or does not hand offsets
/// ([`UiWorld::offset_placed`]).
#[derive(Clone, Copy)]
struct ChildChain {
    hidden: bool,
    aligned: bool,
    hands_offsets: bool,
}

/// Whether the linear part `(a, b, c, d)` only scales, by a positive factor
/// on each axis.
fn upright(a: f32, b: f32, c: f32, d: f32) -> bool {
    b == 0.0 && c == 0.0 && a > 0.0 && d > 0.0
}

/// The inverse of the affine `[a, b, c, d, e, f]` (`x' = a·x + c·y + e`),
/// unless it is singular.
fn invert_affine([a, b, c, d, e, f]: [f32; 6]) -> Option<[f32; 6]> {
    let det = a * d - b * c;
    if det == 0.0 || !det.is_finite() {
        return None;
    }
    Some([
        d / det,
        -b / det,
        -c / det,
        a / det,
        (c * f - d * e) / det,
        (b * e - a * f) / det,
    ])
}

impl UiWorld {
    /// Layout box and viewport box are the same when no ancestor scrolls or
    /// transforms, and the box's accessible box is its layout box (not a
    /// modal frame's surface). The chain's answer is memoized; a skewed node
    /// forces the normal transform walk for itself and its descendants.
    ///
    /// The transform walk ([`Self::accessibility_ancestor_transform`]) reads
    /// each box's effective style, which visuals, overlays and stacking can
    /// make differ from its resolved one; only an open triggered menu's
    /// placement changes what the walk makes of it (its content is fixed,
    /// hanging from the menu). Motion overlays transform any box, and clip
    /// visuals cut it, so those leave the walk; every live modal frame is a
    /// clip visual, so none is left whose surface is its accessible box.
    fn axis_aligned(&self, id: StableNodeId, memo: &mut ProjectionMemo) -> bool {
        let Some(record) = self.nodes.get(id) else {
            return false;
        };
        let above = self.motion_free_unclipped()
            && record
                .hierarchy
                .parent
                .is_none_or(|parent| self.chain_axis_aligned(parent, memo));
        self.axis_aligned_under(id, record, above, memo)
    }

    /// [`Self::axis_aligned`] for `record`, `id`'s, with the chain above it
    /// already answered (`above`).
    fn axis_aligned_under(
        &self,
        id: StableNodeId,
        record: &NodeRecord,
        above: bool,
        memo: &mut ProjectionMemo,
    ) -> bool {
        // The box itself is not memoized, only the chains above it: a
        // projection asks after each of its children once, and their own
        // boxes are rarely asked after again, where every ancestor is.
        let aligned = above && self.box_axis_aligned(id, record);
        #[cfg(any(test, feature = "layout-verify"))]
        if aligned {
            assert_eq!(
                self.visible_accessibility_bounds(id, memo),
                self.component_layout_box(id),
                "{id:?}: an axis-aligned box's accessible box is not its layout box"
            );
        }
        #[cfg(not(any(test, feature = "layout-verify")))]
        let _ = memo;
        aligned
    }

    /// What `id`'s chain answers for the children it holds ([`ChildChain`]).
    fn child_chain(&self, id: StableNodeId, memo: &mut ProjectionMemo) -> ChildChain {
        let free = self.motion_free_unclipped();
        ChildChain {
            hidden: self.ancestor_hidden(id, memo),
            aligned: free && self.chain_axis_aligned(id, memo),
            hands_offsets: free && self.chain_hands_offsets(id, memo),
        }
    }

    /// No motion overlay and no clip visual anywhere: what decides whether
    /// a box is axis-aligned is on its chain.
    fn motion_free_unclipped(&self) -> bool {
        self.layout_length_tracks.is_empty()
            && self.presentation.is_empty()
            && self.clip_visuals == 0
    }

    /// Whether `record`, `id`'s, neither scrolls, transforms nor places
    /// itself off its parent's coordinates.
    fn box_axis_aligned(&self, id: StableNodeId, record: &NodeRecord) -> bool {
        let style = record.resolved_layout.as_ref();
        !(record.scroll_offset != ScrollOffset::default()
            || style.transform.is_some()
            || style.transform_3d.is_some()
            || matches!(style.position, PositionSpec::Fixed | PositionSpec::Sticky)
            || style.css_perspective.is_some()
            || style.preserve_3d
            || (!self.triggered_overlays.is_empty() && self.parent_triggered_overlay(id).is_some()))
    }

    /// [`Self::axis_aligned`] for `id` and every box above it, memoized.
    fn chain_axis_aligned(&self, id: StableNodeId, memo: &mut ProjectionMemo) -> bool {
        if let Some(aligned) = memo.aligned.get(&id) {
            return *aligned;
        }
        if !self.motion_free_unclipped() {
            memo.aligned.insert(id, false);
            return false;
        }
        memo.chain.clear();
        let mut current = Some(id);
        let mut aligned = true;
        while let Some(node) = current {
            if node != id
                && let Some(&known) = memo.aligned.get(&node)
            {
                aligned = known;
                break;
            }
            memo.chain.push(node);
            let Some(record) = self.nodes.get(node) else {
                aligned = false;
                break;
            };
            if !self.box_axis_aligned(node, record) {
                aligned = false;
                break;
            }
            current = record.hierarchy.parent;
        }
        for node in memo.chain.drain(..) {
            memo.aligned.insert(node, aligned);
        }
        aligned
    }

    /// Whether `id` and every box above it hand their children offsets
    /// ([`NodeRecord::hands_accessibility_offsets`]), with no motion overlay
    /// or clip visual anywhere: then every transform above a child of `id`
    /// is a translation and a positive scale. Memoized as
    /// [`Self::chain_axis_aligned`] is.
    fn chain_hands_offsets(&self, id: StableNodeId, memo: &mut ProjectionMemo) -> bool {
        if let Some(offsets) = memo.offsets.get(&id) {
            return *offsets;
        }
        if !self.motion_free_unclipped() {
            memo.offsets.insert(id, false);
            return false;
        }
        memo.chain.clear();
        let mut current = Some(id);
        let mut offsets = true;
        while let Some(node) = current {
            if node != id
                && let Some(&known) = memo.offsets.get(&node)
            {
                offsets = known;
                break;
            }
            memo.chain.push(node);
            let Some(record) = self.nodes.get(node) else {
                offsets = false;
                break;
            };
            if !record.hands_accessibility_offsets() {
                offsets = false;
                break;
            }
            current = record.hierarchy.parent;
        }
        for node in memo.chain.drain(..) {
            memo.offsets.insert(node, offsets);
        }
        offsets
    }

    /// Whether `id`'s accessible bounds and transform in its parent's space
    /// come from its local layout box and its own 2D transform: `record`,
    /// `id`'s, is placed by its offset
    /// ([`NodeRecord::placed_by_accessibility_offset`]) and its parent's
    /// space holds its own box upright at a positive scale `(a, d)`
    /// ([`AccessibleSpace::unit`]). Then its bounds are `(a·(x − sx), d·(y −
    /// sy), a·w, d·h)` -- its local box less the parent's scroll offset,
    /// scaled -- with its own transform, if any, folded in or handed on
    /// ([`Self::local_placement`]), whatever the boxes above stand at, and
    /// however they turn. Nothing in that reads a page position, so a box
    /// that moved with its parent keeps them to the bit.
    fn offset_placed(
        &self,
        id: StableNodeId,
        record: &NodeRecord,
        memo: &mut ProjectionMemo,
    ) -> bool {
        // [`Self::space_of`]'s test, from the parent's space alone: the box
        // itself is asked once and need not be kept.
        let placed = record.hierarchy.parent.is_some_and(|parent| {
            self.motion_free_unclipped()
                && self.geometry_frame.get().is_none()
                && self
                    .accessible_space(parent, memo)
                    .is_some_and(|space| space.unit.is_some())
        }) && self.offset_placed_under(id, record, true);
        #[cfg(any(test, feature = "layout-verify"))]
        assert_eq!(
            Some(placed),
            self.accessible_space(id, memo).map(|space| space.local),
            "{id:?}: placed by its offset, from its parent's space"
        );
        placed
    }

    /// `id`'s accessible space ([`AccessibleSpace`]), memoized with every
    /// box above it.
    fn accessible_space(
        &self,
        id: StableNodeId,
        memo: &mut ProjectionMemo,
    ) -> Option<AccessibleSpace> {
        if let Some(space) = memo.spaces.get(&id) {
            return Some(*space);
        }
        let mut chain = std::mem::take(&mut memo.space_chain);
        chain.clear();
        let mut at = Some(id);
        while let Some(node) = at {
            if memo.spaces.contains_key(&node) {
                break;
            }
            let Some(record) = self.nodes.get(node) else {
                break;
            };
            chain.push(node);
            at = record.hierarchy.parent;
        }
        while let Some(node) = chain.pop() {
            let Some(record) = self.nodes.get(node) else {
                continue;
            };
            let parent = record
                .hierarchy
                .parent
                .and_then(|parent| memo.spaces.get(&parent).copied());
            let space = self.space_of(node, record, parent, memo);
            memo.spaces.insert(node, space);
        }
        memo.space_chain = chain;
        memo.spaces.get(&id).copied()
    }

    /// [`AccessibleSpace`] of `record`, `id`'s, under a parent whose space is
    /// `parent`.
    fn space_of(
        &self,
        id: StableNodeId,
        record: &NodeRecord,
        parent: Option<AccessibleSpace>,
        memo: &mut ProjectionMemo,
    ) -> AccessibleSpace {
        let free = self.motion_free_unclipped();
        let above = parent.filter(|parent| parent.unit.is_some());
        if let Some(above) = above
            && free
            && self.geometry_frame.get().is_none()
            && self.offset_placed_under(id, record, true)
        {
            let [ua, ud] = above.unit.unwrap_or([1.0, 1.0]);
            return match self.own_accessible_transform(id, record) {
                None => AccessibleSpace {
                    local: true,
                    unit: above.unit,
                    turned: above.turned,
                },
                Some([a, b, c, d, ..]) if upright(a, b, c, d) => AccessibleSpace {
                    local: true,
                    unit: Some([ua * a, ud * d]),
                    turned: above.turned,
                },
                Some(_) => AccessibleSpace {
                    local: true,
                    unit: Some([1.0, 1.0]),
                    turned: true,
                },
            };
        }
        // Placed from its viewport box: its space is the viewport's, moved
        // to that box; its own box stands upright in it only where every
        // transform above scales it by a positive factor per axis.
        let unit = (free && record.hands_accessibility_offsets() && !self.is_modal_frame(id))
            .then(|| {
                self.accessibility_ancestor_transform(id, memo)?;
                let ([a, b, c, d, ..], [g, h]) = memo.transforms.get(&id)?.own;
                (g == 0.0 && h == 0.0 && upright(a, b, c, d)).then_some([a, d])
            })
            .flatten();
        AccessibleSpace {
            local: false,
            unit,
            turned: false,
        }
    }

    /// `record`'s own 2D transform, `id`'s, in its own layout px (its box at
    /// the origin, about its transform origin): `None` without one.
    fn own_accessible_transform(&self, _id: StableNodeId, record: &NodeRecord) -> Option<[f32; 6]> {
        let style = record.resolved_layout.as_ref();
        style.transform?;
        let local = record.local_layout;
        style
            .world_scene_transform(0.0, 0.0, local.width, local.height)
            .map(|(matrix, _)| matrix)
    }

    /// `id`'s bounds and transform in `parent`'s space, from its local box,
    /// its parent's scroll and its own transform, where `id` is placed by
    /// its offset ([`Self::offset_placed`]) and `unit` is its parent's
    /// ([`AccessibleSpace::unit`]). An upright transform of its own (a
    /// translation, a positive scale) is folded into the offset and the
    /// size; any other is handed on, scaled by `unit`, with the box's own
    /// size, so that what it holds is placed by offsets inside it.
    fn local_placement(
        &self,
        id: StableNodeId,
        record: &NodeRecord,
        parent: StableNodeId,
        [ua, ud]: [f32; 2],
    ) -> Option<(LayoutBox, Option<nana_ui_core::PaintTransform>)> {
        let scroll = self.nodes.get(parent)?.scroll_offset;
        let local = record.local_layout;
        let (x, y) = (local.x - scroll.x, local.y - scroll.y);
        Some(match self.own_accessible_transform(id, record) {
            None => (
                LayoutBox {
                    x: ua * x,
                    y: ud * y,
                    width: ua * local.width,
                    height: ud * local.height,
                },
                None,
            ),
            Some([a, b, c, d, e, f]) if upright(a, b, c, d) => (
                LayoutBox {
                    x: ua * (x + e),
                    y: ud * (y + f),
                    width: (ua * a) * local.width,
                    height: (ud * d) * local.height,
                },
                None,
            ),
            Some([a, b, c, d, e, f]) => (
                LayoutBox {
                    x: ua * x,
                    y: ud * y,
                    width: local.width,
                    height: local.height,
                },
                Some(nana_ui_core::PaintTransform {
                    a: ua * a,
                    b: ud * b,
                    c: ua * c,
                    d: ud * d,
                    e: ua * e,
                    f: ud * f,
                }),
            ),
        })
    }

    /// The map from `id`'s accessible space to the viewport, where its box
    /// stands upright in it ([`AccessibleSpace::unit`]): its box's map to
    /// the viewport after the inverse of that scale.
    fn accessible_space_to_viewport(
        &self,
        id: StableNodeId,
        memo: &mut ProjectionMemo,
    ) -> Option<[f32; 6]> {
        let [ua, ud] = self.accessible_space(id, memo)?.unit?;
        self.accessibility_ancestor_transform(id, memo)?;
        let transform = memo.transforms.get(&id)?;
        let ([a, b, c, d, e, f], [g, h]) = transform.own;
        if g != 0.0 || h != 0.0 {
            return None;
        }
        let [ox, oy] = transform.origin;
        Some([
            a / ua,
            b / ua,
            c / ud,
            d / ud,
            a * ox + c * oy + e,
            b * ox + d * oy + f,
        ])
    }

    /// [`Self::offset_placed`] with the chain above already answered.
    fn offset_placed_under(&self, id: StableNodeId, record: &NodeRecord, above: bool) -> bool {
        above
            && record.placed_by_accessibility_offset()
            && !matches!(record.kind.as_ref(), NodeKind::Document)
            && (self.triggered_overlays.is_empty() || self.parent_triggered_overlay(id).is_none())
    }

    /// Whether `id`, a child of `parent`, has a viewport box
    /// ([`Self::visible_accessibility_bounds`] is `Some`). A box placed by its
    /// offset ([`Self::offset_placed`]) under a parent whose transform is
    /// known stands where that transform for its children puts its layout
    /// box: it has one when those corners are finite, which is answered
    /// without computing and keeping its own transform and box.
    fn has_accessibility_bounds(
        &self,
        id: StableNodeId,
        parent: StableNodeId,
        memo: &mut ProjectionMemo,
    ) -> bool {
        let Some(record) = self.nodes.get(id) else {
            return self.visible_accessibility_bounds(id, memo).is_some();
        };
        let above = self.motion_free_unclipped() && self.chain_hands_offsets(parent, memo);
        let handed = self.handed_transform(parent, above, memo);
        self.has_accessibility_bounds_under(id, record, parent, above, handed, memo)
    }

    /// What `parent` hands the children that stand at their offsets in it
    /// ([`Self::has_accessibility_bounds_under`]), when its transform is
    /// known and its chain hands offsets (`above`): looked up once for all
    /// of its children.
    fn handed_transform(
        &self,
        parent: StableNodeId,
        above: bool,
        memo: &ProjectionMemo,
    ) -> Option<AccessibleTransform> {
        if !above || self.geometry_frame.get().is_some() {
            return None;
        }
        memo.transforms.get(&parent).copied()
    }

    /// [`Self::has_accessibility_bounds`] for `record`, `id`'s, with whether
    /// the chain above hands offsets already answered (`above`), and
    /// `parent`'s transform when that is known (`handed`,
    /// [`Self::handed_transform`]). The child's page box is its local offset
    /// from `parent`'s origin, which the parent's transform already holds.
    fn has_accessibility_bounds_under(
        &self,
        id: StableNodeId,
        record: &NodeRecord,
        parent: StableNodeId,
        above: bool,
        handed: Option<AccessibleTransform>,
        memo: &mut ProjectionMemo,
    ) -> bool {
        if let Some(transform) = handed
            && record.hierarchy.parent == Some(parent)
            && record.resolved_layout.transform.is_none()
            && self.offset_placed_under(id, record, above)
        {
            let ([a, b, c, d, e, f], [g, h]) = transform.children;
            if g == 0.0 && h == 0.0 {
                let local = record.local_layout;
                let bounds = LayoutBox {
                    x: transform.origin[0] + local.x,
                    y: transform.origin[1] + local.y,
                    ..local
                };
                let corner = |x: f32, y: f32| {
                    (a * x + c * y + e).is_finite() && (b * x + d * y + f).is_finite()
                };
                let finite = corner(bounds.x, bounds.y)
                    && corner(bounds.x + bounds.width, bounds.y + bounds.height);
                #[cfg(any(test, feature = "layout-verify"))]
                {
                    assert_eq!(
                        bounds,
                        self.record_layout_box(record),
                        "{id:?}: its page box is not its parent's origin and its offset"
                    );
                    assert_eq!(
                        finite,
                        self.visible_accessibility_bounds(id, memo).is_some(),
                        "{id:?}: whether it has a viewport box"
                    );
                }
                return finite;
            }
        }
        self.visible_accessibility_bounds(id, memo).is_some()
    }

    fn visible_accessibility_bounds(
        &self,
        id: StableNodeId,
        memo: &mut ProjectionMemo,
    ) -> Option<LayoutBox> {
        if let Some(bounds) = memo.bounds.get(&id) {
            return *bounds;
        }
        let bounds = self.compute_accessibility_bounds(id, memo);
        memo.bounds.insert(id, bounds);
        bounds
    }

    fn compute_accessibility_bounds(
        &self,
        id: StableNodeId,
        memo: &mut ProjectionMemo,
    ) -> Option<LayoutBox> {
        let local = if self.nodes.has_visuals()
            && matches!(
                self.nodes.visual(id),
                Some(StandardVisual::ModalFrame { .. })
            ) {
            match self.component_geometry(id) {
                Some(crate::ComponentGeometry::ModalFrame { surface, .. }) => surface,
                _ => self.component_layout_box(id)?,
            }
        } else {
            self.component_layout_box(id)?
        };
        let mut bounds = self.accessibility_viewport_bounds(id, local, memo)?;
        if self.clip_visuals == 0 {
            return Some(bounds);
        }
        // Only the clip chain clips `id` (#201): an escaped box passes over
        // the ancestors between it and its clip parent, and a `fixed` box,
        // `id` or one above it, ends the chain, as in paint and hit testing.
        let mut below = id;
        for ancestor in self.clip_ancestors(id) {
            if matches!(
                self.nodes.visual(ancestor),
                Some(StandardVisual::EmptyState { .. })
            ) {
                bounds = intersect_layout_boxes(
                    bounds,
                    self.accessibility_viewport_bounds(
                        ancestor,
                        self.component_layout_box(ancestor)?,
                        memo,
                    )?,
                )?;
            }
            if let Some(StandardVisual::ModalFrame { slots, .. }) = self.nodes.visual(ancestor)
                && let Some(crate::ComponentGeometry::ModalFrame { surface, body, .. }) =
                    self.component_geometry(ancestor)
            {
                bounds = intersect_layout_boxes(
                    bounds,
                    self.accessibility_viewport_bounds(ancestor, surface, memo)?,
                )?;
                if slots.body == Some(below) {
                    bounds = intersect_layout_boxes(
                        bounds,
                        self.accessibility_viewport_bounds(ancestor, body, memo)?,
                    )?;
                }
            }
            below = ancestor;
        }
        Some(bounds)
    }
}

impl UiWorld {
    // Match build_hit_forest's composition from the current retained state.
    // A committed transform may precede the hit projection; that projection
    // is not an authority for accessibility coordinates. Memoizing ancestors
    // also avoids repeating hit-index ancestor walks for every projected node.
    fn accessibility_ancestor_transform(
        &self,
        id: StableNodeId,
        memo: &mut ProjectionMemo,
    ) -> Option<[f32; 6]> {
        if let Some(transform) = memo.transforms.get(&id) {
            return Some(transform.own.0);
        }
        memo.chain.clear();
        let mut current = Some(id);
        let mut cumulative = (IDENTITY_AFFINE, [0.0, 0.0]);
        let mut blocks_3d = false;
        while let Some(node) = current {
            if let Some(transform) = memo.transforms.get(&node) {
                cumulative = transform.children;
                blocks_3d = transform.blocks_3d;
                break;
            }
            memo.chain.push(node);
            current = self.nodes.get(node)?.hierarchy.parent;
        }
        while let Some(node) = memo.chain.pop() {
            let record = self.nodes.get(node)?;
            let bounds = self.component_layout_box(node)?;
            let layout = self.hit_motion_layout(node);
            if layout.position == PositionSpec::Fixed {
                // Viewport-relative; ancestors cannot scroll or clip this branch.
                cumulative = self.hanging_branch_transform(node);
                blocks_3d = false;
            } else if let Some(anchor) = self.clip_escape(node) {
                // An escaped box resumes from its clip parent, through the
                // transforms it passes over but none of their scroll (#201).
                // The clip parent is an ancestor, so it is memoized by now.
                let base = match anchor.and_then(|anchor| memo.transforms.get(&anchor)) {
                    Some(transform) => {
                        blocks_3d = transform.blocks_3d;
                        transform.children
                    }
                    None => {
                        blocks_3d = false;
                        (IDENTITY_AFFINE, [0.0, 0.0])
                    }
                };
                cumulative = self.clip_bridge_transform(node, anchor, base, true);
            }
            let local = self.input_local_scene_transform(node, &layout, bounds, blocks_3d);
            cumulative = then_hit(cumulative, self.with_sticky_translation(node, local));
            let own = cumulative;
            let scroll = record.scroll_offset;
            cumulative = then_hit(
                cumulative,
                ([1.0, 0.0, 0.0, 1.0, -scroll.x, -scroll.y], [0.0, 0.0]),
            );
            blocks_3d |= layout.fails_closed_3d_context();
            memo.transforms.insert(
                node,
                AccessibleTransform {
                    own,
                    children: cumulative,
                    blocks_3d,
                    origin: [bounds.x, bounds.y],
                },
            );
        }
        memo.transforms.get(&id).map(|transform| transform.own.0)
    }

    /// AccessKit consumes document/viewport logical coordinates. Layout boxes
    /// deliberately stay unscrolled; compose current ancestor transforms so
    /// pointer and accessibility consumers agree after nested scrolling.
    fn accessibility_viewport_bounds(
        &self,
        id: StableNodeId,
        bounds: LayoutBox,
        memo: &mut ProjectionMemo,
    ) -> Option<LayoutBox> {
        let transform = self.accessibility_ancestor_transform(id, memo)?;
        let [a, b, c, d, e, f] = transform;
        if a == 1.0 && b == 0.0 && c == 0.0 && d == 1.0 && e == 0.0 && f == 0.0 {
            return Some(bounds);
        }
        let corners = [
            (bounds.x, bounds.y),
            (bounds.x + bounds.width, bounds.y),
            (bounds.x, bounds.y + bounds.height),
            (bounds.x + bounds.width, bounds.y + bounds.height),
        ]
        .map(|(x, y)| (a * x + c * y + e, b * x + d * y + f));
        if corners
            .iter()
            .any(|(x, y)| !x.is_finite() || !y.is_finite())
        {
            return None;
        }
        let x = corners.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
        let y = corners.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
        let right = corners
            .iter()
            .map(|p| p.0)
            .fold(f32::NEG_INFINITY, f32::max);
        let bottom = corners
            .iter()
            .map(|p| p.1)
            .fold(f32::NEG_INFINITY, f32::max);
        Some(LayoutBox {
            x,
            y,
            width: right - x,
            height: bottom - y,
        })
    }
}

impl UiWorld {
    /// What `id`'s clip chain leaves of the viewport, in the coordinates of
    /// its accessible bounds: the overflow containers that reach it, each
    /// where it sits after the scroll above it. `None` when none does. Paint
    /// and hit testing cut the box by the same chain.
    fn accessibility_clip(&self, id: StableNodeId, memo: &mut ProjectionMemo) -> Option<LayoutBox> {
        let clip = self.layout_clip_chain(id).next()?;
        self.accessibility_clip_region(clip, memo)
    }

    fn accessibility_clip_region(
        &self,
        clip: StableNodeId,
        memo: &mut ProjectionMemo,
    ) -> Option<LayoutBox> {
        if let Some(region) = memo.clips.get(&clip) {
            return Some(*region);
        }
        let layout = self.component_layout_box(clip)?;
        let (x, y, width, height) = self.hit_motion_layout(clip).overflow_clip_box(
            layout.x,
            layout.y,
            layout.width,
            layout.height,
        )?;
        let own = self.accessibility_viewport_bounds(
            clip,
            LayoutBox {
                x,
                y,
                width,
                height,
            },
            memo,
        )?;
        let region = match self
            .layout_clip_chain(clip)
            .next()
            .and_then(|outer| self.accessibility_clip_region(outer, memo))
        {
            Some(outer) => {
                let left = own.x.max(outer.x);
                let top = own.y.max(outer.y);
                LayoutBox {
                    x: left,
                    y: top,
                    width: ((own.x + own.width).min(outer.x + outer.width) - left).max(0.0),
                    height: ((own.y + own.height).min(outer.y + outer.height) - top).max(0.0),
                }
            }
            None => own,
        };
        memo.clips.insert(clip, region);
        Some(region)
    }
}

#[cfg(test)]
#[path = "accessibility_viewport_tests.rs"]
mod viewport_tests;

#[cfg(test)]
#[path = "accessibility_relative_tests.rs"]
mod relative_tests;

impl UiWorld {
    /// Whether a node or one of its ancestors is explicitly hidden from the
    /// accessibility tree. The retained hierarchy is still used for paint,
    /// layout, and input; this is only the semantic projection boundary.
    fn accessibility_hidden(&self, id: StableNodeId, memo: &mut ProjectionMemo) -> bool {
        // As in `axis_aligned`, only the boxes above are memoized.
        let Some(node) = self.nodes.get(id) else {
            return false;
        };
        if node.accessibility.hidden || self.inline_box_omitted(id) {
            return true;
        }
        node.hierarchy
            .parent
            .is_some_and(|parent| self.ancestor_hidden(parent, memo))
    }

    /// [`Self::accessibility_hidden`], memoized.
    fn ancestor_hidden(&self, id: StableNodeId, memo: &mut ProjectionMemo) -> bool {
        if let Some(hidden) = memo.hidden.get(&id) {
            return *hidden;
        }
        let Some(node) = self.nodes.get(id) else {
            memo.hidden.insert(id, false);
            return false;
        };
        if node.accessibility.hidden || self.inline_box_omitted(id) {
            memo.hidden.insert(id, true);
            return true;
        }
        let parent = node.hierarchy.parent;
        let hidden = parent.is_some_and(|parent| self.ancestor_hidden(parent, memo));
        memo.hidden.insert(id, hidden);
        hidden
    }

    /// What `id` says to assistive technology as the name of a node it
    /// labels: its own label, or its text.
    fn accessible_text(&self, id: StableNodeId) -> Option<Arc<str>> {
        let node = self.nodes.get(id)?;
        node.accessibility
            .label
            .clone()
            .filter(|label| !label.is_empty())
            .or_else(|| {
                (!node.text.value.is_empty())
                    .then(|| self.inline_text(id, true).unwrap_or_default().into())
            })
    }

    /// Every retained node is a mounted, visible, untransformed box. Viewport
    /// bounds are the layout box, and every structural child stays in the
    /// accessibility tree. One scan, then the per-node walk does not re-check
    /// ancestors. A single scrolled or transformed node keeps the normal path.
    fn accessibility_tree_is_plain(&self) -> bool {
        if !self.layout_length_tracks.is_empty()
            || !self.presentation.is_empty()
            || self.z_index_nodes != 0
            || self.nodes.has_visuals()
            || !self.overlay_host_nodes.is_empty()
            || self.clip_visuals != 0
            || self.nodes.has_inline_content()
            || !self.detached_mounted.is_empty()
        {
            return false;
        }
        self.nodes.records().all(|node| {
            node.mount == crate::MountState::Mounted
                && node.scroll_offset == crate::ScrollOffset::default()
                && !node.accessibility.hidden
                && node.resolved.0.visible
                && node.resolved.0.box_visible
                && !matches!(node.kind.as_ref(), NodeKind::Comment)
                && crate::components::layout_style_is_default(&node.resolved_layout)
        })
    }

    fn project_accessibility_node(
        &self,
        id: StableNodeId,
        plain: bool,
        memo: &mut ProjectionMemo,
    ) -> Option<AccessibilityNode> {
        if !plain && (!self.is_mounted(id) || self.accessibility_hidden(id, memo)) {
            return None;
        }
        let (
            parent,
            children,
            kind,
            state,
            text_value,
            document,
            visible,
            box_visible,
            writing,
            scrolls,
            layout_box,
        ) = {
            let node = self.nodes.get(id)?;
            (
                node.hierarchy.parent,
                Arc::clone(&node.hierarchy.children),
                Arc::clone(&node.kind),
                if node.resolved.0.visible {
                    node.accessibility.clone()
                } else {
                    AccessibilityState::default()
                },
                if node.resolved.0.visible {
                    node.text.value.clone()
                } else {
                    crate::TextValue::default()
                },
                node.document,
                node.resolved.0.visible,
                node.resolved.0.box_visible,
                node.resolved.0.writing_context(),
                node.style.layout.overflow_x.scrolls() || node.style.layout.overflow_y.scrolls(),
                self.record_layout_box(node),
            )
        };
        // Keep a neutral structural container so visible descendants never
        // reference an omitted parent. Hidden leaves and omitted layout subtrees
        // contribute neither semantics nor structure.
        if !visible && (!box_visible || children.is_empty()) {
            return None;
        }
        if matches!(kind.as_ref(), NodeKind::Comment) {
            return None;
        }
        let role = if !visible {
            AccessibilityRole::Generic
        } else {
            match (state.role, kind.as_ref()) {
                (AccessibilityRole::Generic, NodeKind::Document) => AccessibilityRole::Document,
                (AccessibilityRole::Generic, NodeKind::Text) => AccessibilityRole::Text,
                (role, _) => role,
            }
        };
        let secure = self.nodes.has_visuals()
            && matches!(
                self.nodes.visual(id),
                Some(StandardVisual::TextInput { secure: true, .. })
            );
        // Scroll positions are part of the semantic node, rather than a
        // second visual-only state. Publish an axis only when the retained
        // geometry says it can actually move; this keeps ordinary containers
        // from advertising meaningless scroll actions.
        let scroll_metrics = (visible && (scrolls || self.nodes.has_visuals()))
            .then(|| {
                self.is_scroll_container(id)
                    .then(|| self.scroll_metrics(id))
            })
            .flatten()
            .flatten();
        let scroll_offset = scroll_metrics.and_then(|_| self.scroll_offset(id));
        // A node another one names (a settings row's label beside its
        // switch) keeps a label of its own first, as `aria-label` wins over
        // `aria-labelledby`; an empty one names nothing. Its own text (a
        // select's shown option) is its value then, not its name.
        let label = match (!self.labelled_by.is_empty())
            .then(|| self.labelled_by(id))
            .flatten()
        {
            Some(source) if visible => state
                .label
                .clone()
                .filter(|label| !label.is_empty())
                .or_else(|| self.accessible_text(source)),
            // A password field's text is its secret, not a name for it.
            _ => state.label.clone().or_else(|| {
                (!secure && !text_value.is_empty()).then(|| {
                    if self.inline_record(id).is_some() {
                        self.inline_text(id, true).unwrap_or_default().into()
                    } else {
                        Arc::<str>::from(text_value.as_str())
                    }
                })
            }),
        };
        let bounds = if plain && self.layout_results.is_empty() {
            layout_box
        } else {
            self.accessible_viewport_bounds(id, plain, memo)?
        };
        let mut children = if plain {
            children.as_ref().clone()
        } else {
            let mut kept = Vec::with_capacity(children.len());
            if !children.is_empty() {
                let chain = self.child_chain(id, memo);
                let mut handed = self.handed_transform(id, chain.hands_offsets, memo);
                kept.extend(children.iter().copied().filter(|&child| {
                    let Some(node) = self.nodes.get(child) else {
                        return false;
                    };
                    let kept = node.resolved.0.box_visible
                        && (node.resolved.0.visible || !node.hierarchy.children.is_empty())
                        && !matches!(node.kind.as_ref(), NodeKind::Comment)
                        && !(chain.hidden
                            || node.accessibility.hidden
                            || self.inline_box_omitted(child))
                        && (self.axis_aligned_under(child, node, chain.aligned, memo) || {
                            // A child off the offset path memoizes the
                            // transform on its way: the next one uses it.
                            if handed.is_none() {
                                handed = self.handed_transform(id, chain.hands_offsets, memo);
                            }
                            self.has_accessibility_bounds_under(
                                child,
                                node,
                                id,
                                chain.hands_offsets,
                                handed,
                                memo,
                            )
                        });
                    #[cfg(any(test, feature = "layout-verify"))]
                    assert_eq!(
                        kept,
                        node.resolved.0.box_visible
                            && (node.resolved.0.visible || !node.hierarchy.children.is_empty())
                            && !matches!(node.kind.as_ref(), NodeKind::Comment)
                            && !self.accessibility_hidden(child, memo)
                            && (self.axis_aligned(child, memo)
                                || self.has_accessibility_bounds(child, id, memo)),
                        "{child:?}: kept by its parent's chain, not by its own"
                    );
                    kept
                }));
            }
            kept
        };
        if self.nodes.has_visuals()
            && let Some(StandardVisual::MenuSurface {
                kind: crate::MenuSurfaceKind::ContextMenu,
                open: true,
                rows,
                ..
            }) = self.nodes.visual(id)
        {
            children.extend(
                crate::menu_row_slots(rows)
                    .into_iter()
                    .filter_map(|slot| crate::virtual_menu_item_id(id, slot)),
            );
        }
        if self.nodes.has_visuals()
            && let Some(StandardVisual::TreeView { rows, .. }) = self.nodes.visual(id)
        {
            children.extend(
                (0..rows.len())
                    .map_while(|index| u16::try_from(index + 1).ok())
                    .filter_map(|slot| crate::virtual_tree_item_id(id, slot)),
            );
        }
        Some(AccessibilityNode {
            id,
            parent,
            children,
            role,
            label,
            description: state.description.clone(),
            role_description: state.role_description.clone(),
            value: if !visible || secure {
                None
            } else if self.nodes.has_text_inputs() {
                self.nodes
                    .text_input(id)
                    .map(|input| input.value_shared())
                    .or_else(|| state.value.as_ref().map(crate::TextValue::from))
            } else {
                state.value.as_ref().map(crate::TextValue::from)
            },
            disabled: visible
                && (state.disabled
                    || self
                        .confirm_action_effect(id)
                        .is_some_and(|effect| effect.0)),
            display_only: state.display_only,
            checked: state.checked,
            mixed: state.mixed,
            orientation: state.orientation,
            selected: state.selected,
            level: None,
            expanded: None,
            active_descendant: None,
            position_in_set: None,
            set_size: None,
            multiselectable: false,
            has_popup: state.has_popup,
            multiline: state.multiline,
            editable: state.editable,
            selection: if visible && self.nodes.has_text_inputs() {
                self.nodes.text_input(id).map(|input| input.selection)
            } else {
                None
            },
            modal: state.modal,
            busy: state.busy,
            invalid: state.invalid,
            // A hidden container stays neutral: it is no live region either,
            // as a `visibility: hidden` element is none on the web.
            live: (visible && !self.live_regions.is_empty())
                .then(|| self.live(id))
                .flatten(),
            numeric_minimum: state.numeric_minimum,
            numeric_maximum: state.numeric_maximum,
            numeric_step: state.numeric_step,
            numeric_value: state.numeric_value,
            scroll_x: scroll_metrics.and_then(|metrics| {
                let min = metrics.min_offset().x;
                let max = metrics.max_offset().x;
                let offset = scroll_offset?;
                (max > min).then_some(crate::AccessibilityScrollAxis {
                    value: offset.x as f64,
                    minimum: min as f64,
                    maximum: max as f64,
                })
            }),
            scroll_y: scroll_metrics.and_then(|metrics| {
                let min = metrics.min_offset().y;
                let max = metrics.max_offset().y;
                let offset = scroll_offset?;
                (max > min).then_some(crate::AccessibilityScrollAxis {
                    value: offset.y as f64,
                    minimum: min as f64,
                    maximum: max as f64,
                })
            }),
            focused: visible && self.input.focused.get(&document) == Some(&id),
            bounds,
            transform: None,
            clip: if plain {
                None
            } else {
                self.accessibility_clip(id, memo)
            },
            writing,
        })
    }
}

impl UiWorld {
    /// The items of an open context menu under it, at the bounds its
    /// geometry gives them on the page; `pages` takes those page boxes.
    fn project_virtual_menu_items(
        &self,
        menu: &AccessibilityNode,
        pages: &mut Vec<LayoutBox>,
        memo: &mut ProjectionMemo,
    ) -> Vec<AccessibilityNode> {
        let Some(StandardVisual::MenuSurface {
            kind: crate::MenuSurfaceKind::ContextMenu,
            open: true,
            rows,
            highlighted,
            ..
        }) = self.nodes.visual(menu.id)
        else {
            return Vec::new();
        };
        let options = match self.component_geometry(menu.id) {
            Some(crate::ComponentGeometry::MenuSurface { options, .. }) => options,
            _ => Vec::new(),
        };
        let count = rows.len().max(1) as f32;
        let slots = crate::menu_row_slots(rows);
        let surface = self.component_layout_box(menu.id).unwrap_or(menu.bounds);
        rows.iter()
            .enumerate()
            .filter_map(|(index, row)| {
                let id = crate::virtual_menu_item_id(menu.id, slots[index])?;
                let page = options
                    .get(index)
                    .map(|option| option.bounds)
                    .unwrap_or_else(|| LayoutBox {
                        x: surface.x,
                        y: surface.y + surface.height * index as f32 / count,
                        width: surface.width,
                        height: surface.height / count,
                    });
                let bounds = self
                    .accessibility_viewport_bounds(menu.id, page, memo)
                    .unwrap_or(page);
                pages.push(page);
                Some(AccessibilityNode {
                    id,
                    parent: Some(menu.id),
                    children: Vec::new(),
                    role: row.mark.role(),
                    label: Some(
                        row.accessible_name
                            .clone()
                            .filter(|name| !name.is_empty())
                            .unwrap_or_else(|| Arc::clone(&row.label)),
                    ),
                    value: None,
                    description: row.hint.clone(),
                    role_description: None,
                    disabled: row.disabled,
                    display_only: false,
                    checked: row.mark.checked(),
                    mixed: false,
                    orientation: None,
                    selected: row.mark.selected(),
                    level: None,
                    expanded: None,
                    active_descendant: None,
                    position_in_set: None,
                    set_size: None,
                    multiselectable: false,
                    has_popup: false,
                    multiline: false,
                    editable: false,
                    selection: None,
                    modal: false,
                    busy: false,
                    invalid: false,
                    live: None,
                    numeric_minimum: None,
                    numeric_maximum: None,
                    numeric_step: None,
                    numeric_value: None,
                    scroll_x: None,
                    scroll_y: None,
                    focused: *highlighted == Some(index),
                    bounds,
                    transform: None,
                    clip: menu.clip,
                    writing: menu.writing,
                })
            })
            .collect()
    }

    /// The visible rows of a tree as `TreeItem` nodes under it, each followed
    /// by its toggles as `Checkbox` nodes under the row (#353). The row the
    /// tree is on is focused while the tree has the focus.
    ///
    /// Each takes the bounds the tree's geometry gives it on the page;
    /// `pages` takes those page boxes.
    fn project_virtual_tree_items(
        &self,
        tree: &AccessibilityNode,
        pages: &mut Vec<LayoutBox>,
        memo: &mut ProjectionMemo,
    ) -> Vec<AccessibilityNode> {
        let Some(StandardVisual::TreeView {
            rows,
            active,
            column_labels,
            ..
        }) = self.nodes.visual(tree.id)
        else {
            return Vec::new();
        };
        let geometry = match self.component_geometry(tree.id) {
            Some(crate::ComponentGeometry::TreeView { rows, .. }) => rows,
            _ => Vec::new(),
        };
        let count = rows.len().max(1) as f32;
        let whole = self.component_layout_box(tree.id).unwrap_or(tree.bounds);
        let mut nodes = Vec::with_capacity(rows.len() * (1 + column_labels.len()));
        for (index, row) in rows.iter().enumerate() {
            let Ok(slot) = u16::try_from(index + 1) else {
                break;
            };
            let Some(id) = crate::virtual_tree_item_id(tree.id, slot) else {
                break;
            };
            let row_geometry = geometry.get(index);
            let page = row_geometry
                .map(|row| row.bounds)
                .unwrap_or_else(|| LayoutBox {
                    x: whole.x,
                    y: whole.y + whole.height * index as f32 / count,
                    width: whole.width,
                    height: whole.height / count,
                });
            let toggles = column_labels
                .iter()
                .enumerate()
                .filter_map(|(column, label)| {
                    if nana_ui_core::tree::toggle_hidden(row.hidden_toggles, column) {
                        return None;
                    }
                    let toggle = crate::virtual_tree_toggle_id(tree.id, slot, column)?;
                    // without geometry, a square of the row's height per
                    // column at its end
                    let icon = row_geometry
                        .and_then(|row| row.toggles.get(column))
                        .map(|(_, icon, _)| *icon)
                        .unwrap_or_else(|| {
                            let side = page.height.min(page.width);
                            let from_end = (column_labels.len() - column) as f32;
                            LayoutBox {
                                x: page.x + page.width - side * from_end,
                                y: page.y,
                                width: side,
                                height: side,
                            }
                        });
                    Some((toggle, column, label, icon))
                })
                .collect::<Vec<_>>();
            let bounds = self
                .accessibility_viewport_bounds(tree.id, page, memo)
                .unwrap_or(page);
            pages.push(page);
            nodes.push(AccessibilityNode {
                id,
                parent: Some(tree.id),
                children: toggles.iter().map(|(toggle, ..)| *toggle).collect(),
                role: crate::AccessibilityRole::TreeItem,
                label: Some(Arc::clone(&row.label)),
                value: None,
                description: None,
                role_description: None,
                disabled: row.disabled,
                display_only: false,
                checked: None,
                mixed: false,
                orientation: None,
                selected: Some(row.selected),
                level: Some(u32::from(row.depth) + 1),
                expanded: row.branch.then_some(row.expanded),
                active_descendant: None,
                position_in_set: None,
                set_size: None,
                multiselectable: false,
                has_popup: false,
                multiline: false,
                editable: false,
                selection: None,
                modal: false,
                busy: false,
                invalid: false,
                live: None,
                numeric_minimum: None,
                numeric_maximum: None,
                numeric_step: None,
                numeric_value: None,
                scroll_x: None,
                scroll_y: None,
                focused: tree.focused && active.as_ref() == Some(&row.id),
                bounds,
                transform: None,
                clip: tree.clip,
                writing: tree.writing,
            });
            nodes.extend(toggles.into_iter().map(|(toggle, column, label, icon)| {
                pages.push(icon);
                let icon = self
                    .accessibility_viewport_bounds(tree.id, icon, memo)
                    .unwrap_or(icon);
                AccessibilityNode {
                    id: toggle,
                    parent: Some(id),
                    children: Vec::new(),
                    role: crate::AccessibilityRole::Checkbox,
                    label: (!label.is_empty()).then(|| Arc::clone(label)),
                    value: None,
                    description: None,
                    role_description: None,
                    disabled: row.disabled,
                    display_only: false,
                    checked: Some(row.toggles.get(column).copied().unwrap_or(false)),
                    mixed: false,
                    orientation: None,
                    selected: None,
                    level: None,
                    expanded: None,
                    active_descendant: None,
                    position_in_set: None,
                    set_size: None,
                    multiselectable: false,
                    has_popup: false,
                    multiline: false,
                    editable: false,
                    selection: None,
                    modal: false,
                    busy: false,
                    invalid: false,
                    live: None,
                    numeric_minimum: None,
                    numeric_maximum: None,
                    numeric_step: None,
                    numeric_value: None,
                    scroll_x: None,
                    scroll_y: None,
                    focused: false,
                    bounds: icon,
                    transform: None,
                    clip: tree.clip,
                    writing: tree.writing,
                }
            }));
        }
        nodes
    }

    /// An open Select, Dropdown or SearchDropdown menu, read as a WAI-ARIA
    /// combobox reads: a list box under the field with one option per row
    /// the menu shows, each known by its value, at the bounds the menu
    /// paints and hits it (the field's [`crate::SelectMenuGeometry`], under
    /// the field's transform). A row scrolled out of the menu, or cut away by
    /// a clip above the field, is not there to read; a row cut in part reads
    /// as the part shown. The field says whether it is expanded and which
    /// option the keyboard is on; the options' set is the rows the menu
    /// lists, so a filtered SearchDropdown counts only its matches.
    ///
    /// `pages` takes the boxes on the page the list box and its options
    /// stand at, in the order they are returned.
    fn project_select_menu(
        &self,
        field: &mut AccessibilityNode,
        pages: &mut Vec<LayoutBox>,
        memo: &mut ProjectionMemo,
    ) -> Vec<AccessibilityNode> {
        let Some(StandardVisual::Select {
            opened,
            options,
            highlighted,
            checkable,
            ..
        }) = self.nodes.visual(field.id)
        else {
            return Vec::new();
        };
        if field.role != AccessibilityRole::ComboBox {
            return Vec::new();
        }
        // Open is `aria-expanded`; the `selected` the field keeps in its
        // state paints it, and a combobox is not itself chosen.
        field.expanded = Some(*opened);
        field.selected = None;
        if !*opened {
            return Vec::new();
        }
        let (
            Some(crate::ComponentGeometry::Select {
                menu: Some(menu), ..
            }),
            Some(listbox),
        ) = (
            self.component_geometry(field.id),
            crate::virtual_listbox_id(field.id),
        )
        else {
            return Vec::new();
        };
        // What the window and the clips above the field leave of the menu,
        // in the field's layout px, cut as hit testing cuts it.
        let room = match self.select_menu_room(field.id) {
            Some(room) => Some(LayoutBox {
                x: room.left,
                y: room.top,
                width: room.right - room.left,
                height: room.bottom - room.top,
            }),
            None if self.document_viewport_of(field.id).is_some() => return Vec::new(),
            None => None,
        };
        let seen = |bounds: LayoutBox| match room {
            Some(room) => intersect_layout_boxes(bounds, room),
            None => Some(bounds),
        };
        let Some((surface_page, surface)) = seen(menu.surface).and_then(|local| {
            Some((
                local,
                self.accessibility_viewport_bounds(field.id, local, memo)?,
            ))
        }) else {
            return Vec::new();
        };
        let set_size = u32::try_from(options.len()).ok();
        let slots = crate::menu_row_slots(options);
        let mut rows = Vec::with_capacity(options.len());
        let mut row_pages = Vec::with_capacity(options.len());
        for (index, (option, row)) in options.iter().zip(&menu.options).enumerate() {
            // Only the part of a row inside the menu's viewport is drawn and
            // takes the pointer.
            let Some((page, bounds)) = intersect_layout_boxes(row.bounds, menu.viewport)
                .and_then(seen)
                .and_then(|local| {
                    Some((
                        local,
                        self.accessibility_viewport_bounds(field.id, local, memo)?,
                    ))
                })
            else {
                continue;
            };
            let Some(id) = crate::virtual_menu_item_id(field.id, slots[index]) else {
                continue;
            };
            if *highlighted == Some(index) {
                field.active_descendant = Some(id);
            }
            row_pages.push(page);
            rows.push(AccessibilityNode {
                id,
                parent: Some(listbox),
                children: Vec::new(),
                role: AccessibilityRole::ListBoxOption,
                label: Some(
                    option
                        .accessible_name
                        .clone()
                        .filter(|name| !name.is_empty())
                        .unwrap_or_else(|| Arc::clone(&option.label)),
                ),
                value: None,
                description: option.hint.clone().filter(|hint| !hint.is_empty()),
                role_description: None,
                disabled: option.disabled,
                display_only: false,
                checked: None,
                mixed: false,
                orientation: None,
                selected: Some(option.mark.checked().unwrap_or(false)),
                level: None,
                expanded: None,
                active_descendant: None,
                position_in_set: u32::try_from(index + 1).ok(),
                set_size,
                multiselectable: false,
                has_popup: false,
                multiline: false,
                editable: false,
                selection: None,
                modal: false,
                busy: false,
                invalid: false,
                live: None,
                numeric_minimum: None,
                numeric_maximum: None,
                numeric_step: None,
                numeric_value: None,
                scroll_x: None,
                scroll_y: None,
                focused: false,
                bounds,
                transform: None,
                clip: field.clip,
                writing: field.writing,
            });
        }
        field.children.push(listbox);
        pages.push(surface_page);
        pages.extend(row_pages);
        let mut nodes = Vec::with_capacity(rows.len() + 1);
        nodes.push(AccessibilityNode {
            id: listbox,
            parent: Some(field.id),
            children: rows.iter().map(|row| row.id).collect(),
            role: AccessibilityRole::ListBox,
            label: field.label.clone(),
            value: None,
            description: None,
            role_description: None,
            disabled: false,
            display_only: false,
            checked: None,
            mixed: false,
            orientation: None,
            selected: None,
            level: None,
            expanded: None,
            active_descendant: None,
            position_in_set: None,
            set_size,
            multiselectable: *checkable,
            has_popup: false,
            multiline: false,
            editable: false,
            selection: None,
            modal: false,
            busy: false,
            invalid: false,
            live: None,
            numeric_minimum: None,
            numeric_maximum: None,
            numeric_step: None,
            numeric_value: None,
            scroll_x: None,
            scroll_y: None,
            focused: false,
            bounds: surface,
            transform: None,
            clip: field.clip,
            writing: field.writing,
        });
        nodes.extend(rows);
        nodes
    }

    /// Where `id`'s accessible box stands in the viewport, as its projection
    /// computes it.
    fn accessible_viewport_bounds(
        &self,
        id: StableNodeId,
        plain: bool,
        memo: &mut ProjectionMemo,
    ) -> Option<LayoutBox> {
        if plain || self.axis_aligned(id, memo) {
            self.component_layout_box(id)
        } else {
            self.visible_accessibility_bounds(id, memo)
        }
    }

    /// `id` and the virtual nodes it holds, their bounds relative to their
    /// parents' ([`AccessibilityNode::bounds`]) when `relative`, in viewport
    /// coordinates otherwise.
    fn project_accessibility_entries(
        &self,
        id: StableNodeId,
        plain: bool,
        relative: bool,
        memo: &mut ProjectionMemo,
    ) -> Vec<AccessibilityNode> {
        let mut pages = Vec::new();
        let mut entries = self.project_viewport_entries(id, plain, &mut pages, memo);
        if relative && !entries.is_empty() {
            self.make_bounds_relative(&mut entries, &pages, plain, memo);
        }
        entries
    }

    /// Move `entries`' bounds from the viewport to their parents' spaces.
    /// The first entry's parent is a retained node; the virtual ones after
    /// it hang from entries before them, and `pages` holds where each of
    /// those stands on the page (`pages[i]` for `entries[i + 1]`).
    fn make_bounds_relative(
        &self,
        entries: &mut [AccessibilityNode],
        pages: &[LayoutBox],
        plain: bool,
        memo: &mut ProjectionMemo,
    ) {
        let owner = entries[0].id;
        // Virtual nodes stand in their owner's space. Where that is turned
        // from the viewport, they are placed in it from their page boxes,
        // upright there as on the page; elsewhere from their viewport boxes.
        let turned = (entries.len() > 1 && !plain)
            .then(|| self.accessible_space(owner, memo))
            .flatten()
            .filter(|space| space.turned)
            .and_then(|space| Some((space.unit?, self.layout_origin(owner)?)));
        let in_owner = |page: LayoutBox| {
            let ([ua, ud], [ox, oy]) = turned.expect("placed in a turned owner");
            LayoutBox {
                x: ua * (page.x - ox),
                y: ud * (page.y - oy),
                width: ua * page.width,
                height: ud * page.height,
            }
        };
        let origins = (entries.len() > 1)
            .then(|| {
                entries
                    .iter()
                    .enumerate()
                    .map(|(index, entry)| match (turned, index) {
                        (Some(_), 0) => (entry.id, 0.0, 0.0),
                        (Some(_), _) => {
                            let placed = in_owner(pages[index - 1]);
                            (entry.id, placed.x, placed.y)
                        }
                        (None, _) => (entry.id, entry.bounds.x, entry.bounds.y),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for (index, entry) in entries.iter_mut().enumerate() {
            let Some(parent) = entry.parent else {
                continue;
            };
            if index == 0 {
                if let Some((bounds, transform)) =
                    self.offset_bounds_in_parent(entry, parent, plain, memo)
                {
                    entry.bounds = bounds;
                    entry.transform = transform;
                    continue;
                }
                if !plain && let Some(inverse) = self.turned_space_inverse(parent, memo) {
                    // Placed from its viewport box under a turned space: the
                    // inverse turn brings that box back upright.
                    let [a, b, c, d, e, f] = inverse;
                    let (x, y) = (entry.bounds.x, entry.bounds.y);
                    entry.bounds.x = a * x + c * y + e;
                    entry.bounds.y = b * x + d * y + f;
                    entry.transform = Some(nana_ui_core::PaintTransform {
                        a,
                        b,
                        c,
                        d,
                        e: 0.0,
                        f: 0.0,
                    });
                    continue;
                }
            } else if turned.is_some() {
                let placed = in_owner(pages[index - 1]);
                let (x, y) = origins
                    .iter()
                    .find(|(id, ..)| *id == parent)
                    .map_or((0.0, 0.0), |&(_, x, y)| (x, y));
                entry.bounds = LayoutBox {
                    x: placed.x - x,
                    y: placed.y - y,
                    ..placed
                };
                continue;
            }
            let origin = if index == 0 {
                if plain && self.layout_results.is_empty() {
                    self.layout_origin(parent).map(|[x, y]| (x, y))
                } else {
                    self.accessible_viewport_bounds(parent, plain, memo)
                        .map(|bounds| (bounds.x, bounds.y))
                }
            } else {
                origins
                    .iter()
                    .find(|(id, ..)| *id == parent)
                    .map(|&(_, x, y)| (x, y))
            };
            if let Some((x, y)) = origin {
                entry.bounds.x -= x;
                entry.bounds.y -= y;
            }
        }
    }

    /// The map from the viewport into `id`'s accessible space, when that
    /// space is turned from the viewport's axes ([`AccessibleSpace::turned`]).
    fn turned_space_inverse(
        &self,
        id: StableNodeId,
        memo: &mut ProjectionMemo,
    ) -> Option<[f32; 6]> {
        if !self.accessible_space(id, memo)?.turned {
            return None;
        }
        invert_affine(self.accessible_space_to_viewport(id, memo)?)
    }

    /// `entry`'s bounds and transform in `parent`'s space, from its local
    /// layout box read straight off the record, when that is what they are:
    /// both accessible boxes are their layout boxes (its offset), or it is
    /// placed by its offset ([`Self::offset_placed`]: the offset less the
    /// parent's scroll, scaled, with its own transform folded in or handed
    /// on). The viewport boxes are chain sums through the transforms, so
    /// their difference would equal this only up to the last bit, and that
    /// bit moves with every ancestor a fractional move shifts; the offset
    /// itself does not, which is what lets a box that follows its parent
    /// keep the bounds it was handed ([`Self::keeps_relative_bounds`]).
    fn offset_bounds_in_parent(
        &self,
        entry: &AccessibilityNode,
        parent: StableNodeId,
        plain: bool,
        memo: &mut ProjectionMemo,
    ) -> Option<(LayoutBox, Option<nana_ui_core::PaintTransform>)> {
        let record = self.nodes.get(entry.id)?;
        if record.layout_frame != crate::LayoutFrame::Parent
            || record.hierarchy.parent != Some(parent)
            || matches!(record.kind.as_ref(), NodeKind::Document)
            || self.geometry_frame.get().is_some()
        {
            return None;
        }
        let local = record.local_layout;
        let offset = LayoutBox {
            x: local.x,
            y: local.y,
            width: entry.bounds.width,
            height: entry.bounds.height,
        };
        // An aligned box's chain runs through its parent, so the parent's
        // accessible box is its layout box too.
        if plain || self.axis_aligned(entry.id, memo) {
            return Some((offset, None));
        }
        if self.offset_placed(entry.id, record, memo) {
            let unit = self.accessible_space(parent, memo)?.unit?;
            let placed = self.local_placement(entry.id, record, parent, unit)?;
            #[cfg(any(test, feature = "layout-verify"))]
            self.check_offset_bounds(entry, parent, placed, memo);
            return Some(placed);
        }
        // Off those paths, only where what was projected for both is their
        // layout boxes, as a cold projection finds them, and the parent's
        // space is the viewport's, moved.
        if self
            .accessible_space(parent, memo)
            .is_some_and(|space| space.turned)
        {
            return None;
        }
        let own = self.record_layout_origin(record);
        let above = self.layout_origin(parent)?;
        let parent_bounds = self.accessible_viewport_bounds(parent, plain, memo)?;
        ([entry.bounds.x, entry.bounds.y] == own && [parent_bounds.x, parent_bounds.y] == above)
            .then_some((offset, None))
    }

    /// `id`'s projection, and the virtual nodes it holds after it, all with
    /// their bounds in the viewport; `pages` takes the page boxes of the
    /// virtual ones, in order.
    fn project_viewport_entries(
        &self,
        id: StableNodeId,
        plain: bool,
        pages: &mut Vec<LayoutBox>,
        memo: &mut ProjectionMemo,
    ) -> Vec<AccessibilityNode> {
        let Some(mut node) = self.project_accessibility_node(id, plain, memo) else {
            return Vec::new();
        };
        if self.nodes.has_visuals()
            && matches!(self.nodes.visual(id), Some(StandardVisual::Select { .. }))
        {
            let menu = self.project_select_menu(&mut node, pages, memo);
            let mut entries = vec![node];
            entries.extend(menu);
            return entries;
        }
        if self.nodes.has_visuals()
            && matches!(self.nodes.visual(id), Some(StandardVisual::TreeView { .. }))
        {
            let rows = self.project_virtual_tree_items(&node, pages, memo);
            let mut entries = vec![node];
            entries.extend(rows);
            return entries;
        }
        let open_context_menu = self.nodes.has_visuals()
            && matches!(
                self.nodes.visual(id),
                Some(StandardVisual::MenuSurface {
                    kind: crate::MenuSurfaceKind::ContextMenu,
                    open: true,
                    ..
                })
            );
        if !open_context_menu {
            return vec![node];
        }
        let items = self.project_virtual_menu_items(&node, pages, memo);
        let mut entries = vec![node];
        entries.extend(items);
        entries
    }

    /// Project one complete incremental accessibility transaction, including
    /// tombstones for nodes removed from the retained world.
    pub fn project_accessibility_delta(&self, work: &SystemWork) -> AccessibilityDelta {
        // Scrolling and transforms leave LayoutBox unchanged and may carry no
        // ACCESSIBILITY dirty bit. Their hit-test subtrees nevertheless moved
        // in viewport space, so the native accessibility cache must see them.
        //
        // Scheduled-layout nodes are deliberately NOT seeds, for the same
        // reason `RuntimeDocument::apply_hit_test_work` refuses them: layout
        // invalidation propagates to ancestors, so any leaf resize puts the
        // document root in `work.layout_frontier_seeds`, and expanding a root seed walks the
        // whole document. The nodes whose box actually moved are already here
        // by a different route -- layout writeback commits `WriteLayout`,
        // which marks INPUT | RENDER | ACCESSIBILITY on exactly those nodes,
        // shifted descendants included.
        //
        // Bounds are relative to the parent's, so a box layout moved with its
        // parent keeps them, and nothing marks it: only the box layout wrote
        // with a new origin is listed (`layout_moved`, `accessibility_moved`).
        // Such a node is no subtree seed; of what it holds, only what did not
        // move with it, or does not keep its offset to the bit, projects
        // again (`reach_moved_subtree`).
        //
        // These sets exist to produce a sorted, de-duplicated id sequence. A
        // `BTreeSet` pays a tree insert and node allocation per id to do that;
        // sorting a flat vec once yields the identical sequence for far less.
        let mut affected = work.accessibility.clone();
        // A node named by another one's text is projected again with it.
        let named = affected
            .iter()
            .filter_map(|id| self.labels.get(id))
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        affected.extend(named);
        let moved = |id: &StableNodeId| work.accessibility_moved.binary_search(id).is_ok();
        // A plain tree has no clips: its styles are all the default.
        let tree_plain =
            affected.len() + work.input_hit_test.len() >= 64 && self.accessibility_tree_is_plain();
        // A box that clips what it holds, or any box under a clip visual,
        // cuts boxes that need not move with it: its whole subtree projects
        // again -- unless layout left the box where and as big as it was (a
        // scroller whose content grew), so it cuts them as before.
        let reshaped = |id: &StableNodeId| work.layout_reshaped.binary_search(id).is_ok();
        let carries = |id: &StableNodeId| {
            moved(id)
                && (tree_plain
                    || (self.clip_visuals == 0 && (!self.clips_overflow(*id) || !reshaped(id))))
        };
        let (moved_seeds, mut pending): (Vec<StableNodeId>, Vec<StableNodeId>) =
            work.input_hit_test.iter().partition(|id| carries(id));
        pending.extend(&work.transform);
        let mut memo = ProjectionMemo::default();
        // A plain tree places every box in flow, from its parent, and has
        // no clip, transform or scroll: what a moved box holds keeps its
        // bounds.
        if !tree_plain {
            // Moved boxes whose subtree places its boxes by their offsets
            // all the way down, but where something pins them: those are
            // found from the pins, not by walking what the box holds.
            let mut pin_roots = Vec::new();
            // Boxes resized where they stood whose own transform turns
            // about a centre the new size moved: their space moved, as a
            // moved box's does, while what they hold kept its offsets in
            // it. They are walked as moved boxes are.
            let mut resized_roots = Vec::new();
            for &id in &moved_seeds {
                if work.layout_moved.binary_search(&id).is_err()
                    && let Some(node) = self.nodes.get(id)
                    && node.resolved_layout.transform.is_some()
                    && self.moved_root_hands_offsets(id, &mut memo)
                {
                    resized_roots.push(id);
                }
            }
            let walk_roots = if resized_roots.is_empty() {
                std::borrow::Cow::Borrowed(&work.layout_moved)
            } else {
                let mut roots = work.layout_moved.clone();
                roots.extend(&resized_roots);
                roots.sort_unstable();
                roots.dedup();
                std::borrow::Cow::Owned(roots)
            };
            for &id in &moved_seeds {
                if work.layout_moved.binary_search(&id).is_ok() {
                    if self.moved_root_hands_offsets(id, &mut memo) {
                        pin_roots.push(id);
                    } else {
                        let mut open = Vec::new();
                        self.reach_moved_subtree(
                            id,
                            &mut open,
                            &walk_roots,
                            false,
                            &mut affected,
                            &mut pending,
                            &mut memo,
                        );
                    }
                } else if resized_roots.binary_search(&id).is_ok() {
                    pin_roots.push(id);
                } else if let Some(node) = self.nodes.get(id) {
                    // Resized where it stood: what it holds stays on the
                    // page, unless its own transform turns about a centre
                    // the new size moved, or its box is turned or projected
                    // in the viewport, so its accessible box is the box
                    // around turned corners, whose origin the new size
                    // moves.
                    let style = node.resolved_layout.as_ref();
                    if style.transform.is_some()
                        || style.transform_3d.is_some()
                        || style.css_perspective.is_some()
                        || !std::iter::successors(Some(node), |record| {
                            record
                                .hierarchy
                                .parent
                                .and_then(|parent| self.nodes.get(parent))
                        })
                        .all(NodeRecord::hands_accessibility_offsets)
                        || self
                            .accessible_space(id, &mut memo)
                            .is_none_or(|space| space.unit.is_none())
                    {
                        affected.extend(
                            node.hierarchy
                                .children
                                .iter()
                                .copied()
                                .filter(|child| !moved(child)),
                        );
                    }
                }
            }
            if !pin_roots.is_empty() {
                pin_roots.sort_unstable();
                let (mut pinned, mut pinned_subtrees) = (Vec::new(), Vec::new());
                self.reach_moved_pins(
                    &pin_roots,
                    &walk_roots,
                    &mut pinned,
                    &mut pinned_subtrees,
                    &mut memo,
                );
                #[cfg(any(test, feature = "layout-verify"))]
                self.check_moved_pins(&pin_roots, &walk_roots, &pinned, &pinned_subtrees);
                affected.extend(pinned);
                pending.extend(pinned_subtrees);
            }
        }
        // `visited` is membership-only, so it does not need ordering at all.
        let mut visited = hashbrown::HashSet::new();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            affected.push(id);
            if let Some(node) = self.nodes.get(id) {
                pending.extend(node.hierarchy.children.iter().copied());
            }
        }
        affected.sort_unstable();
        affected.dedup();
        // A moved box keeps the bounds it was handed only where they are its
        // layout box, and its parent's are too: what a transform or a scroll
        // above makes of a box is not moved with it to the bit.
        let kept = moved_seeds
            .iter()
            .copied()
            .filter(|id| affected.binary_search(id).is_err())
            .collect::<Vec<_>>();
        if !tree_plain && !kept.is_empty() {
            let reprojected = kept
                .iter()
                .copied()
                .filter(|&id| !self.keeps_relative_bounds(id, &mut memo))
                .collect::<Vec<_>>();
            if !reprojected.is_empty() {
                affected.extend(reprojected);
                affected.sort_unstable();
            }
        }
        #[cfg(any(test, feature = "layout-verify"))]
        self.check_kept_accessibility(&moved_seeds, &affected);
        let mut removed = work.accessibility_removals.clone();
        let plain = affected.len() >= 64 && tree_plain;
        let mut updated = Vec::with_capacity(affected.len());
        for id in affected {
            let entries = self.project_accessibility_entries(id, plain, true, &mut memo);
            if entries.is_empty() {
                if self.nodes.contains(id) {
                    removed.push(id);
                }
            } else {
                updated.extend(entries);
            }
        }
        removed.sort_unstable();
        removed.dedup();
        #[cfg(any(test, feature = "layout-verify"))]
        self.accessibility_shipped.record(&updated, &removed);
        AccessibilityDelta {
            generation: work.generation,
            updated,
            removed,
        }
    }

    /// Whether `root`, a box layout moved, holds its own box upright in its
    /// accessible space ([`AccessibleSpace::unit`]: every box from the page
    /// down to it is placed by its offset, however it turns, or stands
    /// upright in the viewport), and is no modal frame: then of what it
    /// holds only the pinned boxes ([`NodeRecord::pins_accessibility`]) and
    /// what is under those can project differently
    /// ([`Self::reach_moved_pins`]). Motion overlays can transform any box,
    /// and a clip visual cuts every box under it, so either leaves the walk.
    fn moved_root_hands_offsets(&self, root: StableNodeId, memo: &mut ProjectionMemo) -> bool {
        if self.clip_visuals != 0
            || !self.presentation.is_empty()
            || !self.layout_length_tracks.is_empty()
            || self.is_modal_frame(root)
        {
            return false;
        }
        // Every box from the page down only translating and scaling by a
        // positive factor: the root's box is upright wherever its space
        // comes from. Read off the records, without the memo, for the
        // thousand rows a reflow moves.
        let upright_chain = std::iter::successors(self.nodes.get(root), |record| {
            record
                .hierarchy
                .parent
                .and_then(|parent| self.nodes.get(parent))
        })
        .all(|record| {
            record.hands_accessibility_offsets()
                && record
                    .resolved_layout
                    .transform
                    .is_none_or(|turn| upright(turn.a, turn.b, turn.c, turn.d))
        });
        #[cfg(any(test, feature = "layout-verify"))]
        if upright_chain {
            assert!(
                self.accessible_space(root, memo)
                    .is_some_and(|space| space.unit.is_some()),
                "{root:?}: an upright chain holds its box upright"
            );
        }
        upright_chain
            || self
                .accessible_space(root, memo)
                .is_some_and(|space| space.unit.is_some())
    }

    fn is_modal_frame(&self, id: StableNodeId) -> bool {
        self.nodes.has_visuals()
            && matches!(
                self.nodes.visual(id),
                Some(StandardVisual::ModalFrame { .. })
            )
    }

    /// What under `roots` -- boxes layout moved that hand their subtrees
    /// offsets ([`Self::moved_root_hands_offsets`]) -- projects again
    /// although nothing marked it, found from the pinned boxes alone. A box
    /// on a path from such a root that nothing pins is placed by its offset,
    /// hands its children offsets and cuts nothing: it, and every box under
    /// it that nothing pins, kept its bounds relative to its parent to the
    /// bit ([`Self::offset_placed`]). So each pin under a root with no box
    /// that changes that between them is where
    /// [`Self::reach_moved_subtree`]'s walk would first find anything; the
    /// walk starts there instead. A pinned box on the way that still hands
    /// offsets (a scale, a sticky offset, a box measured from a block that
    /// moved with the root) is passed over: what it holds is placed from it.
    /// A pin below one that hands its children something else (a rotation,
    /// a 3D context, a clip or a modal frame), or below one that stayed
    /// where it was (measured from a frame above the root), is the upper
    /// one's walk to reach, or nothing's; a pin with another moved box
    /// between it and the root is that box's. O(pins × depth), not
    /// O(boxes moved).
    fn reach_moved_pins(
        &self,
        roots: &[StableNodeId],
        layout_moved: &[StableNodeId],
        affected: &mut Vec<StableNodeId>,
        pending: &mut Vec<StableNodeId>,
        memo: &mut ProjectionMemo,
    ) {
        let mut open = Vec::new();
        // The blocks boxes on the way up are measured from, until the walk
        // passes them: one it does not pass before the root stayed put.
        let mut frames = Vec::new();
        for pin in self.nodes.accessibility_pins() {
            let Some(record) = self.nodes.get(pin) else {
                continue;
            };
            if !record.pins_accessibility(self.nodes.visual(pin)) {
                continue;
            }
            // The nearest box layout moved above the pin, unless a box on
            // the way stayed where it was, or hands what it holds anything
            // but offsets.
            frames.clear();
            let mut at = record.hierarchy.parent;
            let root = loop {
                let Some(id) = at else {
                    break None;
                };
                frames.retain(|&block| block != id);
                if layout_moved.binary_search(&id).is_ok() {
                    break frames.is_empty().then_some(id);
                }
                let Some(above) = self.nodes.get(id) else {
                    break None;
                };
                match above.layout_frame {
                    crate::LayoutFrame::Parent => {}
                    crate::LayoutFrame::Block(block) => frames.push(block),
                    crate::LayoutFrame::Page => break None,
                }
                // A pin on the way placed from its viewport box and not
                // upright there walks what it holds itself, this pin among
                // them: passing it only finds this pin twice.
                if above.resolved_layout.clips_overflow()
                    || !above.hands_accessibility_offsets()
                    || self.is_modal_frame(id)
                {
                    break None;
                }
                at = above.hierarchy.parent;
            };
            let Some(root) = root.filter(|root| roots.binary_search(root).is_ok()) else {
                continue;
            };
            open.push((pin, true));
            self.reach_moved_subtree(
                root,
                &mut open,
                layout_moved,
                false,
                affected,
                pending,
                memo,
            );
        }
    }

    /// What under `root`, a box layout moved, projects again although nothing
    /// marked it, into `affected`, and the boxes whose whole subtree does
    /// into `pending`, walking from the boxes in `open` (each with whether
    /// its parent's accessible box is its layout box) or, when `open` is
    /// empty, from `root`'s children. Everything under `root` moved with it
    /// and kept its offset from its parent but for:
    ///
    /// - a box measured from a frame that did not move (a `fixed` box, an
    ///   absolute one whose containing block is above `root`): it stayed on
    ///   the page, so its offset from its parent changed;
    /// - a box that clips what it holds, which is where the viewport-space
    ///   clips of its subtree come from (and a clip visual anywhere): its
    ///   subtree projects again, as a non-carrying seed's does;
    /// - a box not placed by its offset ([`Self::offset_placed`]: its own
    ///   transform or sticky offset, a rotation or a 3D context above, a
    ///   virtual or modal visual): what moved it does not move it by the
    ///   same offset to the bit ([`Self::keeps_relative_bounds`]).
    ///
    /// The walk stops at a box layout moved itself (in `layout_moved`): it
    /// is walked as a root of its own. Below a box that hands its children
    /// offsets it goes on only when `descend_identity` is set: what is there
    /// that can project again is pinned, and found from the pins
    /// ([`Self::reach_moved_pins`]); below one that does not, it checks each
    /// box. The walk reads each box's record and nothing else on the common
    /// path -- no box, no memo.
    #[allow(clippy::too_many_arguments)]
    fn reach_moved_subtree(
        &self,
        root: StableNodeId,
        open: &mut Vec<(StableNodeId, bool)>,
        layout_moved: &[StableNodeId],
        descend_identity: bool,
        affected: &mut Vec<StableNodeId>,
        pending: &mut Vec<StableNodeId>,
        memo: &mut ProjectionMemo,
    ) {
        let Some(record) = self.nodes.get(root) else {
            open.clear();
            return;
        };
        let special = |id: StableNodeId| {
            self.nodes.has_visuals()
                && crate::store::projects_virtual_accessibility(self.nodes.visual(id))
        };
        // Whether `block` moved with `root`: it is `root` or under it.
        let inside = |block: StableNodeId| {
            std::iter::successors(Some(block), |&at| self.parent_id(at)).any(|at| at == root)
        };
        if open.is_empty() {
            // Whether `root` holds its box upright in its space: then the
            // bounds of what it holds come from their offsets. Motion
            // overlays can transform any box, so they leave only the exact
            // check; a clip visual sends every box to `pending` below.
            let parent_offsets = self.presentation.is_empty()
                && self.layout_length_tracks.is_empty()
                && !self.is_modal_frame(root)
                && self
                    .accessible_space(root, memo)
                    .is_some_and(|space| space.unit.is_some());
            open.extend(
                record
                    .hierarchy
                    .children
                    .iter()
                    .map(|&child| (child, parent_offsets)),
            );
        }
        while let Some((id, parent_offsets)) = open.pop() {
            let Some(record) = self.nodes.get(id) else {
                continue;
            };
            let follows = match record.layout_frame {
                crate::LayoutFrame::Parent => true,
                crate::LayoutFrame::Block(block) => inside(block),
                crate::LayoutFrame::Page => false,
            };
            if !follows {
                affected.push(id);
                continue;
            }
            if layout_moved.binary_search(&id).is_ok() {
                continue;
            }
            if self.clip_visuals != 0 || record.resolved_layout.clips_overflow() {
                pending.push(id);
                continue;
            }
            let keeps = if parent_offsets && record.placed_by_accessibility_offset() && !special(id)
            {
                true
            } else {
                self.keeps_relative_bounds(id, memo)
            };
            if !keeps {
                affected.push(id);
            }
            // A box placed by its offset under one that holds its own box
            // upright holds its own upright too, however it turns; one
            // placed from its viewport box, only when that is upright.
            let handed = parent_offsets
                && record.hands_accessibility_offsets()
                && !self.is_modal_frame(id)
                && (record.placed_by_accessibility_offset()
                    || self
                        .accessible_space(id, memo)
                        .is_some_and(|space| space.unit.is_some()));
            if handed && !descend_identity {
                continue;
            }
            open.extend(
                record
                    .hierarchy
                    .children
                    .iter()
                    .map(|&child| (child, handed)),
            );
        }
    }

    /// Whether `id`'s accessible bounds come from its offset
    /// ([`Self::offset_placed`]), or are its layout box and its parent's
    /// are too, so that a box that kept its offset from its parent keeps
    /// them, to the bit. Virtual nodes are placed from its geometry, and a
    /// clip visual cuts the boxes under it: both project again.
    fn keeps_relative_bounds(&self, id: StableNodeId, memo: &mut ProjectionMemo) -> bool {
        if self.clip_visuals != 0 {
            return false;
        }
        if self.nodes.has_visuals()
            && crate::store::projects_virtual_accessibility(self.nodes.visual(id))
        {
            return false;
        }
        if let Some(record) = self.nodes.get(id)
            && self.offset_placed(id, record, memo)
        {
            return true;
        }
        let parent = self.nodes.get(id).and_then(|node| node.hierarchy.parent);
        if parent.is_some_and(|parent| {
            self.accessible_space(parent, memo)
                .is_some_and(|space| space.turned)
        }) {
            return false;
        }
        [Some(id), parent].into_iter().flatten().all(|node| {
            !(self.nodes.has_visuals()
                && matches!(
                    self.nodes.visual(node),
                    Some(StandardVisual::ModalFrame { .. })
                ))
                && (self.axis_aligned(node, memo)
                    || self.visible_accessibility_bounds(node, memo)
                        == self.component_layout_box(node))
        })
    }
}

/// What the projections handed out, kept where a delta's narrowing is
/// checked: a node a delta leaves alone must still be what it was handed out
/// as. Only small worlds are kept, as the hit index's own check.
#[cfg(any(test, feature = "layout-verify"))]
#[derive(Default)]
pub(crate) struct ShippedAccessibility(
    std::sync::Mutex<hashbrown::HashMap<StableNodeId, AccessibilityNode>>,
);

#[cfg(any(test, feature = "layout-verify"))]
const SHIPPED_ACCESSIBILITY_LIMIT: usize = 4096;

#[cfg(any(test, feature = "layout-verify"))]
impl ShippedAccessibility {
    fn record(&self, updated: &[AccessibilityNode], removed: &[StableNodeId]) {
        let mut shipped = self.0.lock().unwrap_or_else(|error| error.into_inner());
        for id in removed {
            shipped.remove(id);
        }
        if shipped.len() + updated.len() > SHIPPED_ACCESSIBILITY_LIMIT {
            shipped.clear();
            return;
        }
        for node in updated {
            shipped.insert(node.id, node.clone());
        }
    }
}

#[cfg(any(test, feature = "layout-verify"))]
impl UiWorld {
    /// `placed`, what [`Self::offset_placed`] makes of `entry` from its
    /// offset, puts its box where its layout box's turned corners stand in
    /// the viewport, up to the rounding of the page and viewport sums the
    /// offset leaves out: its parent's space taken to the viewport
    /// ([`Self::accessible_space_to_viewport`]), then its bounds' move and
    /// its transform, take the corners of its own box to the corners its
    /// whole chain of transforms takes its page box to.
    fn check_offset_bounds(
        &self,
        entry: &AccessibilityNode,
        parent: StableNodeId,
        (bounds, transform): (LayoutBox, Option<nana_ui_core::PaintTransform>),
        memo: &mut ProjectionMemo,
    ) {
        let above = self
            .accessible_space_to_viewport(parent, memo)
            .expect("a placed box's parent space reaches the viewport");
        let mut own = transform.unwrap_or_default();
        own.e += bounds.x;
        own.f += bounds.y;
        let [a, b, c, d, e, f] = above;
        let to_viewport = nana_ui_core::PaintTransform { a, b, c, d, e, f }.then(own);
        let placed = |x: f32, y: f32| {
            [
                to_viewport.a * x + to_viewport.c * y + to_viewport.e,
                to_viewport.b * x + to_viewport.d * y + to_viewport.f,
            ]
        };
        self.accessibility_ancestor_transform(entry.id, memo)
            .expect("a placed box has a transform");
        let ([ca, cb, cc, cd, ce, cf], _) = memo.transforms[&entry.id].own;
        let page = self
            .component_layout_box(entry.id)
            .expect("a placed box has a box");
        let cold = |x: f32, y: f32| [ca * x + cc * y + ce, cb * x + cd * y + cf];
        let (w, h) = (bounds.width, bounds.height);
        let got = [
            placed(0.0, 0.0),
            placed(w, 0.0),
            placed(0.0, h),
            placed(w, h),
        ];
        let want = [
            cold(page.x, page.y),
            cold(page.x + page.width, page.y),
            cold(page.x, page.y + page.height),
            cold(page.x + page.width, page.y + page.height),
        ];
        let scale = 1.0
            + want
                .iter()
                .chain(&got)
                .flatten()
                .map(|value| value.abs())
                .fold(0.0, f32::max)
                * 4.0
            + page.x.abs()
            + page.y.abs();
        assert!(
            got.iter()
                .flatten()
                .zip(want.iter().flatten())
                .all(|(got, want)| (got - want).abs() <= 2e-5 * scale),
            "{:?}: corners placed from its offset {got:?} are not where its turned box stands, {want:?}",
            entry.id
        );
    }

    /// The pins found under `roots` are what walking every box under them
    /// finds, and every box that pins is noted as a pin. Small worlds only,
    /// as the other checks.
    fn check_moved_pins(
        &self,
        roots: &[StableNodeId],
        layout_moved: &[StableNodeId],
        affected: &[StableNodeId],
        pending: &[StableNodeId],
    ) {
        if self.len() > SHIPPED_ACCESSIBILITY_LIMIT {
            return;
        }
        let noted: hashbrown::HashSet<StableNodeId> = self.nodes.accessibility_pins().collect();
        for id in self.nodes.keys() {
            let record = self.nodes.get(id).expect("a key has a record");
            assert!(
                !record.pins_accessibility(self.nodes.visual(id)) || noted.contains(&id),
                "{id:?} pins accessibility but is not noted as a pin"
            );
        }
        let mut memo = ProjectionMemo::default();
        let (mut walked, mut walked_subtrees, mut open) = (Vec::new(), Vec::new(), Vec::new());
        for &root in roots {
            self.reach_moved_subtree(
                root,
                &mut open,
                layout_moved,
                true,
                &mut walked,
                &mut walked_subtrees,
                &mut memo,
            );
        }
        let sorted = |ids: &[StableNodeId]| {
            let mut ids = ids.to_vec();
            ids.sort_unstable();
            ids.dedup();
            ids
        };
        assert_eq!(
            sorted(affected),
            sorted(&walked),
            "the pins under moved boxes found other boxes to project than a walk"
        );
        assert_eq!(
            sorted(pending),
            sorted(&walked_subtrees),
            "the pins under moved boxes found other subtrees to project than a walk"
        );
    }
}

#[cfg(any(test, feature = "layout-verify"))]
impl UiWorld {
    /// Every node under a moved box the delta does not project is what the
    /// projections last handed out: projected from scratch, it is the same.
    fn check_kept_accessibility(&self, moved_seeds: &[StableNodeId], affected: &[StableNodeId]) {
        if moved_seeds.is_empty() || self.len() > SHIPPED_ACCESSIBILITY_LIMIT {
            return;
        }
        let shipped = self
            .accessibility_shipped
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut memo = ProjectionMemo::default();
        let mut pending = moved_seeds.to_vec();
        let mut visited = hashbrown::HashSet::new();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            if let Some(node) = self.nodes.get(id) {
                pending.extend(node.hierarchy.children.iter().copied());
            }
            if affected.binary_search(&id).is_ok() {
                continue;
            }
            let fresh = self.project_accessibility_entries(id, false, true, &mut memo);
            match fresh.first() {
                Some(fresh) => {
                    if let Some(held) = shipped.get(&id) {
                        assert_eq!(
                            held, fresh,
                            "{id:?}: a delta kept an accessibility node that projects differently now"
                        );
                    }
                }
                None => assert!(
                    !shipped.contains_key(&id),
                    "{id:?}: a delta kept an accessibility node that no longer projects"
                ),
            }
        }
    }
}

impl UiWorld {
    /// The row of `menu` that holds accessibility `slot` now.
    /// A select field's open menu counts as its menu: its options are rows.
    pub(crate) fn virtual_menu_row_index(&self, menu: StableNodeId, slot: u16) -> Option<usize> {
        match self.nodes.visual(menu) {
            Some(StandardVisual::MenuSurface { rows, .. })
            | Some(StandardVisual::Select {
                opened: true,
                options: rows,
                ..
            }) => crate::menu_row_index(rows, slot),
            _ => None,
        }
    }

    /// The visible row of `tree` that holds accessibility `slot` now.
    pub(crate) fn virtual_tree_row(
        &self,
        tree: StableNodeId,
        slot: u16,
    ) -> Option<crate::tree_view::TreeRowData> {
        let index = usize::from(slot).checked_sub(1)?;
        match self.nodes.visual(tree) {
            Some(StandardVisual::TreeView { rows, .. }) => rows.get(index).cloned(),
            _ => None,
        }
    }

    /// The visible row of `tree` in accessibility `slot`, when the tree has
    /// toggle `column`.
    pub(crate) fn virtual_tree_toggle_row(
        &self,
        tree: StableNodeId,
        slot: u16,
        column: usize,
    ) -> Option<crate::tree_view::TreeRowData> {
        let index = usize::from(slot).checked_sub(1)?;
        match self.nodes.visual(tree) {
            Some(StandardVisual::TreeView { rows, columns, .. }) if column < columns.len() => {
                rows.get(index).cloned()
            }
            _ => None,
        }
    }

    /// Project only accessibility nodes named by scheduled dirty work.
    pub fn project_accessibility_nodes(&self, ids: &[StableNodeId]) -> Vec<AccessibilityNode> {
        let projected = self.project_nodes(ids, true);
        #[cfg(any(test, feature = "layout-verify"))]
        self.accessibility_shipped.record(&projected, &[]);
        projected
    }

    fn project_nodes(&self, ids: &[StableNodeId], relative: bool) -> Vec<AccessibilityNode> {
        let mut memo = ProjectionMemo::default();
        let plain = ids.len() >= 64 && self.accessibility_tree_is_plain();
        let mut projected = Vec::with_capacity(ids.len());
        for &id in ids {
            projected.extend(self.project_accessibility_entries(id, plain, relative, &mut memo));
        }
        projected
    }
}

impl UiWorld {
    /// Project visible semantics and their neutral structural ancestors from
    /// the same retained authority, preserving a connected accessibility tree.
    pub fn project_accessibility(&self, document: DocumentId) -> Vec<AccessibilityNode> {
        self.project_accessibility_nodes(&self.document_order(document))
    }

    /// [`Self::project_accessibility`] with every node's `bounds` where it
    /// stands in the viewport rather than relative to its parent: for
    /// automation and tests that find a node on screen. A platform tree is
    /// built from [`Self::project_accessibility`] and its deltas, never
    /// from this.
    pub fn project_accessibility_in_viewport(
        &self,
        document: DocumentId,
    ) -> Vec<AccessibilityNode> {
        self.project_nodes(&self.document_order(document), false)
    }

    /// [`Self::project_accessibility_nodes`] with viewport bounds, as
    /// [`Self::project_accessibility_in_viewport`].
    pub fn project_accessibility_nodes_in_viewport(
        &self,
        ids: &[StableNodeId],
    ) -> Vec<AccessibilityNode> {
        self.project_nodes(ids, false)
    }
}
