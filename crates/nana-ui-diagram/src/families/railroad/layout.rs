//! Railroad drawing: every expression is a box with a track entering at
//! its left edge and leaving at its right edge on the same line. A box
//! knows its width and how far it reaches above (`up`) and below (`down`)
//! that line; compound boxes are built from their parts' boxes and the
//! tracks between them turn through quarter arcs.

use nana_ui_core::{RadiusTier, SemanticColorMix, SemanticColorRole as R};

use super::{Expr, Model};
use crate::families::LayoutCx;
use crate::geom::PathData;
use crate::ir::{Align, Corner, Dash, Geom, Ink, Rect, SceneBuilder, Stroke, StrokeWidth};
use crate::text::{FontSpec, LaidText};
use crate::{DiagramError, DiagramScene, style};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Leaf {
    Terminal,
    NonTerminal,
    Special,
}

enum Kind {
    Leaf(Leaf, LaidText),
    Comment(LaidText),
    Skip,
    Sequence(Vec<Node>),
    /// Branch track offsets below the main line (the first is 0).
    Choice(Vec<Node>, Vec<f32>),
    /// The bypass line's offset (negative: above).
    Optional(Box<Node>, f32),
    Loop {
        item: Box<Node>,
        separator: Option<Box<Node>>,
        note: Option<LaidText>,
        /// The return line's offset below the main line.
        back: f32,
    },
    Group(Box<Node>, Option<LaidText>),
}

struct Node {
    w: f32,
    up: f32,
    down: f32,
    kind: Kind,
}

struct Metrics {
    arc: f32,
    hgap: f32,
    vgap: f32,
    pad_x: f32,
    box_h: f32,
    frame: f32,
    font: FontSpec,
    mono: FontSpec,
    small: FontSpec,
}

pub(super) fn layout(model: &Model, cx: &mut LayoutCx<'_>) -> Result<DiagramScene, DiagramError> {
    let m = *cx.metrics;
    let font = cx.font();
    let k = Metrics {
        arc: m.font_size * 0.6,
        hgap: m.font_size * 0.8,
        vgap: m.font_size * 0.6,
        pad_x: m.font_size * 0.7,
        box_h: cx.text.line_advance(font) + m.font_size * 0.6,
        frame: m.font_size * 0.45,
        font,
        mono: FontSpec { mono: true, ..font },
        small: FontSpec {
            italic: true,
            ..cx.small_font()
        },
    };
    let mut out = SceneBuilder::new();
    let name_font = FontSpec::new(m.small_size).weight(m.strong_weight);
    let lead = k.arc * 2.0;
    let mut y = 0.0;
    for rule in &model.rules {
        let name = cx.text.lay_plain(&rule.name, name_font);
        out.text(&name, [0.0, y], name.width, Align::Start, style::MUTED);
        let node = measure(&rule.expr, cx, &k);
        let track = y + name.height + m.font_size * 0.5 + node.up.max(k.arc);
        let end = lead + node.w + lead;
        let rail = track_stroke();
        for x in [0.0, end] {
            let mut bar = PathData::new();
            bar.move_to([x, track - k.arc]).line_to([x, track + k.arc]);
            out.stroke(Geom::path(bar), rail);
        }
        line(&mut out, [0.0, track], lead);
        draw(&node, lead, track, &mut out, &k);
        line(&mut out, [lead + node.w, track], lead);
        y = track + node.down.max(k.arc) + m.font_size * 1.6;
    }
    let content = if model.rules.is_empty() {
        Rect::new(0.0, 0.0, m.font_size, m.font_size)
    } else {
        out.bounds()
    };
    let content = cx.title_above(&mut out, content);
    Ok(out.finish(content, m.margin))
}

fn track_stroke() -> Stroke {
    Stroke::new(style::EDGE, StrokeWidth::Thick)
}

fn line(out: &mut SceneBuilder, from: [f32; 2], len: f32) {
    if len <= 0.01 {
        return;
    }
    let mut p = PathData::new();
    p.move_to(from).line_to([from[0] + len, from[1]]);
    out.stroke(Geom::path(p), track_stroke());
}

// `measure` and `draw` recurse once per level of the expression, which a
// grammar nests several levels per bracket (`!( ... )***` is seven). They
// keep only the recursion and hand each node's own arithmetic, text and
// paths to the functions below, so a level costs a small frame in debug
// builds too; see `docs/reference/resource-budgets.md`.

fn measure(expr: &Expr, cx: &LayoutCx<'_>, k: &Metrics) -> Node {
    match expr {
        Expr::Sequence(items) => measure_sequence(items, cx, k),
        Expr::Choice(items) => measure_choice(items, cx, k),
        Expr::Optional(inner) => measure_optional(inner, cx, k),
        Expr::OneOrMore {
            item,
            separator,
            note,
        } => measure_loop(item, separator.as_deref(), note.as_deref(), cx, k),
        Expr::Group(inner, label) => measure_group(inner, label.as_deref(), cx, k),
        leaf => measure_leaf(leaf, cx, k),
    }
}

fn measure_sequence(items: &[Expr], cx: &LayoutCx<'_>, k: &Metrics) -> Node {
    sequence_node(measure_all(items, cx, k), k)
}

fn measure_choice(items: &[Expr], cx: &LayoutCx<'_>, k: &Metrics) -> Node {
    choice_node(measure_all(items, cx, k), k)
}

fn measure_optional(inner: &Expr, cx: &LayoutCx<'_>, k: &Metrics) -> Node {
    optional_node(measure(inner, cx, k), k)
}

fn measure_loop(
    item: &Expr,
    separator: Option<&Expr>,
    note: Option<&str>,
    cx: &LayoutCx<'_>,
    k: &Metrics,
) -> Node {
    let item = measure(item, cx, k);
    // Not `Option::map`: its closure would be one more debug frame on the
    // recursion through separators.
    #[allow(clippy::manual_map)]
    let separator = match separator {
        Some(s) => Some(Box::new(measure(s, cx, k))),
        None => None,
    };
    loop_node(item, separator, note, cx, k)
}

fn measure_group(inner: &Expr, label: Option<&str>, cx: &LayoutCx<'_>, k: &Metrics) -> Node {
    group_node(measure(inner, cx, k), label, cx, k)
}

fn measure_all(items: &[Expr], cx: &LayoutCx<'_>, k: &Metrics) -> Vec<Node> {
    let mut nodes = Vec::with_capacity(items.len());
    for e in items {
        nodes.push(measure(e, cx, k));
    }
    nodes
}

fn measure_leaf(expr: &Expr, cx: &LayoutCx<'_>, k: &Metrics) -> Node {
    let leaf = |kind: Leaf, text: &str, font: FontSpec| {
        let laid = cx.text.lay_plain(text, font);
        Node {
            w: laid.width + 2.0 * k.pad_x,
            up: k.box_h / 2.0,
            down: k.box_h / 2.0,
            kind: Kind::Leaf(kind, laid),
        }
    };
    match expr {
        Expr::Terminal(t) => leaf(Leaf::Terminal, t, k.mono),
        Expr::NonTerminal(t) => leaf(Leaf::NonTerminal, t, k.font),
        Expr::Special(t) => leaf(Leaf::Special, t, k.font),
        Expr::Comment(t) => {
            let laid = cx.text.lay_plain(t, k.small);
            Node {
                w: laid.width,
                up: laid.height + 2.0,
                down: 0.0,
                kind: Kind::Comment(laid),
            }
        }
        _ => Node {
            w: 0.0,
            up: 0.0,
            down: 0.0,
            kind: Kind::Skip,
        },
    }
}

fn sequence_node(nodes: Vec<Node>, k: &Metrics) -> Node {
    let w = nodes.iter().map(|n| n.w).sum::<f32>() + k.hgap * nodes.len().saturating_sub(1) as f32;
    Node {
        w,
        up: nodes.iter().map(|n| n.up).fold(0.0, f32::max),
        down: nodes.iter().map(|n| n.down).fold(0.0, f32::max),
        kind: Kind::Sequence(nodes),
    }
}

fn choice_node(nodes: Vec<Node>, k: &Metrics) -> Node {
    let mut ys = Vec::with_capacity(nodes.len());
    let mut y: f32 = 0.0;
    for (i, n) in nodes.iter().enumerate() {
        if i > 0 {
            y = (y + nodes[i - 1].down + k.vgap + n.up).max(y + 2.0 * k.arc);
        }
        ys.push(y);
    }
    let inner = nodes.iter().map(|n| n.w).fold(0.0, f32::max);
    Node {
        w: inner + 4.0 * k.arc,
        up: nodes.first().map_or(0.0, |n| n.up),
        down: nodes.last().map_or(0.0, |n| n.down) + y,
        kind: Kind::Choice(nodes, ys),
    }
}

fn optional_node(item: Node, k: &Metrics) -> Node {
    let bypass = -(item.up + k.vgap).max(2.0 * k.arc);
    Node {
        w: item.w + 4.0 * k.arc,
        up: -bypass,
        down: item.down,
        kind: Kind::Optional(Box::new(item), bypass),
    }
}

fn loop_node(
    item: Node,
    separator: Option<Box<Node>>,
    note: Option<&str>,
    cx: &LayoutCx<'_>,
    k: &Metrics,
) -> Node {
    let note = note.map(|n| cx.text.lay_plain(n, k.small));
    let sep_up = separator.as_ref().map_or(0.0, |s| s.up);
    let sep_down = separator.as_ref().map_or(0.0, |s| s.down);
    let back = (item.down + k.vgap + sep_up).max(2.0 * k.arc);
    let inner = item.w.max(separator.as_ref().map_or(0.0, |s| s.w));
    let note_w = note.as_ref().map_or(0.0, |n| n.width);
    let note_h = note.as_ref().map_or(0.0, |n| n.height + 2.0);
    Node {
        w: inner.max(note_w) + 4.0 * k.arc,
        up: item.up,
        down: back + sep_down.max(1.0) + note_h,
        kind: Kind::Loop {
            item: Box::new(item),
            separator,
            note,
            back,
        },
    }
}

fn group_node(item: Node, label: Option<&str>, cx: &LayoutCx<'_>, k: &Metrics) -> Node {
    let label = label.map(|l| cx.text.lay_plain(l, k.small));
    let label_w = label.as_ref().map_or(0.0, |l| l.width);
    let label_h = label.as_ref().map_or(0.0, |l| l.height);
    Node {
        w: item.w.max(label_w) + 2.0 * k.frame + k.hgap,
        up: item.up + k.frame + label_h,
        down: item.down + k.frame,
        kind: Kind::Group(Box::new(item), label),
    }
}

/// Draw `node` with its track entering at `(x, y)`.
fn draw(node: &Node, x: f32, y: f32, out: &mut SceneBuilder, k: &Metrics) {
    match &node.kind {
        Kind::Sequence(items) => draw_sequence(items, x, y, out, k),
        Kind::Choice(items, ys) => draw_choice(node, items, ys, x, y, out, k),
        Kind::Optional(item, bypass) => draw_optional(node, item, *bypass, x, y, out, k),
        Kind::Loop { .. } => draw_loop(node, x, y, out, k),
        Kind::Group(item, label) => draw_group(node, item, label.as_ref(), x, y, out, k),
        _ => draw_leaf(node, x, y, out),
    }
}

fn draw_sequence(items: &[Node], x: f32, y: f32, out: &mut SceneBuilder, k: &Metrics) {
    let mut cx = x;
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            line(out, [cx, y], k.hgap);
            cx += k.hgap;
        }
        draw(item, cx, y, out, k);
        cx += item.w;
    }
}

fn draw_choice(
    node: &Node,
    items: &[Node],
    ys: &[f32],
    x: f32,
    y: f32,
    out: &mut SceneBuilder,
    k: &Metrics,
) {
    for (item, &dy) in items.iter().zip(ys) {
        let start = choice_branch(node, item, dy, x, y, out, k);
        draw(item, start, y + dy, out, k);
    }
}

fn draw_optional(
    node: &Node,
    item: &Node,
    bypass: f32,
    x: f32,
    y: f32,
    out: &mut SceneBuilder,
    k: &Metrics,
) {
    line(out, [x, y], 2.0 * k.arc);
    draw(item, x + 2.0 * k.arc, y, out, k);
    optional_bypass(node, bypass, x, y, out, k);
}

fn draw_loop(node: &Node, x: f32, y: f32, out: &mut SceneBuilder, k: &Metrics) {
    let Kind::Loop {
        item,
        separator,
        note,
        back,
    } = &node.kind
    else {
        return;
    };
    let start = loop_start(node, item, x, k);
    line(out, [x, y], start - x);
    draw(item, start, y, out, k);
    let right = x + node.w;
    line(out, [start + item.w, y], right - start - item.w);
    let by = y + back;
    let ends = match separator {
        Some(s) => {
            let s0 = loop_start(node, s, x, k);
            draw(s, s0, by, out, k);
            (s0, s0 + s.w)
        }
        None => (x + node.w / 2.0, x + node.w / 2.0),
    };
    loop_return(
        node,
        separator.as_deref(),
        note.as_ref(),
        by,
        ends,
        x,
        y,
        out,
        k,
    );
}

fn draw_group(
    node: &Node,
    item: &Node,
    label: Option<&LaidText>,
    x: f32,
    y: f32,
    out: &mut SceneBuilder,
    k: &Metrics,
) {
    let item_x = group_item_x(node, item, x, k);
    line(out, [x, y], item_x - x);
    draw(item, item_x, y, out, k);
    group_frame(node, item, item_x, label, x, y, out, k);
}

fn draw_leaf(node: &Node, x: f32, y: f32, out: &mut SceneBuilder) {
    match &node.kind {
        Kind::Leaf(kind, laid) => {
            let rect = Rect::new(x, y - node.up, node.w, node.up + node.down);
            let (corner, fill, stroke, ink) = match kind {
                Leaf::Terminal => (
                    RadiusTier::Full,
                    Ink::Mix(SemanticColorMix::alpha(R::Accent, 0.12)),
                    Stroke::hairline(style::ACCENT),
                    style::TEXT,
                ),
                Leaf::NonTerminal => (
                    RadiusTier::Xs,
                    style::NODE_FILL,
                    style::node_stroke(),
                    style::TEXT,
                ),
                Leaf::Special => (
                    RadiusTier::Sm,
                    Ink::Role(R::Subtle),
                    Stroke::hairline(Ink::Role(R::Border)).dash(Dash::Dashed),
                    style::MUTED,
                ),
            };
            out.shape(
                Geom::Rect {
                    rect,
                    corner: Corner::Tier(corner),
                },
                Some(fill),
                Some(stroke),
            );
            out.text_centered(laid, rect.center(), ink);
        }
        Kind::Comment(laid) => {
            line(out, [x, y], node.w);
            out.text(
                laid,
                [x, y - node.up],
                laid.width,
                Align::Start,
                style::MUTED,
            );
        }
        _ => line(out, [x, y], node.w),
    }
}

/// The tracks into and out of one branch of a choice; returns where the
/// branch starts.
fn choice_branch(
    node: &Node,
    item: &Node,
    dy: f32,
    x: f32,
    y: f32,
    out: &mut SceneBuilder,
    k: &Metrics,
) -> f32 {
    let r = k.arc;
    let inner = node.w - 4.0 * r;
    let right = x + node.w;
    let off = (inner - item.w) / 2.0;
    let start = x + 2.0 * r + off;
    let end = start + item.w;
    if dy == 0.0 {
        line(out, [x, y], start - x);
        line(out, [end, y], right - end);
    } else {
        let by = y + dy;
        let mut p = PathData::new();
        p.move_to([x, y])
            .quad_to([x + r, y], [x + r, y + r])
            .line_to([x + r, by - r])
            .quad_to([x + r, by], [x + 2.0 * r, by])
            .line_to([start, by]);
        out.stroke(Geom::path(p), track_stroke());
        let mut q = PathData::new();
        q.move_to([end, by])
            .line_to([right - 2.0 * r, by])
            .quad_to([right - r, by], [right - r, by - r])
            .line_to([right - r, y + r])
            .quad_to([right - r, y], [right, y]);
        out.stroke(Geom::path(q), track_stroke());
    }
    start
}

/// An optional's track out of its item and the bypass above it.
fn optional_bypass(node: &Node, bypass: f32, x: f32, y: f32, out: &mut SceneBuilder, k: &Metrics) {
    let r = k.arc;
    let right = x + node.w;
    line(out, [right - 2.0 * r, y], 2.0 * r);
    let by = y + bypass;
    let mut p = PathData::new();
    p.move_to([x, y])
        .quad_to([x + r, y], [x + r, y - r])
        .line_to([x + r, by + r])
        .quad_to([x + r, by], [x + 2.0 * r, by])
        .line_to([right - 2.0 * r, by])
        .quad_to([right - r, by], [right - r, by + r])
        .line_to([right - r, y - r])
        .quad_to([right - r, y], [right, y]);
    out.stroke(Geom::path(p), track_stroke());
}

/// Where a loop's item or separator starts: centred in the loop.
fn loop_start(node: &Node, part: &Node, x: f32, k: &Metrics) -> f32 {
    let r = k.arc;
    let inner = node.w - 4.0 * r;
    x + 2.0 * r + (inner - part.w) / 2.0
}

/// A loop's return line under its item, through the separator at
/// `sep_start..sep_end`, with its arrow and count note.
#[allow(clippy::too_many_arguments)]
fn loop_return(
    node: &Node,
    separator: Option<&Node>,
    note: Option<&LaidText>,
    by: f32,
    (sep_start, sep_end): (f32, f32),
    x: f32,
    y: f32,
    out: &mut SceneBuilder,
    k: &Metrics,
) {
    let r = k.arc;
    let right = x + node.w;
    let mut p = PathData::new();
    p.move_to([right - 2.0 * r, y])
        .quad_to([right - r, y], [right - r, y + r])
        .line_to([right - r, by - r])
        .quad_to([right - r, by], [right - 2.0 * r, by])
        .line_to([sep_end, by]);
    out.stroke(Geom::path(p), track_stroke());
    let mut q = PathData::new();
    q.move_to([sep_start, by])
        .line_to([x + 2.0 * r, by])
        .quad_to([x + r, by], [x + r, by - r])
        .line_to([x + r, y + r])
        .quad_to([x + r, y], [x + 2.0 * r, y]);
    out.stroke(Geom::path(q), track_stroke());
    // The loop runs right to left.
    let head = if separator.is_some() {
        (sep_end + right - 2.0 * r) / 2.0
    } else {
        x + node.w / 2.0
    };
    let s = r * 0.55;
    out.fill(
        Geom::path(PathData::polygon(&[
            [head - s * 0.6, by],
            [head + s * 0.6, by - s * 0.7],
            [head + s * 0.6, by + s * 0.7],
        ])),
        style::EDGE,
    );
    if let Some(note) = note {
        let sep_down = separator.map_or(0.0, |s| s.down).max(1.0);
        out.text(
            note,
            [x + (node.w - note.width) / 2.0, by + sep_down + 2.0],
            note.width,
            Align::Center,
            style::MUTED,
        );
    }
}

/// Where a group's item starts: centred in the frame.
fn group_item_x(node: &Node, item: &Node, x: f32, k: &Metrics) -> f32 {
    let left = x + k.hgap / 2.0;
    let frame_w = node.w - k.hgap;
    left + (frame_w - item.w) / 2.0
}

/// A group's track out of its item, its dashed frame and its label.
#[allow(clippy::too_many_arguments)]
fn group_frame(
    node: &Node,
    item: &Node,
    item_x: f32,
    label: Option<&LaidText>,
    x: f32,
    y: f32,
    out: &mut SceneBuilder,
    k: &Metrics,
) {
    let left = x + k.hgap / 2.0;
    let frame_w = node.w - k.hgap;
    line(out, [item_x + item.w, y], x + node.w - item_x - item.w);
    let frame = Rect::new(left, y - node.up, frame_w, node.up + node.down);
    out.stroke(
        Geom::Rect {
            rect: frame,
            corner: Corner::Tier(RadiusTier::Sm),
        },
        Stroke::hairline(style::MUTED).dash(Dash::Dashed),
    );
    if let Some(label) = label {
        out.text(
            label,
            [left + k.frame, frame.y + 1.0],
            label.width,
            Align::Start,
            style::MUTED,
        );
    }
}
