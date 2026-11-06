# General Fragment Materials

`MaterialProgram`, `MaterialRef`, and `MaterialTexture` in `nana-ui-core` are the paint contract shared by text and ordinary shapes. They contain no device, window, or application state. `PaintStyle.background_material`, `border_material`, and `text_material` reference the same kind of material; `RichPaintStyle.material` can override a node's text material.

This stage covers the fill and border of ordinary Quads (including rounded corners and polygon clip) and the glyph fill of Text / RichText. Path, custom Painters, icons, color emoji, and embedded images keep their own paint paths. Shadows, text strokes, and decoration lines stay unchanged.

## Programs and instances

Authors supply a pure WGSL function, with no vertex stage or bindings:

```wgsl
fn material(i: MaterialInput) -> vec4<f32> {
    let stripe = select(0.4, 1.0, fract(i.position.x / 4.0) < 0.5);
    return vec4<f32>(i.color.rgb, stripe);
}
```

Another material can use instance parameters directly, with the same program contract:

```wgsl
fn material(i: MaterialInput) -> vec4<f32> {
    return vec4<f32>(material_param(0u).rgb, 1.0);
}
```

In `MaterialInput`, `position` is in local logical pixels, `uv` is local normalized coordinates, `size` is the local logical size, and `color.rgb` is the resolved linear scRGB color. For a shape, the local region is the whole box and `glyph_uv` is zero; for text, the local region is the current glyph cell, and `glyph_uv` and `uv` are both coordinates within that cell. Coordinates are taken before the presentation transform, so strokes move with the text rather than sliding across window coordinates.

The color input's alpha is 1; the material output's alpha is extra opacity. The framework clamps it to 0–1 and then multiplies it into the original color alpha, glyph or shape coverage, reveal, clipping, and ancestor opacity; a material cannot paint outside the glyph or outline. RGB is not clamped to SDR; the framework handles premultiplication and the final presentation conversion. Explicit span colors and rainbow effects are resolved first and then passed to the text material.

`material_param(index)` reads a vec4 parameter; an out-of-range index takes the last set, and empty parameters read as zero. At most 256 finite vec4 sets. `material_sample0(uv)` / `material_sample1(uv)` sample up to two straight-alpha sRGB textures and return linear RGBA. UV is clamped to 0–1. The maximum texture edge is the smaller of the device limit and 4096, and the RGBA byte count must match the size.

Rust callers use `MaterialProgram::new(wgsl)` and `MaterialRef::new(program)`, then build immutable instances with `with_parameters` / `with_textures`. Clones share the reference; replacing parameters or textures keeps instance identity and produces a new revision. Reloading after serialization produces a new runtime identity.

## Lifecycle and fallback

Material resources are created on first paint; text and shapes on the same device share parameters, texture layouts, and instance bindings. The program cache distinguishes device generation, adapter type, target format, sample count, and program content; programs and bindings with no live references are cleaned up in a later prepare phase. After device replacement they are rebuilt for the new generation.

Materials do not implicitly subscribe to a clock or request frames. For dynamic effects the caller updates parameters and uses the existing presentation/frame scheduling contract. A parameter change only invalidates painting and reuses text shaping and the glyph atlas; glyphs are not re-uploaded in steady state.

On compile failure or invalid resources, the original style is used. Driver compile errors are reported through the `gpu.material.rejected` diagnostic event. Programs are supplied by the host application; an application that loads third-party programs is still responsible for validating source, complexity, and resource budgets, and must not treat driver validation as a sandbox for untrusted code.

A host that runs third-party programs can install a process-wide `MaterialPolicy` with `set_material_policy`, which decides by `MaterialProgram::key()` and device generation:

- `admit` is asked before every compile and before every paint. A rejected program is not compiled and its failure is not cached; shapes fall back to the native pipeline, and the text slot uses the identity material. Admitting it again later recompiles it.
- `charge` reports, after each admitted paint, the device pixels that program shaded: for shapes, the sum of box areas in the batch; for text, the painted extent of the text run; neither exceeds that paint's clip rectangle. The host uses this to implement compute budgets, freezing, or disabling.

With no policy installed, everything is admitted and nothing is charged. The device generation lets the host attribute a device loss to the program painting at the time; the framework itself keeps no budgets, blocklists, or crash records.

GPU evidence lives in `nana-ui-devtools/tests/materials.rs`, covering fill, independent borders, rich-text overrides, texture replacement, static determinism, the identity material, and failure fallback; `material_policy.rs` covers rejection fallback, re-admission within the same painter, and charging. Text parameter reuse tests are in the text module, and device recovery tests cover text materials in a retained scene.

## General presentation mask: PresentationMask

`PresentationMask` is a content-independent coverage contract for a single paint object: a material reference, a start time, a finite duration, and a local / target coordinate space. `progress(now)` returns an exact 0–1 progress, and `PresentationMaskSample` carries the sampled progress and the target rectangle in the object's local coordinates. It owns no text, glyph atlas, clipping, or color, and prescribes no playback logic.

A mask reuses `MaterialRef` and `fn material(i: MaterialInput) -> vec4<f32>`, consuming only the alpha of the return value, clamped to 0–1 and multiplied into the original paint coverage; RGB keeps the original pipeline's result. `progress`, `target_position`, and `target_size` are new public inputs. Local coordinates are logical pixels before the transform; target coordinates are always relative to the target rectangle. For ordinary color materials progress is 1, so existing WGSL needs no changes.

Text is one adapter of this contract: `GlyphPresentation.transitions` references separate enter and exit masks. Enter samples progress at the per-character times of `RevealSchedule`, and exit uses its own finite clock. `TextTransitions.freeze_reveal` freezes the reveal times of the old text, and `viewport` can specify the text-local visible viewport; when omitted, the painter derives it. `Local` uses glyph or inline object coordinates, and `Target` uses whole body viewport coordinates; the extra `glyph_uv` is text adapter information, not a prerequisite of the general mask.

Glyph fill, stroke, shadow, decoration lines, ruby, and color emoji apply the same mask. Quads of inline images / tags use the same `PresentationMaskSample` contract through `QuadSurfacePaint.presentation_masks`, keeping the original image colors; the first round supports stacking two masks. Animation updates only presentation or paint data and reuses text layout, the atlas, and material resources; continuous frames stop once the deadline passes.

This round implements only single paint objects, with no post-processing of arbitrary subtrees. A future container could composite its child content first and treat the result as one paint object, keeping the same local coordinate, target coordinate, and coverage semantics; there is currently no placeholder implementation of container compositing. NanaType's per-character rhythm, sentence-change waits, and crayon algorithm are not part of the framework.
