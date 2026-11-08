//! Shared visual building blocks: quads, images, contrast rings, focus
//! rings, strokes and batched quads.

use super::*;

pub(super) const fn corner_radii(r: f32) -> [f32; 4] {
    [r; 4]
}

fn focus_ring_corner_radius(
    style: &nana_ui_core::LayoutStyle,
    bounds: SceneRect,
    outset: f32,
) -> [f32; 4] {
    let radii = style.resolved_border_radii(bounds.width, bounds.height);
    let max_r = radii.into_iter().fold(0.0f32, f32::max);
    corner_radii(max_r + outset)
}

/// The ring's stroke width and offset, each finite and non-negative: a value
/// the theme compiler would refuse reads as the built-in one.
pub(super) fn focus_ring_stroke(ring: nana_ui_core::FocusRingMetrics) -> (f32, f32) {
    let valid = |value: f32, fallback: f32| {
        if value.is_finite() && value >= 0.0 {
            value
        } else {
            fallback
        }
    };
    (
        valid(ring.width, nana_ui_core::FOCUS_RING_METRICS.width),
        valid(ring.offset, nana_ui_core::FOCUS_RING_METRICS.offset),
    )
}

/// The keyboard focus ring around `bounds` in `color`: the theme's stroke and
/// offset ([`nana_ui_core::ThemeMetrics::focus_ring`]), pushed `extra_gap`
/// further out where the indicator sits inside a larger box.
pub(super) fn focus_ring_quad(
    style: &nana_ui_core::LayoutStyle,
    bounds: SceneRect,
    ring: nana_ui_core::FocusRingMetrics,
    extra_gap: f32,
    color: [f32; 4],
) -> (SceneRect, VisualQuadStyle) {
    let (width, offset) = focus_ring_stroke(ring);
    let outset = offset + extra_gap + width;
    let rect = SceneRect {
        x: bounds.x - outset,
        y: bounds.y - outset,
        width: bounds.width + outset * 2.0,
        height: bounds.height + outset * 2.0,
    };
    (
        rect,
        VisualQuadStyle {
            background: None,
            border_color: Some(color),
            border_width: width,
            corner_radius: focus_ring_corner_radius(style, rect, outset),
        },
    )
}

pub(super) struct VisualPrimitiveContext<'a> {
    pub(super) node: StableNodeId,
    pub(super) transform: AffineTransform,
    pub(super) clips: &'a Arc<[ClipRegion]>,
    pub(super) opacity: f32,
    pub(super) z_index: i32,
    pub(super) document_order: usize,
}

pub(super) struct VisualQuadStyle {
    pub(super) background: Option<[f32; 4]>,
    pub(super) border_color: Option<[f32; 4]>,
    pub(super) border_width: f32,
    pub(super) corner_radius: [f32; 4],
}

impl VisualQuadStyle {
    /// 纯色填充：无边框、直角。装饰条、参考线、高亮底色共用。
    pub(super) fn solid(background: [f32; 4]) -> Self {
        Self {
            background: Some(background),
            border_color: None,
            border_width: 0.0,
            corner_radius: corner_radii(0.0),
        }
    }
}

pub(super) fn visual_quad(
    context: &VisualPrimitiveContext<'_>,
    slot: u64,
    bounds: SceneRect,
    style: VisualQuadStyle,
) -> ScenePrimitive {
    ScenePrimitive {
        id: PrimitiveId {
            node: context.node,
            slot,
        },
        node: context.node,
        bounds,
        transform: context.transform,
        clips: Arc::clone(context.clips),
        opacity: context.opacity,
        z_index: context.z_index,
        document_order: context.document_order,
        kind: ScenePrimitiveKind::Quad {
            background: style.background.map(nana_ui_core::PaintColor::srgb),
            border_color: style.border_color.map(nana_ui_core::PaintColor::srgb),
            border_width: style.border_width,
            corner_radius: style.corner_radius,
            shadow: None,
            surface: Box::default(),
        },
    }
}

/// A row's round picture ([`nana_ui_runtime::ComponentImage`]): the
/// placeholder circle at slot `backdrop`, and over it, at slot `picture`, the
/// host texture cover-fitted into the same circle, the `nana.host-texture`
/// node (revision 0) an `Avatar` of that slot paints, so the two can show
/// one slot in a frame. An empty slot paints only the circle; a slot the
/// host has not registered is skipped by the painter and leaves it showing.
pub(super) fn emit_component_image(
    context: &VisualPrimitiveContext<'_>,
    backdrop: u64,
    picture: u64,
    image: &nana_ui_runtime::ComponentImage,
    emit: &mut impl FnMut(ScenePrimitive),
) {
    let bounds = scene_rect(image.bounds);
    let radius = corner_radii(bounds.width.min(bounds.height) * 0.5);
    emit(visual_quad(
        context,
        backdrop,
        bounds,
        VisualQuadStyle {
            background: Some(image.placeholder),
            border_color: None,
            border_width: 0.0,
            corner_radius: radius,
        },
    ));
    if image.resource.is_empty() {
        return;
    }
    emit(ScenePrimitive {
        id: PrimitiveId {
            node: context.node,
            slot: picture,
        },
        node: context.node,
        bounds,
        transform: context.transform,
        clips: Arc::clone(context.clips),
        opacity: context.opacity,
        z_index: context.z_index,
        document_order: context.document_order,
        kind: ScenePrimitiveKind::Custom {
            node: nana_ui_runtime::CustomRenderNode::new(
                nana_ui_runtime::HOST_TEXTURE_RENDERER,
                image.resource.to_resource(),
                0,
            )
            .with_fit(nana_ui_core::ContentFit::Cover),
            mask: None,
            corner_radius: radius,
        },
    });
}

/// Build a component quad that keeps an explicit authoring-space colour.
/// StandardVisual geometry resolves its slots to sRGB in `VisualQuadStyle`;
/// where the node's authored colour is still that slot's colour, the quad
/// carries the authored value instead, so the GPU converts it to linear scRGB
/// without an sRGB round trip.
pub(super) fn visual_quad_with_paint(
    context: &VisualPrimitiveContext<'_>,
    slot: u64,
    bounds: SceneRect,
    style: VisualQuadStyle,
    background_color: Option<nana_ui_core::PaintColor>,
    border_color_space: Option<nana_ui_core::PaintColor>,
) -> ScenePrimitive {
    let mut primitive = visual_quad(context, slot, bounds, style);
    if let ScenePrimitiveKind::Quad {
        background,
        border_color,
        ..
    } = &mut primitive.kind
    {
        *background = background_color.or(*background);
        *border_color = border_color_space.or(*border_color);
    }
    primitive
}

/// A [`ContrastRing`] thumb filling `rect`, on three consecutive slots from
/// `first_slot`: the outer dark edge, the light ring, the inner dark edge.
/// The middle stays open, so the colour the thumb marks shows through it;
/// the light ring reads on dark colours, the edges on light ones.
pub(super) fn contrast_ring_quads(
    context: &VisualPrimitiveContext<'_>,
    first_slot: u64,
    rect: SceneRect,
    ring: ContrastRing,
) -> [ScenePrimitive; 3] {
    let edge = nana_ui_core::HAIRLINE;
    let band = nana_ui_core::space::XXS;
    let inset = |rect: SceneRect, by: f32| SceneRect {
        x: rect.x + by,
        y: rect.y + by,
        width: (rect.width - by * 2.0).max(0.0),
        height: (rect.height - by * 2.0).max(0.0),
    };
    let stroke = |slot: u64, rect: SceneRect, color: [f32; 4], width: f32| {
        visual_quad(
            context,
            slot,
            rect,
            VisualQuadStyle {
                background: None,
                border_color: Some(color),
                border_width: width,
                corner_radius: corner_radii(rect.width.max(rect.height) / 2.0),
            },
        )
    };
    [
        stroke(first_slot, rect, ring.edge, edge),
        stroke(first_slot + 1, inset(rect, edge), ring.ring, band),
        stroke(first_slot + 2, inset(rect, edge + band), ring.edge, edge),
    ]
}

/// The one colour a scene slot paints: the authored value when there is one,
/// else the resolved sRGB colour.
pub(crate) fn slot_color(
    paint: Option<nana_ui_core::PaintColor>,
    resolved: Option<[f32; 4]>,
) -> Option<nana_ui_core::PaintColor> {
    paint.or(resolved.map(nana_ui_core::PaintColor::srgb))
}

/// Keep explicit metadata only when it describes the resolved legacy colour
/// used by a component slot.  A StandardVisual often has separate semantic
/// colours for its track, thumb, or indicator, so blindly copying the node's
/// PaintColor would tint the wrong slot.
pub(super) fn matching_paint_color(
    paint: Option<nana_ui_core::PaintColor>,
    legacy: Option<[f32; 4]>,
) -> Option<nana_ui_core::PaintColor> {
    paint.filter(|paint| legacy == Some(paint.to_srgb()))
}

/// `ring_color` is the node's resolved [`nana_ui_runtime::NodeStyle::ring`]:
/// an outline with no colour of its own takes it before the current colour.
pub(super) fn quad_surface_from_style(
    style: &nana_ui_core::LayoutStyle,
    ring_color: Option<[f32; 4]>,
    width: f32,
    height: f32,
) -> QuadSurfacePaint {
    let border_fallback = style.paint_colors.border;
    let outline_color = style
        .paint
        .outline
        .is_active()
        .then_some(style.paint.outline.color)
        .flatten();
    QuadSurfacePaint {
        presentation_masks: Vec::new(),
        background_material: style.paint.background_material.clone(),
        border_material: style.paint.border_material.clone(),
        background_image: style.paint.background_image.clone(),
        background_layers: style.paint.background_layers.clone(),
        content_image: style.paint.content_image.clone(),
        mask: style.paint.mask.clone(),
        polygon_clip: style
            .paint
            .clip_path
            .as_ref()
            .and_then(|path| path.resolve_polygon_points(width, height)),
        filter: style.paint.filter.filter(|filter| !filter.is_identity()),
        backdrop_filter: style
            .paint
            .backdrop_filter
            .filter(|filter| filter.is_active()),
        extra_shadows: style
            .paint
            .box_shadows
            .iter()
            .skip(1)
            .copied()
            .map(ComponentElevation::from_box_shadow)
            .collect(),
        outline_width: if style.paint.outline.is_active() {
            style.paint.outline.width
        } else {
            0.0
        },
        // An outline never takes the border's colour: its own, then the
        // ring's theme role, then the current colour as CSS gives an outline
        // without one. A colour set through `PaintStyle::outline` wins over a
        // slot that disagrees.
        outline_color: match outline_color {
            Some(legacy) => slot_color(
                matching_paint_color(style.paint_colors.outline, Some(legacy)),
                Some(legacy),
            ),
            None => style
                .paint_colors
                .outline
                .or(ring_color.map(nana_ui_core::PaintColor::srgb))
                .or(style.paint_colors.color)
                .or(style.color.map(nana_ui_core::PaintColor::srgb)),
        },
        mix_blend: style.paint.mix_blend,
        border_widths: [0.0; 4],
        border_colors: [
            style.paint_colors.border_top.or(border_fallback),
            style.paint_colors.border_right.or(border_fallback),
            style.paint_colors.border_bottom.or(border_fallback),
            style.paint_colors.border_left.or(border_fallback),
        ],
        border_styles: [0; 4],
        border_image: if style.paint.unsupported_border_image {
            None
        } else {
            style.paint.border_image.clone()
        },
    }
}

#[cfg(any(feature = "charts", feature = "graph-canvas", feature = "rich-text"))]
pub(super) fn visual_stroke(
    context: &VisualPrimitiveContext<'_>,
    slot: u64,
    bounds: SceneRect,
    points: Vec<[f32; 2]>,
    width: f32,
    color: [f32; 4],
) -> ScenePrimitive {
    // Graph/TimeSeries keep `pattern: None`. SVG pathLength stays in SVG markup (resvg).
    ScenePrimitive {
        id: PrimitiveId {
            node: context.node,
            slot,
        },
        node: context.node,
        bounds,
        transform: context.transform,
        clips: Arc::clone(context.clips),
        opacity: context.opacity,
        z_index: context.z_index,
        document_order: context.document_order,
        kind: ScenePrimitiveKind::Stroke {
            points,
            width,
            color,
            widths: Vec::new(),
            cap: StrokeCap::Round,
            pattern: None,
        },
    }
}

/// Markdown inline runs still mark their decorations with box-wide strokes;
/// plain text nodes draw theirs per line and run in the text painter.
#[cfg(feature = "rich-text")]
pub(super) fn insert_text_decoration_strokes(
    context: &VisualPrimitiveContext<'_>,
    bounds: SceneRect,
    color: [f32; 4],
    deco: nana_ui_core::TextDecorationLine,
    mut sink: impl FnMut(ScenePrimitive),
) {
    let width = 1.0_f32.max(bounds.height * 0.06);
    let mut emit = |slot: u64, y: f32| {
        sink(ScenePrimitive {
            id: PrimitiveId {
                node: context.node,
                slot,
            },
            node: context.node,
            bounds: SceneRect {
                x: bounds.x,
                y: y - width * 0.5,
                width: bounds.width,
                height: width,
            },
            transform: context.transform,
            clips: Arc::clone(context.clips),
            opacity: context.opacity,
            z_index: context.z_index,
            document_order: context.document_order,
            kind: ScenePrimitiveKind::Stroke {
                points: vec![[bounds.x, y], [bounds.x + bounds.width, y]],
                width,
                color,
                widths: Vec::new(),
                cap: StrokeCap::Butt,
                pattern: None,
            },
        });
    };
    if deco.underline {
        emit(12, bounds.y + bounds.height - width);
    }
    if deco.line_through {
        emit(13, bounds.y + bounds.height * 0.5);
    }
}

/// 批次图元共用的批次壳：对逐项 bounds 求并集作为图元 bounds，kind 由
/// 调用方给出（QuadBatch / QuadColorBatch / IconBatch）。
pub(super) fn batch_primitive(
    context: &VisualPrimitiveContext<'_>,
    slot: u64,
    quad_bounds: Vec<SceneRect>,
    kind: impl FnOnce(Vec<SceneRect>) -> ScenePrimitiveKind,
) -> ScenePrimitive {
    debug_assert!(!quad_bounds.is_empty());
    let bounds = quad_bounds
        .iter()
        .copied()
        .reduce(|left, right| {
            let x = left.x.min(right.x);
            let y = left.y.min(right.y);
            let right_edge = (left.x + left.width).max(right.x + right.width);
            let bottom_edge = (left.y + left.height).max(right.y + right.height);
            SceneRect {
                x,
                y,
                width: right_edge - x,
                height: bottom_edge - y,
            }
        })
        .unwrap_or_default();
    ScenePrimitive {
        id: PrimitiveId {
            node: context.node,
            slot,
        },
        node: context.node,
        bounds,
        transform: context.transform,
        clips: Arc::clone(context.clips),
        opacity: context.opacity,
        z_index: context.z_index,
        document_order: context.document_order,
        kind: kind(quad_bounds),
    }
}

pub(super) fn visual_quad_batch(
    context: &VisualPrimitiveContext<'_>,
    slot: u64,
    bounds: impl IntoIterator<Item = SceneRect>,
    style: VisualQuadStyle,
) -> ScenePrimitive {
    let quad_bounds = bounds.into_iter().collect::<Vec<_>>();
    batch_primitive(context, slot, quad_bounds, |bounds| {
        ScenePrimitiveKind::QuadBatch {
            bounds,
            background: style.background.map(nana_ui_core::PaintColor::srgb),
            border_color: style.border_color.map(nana_ui_core::PaintColor::srgb),
            border_width: style.border_width,
            corner_radius: style.corner_radius,
            shadow: None,
            surface: Box::default(),
        }
    })
}

pub(super) fn visual_quad_color_batch(
    context: &VisualPrimitiveContext<'_>,
    slot: u64,
    items: impl IntoIterator<Item = (SceneRect, [f32; 4])>,
    style: VisualQuadStyle,
) -> ScenePrimitive {
    let (quad_bounds, colors): (Vec<SceneRect>, Vec<[f32; 4]>) = items.into_iter().unzip();
    debug_assert_eq!(quad_bounds.len(), colors.len());
    batch_primitive(context, slot, quad_bounds, |bounds| {
        ScenePrimitiveKind::QuadColorBatch {
            bounds,
            colors,
            border_color: style.border_color,
            border_width: style.border_width,
            corner_radius: style.corner_radius,
        }
    })
}

/// 锚定浮层面板的共享绘制原语（补全弹层 slot 90 与 hover 浮窗 slot 120
/// 共用）：圆角面板底 + 1px 边框，浮在编辑器内容之上。
pub(super) fn overlay_panel_primitive(
    context: &VisualPrimitiveContext<'_>,
    slot: u64,
    bounds: SceneRect,
    background: [f32; 4],
    border: [f32; 4],
) -> ScenePrimitive {
    visual_quad(
        context,
        slot,
        bounds,
        VisualQuadStyle {
            background: Some(background),
            border_color: Some(border),
            border_width: 1.0,
            corner_radius: corner_radii(6.0),
        },
    )
}
