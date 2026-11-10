//! Shared Nana artwork for runtime icons and packaged ICO/ICNS resources.

use std::sync::OnceLock;

use image::{RgbaImage, imageops};

/// Rasterize the default mark as unpremultiplied RGBA.
pub fn rasterize(size: u32) -> Vec<u8> {
    static SOURCE: OnceLock<RgbaImage> = OnceLock::new();
    let source = SOURCE.get_or_init(|| {
        let mut source = image::load_from_memory(include_bytes!("../assets/nana.png"))
            .expect("bundled Nana icon must be a valid PNG")
            .into_rgba8();
        // Filter in premultiplied space to avoid dark fringes at transparent edges.
        for pixel in source.pixels_mut() {
            let alpha = u16::from(pixel[3]);
            for channel in &mut pixel.0[..3] {
                *channel = ((u16::from(*channel) * alpha + 127) / 255) as u8;
            }
        }
        source
    });
    let size = size.max(1);
    // The source already has a small transparent border. A 90% image box puts
    // the visible plate at approximately 83% of the canvas, matching our grid.
    let content = ((size as f64 * 0.90).round() as u32).max(1);
    let scale = content as f64 / source.width().max(source.height()) as f64;
    let width = ((source.width() as f64 * scale).round() as u32).max(1);
    let height = ((source.height() as f64 * scale).round() as u32).max(1);
    let mut resized = imageops::resize(source, width, height, imageops::FilterType::Lanczos3);
    for pixel in resized.pixels_mut() {
        let alpha = u32::from(pixel[3]);
        for channel in &mut pixel.0[..3] {
            *channel = if alpha == 0 {
                0
            } else {
                (u32::from(*channel) * 255 + alpha / 2)
                    .checked_div(alpha)
                    .unwrap_or(0)
                    .min(255) as u8
            };
        }
    }
    let mut canvas = RgbaImage::new(size, size);
    imageops::overlay(
        &mut canvas,
        &resized,
        ((size - width) / 2) as i64,
        ((size - height) / 2) as i64,
    );
    canvas.into_raw()
}

/// Side of the mark a window shows without an icon of its own.
#[cfg(any(test, feature = "runtime"))]
pub const RUNTIME_SIZE: u32 = 256;

/// The mark at [`RUNTIME_SIZE`], unpremultiplied RGBA: `rasterize` run ahead
/// of time (`tests::regenerate_runtime_mark`).
///
/// An application links this instead of the source artwork. The source is
/// about 0.9 MB and only [`rasterize`] reads it, which leaves it to the
/// packaging paths (ICO, ICNS) that run at build time.
#[cfg(any(test, feature = "runtime"))]
pub fn runtime_mark() -> Vec<u8> {
    let mark = image::load_from_memory(include_bytes!("../assets/nana-256.png"))
        .expect("bundled runtime mark must be a valid PNG")
        .into_rgba8();
    debug_assert_eq!((mark.width(), mark.height()), (RUNTIME_SIZE, RUNTIME_SIZE));
    mark.into_raw()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Resampling is floating point, so another target may round a channel
    /// the other way; anything past that means the source artwork changed
    /// and the runtime mark was not regenerated.
    #[test]
    fn runtime_mark_is_the_source_rasterized_at_its_size() {
        let ahead = runtime_mark();
        let now = rasterize(RUNTIME_SIZE);
        assert_eq!(ahead.len(), now.len());
        let drift = ahead
            .iter()
            .zip(&now)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap_or(0);
        assert!(
            drift <= 2,
            "assets/nana-256.png is stale (channel drift {drift}); run \
             `cargo test -p nana-app-icon -- --ignored regenerate_runtime_mark`"
        );
    }

    #[test]
    #[ignore = "rewrites assets/nana-256.png from assets/nana.png"]
    fn regenerate_runtime_mark() {
        let png = crate::encode::png(RUNTIME_SIZE, RUNTIME_SIZE, &rasterize(RUNTIME_SIZE))
            .expect("runtime mark encodes");
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/nana-256.png");
        std::fs::write(path, png).expect("runtime mark is written");
    }

    #[test]
    fn mark_preserves_transparency_at_runtime_and_package_sizes() {
        for size in [16, 32, 48, 128, 256, 512] {
            let pixels = rasterize(size);
            assert_eq!(pixels.len(), (size * size * 4) as usize);
            assert!(pixels.as_chunks::<4>().0.iter().any(|pixel| pixel[3] > 245));
            for i in 0..size as usize {
                let side = size as usize;
                for index in [i, (side - 1) * side + i, i * side, i * side + side - 1] {
                    assert_eq!(pixels[index * 4 + 3], 0, "transparent margin at {size}px");
                }
            }
        }
    }
}
