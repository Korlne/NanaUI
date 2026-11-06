//! An egui panel living inside a NanaUI tree.
//!
//! The egui UI is a `GpuView` node: NanaUI lays it out, clips it and orders
//! it between its siblings, and a `SceneGpuRenderer` records egui's meshes
//! into the same pass and the same device as the rest of the window. Pointer
//! and text input reach egui through `RuntimeProgram::input_event`; the
//! NanaUI side reads and writes the same value egui edits.
//!
//! `cargo run -p nana-ui --example egui-in-nanaui --features hosted,bundled-fonts,wgpu-interop`

use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use nana_ui::runtime::view::{entity_ref, widget, with_refs};
use nana_ui::runtime::{
    Activate, AlignSpec, Button, Card, DocumentId, Entity, FrameworkError, GPU_VIEW_RENDERER,
    GpuView, GpuViewMode, LengthSpec, NodeStyle, RuntimeDocument, Stack, Text,
};
use nana_ui::{
    ButtonKind, CardKind, RoutedInput, RuntimeProgram, RuntimeProgramContext, RuntimeProgramUpdate,
    SceneGpuNode, SceneGpuPrepareContext, SceneGpuRenderContext, SceneGpuRenderer,
    SceneGpuRendererRegistry, ThemeAppearance, WindowDescriptor, WindowHandle, run_runtime,
};
use nana_ui_core::TypeRole;
use nana_ui_platform::{InputPayload, PointerPhase, WheelUnit, WindowEvent, WindowId};

/// What both toolkits look at.
struct Shared {
    strength: f32,
    enabled: bool,
    note: String,
}

/// egui's side: its context, its wgpu renderer and the input queued since
/// the last frame.
struct EguiState {
    ctx: egui::Context,
    renderer: Option<(u64, wgpu::TextureFormat, egui_wgpu::Renderer)>,
    events: Vec<egui::Event>,
    modifiers: egui::Modifiers,
    /// Where NanaUI placed the node, in logical window coordinates.
    bounds: egui::Rect,
    scale: f32,
    started: Instant,
    /// Lets egui ask for the next frame (its first pass only measures).
    window: Option<WindowHandle>,
}

#[derive(Clone)]
struct EguiNode {
    state: Arc<Mutex<EguiState>>,
    shared: Arc<Mutex<Shared>>,
}

impl std::fmt::Debug for EguiNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EguiNode")
    }
}

impl EguiNode {
    /// egui's controls, inset inside the node NanaUI laid out. The card
    /// around them is NanaUI's, so egui draws no frame of its own.
    fn ui(ctx: &egui::Context, bounds: egui::Rect, shared: &mut Shared) {
        egui::Area::new(egui::Id::unique("nana-egui"))
            .fixed_pos(bounds.min)
            .show(ctx, |ui| {
                ui.set_clip_rect(bounds);
                ui.set_width(bounds.width());
                ui.spacing_mut().item_spacing = egui::vec2(12.0, 14.0);
                ui.spacing_mut().slider_width = (bounds.width() - 140.0).max(120.0);
                egui::Grid::new(egui::Id::unique("nana-egui-grid"))
                    .num_columns(2)
                    .spacing([16.0, 14.0])
                    .show(ui, |ui| {
                        ui.label("强度");
                        ui.add(
                            egui::Slider::new(&mut shared.strength, 0.0..=1.0)
                                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
                        );
                        ui.end_row();
                        ui.label("启用");
                        ui.checkbox(&mut shared.enabled, "");
                        ui.end_row();
                        ui.label("备注");
                        ui.add(
                            egui::TextEdit::singleline(&mut shared.note)
                                .desired_width(bounds.width() - 72.0),
                        );
                        ui.end_row();
                        ui.label("进度");
                        ui.add(
                            egui::ProgressBar::new(shared.strength)
                                .desired_width(bounds.width() - 72.0),
                        );
                        ui.end_row();
                    });
            });
    }
}

impl SceneGpuRenderer for EguiNode {
    fn prepare(&self, _node: &SceneGpuNode, context: SceneGpuPrepareContext<'_>) {
        let mut state = self.state.lock().unwrap();
        let b = context.bounds;
        state.bounds =
            egui::Rect::from_min_size(egui::pos2(b.x, b.y), egui::vec2(b.width, b.height));
        state.scale = context.scale_factor;
    }

    fn render(&self, _node: &SceneGpuNode, mut context: SceneGpuRenderContext<'_>) {
        let mut state = self.state.lock().unwrap();
        let state = &mut *state;
        let scale = state.scale.max(0.5);
        let [width, height] = context.dest_size;

        // egui sees the whole window and keeps its own UI inside the node's
        // rectangle, so its scissor rects line up with the target.
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width as f32 / scale, height as f32 / scale),
            )),
            time: Some(state.started.elapsed().as_secs_f64()),
            events: std::mem::take(&mut state.events),
            ..Default::default()
        };
        state.ctx.set_pixels_per_point(scale);
        let bounds = state.bounds;
        let output = {
            let mut shared = self.shared.lock().unwrap();
            state
                .ctx
                .run_pass(raw, |ctx| Self::ui(ctx, bounds, &mut shared))
        };
        let primitives = state.ctx.tessellate(output.shapes, output.pixels_per_point);
        let repaint_now = output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .is_some_and(|viewport| viewport.repaint_delay.is_zero());
        if repaint_now && let Some(window) = &state.window {
            drop(window.request_redraw());
        }

        let gpu = context.gpu.wgpu();
        let format = context.target_format.wgpu();
        let generation = context.gpu.generation().get();
        if !matches!(&state.renderer, Some((g, f, _)) if *g == generation && *f == format) {
            let renderer = egui_wgpu::Renderer::new(
                gpu.device(),
                format,
                egui_wgpu::RendererOptions::default(),
            );
            state.renderer = Some((generation, format, renderer));
        }
        let (_, _, renderer) = state.renderer.as_mut().unwrap();
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                renderer.update_texture(gpu.device(), gpu.queue(), *id, delta);
            }
        }
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: output.pixels_per_point,
        };
        let extra = renderer.update_buffers(
            gpu.device(),
            gpu.queue(),
            context.wgpu_encoder(),
            &primitives,
            &screen,
        );
        debug_assert!(extra.is_empty(), "no egui paint callbacks are used");
        // egui sets its own scissor per mesh; every mesh is clipped to the
        // node's rectangle by `EguiNode::ui`.
        let target = context.wgpu_target().clone();
        let pass = context
            .wgpu_encoder()
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        renderer.render(&mut pass.forget_lifetime(), &primitives, &screen);
        let mut textures = output.textures_delta;
        for id in &textures.free {
            renderer.free_texture(id);
        }
        // Every delta above was applied.
        textures.clear();
    }
}

const EGUI_SLOT: u64 = 1;

/// An egui context that draws CJK text with the face NanaUI bundles.
fn egui_context() -> egui::Context {
    let ctx = egui::Context::default();
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "noto-sans-sc".to_owned(),
        Arc::new(egui::FontData::from_static(
            nana_ui_core::fonts::UI_FONT_REGULAR,
        )),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("noto-sans-sc".to_owned());
    }
    ctx.set_fonts(fonts);
    ctx.set_visuals(nana_visuals(ThemeAppearance::Dark));
    ctx
}

fn color(c: nana_ui::SemanticColor) -> egui::Color32 {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    egui::Color32::from_rgba_unmultiplied(byte(c.r), byte(c.g), byte(c.b), byte(c.a))
}

/// egui visuals taken from NanaUI's theme palette, so both halves of the
/// window read as one interface.
fn nana_visuals(theme: ThemeAppearance) -> egui::Visuals {
    let palette = theme.palette();
    let mut visuals = if theme == ThemeAppearance::Dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    let radius = egui::CornerRadius::same(6);
    visuals.override_text_color = Some(color(palette.text));
    visuals.panel_fill = egui::Color32::TRANSPARENT;
    visuals.window_fill = color(palette.surface);
    // Text fields and slider rails sit on the raised tone, like NanaUI inputs.
    visuals.extreme_bg_color = color(palette.raised);
    visuals.selection.bg_fill = color(palette.accent);
    visuals.selection.stroke.color = color(palette.accent_strong);
    visuals.hyperlink_color = color(palette.accent);
    for (widget, fill, stroke) in [
        (
            &mut visuals.widgets.noninteractive,
            palette.surface,
            palette.border_soft,
        ),
        (&mut visuals.widgets.inactive, palette.hover, palette.border),
        (
            &mut visuals.widgets.hovered,
            palette.active,
            palette.border_strong,
        ),
        (
            &mut visuals.widgets.active,
            palette.accent_soft,
            palette.accent,
        ),
        (
            &mut visuals.widgets.open,
            palette.active,
            palette.border_strong,
        ),
    ] {
        widget.bg_fill = color(fill);
        widget.weak_bg_fill = color(fill);
        widget.bg_stroke.color = color(stroke);
        widget.corner_radius = radius;
    }
    visuals
}

/// The egui node fills the width of its card.
fn egui_node_style() -> NodeStyle {
    let mut style = NodeStyle::default();
    let layout = Arc::make_mut(&mut style.layout);
    layout.width = Some(LengthSpec::Fill);
    layout.height = Some(LengthSpec::Px(160.0));
    style
}

/// A card that takes the remaining width of the row.
fn grow(mut card: Card) -> Card {
    let layout = Arc::make_mut(&mut card.style.layout);
    layout.width = Some(LengthSpec::Fill);
    layout.min_width = Some(LengthSpec::Px(0.0));
    layout.flex_grow = Some(1.0);
    layout.flex_shrink = Some(1.0);
    layout.allow_shrink = true;
    card
}

/// A card of a fixed width.
fn fixed(mut card: Card, width: f32) -> Card {
    Arc::make_mut(&mut card.style.layout).width = Some(LengthSpec::Px(width));
    card
}

struct Demo {
    document: RuntimeDocument,
    egui: EguiNode,
    readout: [Entity<Text>; 3],
    view: Entity<GpuView>,
}

fn readout(shared: &Shared) -> [String; 3] {
    let note = if shared.note.is_empty() {
        "（空）"
    } else {
        shared.note.as_str()
    };
    [
        format!("强度　{:.0}%", shared.strength * 100.0),
        format!(
            "状态　{}",
            if shared.enabled {
                "已启用"
            } else {
                "未启用"
            }
        ),
        format!("备注　{note}"),
    ]
}

impl RuntimeProgram for Demo {
    type Message = ();
    type Error = Infallible;

    fn initialize(
        context: &RuntimeProgramContext<Self::Message>,
    ) -> Result<(Self, Vec<Self::Message>), Self::Error> {
        let shared = Arc::new(Mutex::new(Shared {
            strength: 0.4,
            enabled: true,
            note: String::from("混合界面"),
        }));
        let egui = EguiNode {
            state: Arc::new(Mutex::new(EguiState {
                ctx: egui_context(),
                renderer: None,
                events: Vec::new(),
                modifiers: egui::Modifiers::default(),
                bounds: egui::Rect::NOTHING,
                scale: 1.0,
                started: Instant::now(),
                window: Some(context.window()),
            })),
            shared: Arc::clone(&shared),
        };
        let document_id = DocumentId::new(1).expect("document id");
        let mut document = RuntimeDocument::new(document_id);
        let initial = readout(&shared.lock().unwrap());
        let (_, (readout_text, view)) = document
            .context_mut()
            .mount_view_root(document_id, || {
                let readout_text = [entity_ref::<Text>(), entity_ref::<Text>(), entity_ref::<Text>()];
                let view = entity_ref::<GpuView>();
                let reset_shared = Arc::clone(&shared);
                let root = widget(Stack::column(20.0).padding(28.0)).children((
                    widget(Stack::column(6.0)).children((
                        widget(Text::new("NanaUI + egui").typography(TypeRole::Heading)),
                        widget(
                            Text::new(
                                "右侧面板由 egui 绘制，它是 NanaUI 树上的一个 GpuView 节点。",
                            )
                            .typography(TypeRole::Meta),
                        ),
                    )),
                    widget(Stack::row(16.0).with_layout(|layout| {
                        layout.width = Some(LengthSpec::Fill);
                        layout.align_items = AlignSpec::Stretch;
                    }))
                    .children((
                        widget(fixed(
                            Card::new().title("NanaUI").kind(CardKind::Outlined).padding(20.0),
                            260.0,
                        ))
                        .children((
                            widget(Stack::column(8.0).with_layout(|layout| {
                                layout.padding_bottom = Some(LengthSpec::Px(12.0));
                            }))
                            .children((
                                widget(Text::new(initial[0].clone())).entity_ref(readout_text[0]),
                                widget(Text::new(initial[1].clone())).entity_ref(readout_text[1]),
                                widget(Text::new(initial[2].clone())).entity_ref(readout_text[2]),
                            )),
                            widget(Button::new("重置").kind(ButtonKind::Primary)).on_cx(
                                move |_button, _event: &Activate, cx| {
                                    let mut shared = reset_shared.lock().unwrap();
                                    shared.strength = 0.0;
                                    shared.enabled = false;
                                    shared.note.clear();
                                    cx.dispatch_program(());
                                },
                            ),
                        )),
                        widget(grow(
                            Card::new().title("egui").kind(CardKind::Outlined).padding(20.0),
                        ))
                        .children(
                            widget(
                                GpuView::new(EGUI_SLOT)
                                    .mode(GpuViewMode::Standalone)
                                    .style(egui_node_style()),
                            )
                            .entity_ref(view),
                        ),
                    )),
                    widget(
                        Text::new(
                            "两边读写同一份状态。egui 的网格与 NanaUI 共用一个 WGPU 设备，在同一帧里绘制。",
                        )
                        .typography(TypeRole::Meta),
                    ),
                ));
                with_refs(root, (readout_text, view))
            })
            .expect("mount");
        Ok((
            Self {
                document,
                egui,
                readout: readout_text,
                view,
            },
            Vec::new(),
        ))
    }

    fn with_document<R>(
        &self,
        id: WindowId,
        f: impl FnOnce(&RuntimeDocument) -> R,
    ) -> Result<Option<R>, nana_ui::DocumentAccessError> {
        Ok((id == WindowId::PRIMARY).then(|| f(&self.document)))
    }

    fn with_document_mut<R>(
        &mut self,
        id: WindowId,
        f: impl FnOnce(&mut RuntimeDocument) -> R,
    ) -> Result<Option<R>, nana_ui::DocumentAccessError> {
        Ok((id == WindowId::PRIMARY).then(|| f(&mut self.document)))
    }

    fn update(
        &mut self,
        _message: Self::Message,
        _context: &RuntimeProgramContext<Self::Message>,
    ) -> RuntimeProgramUpdate {
        self.sync();
        RuntimeProgramUpdate::redraw_all()
    }

    fn scene_gpu_renderers(&self, _id: WindowId) -> Option<SceneGpuRendererRegistry> {
        let mut registry = SceneGpuRendererRegistry::new();
        registry.insert(GPU_VIEW_RENDERER, Arc::new(self.egui.clone()));
        Some(registry)
    }

    fn theme(&self) -> Arc<nana_ui::CompiledTheme> {
        nana_ui::builtin_theme_arc(ThemeAppearance::Dark)
    }

    fn window_event(
        &mut self,
        event: WindowEvent,
        _context: &RuntimeProgramContext<Self::Message>,
    ) -> RuntimeProgramUpdate {
        match event {
            WindowEvent::CloseRequested { .. } => RuntimeProgramUpdate::exit(),
            _ => RuntimeProgramUpdate::default(),
        }
    }

    fn input_event(
        &mut self,
        id: WindowId,
        input: RoutedInput<'_>,
        _context: &RuntimeProgramContext<Self::Message>,
    ) -> Result<RuntimeProgramUpdate, FrameworkError> {
        {
            let mut state = self.egui.state.lock().unwrap();
            let inside = |x: f32, y: f32| state.bounds.contains(egui::pos2(x, y));
            let mut forwarded = Vec::new();
            match &input.event.payload {
                InputPayload::Pointer(p) => {
                    let pos = egui::pos2(p.x, p.y);
                    let modifiers = egui::Modifiers {
                        alt: p.modifiers.alt,
                        ctrl: p.modifiers.control,
                        shift: p.modifiers.shift,
                        mac_cmd: p.modifiers.meta && cfg!(target_os = "macos"),
                        command: if cfg!(target_os = "macos") {
                            p.modifiers.meta
                        } else {
                            p.modifiers.control
                        },
                    };
                    match p.phase {
                        PointerPhase::Move => forwarded.push(egui::Event::PointerMoved(pos)),
                        PointerPhase::Down if inside(p.x, p.y) => {
                            forwarded.push(egui::Event::PointerButton {
                                pos,
                                button: egui::PointerButton::Primary,
                                pressed: true,
                                modifiers,
                            })
                        }
                        PointerPhase::Up | PointerPhase::Cancel => {
                            forwarded.push(egui::Event::PointerButton {
                                pos,
                                button: egui::PointerButton::Primary,
                                pressed: false,
                                modifiers,
                            })
                        }
                        _ => {}
                    }
                    state.modifiers = modifiers;
                }
                InputPayload::PointerLeave { .. } => forwarded.push(egui::Event::PointerGone),
                InputPayload::Wheel(w) if inside(w.x, w.y) => {
                    forwarded.push(egui::Event::MouseWheel {
                        unit: match w.unit {
                            WheelUnit::Pixels => egui::MouseWheelUnit::Point,
                            WheelUnit::Lines => egui::MouseWheelUnit::Line,
                        },
                        delta: egui::vec2(-w.delta_x, -w.delta_y),
                        modifiers: state.modifiers,
                        phase: egui::TouchPhase::Move,
                        source: egui::MouseWheelSource::Unknown,
                    })
                }
                InputPayload::Text(text) if state.ctx.egui_wants_keyboard_input() => {
                    forwarded.push(egui::Event::Text(text.text.clone()))
                }
                _ => {}
            }
            state.events.extend(forwarded);
        }
        self.sync();
        Ok(RuntimeProgramUpdate::redraw(id))
    }
}

impl Demo {
    /// Show what egui changed in a NanaUI label, and tell the egui node its
    /// content changed when NanaUI changed the value.
    fn sync(&mut self) {
        let lines = readout(&self.egui.shared.lock().unwrap());
        for (entity, text) in self.readout.into_iter().zip(lines) {
            let _ = self
                .document
                .context_mut()
                .update_component(entity, |label, _| {
                    if label.value != text {
                        label.value = text;
                    }
                });
        }
        let _ = self
            .document
            .context_mut()
            .update_component(self.view, |view, _| {
                view.invalidate_content();
            });
    }
}

fn main() -> Result<(), nana_ui::HostedRunError> {
    run_runtime::<Demo>(
        WindowDescriptor::new("NanaUI + egui")
            .initial_size(920.0, 410.0)
            .system_caption(true),
    )
}
