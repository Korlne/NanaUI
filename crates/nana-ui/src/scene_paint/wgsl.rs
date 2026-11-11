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

/// A framework program, handed to the device as naga IR one entry point at a
/// time.
///
/// Given a WGSL module with several entry points, wgpu compiles every
/// pipeline stage from a copy of the whole module that it cuts down to the
/// stage's entry point, and the pipeline keeps the whole module alive (on
/// Metal its IR, since MSL is written per pipeline). Here the program is
/// parsed and validated once, when its first stage is wanted, and each entry
/// point becomes a module of its own that holds only what that entry point
/// reaches (`naga::compact`). Pipelines sharing an entry point share its
/// module, the device validates only the cut-down IR, and a pipeline keeps
/// only its own stages' code.
///
/// A program that does not parse or validate here (none of the framework's
/// does; the tests compile them all) goes to the device as WGSL, whole,
/// which then reports the error as it always has.
pub(crate) struct Program {
    label: &'static str,
    source: std::borrow::Cow<'static, str>,
    ir: std::cell::OnceCell<Option<naga::Module>>,
    stages: std::cell::RefCell<Vec<(String, Stage)>>,
    whole: std::cell::OnceCell<wgpu::ShaderModule>,
    /// DXIL compiled from this program, while the device takes it; cleared
    /// when the device refuses it.
    #[cfg(target_os = "windows")]
    precompiled: std::cell::Cell<Option<&'static super::precompiled::Family>>,
}

/// One entry point's module, from [`Program::stage`].
#[derive(Clone)]
pub(crate) struct Stage {
    pub(crate) module: wgpu::ShaderModule,
    /// The overrides the module still declares; `None` for the whole program.
    overrides: Option<Vec<String>>,
}

impl Stage {
    /// `constants` less those the stage does not use: a cut-down module no
    /// longer declares them, and a pipeline may only set what its module
    /// declares.
    pub(super) fn constants<'a>(&self, constants: &[(&'a str, f64)]) -> Vec<(&'a str, f64)> {
        constants
            .iter()
            .copied()
            .filter(|(name, _)| {
                self.overrides
                    .as_ref()
                    .is_none_or(|declared| declared.iter().any(|declared| declared == name))
            })
            .collect()
    }
}

impl Program {
    pub(crate) fn new(
        label: &'static str,
        source: impl Into<std::borrow::Cow<'static, str>>,
    ) -> Self {
        Self {
            label,
            source: source.into(),
            ir: std::cell::OnceCell::new(),
            stages: std::cell::RefCell::new(Vec::new()),
            whole: std::cell::OnceCell::new(),
            #[cfg(target_os = "windows")]
            precompiled: std::cell::Cell::new(None),
        }
    }

    /// The program, whose pipelines load `family`'s DXIL when `runs` (the
    /// device takes precompiled stages, [`super::precompiled::runs_on`]) and
    /// the family was compiled from this very program laid out as `layout`.
    /// Only a Windows build names a family, which keeps the DXIL out of
    /// other targets' binaries.
    #[cfg(target_os = "windows")]
    pub(super) fn precompiled(
        self,
        family: &'static super::precompiled::Family,
        runs: bool,
        layout: impl FnOnce() -> u64,
    ) -> Self {
        if runs && family.fresh(&self.source, layout()) {
            self.precompiled.set(Some(family));
        }
        self
    }

    /// The pipeline `build` makes from the stages for `vertex` and
    /// `fragment`, with `constants` for the fragment stage: the precompiled
    /// ones when there are, else cut from the program ([`Self::stage`]).
    /// `build` gets the constants the fragment module takes (none for a
    /// precompiled stage, which was compiled with them).
    pub(super) fn pipeline(
        &self,
        device: &wgpu::Device,
        vertex: &str,
        fragment: &str,
        constants: &[(&str, f64)],
        build: impl Fn(&wgpu::ShaderModule, &wgpu::ShaderModule, &[(&str, f64)]) -> wgpu::RenderPipeline,
    ) -> wgpu::RenderPipeline {
        #[cfg(target_os = "windows")]
        if let Some(family) = self.precompiled.get()
            && family.stage(vertex, &[]).is_some()
            && family.stage(fragment, constants).is_some()
        {
            match super::precompiled::pipeline(
                device,
                family,
                [(vertex, &[]), (fragment, constants)],
                |[vertex, fragment]| build(vertex, fragment, &[]),
            ) {
                Some(pipeline) => return pipeline,
                // Refused once, refused for every pipeline of the program.
                None => self.precompiled.set(None),
            }
        }
        #[cfg(all(test, target_os = "windows"))]
        super::precompiled::tests::requesting(&self.source, vertex, fragment, constants);
        let vertex = self.stage(device, vertex);
        let fragment = self.stage(device, fragment);
        build(
            &vertex.module,
            &fragment.module,
            &fragment.constants(constants),
        )
    }

    /// The program [`joined`] from `files`.
    pub(super) fn joined(label: &'static str, files: &[&str]) -> Self {
        Self::new(label, joined(files))
    }

    /// The program's text, as the device would parse it.
    #[cfg(test)]
    pub(super) fn source(&self) -> &str {
        &self.source
    }

    fn ir(&self) -> Option<&naga::Module> {
        self.ir
            .get_or_init(|| {
                let module = naga::front::wgsl::parse_str(&self.source).ok()?;
                // Compaction requires a valid module. The device validates
                // each stage again against its own capabilities.
                naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::all(),
                )
                .validate(&module)
                .ok()?;
                Some(module)
            })
            .as_ref()
    }

    /// The module for the stage whose entry point is `entry`: only what it
    /// reaches, or the whole program as WGSL when that cannot be cut out.
    pub(crate) fn stage(&self, device: &wgpu::Device, entry: &str) -> Stage {
        if let Some((_, stage)) = self.stages.borrow().iter().find(|(name, _)| name == entry) {
            return stage.clone();
        }
        let stage = match self.ir().and_then(|ir| cut(ir, entry)) {
            Some(ir) => {
                let overrides = ir
                    .overrides
                    .iter()
                    .filter_map(|(_, declared)| declared.name.clone())
                    .collect();
                Stage {
                    module: device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some(self.label),
                        source: wgpu::ShaderSource::Naga(std::borrow::Cow::Owned(ir)),
                    }),
                    overrides: Some(overrides),
                }
            }
            None => Stage {
                module: self.whole(device),
                overrides: None,
            },
        };
        self.stages
            .borrow_mut()
            .push((entry.to_owned(), stage.clone()));
        stage
    }

    fn whole(&self, device: &wgpu::Device) -> wgpu::ShaderModule {
        self.whole
            .get_or_init(|| {
                device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(self.label),
                    source: wgpu::ShaderSource::Wgsl(self.source.clone()),
                })
            })
            .clone()
    }
}

/// `module` with `entry` its only entry point and nothing it does not
/// reach; `None` when it has no such entry point.
pub(super) fn cut(module: &naga::Module, entry: &str) -> Option<naga::Module> {
    let mut module = module.clone();
    module.entry_points.retain(|point| point.name == entry);
    if module.entry_points.is_empty() {
        return None;
    }
    naga::compact::compact(&mut module, naga::compact::KeepUnused::No);
    Some(module)
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

    /// The scene's programs with more than one entry point, joined as their
    /// families join them.
    fn programs() -> Vec<(&'static str, Program)> {
        vec![
            (
                "quad",
                Program::joined(
                    "quad",
                    &[
                        MOTION,
                        COLOR,
                        QUAD,
                        VERTEX,
                        QUAD_PAINT_DATA,
                        QUAD_PAINT,
                        QUAD_SOLID,
                    ],
                ),
            ),
            (
                "mesh",
                Program::joined("mesh", &[TRIANGLE, TRIANGLE_SOLID, PATH, COLOR]),
            ),
            ("chart", Program::joined("chart", &[CHART, COLOR])),
            (
                "group blend",
                Program::joined("group blend", &[COLOR, LAYER, LAYER_BLEND]),
            ),
            ("group", Program::joined("group", &[COLOR, LAYER])),
            ("blit", Program::new("blit", BLIT)),
            ("motion", Program::new("motion", MOTION)),
            (
                "backdrop composite",
                Program::joined(
                    "backdrop composite",
                    &[COLOR, QUAD_PAINT_DATA, BACKDROP_COMPOSITE],
                ),
            ),
        ]
    }

    fn size(module: &naga::Module) -> usize {
        module
            .functions
            .iter()
            .map(|(_, function)| function.expressions.len())
            .chain(
                module
                    .entry_points
                    .iter()
                    .map(|point| point.function.expressions.len()),
            )
            .sum()
    }

    /// Every entry point of every program becomes a valid module of its
    /// own, smaller than the program, with nothing it does not reach.
    #[test]
    fn every_entry_point_cuts_to_a_valid_module_of_its_own() {
        for (name, program) in programs() {
            let ir = program
                .ir()
                .unwrap_or_else(|| panic!("{name} parses and validates"));
            assert!(ir.entry_points.len() > 1, "{name}");
            for point in &ir.entry_points {
                let stage = cut(ir, &point.name).unwrap();
                assert_eq!(stage.entry_points.len(), 1);
                naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::all(),
                )
                .validate(&stage)
                .unwrap_or_else(|error| panic!("{name}::{}: {error:?}", point.name));
                assert!(
                    size(&stage) < size(ir),
                    "{name}::{} carries the whole program",
                    point.name
                );
            }
            assert!(cut(ir, "no_such_entry").is_none());
        }
    }

    /// The program text each family hands the device is the files joined.
    #[test]
    fn a_joined_program_is_its_files_in_a_row() {
        let program = Program::joined("group", &[COLOR, LAYER]);
        assert_eq!(program.source(), format!("{COLOR}\n{LAYER}"));
    }

    /// A stage keeps only the pipeline constants its module still declares;
    /// a program that cannot be cut keeps them all, for the device to judge.
    #[test]
    fn a_stage_keeps_only_the_constants_it_declares() {
        let (device, _queue) = crate::test_gpu::device();
        let mesh = Program::joined("mesh", &[TRIANGLE, TRIANGLE_SOLID, PATH, COLOR]);
        let samples = [("PATH_SAMPLES", 4.0)];
        assert_eq!(
            mesh.stage(&device, "path_fs_main").constants(&samples),
            samples
        );
        assert!(
            mesh.stage(&device, "path_keep_fs_main")
                .constants(&samples)
                .is_empty()
        );
        let broken = Program::new("broken", "fn nope( {");
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        assert_eq!(
            broken.stage(&device, "fs_main").constants(&samples),
            samples
        );
        assert!(pollster::block_on(scope.pop()).is_some());
    }
}
