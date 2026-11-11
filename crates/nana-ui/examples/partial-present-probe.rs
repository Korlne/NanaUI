//! What a frame costs when almost nothing on it changed.
//!
//! Opens one window on the hosted GPU path, fills it with an editor-sized
//! page (wrapped text, a toolbar of buttons) and then changes one small thing
//! per frame:
//!
//! - `caret`: a 2 × 18 px bar flips between the text colour and the page
//!   background, the paint a blinking caret asks for;
//! - `hover`: one toolbar-sized box flips between its surface and hover fill;
//! - `still`: nothing changes, so the painter answers from its retained batch.
//!
//! Every frame is painted and presented in full, as the scene host does
//! today. One frame per 60 Hz period, each waited for before the next, and
//! the run stops after 30 s, so the GPU is never loaded beyond one frame in
//! flight. Two passes per run: the first times the CPU side (runtime flush,
//! paint recording, submit, the wait in acquire, the present call); the
//! second reads GPU timestamps around the painter's work. Prints one JSON
//! object.
//!
//! ```text
//! cargo run --release -p nana-ui --example partial-present-probe \
//!     --features hosted,bundled-fonts,wgpu-interop -- caret
//! ```
//!
//! The JSON also carries the changed node's physical rectangle, the damage a
//! partial present would report for that frame.

#[cfg(target_os = "windows")]
mod probe {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use nana_ui::GpuRenderTarget;
    use nana_ui::runtime::{
        Button, Card, DocumentId, Entity, FlexDirection, FlexWrap, LayoutStyle, LayoutViewport,
        LengthSpec, List, NodeStyle, RuntimeDocument, SemanticColorRole, Text,
    };
    use nana_ui::{
        HostedGpuContext, HostedGpuShared, HostedGpuSurface, HostedSurfaceFrame, NanaTextShaper,
        ScenePaintViewport, SceneWgpuPainter,
    };
    use winit::application::ApplicationHandler;
    use winit::event::WindowEvent;
    use winit::event_loop::{ActiveEventLoop, EventLoop};
    use winit::window::{Window, WindowAttributes, WindowId};

    const LOGICAL: [f64; 2] = [1600.0, 1000.0];
    const WARMUP: usize = 30;
    const FRAMES: usize = 240;
    const PERIOD: Duration = Duration::from_micros(16_667);
    const TIME_CAP: Duration = Duration::from_secs(30);
    const LINE: &str =
        "    let layout = text.layout(constraints, &mut shaper).expect(\"wrapped line\");";

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum Scene {
        Caret,
        Hover,
        Still,
    }

    struct Page {
        document: RuntimeDocument,
        caret: Entity<Card>,
        hover: Entity<Card>,
    }

    fn sized(width: f32, height: f32, background: SemanticColorRole) -> NodeStyle {
        let mut style = NodeStyle {
            background: Some(background),
            ..NodeStyle::default()
        };
        let layout = Arc::make_mut(&mut style.layout);
        layout.width = Some(LengthSpec::Px(width));
        layout.height = Some(LengthSpec::Px(height));
        style
    }

    fn page() -> Page {
        let id = DocumentId::new(1).expect("document id");
        let mut document = RuntimeDocument::new(id);
        let mut root_style = NodeStyle::default();
        *Arc::make_mut(&mut root_style.layout) = LayoutStyle {
            direction: Some(FlexDirection::Row),
            flex_wrap: FlexWrap::Wrap,
            ..LayoutStyle::default()
        };
        let context = document.context_mut();
        let root = context
            .create_component(id, List::new().label("page").style(root_style))
            .expect("root");
        for index in 0..12 {
            let button = context
                .create_component(id, Button::new(format!("Action {index}")))
                .expect("button");
            context.append_child(root, button).expect("button child");
        }
        let hover = context
            .create_component(
                id,
                Card::new().style(sized(120.0, 32.0, SemanticColorRole::Surface)),
            )
            .expect("hover box");
        context.append_child(root, hover).expect("hover child");
        let caret = context
            .create_component(
                id,
                Card::new().style(sized(2.0, 18.0, SemanticColorRole::Text)),
            )
            .expect("caret");
        context.append_child(root, caret).expect("caret child");
        for _ in 0..90 {
            let line = context.create_component(id, Text::new(LINE)).expect("line");
            context.append_child(root, line).expect("line child");
        }
        Page {
            document,
            caret,
            hover,
        }
    }

    impl Page {
        fn step(&mut self, scene: Scene, frame: usize) {
            let on = frame.is_multiple_of(2);
            let (entity, card) = match scene {
                Scene::Still => return,
                Scene::Caret => (
                    self.caret,
                    Card::new().style(sized(
                        2.0,
                        18.0,
                        if on {
                            SemanticColorRole::Text
                        } else {
                            SemanticColorRole::Background
                        },
                    )),
                ),
                Scene::Hover => (
                    self.hover,
                    Card::new().style(sized(
                        120.0,
                        32.0,
                        if on {
                            SemanticColorRole::Hover
                        } else {
                            SemanticColorRole::Surface
                        },
                    )),
                ),
            };
            self.document
                .context_mut()
                .set_component(entity, card)
                .expect("toggle");
        }

        /// The changed node's box in physical pixels: this frame's damage.
        fn damage(&self, scene: Scene, scale: f32) -> Option<[u32; 4]> {
            let entity = match scene {
                Scene::Still => return None,
                Scene::Caret => self.caret,
                Scene::Hover => self.hover,
            };
            let rect = self
                .document
                .context()
                .world()
                .layout_box(entity.stable_id())?;
            let x0 = (rect.x * scale).floor().max(0.0) as u32;
            let y0 = (rect.y * scale).floor().max(0.0) as u32;
            let x1 = ((rect.x + rect.width) * scale).ceil() as u32;
            let y1 = ((rect.y + rect.height) * scale).ceil() as u32;
            Some([x0, y0, x1 - x0, y1 - y0])
        }
    }

    #[derive(Default)]
    struct Samples {
        flush: Vec<Duration>,
        paint: Vec<Duration>,
        submit: Vec<Duration>,
        acquire: Vec<Duration>,
        present: Vec<Duration>,
        interval: Vec<Duration>,
        gpu: Vec<Duration>,
        draw_calls: usize,
        batch_rebuilds: usize,
    }

    fn summary(samples: &[Duration]) -> String {
        if samples.is_empty() {
            return "null".into();
        }
        let mut ms: Vec<f64> = samples.iter().map(|d| d.as_secs_f64() * 1e3).collect();
        ms.sort_by(f64::total_cmp);
        let at = |q: f64| ms[((ms.len() - 1) as f64 * q).round() as usize];
        format!(
            "{{\"p50\":{:.4},\"p90\":{:.4},\"max\":{:.4},\"mean\":{:.4}}}",
            at(0.5),
            at(0.9),
            ms[ms.len() - 1],
            ms.iter().sum::<f64>() / ms.len() as f64
        )
    }

    struct Timestamps {
        queries: wgpu::QuerySet,
        resolved: wgpu::Buffer,
        mapped: wgpu::Buffer,
    }

    impl Timestamps {
        fn new(device: &wgpu::Device) -> Self {
            Self {
                queries: device.create_query_set(&wgpu::QuerySetDescriptor {
                    label: Some("partial-present-probe"),
                    ty: wgpu::QueryType::Timestamp,
                    count: 2,
                }),
                resolved: device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: 16,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
                mapped: device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: 16,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }),
            }
        }

        fn read(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Duration {
            let slice = self.mapped.slice(..);
            slice.map_async(wgpu::MapMode::Read, |result| {
                result.expect("map timestamps")
            });
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("timestamps");
            let ticks = {
                let bytes = slice.get_mapped_range().expect("mapped timestamps");
                let words = bytes.as_chunks::<8>().0;
                [u64::from_le_bytes(words[0]), u64::from_le_bytes(words[1])]
            };
            self.mapped.unmap();
            Duration::from_nanos(
                (ticks[1].saturating_sub(ticks[0]) as f64 * queue.get_timestamp_period() as f64)
                    as u64,
            )
        }
    }

    struct Probe {
        scene: Scene,
        window: Option<Arc<dyn Window>>,
        shared: Option<HostedGpuShared>,
        surface: Option<HostedGpuSurface>,
        painter: Option<SceneWgpuPainter>,
        timestamps: Option<Timestamps>,
        page: Option<Page>,
        shaper: NanaTextShaper,
        frame: usize,
        last_present: Option<Instant>,
        frame_started: Instant,
        run_started: Instant,
        samples: Samples,
        adapter: String,
        physical: [u32; 2],
        damage_rect: Option<[u32; 4]>,
    }

    impl Probe {
        fn frame(&mut self) -> bool {
            let timed = self.frame >= WARMUP + FRAMES;
            let sampling = self.frame >= WARMUP;
            let done = self.frame >= WARMUP + 2 * FRAMES;
            if done {
                return false;
            }
            if self.run_started.elapsed() > TIME_CAP {
                return false;
            }
            self.frame_started = Instant::now();
            let window = self.window.clone().expect("window");
            let scale = window.scale_factor() as f32;
            let shared = self.shared.as_ref().expect("gpu");
            let surface = self.surface.as_mut().expect("surface");
            let painter = self.painter.as_mut().expect("painter");
            let page = self.page.as_mut().expect("page");
            let (width, height) = surface.physical_size();
            self.physical = [width, height];
            let logical = [width as f32 / scale, height as f32 / scale];

            page.step(self.scene, self.frame);
            let started = Instant::now();
            page.document
                .flush(
                    LayoutViewport::new(logical[0], logical[1]),
                    &mut self.shaper,
                )
                .expect("flush");
            let flush = started.elapsed();

            let mut recording = shared.gpu().begin_frame("partial-present-probe");
            let started = Instant::now();
            let frame = match shared.acquire_surface_frame(surface).expect("acquire") {
                HostedSurfaceFrame::Ready(frame) => frame,
                _ => return true,
            };
            let acquire = started.elapsed();
            let target = GpuRenderTarget::from_wgpu(
                shared.gpu(),
                frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default()),
                frame.texture.format(),
                [frame.texture.width(), frame.texture.height()],
            );
            if let (true, Some(stamps)) = (timed, &self.timestamps) {
                recording.wgpu_encoder().write_timestamp(&stamps.queries, 0);
            }
            let started = Instant::now();
            painter
                .paint(
                    page.document.scene(),
                    &mut recording,
                    &target,
                    ScenePaintViewport {
                        logical_size: logical,
                        physical_size: [width, height],
                        scale_factor: scale,
                        scene_origin: [0.0, 0.0],
                        target_origin: [0.0, 0.0],
                        clear_color: [0.012, 0.013, 0.016, 1.0],
                        clear: true,
                    },
                    None,
                    None,
                )
                .expect("paint");
            let paint = started.elapsed();
            if let (true, Some(stamps)) = (timed, &self.timestamps) {
                let encoder = recording.wgpu_encoder();
                encoder.write_timestamp(&stamps.queries, 1);
                encoder.resolve_query_set(&stamps.queries, 0..2, &stamps.resolved, 0);
                encoder.copy_buffer_to_buffer(&stamps.resolved, 0, &stamps.mapped, 0, 16);
            }
            let submission = recording.submit();
            painter.record_submit(&submission);
            let submit = submission.cpu_duration();
            drop(target);
            self.damage_rect = page.damage(self.scene, scale);
            let started = Instant::now();
            shared.present(frame);
            let present = started.elapsed();
            let now = Instant::now();
            let interval = self.last_present.map(|last| now - last);
            self.last_present = Some(now);
            // Bounded load: one frame per 60 Hz period and nothing queued
            // behind it, as an isolated blink or hover frame is.
            let device = shared.gpu().wgpu().device();
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("frame completion");
            if let Some(rest) = PERIOD.checked_sub(self.frame_started.elapsed()) {
                std::thread::sleep(rest);
            }

            if sampling {
                let work = painter.last_gpu_work().unwrap_or_default();
                self.samples.draw_calls = self.samples.draw_calls.max(work.draw_calls);
                self.samples.batch_rebuilds += work.batch_rebuilds;
                if timed {
                    let device = shared.gpu().wgpu().device();
                    let queue = shared.gpu().wgpu().queue();
                    if let Some(stamps) = &self.timestamps {
                        self.samples.gpu.push(stamps.read(device, queue));
                    }
                } else {
                    self.samples.flush.push(flush);
                    self.samples.paint.push(paint);
                    self.samples.submit.push(submit);
                    self.samples.acquire.push(acquire);
                    self.samples.present.push(present);
                    if let Some(interval) = interval {
                        self.samples.interval.push(interval);
                    }
                }
            }
            self.frame += 1;
            true
        }

        fn report(&self) {
            let s = &self.samples;
            println!(
                "{{\"scene\":\"{:?}\",\"damage_rect\":{:?},\"adapter\":\"{}\",\
                 \"physical\":{:?},\"frames\":{},\
                 \"draw_calls\":{},\"batch_rebuilds\":{},\
                 \"flush_ms\":{},\"paint_record_ms\":{},\"submit_ms\":{},\
                 \"acquire_wait_ms\":{},\"present_call_ms\":{},\"frame_interval_ms\":{},\
                 \"gpu_paint_ms\":{}}}",
                self.scene,
                self.damage_rect,
                self.adapter,
                self.physical,
                FRAMES,
                s.draw_calls,
                s.batch_rebuilds,
                summary(&s.flush),
                summary(&s.paint),
                summary(&s.submit),
                summary(&s.acquire),
                summary(&s.present),
                summary(&s.interval),
                summary(&s.gpu),
            );
        }
    }

    impl ApplicationHandler for Probe {
        fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
            if self.window.is_some() {
                return;
            }
            let window: Arc<dyn Window> = Arc::from(
                event_loop
                    .create_window(
                        WindowAttributes::default()
                            .with_title("NanaUI partial present probe")
                            .with_surface_size(winit::dpi::LogicalSize::new(
                                LOGICAL[0], LOGICAL[1],
                            )),
                    )
                    .expect("window"),
            );
            let features =
                wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
            let graphics =
                pollster::block_on(HostedGpuContext::new(window.clone(), features, false))
                    .or_else(|_| {
                        pollster::block_on(HostedGpuContext::new(
                            window.clone(),
                            wgpu::Features::empty(),
                            false,
                        ))
                    })
                    .expect("hosted GPU context");
            let (shared, surface) = graphics.into_parts();
            let info = shared.gpu().wgpu().adapter_info().clone();
            self.adapter = format!("{} ({:?})", info.name, info.backend);
            let device = shared.gpu().wgpu().device();
            self.timestamps = device
                .features()
                .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS)
                .then(|| Timestamps::new(device));
            self.painter = Some(SceneWgpuPainter::new_with_presentation(
                shared.gpu(),
                surface.profile().scene_profile(),
            ));
            self.page = Some(page());
            self.shared = Some(shared);
            self.surface = Some(surface);
            self.window = Some(window.clone());
            window.request_redraw();
        }

        fn window_event(
            &mut self,
            event_loop: &dyn ActiveEventLoop,
            _: WindowId,
            event: WindowEvent,
        ) {
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::RedrawRequested => {
                    if self.frame() {
                        self.window.as_ref().expect("window").request_redraw();
                    } else {
                        self.report();
                        event_loop.exit();
                    }
                }
                _ => {}
            }
        }
    }

    pub fn run(scene: Scene) {
        let probe = Probe {
            scene,
            window: None,
            shared: None,
            surface: None,
            painter: None,
            timestamps: None,
            page: None,
            shaper: NanaTextShaper::default(),
            frame: 0,
            last_present: None,
            frame_started: Instant::now(),
            run_started: Instant::now(),
            samples: Samples::default(),
            adapter: String::new(),
            physical: [0, 0],
            damage_rect: None,
        };
        EventLoop::new()
            .expect("event loop")
            .run_app(probe)
            .expect("run");
    }
}

#[cfg(target_os = "windows")]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let scene = match args
        .iter()
        .find(|arg| !arg.starts_with("--"))
        .map(String::as_str)
    {
        Some("hover") => probe::Scene::Hover,
        Some("still") => probe::Scene::Still,
        _ => probe::Scene::Caret,
    };
    probe::run(scene);
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("partial-present-probe measures the DX12 path and runs on Windows only");
}
