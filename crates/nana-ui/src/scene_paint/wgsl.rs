//! The scene's WGSL files, one copy of each.
//!
//! A program is several files in a row: the colour maths, the motion
//! evaluator, a family's own stages. Spelled with `concat!`, each program
//! embedded its own copy of every file it shares (`color.wgsl` nine times,
//! `motion.wgsl` twice), 309 KiB of text for 163 KiB of source. The files
//! are embedded once here, and a program is joined when its module is
//! created, which is once per device.

pub(super) const BACKDROP_BLUR: &str = include_str!("shader/backdrop_blur.wgsl");
pub(super) const BACKDROP_COMPOSITE: &str = include_str!("shader/backdrop_composite.wgsl");
pub(super) const BACKDROP_COPY: &str = include_str!("shader/backdrop_copy.wgsl");
pub(super) const BLIT: &str = include_str!("shader/blit.wgsl");
pub(super) const CHART: &str = include_str!("shader/chart.wgsl");
pub(super) const COLOR: &str = include_str!("shader/color.wgsl");
pub(super) const LAYER: &str = include_str!("shader/layer.wgsl");
pub(super) const LAYER_BLEND: &str = include_str!("shader/layer_blend.wgsl");
pub(super) const MOTION: &str = include_str!("shader/motion.wgsl");
pub(super) const PATH: &str = include_str!("shader/path.wgsl");
pub(super) const QUAD: &str = include_str!("shader/quad.wgsl");
pub(super) const QUAD_PAINT: &str = include_str!("shader/quad_paint.wgsl");
pub(super) const QUAD_PAINT_DATA: &str = include_str!("shader/quad_paint_data.wgsl");
pub(super) const QUAD_SOLID: &str = include_str!("shader/quad_solid.wgsl");
pub(super) const TEXT_ATLAS: &str = include_str!("shader/text_atlas.wgsl");
pub(super) const TRIANGLE: &str = include_str!("shader/triangle.wgsl");
pub(super) const TRIANGLE_SOLID: &str = include_str!("shader/triangle_solid.wgsl");
pub(super) const VERTEX: &str = include_str!("shader/vertex.wgsl");

/// `files` in a row with a line break between each.
pub(super) fn joined(files: &[&str]) -> String {
    files.join("\n")
}

/// The program [`joined`] from `files`, as a module's source.
pub(super) fn source(files: &[&str]) -> wgpu::ShaderSource<'static> {
    wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(joined(files)))
}
