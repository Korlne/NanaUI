//! The scene's WGSL files, one copy of each, without their comments.
//!
//! A program is several files in a row: the colour maths, the motion
//! evaluator, a family's own stages. Spelled with `concat!`, each program
//! embedded its own copy of every file it shares (`color.wgsl` nine times,
//! `motion.wgsl` twice), 309 KiB of text for 163 KiB of source. The files
//! are embedded once here, and a program is joined when its module is
//! created, which is once per device.
//!
//! What is embedded is the file as the shader compiler needs it: comments
//! and trailing blanks are taken out when this crate is compiled
//! ([`stripped`]), about a fifth of the text. Line breaks and indentation
//! stay, so a compile error still names the line of the file, and the
//! material programs that rewrite a statement by its spelling still find
//! it.

/// `source` without its `//` comments and trailing blanks, as a constant.
macro_rules! stripped {
    ($source:expr) => {{
        const SOURCE: &str = $source;
        const LEN: usize = $crate::scene_paint::wgsl::strip::<0>(SOURCE.as_bytes()).1;
        #[allow(long_running_const_eval)]
        const BYTES: [u8; LEN] = $crate::scene_paint::wgsl::strip::<LEN>(SOURCE.as_bytes()).0;
        match ::core::str::from_utf8(&BYTES) {
            Ok(text) => text,
            Err(_) => panic!("stripping WGSL split a character"),
        }
    }};
}
pub(super) use stripped;

/// `source` with every `//` comment and the blanks before a line break
/// taken out, written into the first bytes of an array of `N`, and how many
/// bytes that is. With `N` of zero nothing is written and only the count is
/// right, which is how [`stripped`] sizes the array.
///
/// WGSL has no string literals, so `//` starts a comment wherever it is;
/// none of the scene's files uses a block comment. Cutting at an ASCII
/// byte and dropping ASCII blanks cannot split a character.
pub(super) const fn strip<const N: usize>(source: &[u8]) -> ([u8; N], usize) {
    let mut out = [0u8; N];
    let mut written = 0;
    let mut index = 0;
    while index < source.len() {
        // Where the line's last byte that is not a blank ends.
        let mut kept = written;
        while index < source.len() && source[index] != b'\n' {
            let byte = source[index];
            if byte == b'/' && index + 1 < source.len() && source[index + 1] == b'/' {
                while index < source.len() && source[index] != b'\n' {
                    index += 1;
                }
                break;
            }
            if written < N {
                out[written] = byte;
            }
            written += 1;
            if !matches!(byte, b' ' | b'\t' | b'\r') {
                kept = written;
            }
            index += 1;
        }
        written = kept;
        if index < source.len() {
            if written < N {
                out[written] = b'\n';
            }
            written += 1;
            index += 1;
        }
    }
    (out, written)
}

pub(super) const BACKDROP_BLUR: &str = stripped!(include_str!("shader/backdrop_blur.wgsl"));
pub(super) const BACKDROP_COMPOSITE: &str =
    stripped!(include_str!("shader/backdrop_composite.wgsl"));
pub(super) const BACKDROP_COPY: &str = stripped!(include_str!("shader/backdrop_copy.wgsl"));
pub(super) const BLIT: &str = stripped!(include_str!("shader/blit.wgsl"));
pub(super) const CHART: &str = stripped!(include_str!("shader/chart.wgsl"));
pub(super) const COLOR: &str = stripped!(include_str!("shader/color.wgsl"));
pub(super) const LAYER: &str = stripped!(include_str!("shader/layer.wgsl"));
pub(super) const LAYER_BLEND: &str = stripped!(include_str!("shader/layer_blend.wgsl"));
pub(super) const MOTION: &str = stripped!(include_str!("shader/motion.wgsl"));
pub(super) const PATH: &str = stripped!(include_str!("shader/path.wgsl"));
pub(super) const QUAD: &str = stripped!(include_str!("shader/quad.wgsl"));
pub(super) const QUAD_PAINT: &str = stripped!(include_str!("shader/quad_paint.wgsl"));
pub(super) const QUAD_PAINT_DATA: &str = stripped!(include_str!("shader/quad_paint_data.wgsl"));
pub(super) const QUAD_SOLID: &str = stripped!(include_str!("shader/quad_solid.wgsl"));
pub(super) const TEXT_ATLAS: &str = stripped!(include_str!("shader/text_atlas.wgsl"));
pub(super) const TRIANGLE: &str = stripped!(include_str!("shader/triangle.wgsl"));
pub(super) const TRIANGLE_SOLID: &str = stripped!(include_str!("shader/triangle_solid.wgsl"));
pub(super) const VERTEX: &str = stripped!(include_str!("shader/vertex.wgsl"));

/// `files` in a row with a line break between each.
pub(super) fn joined(files: &[&str]) -> String {
    files.join("\n")
}

/// The program [`joined`] from `files`, as a module's source.
pub(super) fn source(files: &[&str]) -> wgpu::ShaderSource<'static> {
    wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(joined(files)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip_text(source: &str) -> String {
        let mut out = String::new();
        // The same pass, a byte at a time: sized once, then written.
        let len = strip::<0>(source.as_bytes()).1;
        let (bytes, written) = strip::<8192>(source.as_bytes());
        assert_eq!(written, len);
        out.push_str(std::str::from_utf8(&bytes[..written]).unwrap());
        out
    }

    #[test]
    fn comments_and_trailing_blanks_go_and_lines_stay_where_they_were() {
        let source = "// A file.\r\nfn a() -> f32 { // why\n    return 1.0 / 2.0;  \n}\n\n  // 注释\nlet b = 1;";
        assert_eq!(
            strip_text(source),
            "\nfn a() -> f32 {\n    return 1.0 / 2.0;\n}\n\n\nlet b = 1;"
        );
        assert_eq!(strip_text(""), "");
        assert_eq!(strip_text("//"), "");
        assert_eq!(strip_text("a /"), "a /");
    }

    /// A compile error names a line: it has to be the file's.
    #[test]
    fn every_file_keeps_its_lines_and_loses_its_comments() {
        for (stripped, file) in [
            (COLOR, include_str!("shader/color.wgsl")),
            (MOTION, include_str!("shader/motion.wgsl")),
            (QUAD_SOLID, include_str!("shader/quad_solid.wgsl")),
            (TEXT_ATLAS, include_str!("shader/text_atlas.wgsl")),
            (CHART, include_str!("shader/chart.wgsl")),
        ] {
            assert_eq!(stripped.lines().count(), file.lines().count());
            assert!(!stripped.contains("//"));
            assert!(stripped.len() < file.len());
            for (kept, line) in stripped.lines().zip(file.lines()) {
                assert_eq!(kept, line.split("//").next().unwrap().trim_end());
            }
        }
    }
}
