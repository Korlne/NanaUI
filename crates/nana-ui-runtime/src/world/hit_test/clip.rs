//! Clip shapes a hit entry tests a point against: rounded rectangles and
//! path clips.

use super::*;

#[derive(Debug, Clone)]
pub(in crate::world) enum HitPathClip {
    Rect(LayoutBox),
    RoundedRect { rect: LayoutBox, radius: f32 },
    Polygon(Vec<[f32; 2]>),
    Ellipse { rect: LayoutBox },
}

impl HitPathClip {
    pub(super) fn contains(&self, transform: [f32; 6], persp: [f32; 2], x: f32, y: f32) -> bool {
        let Some((x, y)) = transformed_point(transform, persp, x, y) else {
            return false;
        };
        match self {
            Self::Rect(bounds) => bounds.contains(x, y),
            Self::RoundedRect { rect, radius } => rounded_rect_contains(*rect, *radius, x, y),
            Self::Ellipse { rect } => {
                let rx = rect.width * 0.5;
                let ry = rect.height * 0.5;
                if rx <= 0.0 || ry <= 0.0 {
                    return false;
                }
                let dx = (x - (rect.x + rx)) / rx;
                let dy = (y - (rect.y + ry)) / ry;
                dx * dx + dy * dy <= 1.0
            }
            Self::Polygon(points) => point_in_polygon(points, x, y),
        }
    }
}

fn rounded_rect_contains(rect: LayoutBox, radius: f32, x: f32, y: f32) -> bool {
    if !rect.contains(x, y) {
        return false;
    }
    let radius = radius.min(rect.width * 0.5).min(rect.height * 0.5);
    if radius <= 0.0 {
        return true;
    }
    let nearest_x = if x < rect.x + radius {
        rect.x + radius
    } else if x > rect.x + rect.width - radius {
        rect.x + rect.width - radius
    } else {
        x
    };
    let nearest_y = if y < rect.y + radius {
        rect.y + radius
    } else if y > rect.y + rect.height - radius {
        rect.y + rect.height - radius
    } else {
        y
    };
    (x - nearest_x).powi(2) + (y - nearest_y).powi(2) <= radius * radius
}

pub(super) fn path_clips_contain(
    clips: &[HitPathClip],
    transform: [f32; 6],
    persp: [f32; 2],
    x: f32,
    y: f32,
) -> bool {
    for clip in clips {
        if !clip.contains(transform, persp, x, y) {
            nana_diagnostics::metric!(nana_diagnostics::framework::runtime::INPUT_CLIP_REJECTIONS);
            return false;
        }
    }
    true
}

fn point_in_polygon(points: &[[f32; 2]], x: f32, y: f32) -> bool {
    if points.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = points[points.len() - 1];
    for &current in points {
        if (current[1] > y) != (previous[1] > y) {
            let at_x = (previous[0] - current[0]) * (y - current[1]) / (previous[1] - current[1])
                + current[0];
            if x < at_x {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
}

pub(super) fn path_clips_for(
    style: &nana_ui_core::LayoutStyle,
    layout: LayoutBox,
) -> Vec<HitPathClip> {
    let Some(path) = style.paint.clip_path.as_ref() else {
        return Vec::new();
    };
    match path {
        nana_ui_core::ClipPath::Inset(inset) => {
            let [top, right, bottom, left] = inset.resolve_offsets(layout.width, layout.height);
            let rect = LayoutBox {
                x: left,
                y: top,
                width: (layout.width - left - right).max(0.0),
                height: (layout.height - top - bottom).max(0.0),
            };
            let radius = inset.resolve_round(layout.width, layout.height);
            if radius > 0.0 {
                vec![HitPathClip::RoundedRect { rect, radius }]
            } else {
                vec![HitPathClip::Rect(rect)]
            }
        }
        nana_ui_core::ClipPath::Polygon(_) => path
            .resolve_polygon_points(layout.width, layout.height)
            .map_or_else(Vec::new, |points| vec![HitPathClip::Polygon(points)]),
        nana_ui_core::ClipPath::Circle(_) | nana_ui_core::ClipPath::Ellipse(_) => path
            .resolve_ellipse_rect(layout.width, layout.height)
            .map_or_else(Vec::new, |[x, y, width, height]| {
                vec![HitPathClip::Ellipse {
                    rect: LayoutBox {
                        x,
                        y,
                        width,
                        height,
                    },
                }]
            }),
    }
}
