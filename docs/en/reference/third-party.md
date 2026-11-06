# Third-party code and license audit

NanaUI's own code is dual-licensed under **MIT or Apache-2.0** (see [LICENSE-MIT](https://github.com/sena-nana/NanaUI/blob/main/LICENSE-MIT) and [LICENSE-APACHE](https://github.com/sena-nana/NanaUI/blob/main/LICENSE-APACHE) in the repository root). This page is for maintainers who package and distribute NanaUI applications or run a legal compliance review. It records the license makeup of the current dependency graph, the removal status of the old forks after the switch to the in-house text engine (#99), and the two explicitly attributed ported code fragments in the repository.

## Core compliance findings

1. **Permissive licenses only across the whole graph**: the `normal + build` dependency graph for all platforms (`--target all`) contains **572 external crates**, all under permissive open-source licenses (MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD, ISC, Zlib, Unicode-3.0, 0BSD, CC0, BSL-1.0, CDLA-Permissive-2.0, and combinations of these). There is no dependency edge that offers only a copyleft license (the two `MIT OR Apache-2.0 OR LGPL-2.1-or-later` crates are taken under the MIT / Apache-2.0 option of the multi-license).
2. **`cryoglyph` and `cosmic-text` are fully removed from the product and from `Cargo.lock`**: neither appears as a normal dependency or as a dev dependency. In CI, `scripts/check-engine-boundary.py` checks the whole workspace: no member, under any feature, may introduce a non-dev dependency edge leading to `cosmic-text`, `cryoglyph`, or `glyphon` (including renamed forks), and none of these three identifiers may appear under `nana-text/src`.
3. **Both ported code fragments keep their original copyright and license notices in place** (see "Explicit attribution and ported fragments" below).

## Low-level algorithm libraries used by the text stack

`nana-text` owns its lifecycle, text IR, tiered caches, and paragraph composition logic, but **does not reinvent the underlying Unicode standard tables or font parsing algorithms**. The external libraries below are confined to their own private modules (guarded by a per-file-path allowlist in `scripts/check-engine-boundary.py`; their types never leak into the public API of `nana-text`):

| External crate | Version (per `Cargo.lock`) | License | Role and the only private module allowed to use it |
| --- | --- | --- | --- |
| `harfrust` | `0.12.0` | MIT | OpenType shaping (GSUB / GPOS), only in `shaping/opentype.rs` |
| `skrifa` | `0.44.0` | MIT OR Apache-2.0 | Variable font axes, named instances, color glyph tables, and `cmap`, only in `font/face.rs` |
| `read-fonts` | `0.41.0` | MIT OR Apache-2.0 | Low-level font table reading shared by `skrifa` and `harfrust` |
| `fontdb` | `0.24.0` | MIT | System font directory scanning and `name` / `OS/2` metadata reading, only in `font/discovery.rs` |
| `ttf-parser` | `0.25.1` | MIT OR Apache-2.0 | Font table parser used by `fontdb` |
| `icu_properties` | `2.3.0` | Unicode-3.0 | Unicode Script, `Emoji_Presentation`, and `Default_Ignorable` properties, only in `font/unicode.rs` |
| `unicode-bidi` | `0.3.18` | MIT OR Apache-2.0 | Unicode Bidirectional Algorithm (UBA), only in `shaping/bidi.rs` |
| `unicode-linebreak` | `0.1.5` | Apache-2.0 | UAX #14 line break opportunity tables, only in `layout/breaks.rs` |
| `unicode-segmentation` | `1.13.3` | MIT OR Apache-2.0 | Grapheme cluster and word boundary segmentation |
| `swash` / `zeno` / `yazi` | `0.2.10` / `0.3.3` / `0.2.1` | Apache-2.0 OR MIT | Glyph outline scaling, path rasterization, and WOFF2 decompression, only in `scene_paint/text/raster.rs` of `nana-ui` |
| `windows` | `0.62.2` | MIT OR Apache-2.0 | Windows only: DirectWrite outline rasterization and reading the system ClearType settings, only in `scene_paint/text/raster_dwrite.rs` (color, bitmap, and sideways glyphs and fallbacks still go through `swash`) |
| `etagere` | `0.2.15` | MIT OR Apache-2.0 | Rectangle packing for the glyph atlas, only in `scene_paint/text/atlas.rs` |

(Note: `fontdb 0.23.0`, also present in the lockfile, comes in through the math formula rendering chain `ratex-svg → ratex-font-loader → ratex-unicode-font` and is not visible to `nana-text`, nor is `nana-text` visible to it.)

## Status of the two removed historical forks

### 1. `cryoglyph` (old GPU text renderer)

Fully removed from the product and from `Cargo.lock`. `SceneWgpuPainter::text` (in `crates/nana-ui/src/scene_paint/text/`) independently implements the whole glyph IR, the rasterization boundary, the raster cache, the generation-refcounted multi-page GPU atlas (`atlas.rs`), the upload queue, and the WGSL text pipeline (`text_atlas.wgsl`). This repository **copies no source code from `cryoglyph`** (the `0.04045 / 12.92 / 1.055 / 2.4` coefficients both share come from the standard sRGB EOTF formula in IEC 61966-2-1).

### 2. `cosmic-text` (old shaping and layout engine)

Fully removed from the product and dev dependencies. The reference engine used for comparison during the migration (`crates/nana-text/tests/reference/`) and its comparison scripts have been deleted as well. The remaining `crates/nana-text/corpus/golden/TX-*.layout.json` files are frozen layout baseline data that the native `nana-text` engine is verified against directly.

## Explicit attribution and ported fragments

Only the following two algorithm fragments in the repository are ported from external open-source projects. Both keep the full original copyright notice and license in the header of the corresponding source file:

1. **`SubpixelBin::split` (`crates/nana-ui/src/scene_paint/text/glyph.rs`)**:
   Ported from `SubpixelBin::new` in `cosmic-text` (`https://github.com/pop-os/cosmic-text`). It deliberately keeps the same quantization thresholds, branch order, and carry-to-whole-pixel logic, so that the same glyph lands in exactly the same subpixel bin before and after the engine switch. Original copyright: `Copyright (c) 2022 System76`, licensed `MIT OR Apache-2.0`.
2. **DirectWrite grayscale / ClearType coverage gamma correction (`crates/nana-ui/src/scene_paint/text/gamma.rs` and `shader/text_atlas.wgsl`)**:
   The `GAMMA_INCORRECT_TARGET_RATIOS` coefficient table, the `gamma_ratios` normalization, and the shader's `enhance_contrast`, `apply_alpha_correction`, and light-on-dark contrast attenuation (`corrected_coverage`) are ported from `dwrite.cpp` and `dwrite.hlsl` in the Windows Terminal AtlasEngine (`https://github.com/microsoft/terminal`). Original copyright: `Copyright (c) Microsoft Corporation`, licensed `MIT`. The subsequent step that converts the gamma-space result into the coverage needed for linear blending is this repository's own implementation.

In addition, the `14°` shear angle for synthetic italics (`OBLIQUE_DEGREES` in `raster.rs`) and the design that splits the atlas into mask and color pages are common industry behavior for compatibility and a general architectural approach, not copied source code.

## Verification commands

```bash
# 1. Verify the dependency graph has no replaced old engine, nana-text/src has no banned identifiers, and the wgpu boundary holds (required in CI)
python3 scripts/check-engine-boundary.py

# 2. Export the external crates and licenses of the all-platform normal + build dependency graph
cargo tree --edges normal,build --workspace --locked --target all --prefix none -f '{p}|{l}' \
  | sed 's/ (\*)//' | sort -u | grep -v "(/"
```
