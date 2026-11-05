//! Component text primitives: editor scroll, select handles, text spans and
//! icon alignment beside text.

use super::*;

/// What of where a node stands its build read
/// ([`UiScene::icon_y_aligned_to_adjacent_text`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PlacementRead {
    None,
    /// Its own offset in its parent's coordinates.
    Offset,
    /// Where it stands on the page, or where another node does.
    Page,
}

/// Draws what moves with an editor's text along x under its scroll's
/// translation, each box less it: the painter snaps that translation, so the
/// value keeps its glyphs and the caret, selection and marks stay on them.
#[derive(Clone, Copy)]
pub(super) struct EditorScroll(pub(super) Option<TextInputScroll>);

impl EditorScroll {
    pub(super) fn transform(self, transform: AffineTransform) -> AffineTransform {
        match self.0 {
            Some(scroll) => transform.then(AffineTransform::from_matrix([
                1.0,
                0.0,
                0.0,
                1.0,
                scroll.offset_x,
                0.0,
            ])),
            None => transform,
        }
    }

    /// A box that moves with the text, in the translated space.
    pub(super) fn rect(self, bounds: LayoutBox) -> SceneRect {
        let mut rect = scene_rect(bounds);
        if let Some(scroll) = self.0 {
            rect.x -= scroll.offset_x;
        }
        rect
    }

    /// Text over the value: a diagnostic's message, an atom's label.
    pub(super) fn text(self, mut primitive: ScenePrimitive) -> ScenePrimitive {
        if let Some(scroll) = self.0 {
            primitive.bounds.x -= scroll.offset_x;
            primitive.transform = self.transform(primitive.transform);
        }
        primitive
    }

    /// The value, at its exact unscrolled x.
    pub(super) fn value(self, primitive: ScenePrimitive) -> ScenePrimitive {
        let mut primitive = self.text(primitive);
        if let Some(scroll) = self.0 {
            primitive.bounds.x = scroll.text_x;
        }
        primitive
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_select_handle(
    emit: &mut impl FnMut(ScenePrimitive),
    id: StableNodeId,
    handle: &LayoutBox,
    color: [f32; 4],
    transform: AffineTransform,
    clips: &Arc<[ClipRegion]>,
    opacity: f32,
    z_index: i32,
    document_order: usize,
) {
    let center_x = handle.x + handle.width / 2.0;
    let center_y = handle.y + handle.height / 2.0;
    let widths = [8.0, 6.0, 4.0, 2.0];
    for (index, width) in widths.iter().copied().enumerate() {
        emit(visual_quad(
            &VisualPrimitiveContext {
                node: id,
                transform,
                clips,
                opacity,
                z_index,
                document_order,
            },
            3 + index as u64,
            SceneRect {
                x: center_x - width / 2.0,
                y: center_y - 1.5 + index as f32,
                width,
                height: 1.0,
            },
            VisualQuadStyle::solid(color),
        ));
    }
}

pub(super) fn component_geometry_owns_text(geometry: Option<&ComponentGeometry>) -> bool {
    matches!(
        geometry,
        Some(
            ComponentGeometry::Button { .. }
                | ComponentGeometry::TextInput { .. }
                | ComponentGeometry::Switch { .. }
                | ComponentGeometry::Range { .. }
                | ComponentGeometry::Card { .. }
                | ComponentGeometry::StatusBadge { .. }
                | ComponentGeometry::ValidationMessage { .. }
                | ComponentGeometry::EmptyState { .. }
                | ComponentGeometry::PathText { .. }
                | ComponentGeometry::LabeledValue { .. }
                | ComponentGeometry::SelectionOption { .. }
                | ComponentGeometry::ModalFrame { .. }
                | ComponentGeometry::Progress { .. }
                | ComponentGeometry::FormField { .. }
                | ComponentGeometry::Select { .. }
                | ComponentGeometry::ActionMenuItem { .. }
                | ComponentGeometry::MenuSurface { .. }
                | ComponentGeometry::TreeView { .. }
                | ComponentGeometry::CommandPalette { .. }
                | ComponentGeometry::CalendarHeatmap { .. }
                | ComponentGeometry::ReorderList { .. }
                | ComponentGeometry::NativeMarkdown { .. }
                | ComponentGeometry::SelectableRichText { .. }
                | ComponentGeometry::GraphCanvas { .. }
                | ComponentGeometry::ImageViewer { .. }
                | ComponentGeometry::KeyCaptureLayer { .. }
                | ComponentGeometry::KeymapLayer { .. }
        )
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn component_text_primitive(
    id: StableNodeId,
    slot: u64,
    region: &ComponentTextRegion,
    horizontal_alignment: TextHorizontalAlignment,
    ellipsis: bool,
    node: &ExtractedNode,
    transform: AffineTransform,
    clips: Arc<[ClipRegion]>,
    opacity: f32,
    document_order: usize,
) -> ScenePrimitive {
    let paint_color = node
        .style
        .paint_colors
        .color
        .filter(|paint| region.color.is_none_or(|color| color == paint.to_srgb()));
    let multiline = matches!(
        node.component_geometry.as_deref(),
        Some(ComponentGeometry::TextInput {
            multiline: true,
            ..
        })
    );
    // Validation feedback is measured as an ordinary paragraph. Its paint
    // must keep that wrapping/overflow intent, including explicit overrides,
    // instead of taking the single-line chrome default from its caller.
    let validation = matches!(
        node.standard_visual,
        Some(StandardVisual::ValidationMessage { .. })
    );
    let card_title = matches!(node.standard_visual, Some(StandardVisual::Card { .. }));
    let (ellipsis, max_lines) = if card_title {
        (true, Some(1))
    } else if validation {
        (
            node.source_style.layout.uses_text_ellipsis(),
            node.source_style.layout.resolved_line_clamp(),
        )
    } else {
        (ellipsis, None)
    };
    // A modal's and an empty state's own copy is measured at the font's
    // natural line height whatever the node inherits (`shape_modal_text`,
    // `shape_empty_state_text`); painting it at another line height runs it
    // past the part layout sized for it, and the last line is elided.
    let intrinsic_copy = matches!(
        node.standard_visual,
        Some(StandardVisual::EmptyState { .. } | StandardVisual::ModalFrame { .. })
    );
    let intrinsic_multiline = intrinsic_copy
        || matches!(
            node.component_geometry.as_deref(),
            Some(
                ComponentGeometry::NativeMarkdown { .. }
                    | ComponentGeometry::SelectableRichText { .. }
            )
        )
        || (validation && node.source_style.layout.text_wraps());
    let advanced_content = slot == 2
        && matches!(
            node.standard_visual,
            Some(
                StandardVisual::Button { .. }
                    | StandardVisual::ActionMenuItem { .. }
                    | StandardVisual::Card { .. }
            )
        )
        && (card_title || node.rich_text.is_some() || !node.children.is_empty())
        && node
            .text
            .as_ref()
            .is_some_and(|text| text.value.as_str() == region.content.as_ref());
    let retained = advanced_content
        .then_some(node.text_layout.as_ref())
        .flatten();
    let mut text_bounds = scene_rect(region.bounds);
    if let Some(retained) = retained
        && !retained.layout.is_vertical()
        && retained.layout.constraints.max_width_px != Some(region.bounds.width)
    {
        text_bounds.x -= retained.layout.bounds.x;
        text_bounds.width = retained
            .layout
            .constraints
            .max_width_px
            .unwrap_or(text_bounds.width);
    }
    // An ellipsis can be wider than a tiny title budget. Keep that glyph and
    // its coverage inside the published Header part, clear of the spinner.
    let clips = if card_title
        && retained.is_none_or(|retained| retained.layout.overflow != Default::default())
    {
        let mut title_clips = clips.to_vec();
        title_clips.push(ClipRegion::axis_aligned(
            scene_rect(region.bounds),
            transform,
        ));
        title_clips.into()
    } else {
        clips
    };
    ScenePrimitive {
        id: PrimitiveId { node: id, slot },
        node: id,
        bounds: text_bounds,
        transform,
        clips,
        opacity,
        z_index: node.z_index,
        document_order,
        kind: ScenePrimitiveKind::Text {
            content: region.content.clone(),
            color: slot_color(
                paint_color,
                region
                    .color
                    .or_else(|| {
                        node.style
                            .paint_colors
                            .color
                            .map(nana_ui_core::PaintColor::to_srgb)
                    })
                    .or(node.style.color),
            ),
            size: region.font_size,
            weight: region.font_weight,
            family: node.style.font_family.as_deref().map(str::to_owned),
            line_height: if card_title
                || matches!(
                    node.component_geometry.as_deref(),
                    Some(ComponentGeometry::Switch { .. } | ComponentGeometry::ListItem { .. })
                ) {
                Some(LineHeightSpec::Absolute(region.bounds.height))
            } else if intrinsic_copy {
                None
            } else {
                node.style.line_height
            },
            letter_spacing: node.style.letter_spacing,
            wrap: multiline || intrinsic_multiline,
            ellipsis,
            max_lines,
            shaping: if node.editable {
                TextShaping::Advanced
            } else {
                TextShaping::Auto
            },
            horizontal_alignment,
            vertical_alignment: if multiline || intrinsic_multiline {
                TextVerticalAlignment::Top
            } else {
                TextVerticalAlignment::Center
            },
            spans: scene_text_spans(node, Some(region), region.content.as_ref()),
            text_shadow: node.source_style.layout.paint.text_shadow,
            underline: node
                .source_style
                .layout
                .text_decoration
                .is_some_and(|d| d.underline),
            line_through: node
                .source_style
                .layout
                .text_decoration
                .is_some_and(|d| d.line_through),
            font_features: node
                .source_style
                .layout
                .font_features
                .clone()
                .unwrap_or_default(),
            italic: node.style.italic,
            wrap_break: node.source_style.layout.text_wrap_break(),
            opentype: SceneTextOpenType {
                // What this node was measured with. An editor's value keeps
                // the newlines the user typed whatever `white-space` says, and
                // a renderer that folds them paints one line where six were
                // measured.
                preserve_lines: !card_title && node.text_preserve_lines,
                ..SceneTextOpenType::from_computed(&node.style)
            },
            layout: retained.cloned(),
            rich: advanced_content
                .then(|| {
                    SceneRichPaint::from_base(
                        css_text_effects(&node.source_style.layout),
                        node.rich_text.as_ref(),
                    )
                })
                .flatten(),
            presentation: None,
        },
    }
}

pub(super) fn surface_corner_radii(
    style: &nana_ui_core::LayoutStyle,
    width: f32,
    height: f32,
) -> [f32; 4] {
    let mut radii = style.resolved_border_radii(width, height);
    if let Some(ClipPath::Inset(inset)) = style.paint.clip_path.as_ref() {
        let round = inset.resolve_round(width, height);
        if round > 0.0 {
            radii = radii.map(|radius| radius.max(round));
        }
    }
    radii
}

pub(super) fn scene_text_spans(
    node: &ExtractedNode,
    region: Option<&ComponentTextRegion>,
    content: &str,
) -> Vec<SceneTextSpan> {
    if node.text_spans.is_empty() && node.rich_text.is_none() {
        return Vec::new();
    }
    // TextInput 主文本区域：span 由 extraction 按 TextInput 呈现内容
    // （折叠态经显示视图重投后的显示串）空间产出，直接按 content 校验
    // 边界。其余区域（行号标签、钉住行、Select 标题、普通文本节点等）
    // 不承载该空间，维持值空间等值守卫——内容与 span 空间不一致时整批
    // 丢弃。
    let editor_main_text = region.is_some_and(|region| {
        matches!(
            node.component_geometry.as_deref(),
            Some(ComponentGeometry::TextInput { text, .. }) if std::ptr::eq(text, region)
        )
    });
    if !editor_main_text {
        let Some(source) = node.text.as_ref() else {
            return Vec::new();
        };
        if source.value != content {
            return Vec::new();
        }
    }
    let spans = node
        .text_spans
        .iter()
        .filter(|span| {
            span.start < span.end
                && span.end <= content.len()
                && content.is_char_boundary(span.start)
                && content.is_char_boundary(span.end)
        })
        .map(|span| SceneTextSpan {
            start: span.start,
            end: span.end,
            color: slot_color(span.paint_color, Some(span.color))
                .expect("a span has a resolved colour"),
        })
        .collect();
    rich_fill_spans(content, node.rich_text.as_ref(), spans)
}

/// 锚定浮层文本的共享绘制原语：与编辑器文本同族，但不换行（浮层行高
/// 固定），超宽用省略号截断。
#[allow(clippy::too_many_arguments)]
pub(super) fn overlay_text_primitive(
    id: StableNodeId,
    slot: u64,
    region: &ComponentTextRegion,
    horizontal_alignment: TextHorizontalAlignment,
    node: &ExtractedNode,
    transform: AffineTransform,
    clips: Arc<[ClipRegion]>,
    opacity: f32,
    document_order: usize,
) -> ScenePrimitive {
    let paint_color = node
        .style
        .paint_colors
        .color
        .filter(|paint| region.color.is_none_or(|color| color == paint.to_srgb()));
    ScenePrimitive {
        id: PrimitiveId { node: id, slot },
        node: id,
        bounds: scene_rect(region.bounds),
        transform,
        clips,
        opacity,
        z_index: node.z_index,
        document_order,
        kind: ScenePrimitiveKind::Text {
            content: region.content.clone(),
            color: slot_color(
                paint_color,
                region
                    .color
                    .or_else(|| {
                        node.style
                            .paint_colors
                            .color
                            .map(nana_ui_core::PaintColor::to_srgb)
                    })
                    .or(node.style.color),
            ),
            size: region.font_size,
            weight: region.font_weight,
            family: node.style.font_family.as_deref().map(str::to_owned),
            line_height: node.style.line_height,
            letter_spacing: node.style.letter_spacing,
            wrap: false,
            ellipsis: true,
            max_lines: None,
            shaping: if node.editable {
                TextShaping::Advanced
            } else {
                TextShaping::Auto
            },
            horizontal_alignment,
            vertical_alignment: TextVerticalAlignment::Center,
            spans: Vec::new(),
            text_shadow: None,
            underline: false,
            line_through: false,
            font_features: Vec::new(),
            italic: false,
            wrap_break: nana_ui_core::TextWrapBreak::default(),
            opentype: SceneTextOpenType::default(),
            layout: None,
            rich: None,
            presentation: None,
        },
    }
}

impl UiScene {
    /// Skip duplicate `#text` children when the host already paints that
    /// string. Independent element children, such as list rows inside a Card,
    /// keep their own labels.
    pub(super) fn parent_already_paints_text(&self, node: &ExtractedNode) -> bool {
        if node.layout_result.as_ref().is_some_and(|result| {
            result.participation == Some(nana_ui_runtime::ParticipationKind::AtomicInline)
        }) {
            return false;
        }

        if !matches!(&*node.kind, NodeKind::Text) {
            return false;
        }
        let Some(parent) = node.parent.and_then(|id| self.nodes.get(&id)) else {
            return false;
        };
        if let Some(ComponentGeometry::Card { title, .. }) = parent.component_geometry.as_deref() {
            return title.as_ref().is_some_and(|title| {
                node.text
                    .as_ref()
                    .is_some_and(|text| text.value == title.content.as_ref())
            });
        }
        component_geometry_owns_text(parent.component_geometry.as_deref())
            || parent
                .text
                .as_ref()
                .is_some_and(|text| !text.value.is_empty())
    }

    /// Compact leading glyphs share the parent (or own) text line-box center.
    ///
    /// The second half of the answer is what of where the node stands it
    /// read: nothing, its own offset in its parent's coordinates (it is built
    /// again when it is the root of a move, [`Self::offset_bound`]), or the
    /// page (when it or a box above it moves, [`Self::origin_bound`]).
    pub(super) fn icon_y_aligned_to_adjacent_text(
        &self,
        node: &ExtractedNode,
        icon_bounds: SceneRect,
        extent: f32,
    ) -> (f32, PlacementRead) {
        let geometric = icon_bounds.y + (icon_bounds.height - extent) / 2.0;
        if node.layout.height > extent + 0.5 || node.layout.width > extent + 0.5 {
            return (geometric, PlacementRead::None);
        }
        let host = if node
            .text
            .as_ref()
            .is_some_and(|text| !text.value.is_empty())
        {
            node
        } else {
            match node.parent.and_then(|id| self.nodes.get(&id)) {
                Some(parent)
                    if parent
                        .text
                        .as_ref()
                        .is_some_and(|text| !text.value.is_empty())
                        || matches!(
                            parent.standard_visual.as_ref(),
                            Some(StandardVisual::ListItem { .. })
                        ) =>
                {
                    parent
                }
                _ => return (geometric, PlacementRead::None),
            }
        };
        // In the space the host's record is in.
        let mut text_box = match host.component_geometry.as_deref() {
            Some(ComponentGeometry::ListItem {
                content: Some(content),
                ..
            }) => *content,
            _ => Self::record_box(host),
        };
        // Into this node's: a child measured from its parent stands at its
        // local box in the parent's coordinates, wherever the two stand.
        let mut read = PlacementRead::None;
        if host.id != node.id {
            if node.layout_frame == nana_ui_runtime::LayoutFrame::Parent
                && !record_on_page(host)
                && !record_on_page(node)
            {
                text_box.y -= node.local_layout.y;
                read = PlacementRead::Offset;
            } else {
                text_box.y += self.paint_origin(host)[1] - self.paint_origin(node)[1];
                read = PlacementRead::Page;
            }
        }
        let centered = matches!(
            host.source_style.text_vertical_alignment,
            TextVerticalAlignment::Center
        );
        (
            icon_y_on_text_glyph_center(
                text_box.y,
                text_box.height,
                host.style.font_size,
                host.style.line_height,
                centered,
                extent,
            ),
            read,
        )
    }
}
