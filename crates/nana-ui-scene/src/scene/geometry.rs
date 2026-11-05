//! Scene geometry: rectangles, affine transforms and clip regions.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SceneRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AffineTransform(pub [f32; 6], pub [f32; 2]);

impl AffineTransform {
    pub const IDENTITY: Self = Self([1.0, 0.0, 0.0, 1.0, 0.0, 0.0], [0.0, 0.0]);

    pub const fn from_matrix(matrix: [f32; 6]) -> Self {
        Self(matrix, [0.0, 0.0])
    }

    pub fn is_projective(self) -> bool {
        self.1[0].abs() > 1e-8 || self.1[1].abs() > 1e-8
    }

    pub fn then(self, rhs: Self) -> Self {
        let [a, b, c, d, e, f] = self.0;
        let [g, h] = self.1;
        let [ra, rb, rc, rd, re, rf] = rhs.0;
        let [rg, rh] = rhs.1;
        let na = a * ra + c * rb + e * rg;
        let nb = b * ra + d * rb + f * rg;
        let nc = a * rc + c * rd + e * rh;
        let nd = b * rc + d * rd + f * rh;
        let ne = a * re + c * rf + e;
        let nf = b * re + d * rf + f;
        let ng = g * ra + h * rb + rg;
        let nh = g * rc + h * rd + rh;
        let ni = g * re + h * rf + 1.0;
        // Keep the CPU contract wide enough to match WGSL f32/FMA rounding.
        if !ni.is_finite() || ni.abs() < 1e-6 {
            return Self::IDENTITY;
        }
        let inv = 1.0 / ni;
        Self(
            [na * inv, nb * inv, nc * inv, nd * inv, ne * inv, nf * inv],
            [ng * inv, nh * inv],
        )
    }
}

impl From<[f32; 6]> for AffineTransform {
    fn from(matrix: [f32; 6]) -> Self {
        Self::from_matrix(matrix)
    }
}

impl Default for AffineTransform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClipRegion {
    pub bounds: SceneRect,
    pub transform: AffineTransform,
    /// Rounded inset clip radius (px in border-box space). Zero = axis-aligned rect.
    pub corner_radius: f32,
    /// `clip-path: polygon(...)` vertices in [`Self::bounds`] local space (px).
    pub polygon_clip: Option<Vec<[f32; 2]>>,
}

impl ClipRegion {
    pub fn axis_aligned(bounds: SceneRect, transform: AffineTransform) -> Self {
        Self {
            bounds,
            transform,
            corner_radius: 0.0,
            polygon_clip: None,
        }
    }

    /// Ellipse filling `bounds` (`clip-path: circle()` / `ellipse()`).
    pub fn ellipse(bounds: SceneRect, transform: AffineTransform) -> Self {
        Self {
            bounds,
            transform,
            corner_radius: 0.0,
            polygon_clip: Some(vec![[f32::NEG_INFINITY, f32::NEG_INFINITY]]),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn scene_rect(bounds: LayoutBox) -> SceneRect {
    SceneRect {
        x: bounds.x,
        y: bounds.y,
        width: bounds.width.max(0.0),
        height: bounds.height.max(0.0),
    }
}
