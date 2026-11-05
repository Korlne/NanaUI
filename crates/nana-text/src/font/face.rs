//! What `skrifa` can tell about one face. The only file allowed to name it.
//!
//! Everything returned here is a Nana type, so no `skrifa` or `read_fonts`
//! item can escape through the font system's public API.

use super::coverage::CoverageSet;
use super::variations::{AxisCoord, FontAxis, NamedInstance};
use crate::metrics::RunMetrics;
use skrifa::prelude::Size;
use skrifa::{FontRef, MetadataProvider, Tag};

/// Colour glyph tables. Presence only: painting them is a renderer concern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ColorTables {
    pub colr: bool,
    pub cbdt: bool,
    pub sbix: bool,
    pub svg: bool,
}

impl ColorTables {
    pub fn any(self) -> bool {
        self.colr || self.cbdt || self.sbix || self.svg
    }
}

/// Face facts that need the font bytes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FaceDetails {
    pub axes: Vec<FontAxis>,
    pub named_instances: Vec<NamedInstance>,
    pub color: ColorTables,
}

impl FaceDetails {
    pub fn axis(&self, tag: [u8; 4]) -> Option<&FontAxis> {
        self.axes.iter().find(|axis| axis.tag == tag)
    }
}

/// Parses the details of face `index` in `data`, or `None` when the bytes are
/// not a face `skrifa` can read.
pub fn read_details(data: &[u8], index: u32) -> Option<FaceDetails> {
    let font = FontRef::from_index(data, index).ok()?;
    let axes: Vec<FontAxis> = font
        .axes()
        .iter()
        .map(|axis| FontAxis {
            tag: axis.tag().to_be_bytes(),
            min: axis.min_value(),
            default: axis.default_value(),
            max: axis.max_value(),
        })
        .collect();
    let named_instances = font
        .named_instances()
        .iter()
        .map(|instance| NamedInstance {
            name: font
                .localized_strings(instance.subfamily_name_id())
                .english_or_first()
                .map(|name| name.chars().collect()),
            coords: axes
                .iter()
                .zip(instance.user_coords())
                .map(|(axis, value)| AxisCoord {
                    tag: axis.tag,
                    value,
                })
                .collect(),
        })
        .collect();
    let has = |tag: &[u8; 4]| font.table_data(Tag::new(tag)).is_some();
    Some(FaceDetails {
        axes,
        named_instances,
        color: ColorTables {
            colr: has(b"COLR"),
            cbdt: has(b"CBDT"),
            sbix: has(b"sbix"),
            svg: has(b"SVG "),
        },
    })
}

/// Ascent, descent and line gap in px at `coords` and `size_px`, as positive
/// numbers, plus the underline (`post`) and strikeout (`OS/2`) the face
/// suggests.
///
/// A face that leaves either out — or states a thickness of zero — gets a
/// stand-in proportional to its size: a 1/14 em stroke, the underline a tenth
/// of an em under the baseline, the strikeout centred on half the x-height.
pub fn read_metrics(
    data: &[u8],
    index: u32,
    coords: &[AxisCoord],
    size_px: f32,
) -> Option<RunMetrics> {
    let font = FontRef::from_index(data, index).ok()?;
    let location = font.axes().location(
        coords
            .iter()
            .map(|coord| (Tag::new(&coord.tag), coord.value)),
    );
    let metrics = font.metrics(Size::new(size_px), &location);
    let fallback_thickness = (size_px / 14.0).max(f32::MIN_POSITIVE);
    let usable = |decoration: Option<skrifa::metrics::Decoration>| {
        decoration.filter(|decoration| {
            decoration.thickness.is_finite()
                && decoration.thickness > 0.0
                && decoration.offset.is_finite()
        })
    };
    let (underline_offset_px, underline_thickness_px) = usable(metrics.underline).map_or(
        (size_px * 0.1, fallback_thickness),
        // skrifa's offset is to the top of the stroke, y-up; this one is
        // positive down.
        |decoration| (-decoration.offset, decoration.thickness),
    );
    let x_height = metrics
        .x_height
        .filter(|height| height.is_finite() && *height > 0.0)
        .unwrap_or(metrics.ascent * 0.5);
    let (strikeout_offset_px, strikeout_thickness_px) = usable(metrics.strikeout).map_or(
        (
            x_height * 0.5 + fallback_thickness * 0.5,
            fallback_thickness,
        ),
        |decoration| (decoration.offset, decoration.thickness),
    );
    let (ascent_px, descent_px) = centred_on_ideographic_em_box(
        &font,
        size_px,
        metrics.units_per_em,
        metrics.ascent,
        metrics.descent.abs(),
    );
    Some(RunMetrics {
        ascent_px,
        descent_px,
        line_gap_px: metrics.leading,
        underline_offset_px,
        underline_thickness_px,
        strikeout_offset_px,
        strikeout_thickness_px,
    })
}

/// A CJK face's ascent and descent, moved so their midpoint is the middle of
/// the ideographic em box the face declares; any other face's, unchanged.
///
/// CJK faces set `hhea` generously and lopsidedly (Source Han / Noto CJK:
/// 1.16 above, 0.288 below) while their ideographs fill an em box at
/// 0.88 / −0.12. Half-leading splits the line around the `hhea` midpoint, so
/// a single line in a box of any height sits about 0.06 em low. The face's
/// `BASE` table states the em box (`ideo`, and `idtp` or one em above it);
/// centring on it is what the central baseline means for these faces. The
/// sum, and with it a `normal` line height, is unchanged, so a paragraph's
/// line pitch is too. Only faces that map a Han, kana or Hangul character
/// count: a Latin face may carry an `ideo` baseline for alignment against
/// CJK, but its own letters are not drawn on that em box.
fn centred_on_ideographic_em_box(
    font: &FontRef<'_>,
    size_px: f32,
    units_per_em: u16,
    ascent: f32,
    descent: f32,
) -> (f32, f32) {
    let Some((top, bottom)) = ideographic_em_box(font, units_per_em) else {
        return (ascent, descent);
    };
    let scale = size_px / f32::from(units_per_em.max(1));
    let shift = ((ascent - descent) - (top + bottom) * scale) * 0.5;
    if !shift.is_finite() {
        return (ascent, descent);
    }
    (ascent - shift, descent + shift)
}

/// The ideographic em box from the face's horizontal `BASE` axis, in font
/// units with y up: its top and bottom edges. `None` unless the face draws
/// CJK and states an `ideo` baseline.
fn ideographic_em_box(font: &FontRef<'_>, units_per_em: u16) -> Option<(f32, f32)> {
    use skrifa::raw::TableProvider;
    let charmap = font.charmap();
    if !['\u{4E00}', '\u{3042}', '\u{30A2}', '\u{AC00}']
        .into_iter()
        .any(|ch| charmap.map(ch).is_some())
    {
        return None;
    }
    let axis = font.base().ok()?.horiz_axis()?.ok()?;
    let tags = axis.base_tag_list()?.ok()?;
    let tags = tags.baseline_tags();
    let index = |tag: &[u8; 4]| tags.iter().position(|t| t.get() == Tag::new(tag));
    let ideo = index(b"ideo")?;
    let idtp = index(b"idtp");
    let list = axis.base_script_list().ok()?;
    let records = list.base_script_records();
    // The em box is one per face in practice; prefer the Han record, then any
    // CJK one, then the default.
    let record = [b"hani", b"kana", b"hang", b"DFLT"]
        .into_iter()
        .find_map(|script| {
            records
                .iter()
                .find(|record| record.base_script_tag() == Tag::new(script))
        })
        .or_else(|| records.first())?;
    let values = record
        .base_script(list.offset_data())
        .ok()?
        .base_values()?
        .ok()?;
    let coords = values.base_coords();
    let coord =
        |index: usize| -> Option<f32> { Some(f32::from(coords.get(index).ok()?.coordinate())) };
    let bottom = coord(ideo)?;
    let top = idtp
        .and_then(coord)
        .unwrap_or(bottom + f32::from(units_per_em));
    (top > bottom).then_some((top, bottom))
}

/// The left and right edges of glyph `glyph_id`'s ink at `size_px`, from
/// its pen position along a horizontal line; `None` when the face gives no
/// bounds for it.
pub fn read_glyph_ink(
    data: &[u8],
    index: u32,
    coords: &[AxisCoord],
    size_px: f32,
    glyph_id: u32,
) -> Option<(f32, f32)> {
    let font = FontRef::from_index(data, index).ok()?;
    let location = font.axes().location(
        coords
            .iter()
            .map(|coord| (Tag::new(&coord.tag), coord.value)),
    );
    let bounds = font
        .glyph_metrics(Size::new(size_px), &location)
        .bounds(skrifa::GlyphId::new(glyph_id))?;
    (bounds.x_min.is_finite() && bounds.x_max.is_finite() && bounds.x_max > bounds.x_min)
        .then_some((bounds.x_min, bounds.x_max))
}

/// Every codepoint the face's best cmap maps to a glyph it can draw,
/// folded into ranges.
///
/// A visible character mapped to an empty outline is left out, so fallback
/// looks further: some faces map a character they do not draw (macOS's
/// Songti SC Black maps U+00A5 YEN SIGN to a blank glyph), and taking the
/// mapping at its word drew a gap where the character belonged.
pub fn read_coverage(data: &[u8], index: u32) -> CoverageSet {
    let Ok(font) = FontRef::from_index(data, index) else {
        return CoverageSet::default();
    };
    let blank = blank_outlines(&font);
    let mut codepoints: Vec<u32> = font
        .charmap()
        .mappings()
        .filter(|(_, glyph)| glyph.to_u32() != 0)
        .filter(|(codepoint, glyph)| {
            !blank.as_ref().is_some_and(|blank| blank(glyph.to_u32()))
                || char::from_u32(*codepoint).is_none_or(|ch| !visible(ch))
        })
        .map(|(codepoint, _)| codepoint)
        .collect();
    codepoints.sort_unstable();
    codepoints.dedup();
    CoverageSet::from_sorted_codepoints(&codepoints)
}

/// Which glyphs have no outline, for a face drawn from `glyf` outlines
/// alone: a glyph whose `loca` entry and the next one are equal is empty.
/// `None` for any other face. A colour or bitmap face may leave a glyph's
/// outline empty and draw it from its own table, and a CFF face's empty
/// glyphs cannot be told without running their charstrings.
fn blank_outlines<'a>(font: &FontRef<'a>) -> Option<impl Fn(u32) -> bool + 'a> {
    use skrifa::raw::TableProvider;
    let drawn_elsewhere = [b"COLR", b"sbix", b"CBDT", b"EBDT", b"SVG "]
        .iter()
        .any(|tag| font.table_data(Tag::new(tag)).is_some());
    if drawn_elsewhere || font.table_data(Tag::new(b"glyf")).is_none() {
        return None;
    }
    let loca = font.loca(None).ok()?;
    Some(move |glyph: u32| {
        let glyph = glyph as usize;
        matches!(
            (loca.get_raw(glyph), loca.get_raw(glyph + 1)),
            (Some(start), Some(end)) if start == end
        )
    })
}

/// A character that draws ink: not white space, a control or a default
/// ignorable, whose glyphs are rightly empty.
fn visible(ch: char) -> bool {
    !ch.is_whitespace() && !ch.is_control() && !super::unicode::is_default_ignorable(ch)
}

#[cfg(test)]
mod blank_glyph_tests {
    use super::*;
    use skrifa::raw::TableProvider;

    const KR: &[u8] = include_bytes!("../../fonts/noto-sans-kr.ttf");
    const COLOR: &[u8] = include_bytes!("../../fonts/nana-test-color.ttf");

    fn mapped(data: &[u8]) -> Vec<(u32, u32)> {
        FontRef::new(data)
            .unwrap()
            .charmap()
            .mappings()
            .map(|(c, g)| (c, g.to_u32()))
            .filter(|(_, g)| *g != 0)
            .collect()
    }

    /// `data` with glyph `glyph` emptied: its next `loca` entry made equal
    /// to its own, as a face that maps a character it does not draw has it.
    fn emptied(data: &[u8], glyph: u32) -> Vec<u8> {
        let font = FontRef::new(data).unwrap();
        let long = font.head().unwrap().index_to_loc_format() == 1;
        let record = font
            .table_directory
            .table_records()
            .iter()
            .find(|r| r.tag() == Tag::new(b"loca"))
            .unwrap();
        let at = record.offset() as usize;
        let mut out = data.to_vec();
        let (size, i) = (if long { 4 } else { 2 }, glyph as usize);
        let (from, to) = (at + i * size, at + (i + 1) * size);
        let entry = out[from..from + size].to_vec();
        out[to..to + size].copy_from_slice(&entry);
        out
    }

    #[test]
    fn a_character_mapped_to_an_empty_outline_is_not_covered() {
        let font = FontRef::new(KR).unwrap();
        let loca = font.loca(None).unwrap();
        let blank = |g: u32| loca.get_raw(g as usize) == loca.get_raw(g as usize + 1);
        let (ch, glyph) = mapped(KR)
            .into_iter()
            .find(|(c, g)| {
                char::from_u32(*c).is_some_and(visible)
                    && !blank(*g)
                    && (*g as usize + 2) < loca.len()
            })
            .expect("a drawn character");
        assert!(read_coverage(KR, 0).contains(ch));
        let patched = emptied(KR, glyph);
        assert!(
            !read_coverage(&patched, 0).contains(ch),
            "U+{ch:04X} maps to a blank glyph, so fallback must look further"
        );
        // White space draws nothing and stays covered.
        if mapped(KR).iter().any(|(c, _)| *c == 0x20) {
            assert!(read_coverage(&patched, 0).contains(0x20));
        }
    }

    #[test]
    fn a_colour_face_keeps_every_mapping() {
        let coverage = read_coverage(COLOR, 0);
        for (ch, _) in mapped(COLOR) {
            assert!(coverage.contains(ch), "U+{ch:04X}");
        }
    }
}
