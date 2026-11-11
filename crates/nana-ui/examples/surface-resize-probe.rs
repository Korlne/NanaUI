//! Measures what a window resize costs while other GPU work is in flight.
//!
//! A Win32 window is resized `--resizes` times (30 by default); each step reconfigures its
//! surface and presents one frame. Meanwhile a producer thread keeps the same
//! queue busy with offscreen passes (`--load` draws of a fragment shader per
//! submit, 1 by default), the way a video or model producer does. The producer
//! keeps one submission in flight and the run aborts after 20 s, so the desktop
//! stays responsive; raise `--load` step by step. The probe reports how
//! long `Surface::configure` and each frame take, the longest producer submit,
//! and how many configures WGPU rejected.
//!
//! `--guard` serializes producer submits against configure with a lock, the
//! way NanaUI's former submission guard did.
//!
//! `--frame-context` (needs the `wgpu-interop` feature) routes both sides
//! through `nana-gpu`: the producer records `begin_producer_frame` frames and
//! submits them back to back with no wait of its own, so only the producer
//! submission budget bounds it; the window frames use `begin_frame`.
//!
//! `--load` is clamped to 16 and `--resizes` to 200: 16 is the highest load
//! verified to keep the desktop responsive. Never run two GPU probes at once.
//!
//! ```text
//! cargo run -p nana-ui --example surface-resize-probe --features gpu --release -- --backend dx12
//! cargo run -p nana-ui --example surface-resize-probe --features gpu --release -- --backend vulkan --guard
//! cargo run -p nana-ui --example surface-resize-probe --features gpu,wgpu-interop --release -- --frame-context --load 4
//! ```

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("surface-resize-probe drives a Win32 window and runs on Windows only");
}

#[cfg(target_os = "windows")]
fn main() {
    probe::run();
}

#[cfg(target_os = "windows")]
mod probe {
    use std::num::NonZeroIsize;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex, RwLock};
    use std::time::{Duration, Instant};

    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, MSG,
        PM_REMOVE, PeekMessageW, RegisterClassW, SWP_NOMOVE, SWP_NOZORDER, SetWindowPos,
        TranslateMessage, WINDOW_EX_STYLE, WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
    };
    use windows::core::w;

    const OFFSCREEN: u32 = 1024;
    /// The whole run aborts past this, so a heavy setting cannot hold the GPU.
    const WALL_CLOCK_CAP: Duration = Duration::from_secs(20);
    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    /// Highest `--load` verified to leave the desktop responsive.
    const MAX_LOAD: u32 = 16;
    const MAX_RESIZES: u32 = 200;

    struct Options {
        backend: wgpu::Backends,
        resizes: u32,
        load: u32,
        producer: bool,
        guard: bool,
        frame_context: bool,
    }

    fn options() -> Options {
        let mut options = Options {
            backend: wgpu::Backends::DX12,
            resizes: 30,
            load: 1,
            producer: true,
            guard: false,
            frame_context: false,
        };
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--backend" => {
                    options.backend = match args.next().as_deref() {
                        Some("dx12") => wgpu::Backends::DX12,
                        Some("vulkan") => wgpu::Backends::VULKAN,
                        Some("gl") => wgpu::Backends::GL,
                        other => panic!("unknown backend {other:?}"),
                    }
                }
                "--resizes" => options.resizes = args.next().unwrap().parse().unwrap(),
                "--load" => options.load = args.next().unwrap().parse().unwrap(),
                "--no-producer" => options.producer = false,
                "--guard" => options.guard = true,
                "--frame-context" => options.frame_context = true,
                other => panic!("unknown argument {other}"),
            }
        }
        if options.load > MAX_LOAD || options.resizes > MAX_RESIZES {
            println!("clamping to --load <= {MAX_LOAD} and --resizes <= {MAX_RESIZES}");
        }
        options.load = options.load.clamp(1, MAX_LOAD);
        options.resizes = options.resizes.clamp(1, MAX_RESIZES);
        assert!(
            !(options.frame_context && options.guard),
            "--guard applies to the raw producer only"
        );
        assert!(
            !options.frame_context || cfg!(feature = "wgpu-interop"),
            "--frame-context needs --features gpu,wgpu-interop"
        );
        options
    }

    extern "system" fn window_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        unsafe { DefWindowProcW(window, message, wparam, lparam) }
    }

    fn create_window() -> HWND {
        unsafe {
            let instance = GetModuleHandleW(None).unwrap();
            let class = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: instance.into(),
                lpszClassName: w!("NanaSurfaceResizeProbe"),
                ..Default::default()
            };
            RegisterClassW(&class);
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("NanaSurfaceResizeProbe"),
                w!("surface-resize-probe"),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                100,
                100,
                800,
                600,
                None,
                None,
                Some(instance.into()),
                None,
            )
            .unwrap()
        }
    }

    fn pump() {
        unsafe {
            let mut message = MSG::default();
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    fn client_size(window: HWND) -> (u32, u32) {
        let mut rect = RECT::default();
        unsafe { GetClientRect(window, &mut rect).unwrap() };
        (
            (rect.right - rect.left).max(1) as u32,
            (rect.bottom - rect.top).max(1) as u32,
        )
    }

    const SHADER: &str = r#"
@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

@fragment
fn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    var v = p.x * 0.001 + p.y * 0.002;
    for (var i = 0; i < 24; i++) {
        v = sin(v * 1.7 + 0.3) * cos(v + f32(i) * 0.01);
    }
    return vec4<f32>(v, v * 0.5, 0.25, 1.0);
}
"#;

    #[derive(Default)]
    struct Samples(Vec<f64>);

    impl Samples {
        fn push(&mut self, duration: Duration) {
            self.0.push(duration.as_secs_f64() * 1000.0);
        }

        fn summary(&self) -> String {
            if self.0.is_empty() {
                return "n=0".into();
            }
            let mut sorted = self.0.clone();
            sorted.sort_by(f64::total_cmp);
            let at = |q: f64| sorted[((sorted.len() - 1) as f64 * q).round() as usize];
            let mean = sorted.iter().sum::<f64>() / sorted.len() as f64;
            format!(
                "n={} mean={:.2}ms p50={:.2}ms p95={:.2}ms max={:.2}ms",
                sorted.len(),
                mean,
                at(0.5),
                at(0.95),
                sorted[sorted.len() - 1]
            )
        }
    }

    pub fn run() {
        let options = options();
        // A hang inside the driver cannot be aborted from the loop: end the
        // process if the run outlives its cap by a margin.
        std::thread::spawn(|| {
            std::thread::sleep(WALL_CLOCK_CAP + Duration::from_secs(10));
            println!("aborted: watchdog fired, the run hung");
            std::process::exit(2);
        });
        let window = create_window();
        pump();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: options.backend,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let mut handle = wgpu::rwh::Win32WindowHandle::new(
            NonZeroIsize::new(window.0 as isize).expect("window handle"),
        );
        let module = unsafe { GetModuleHandleW(None) }.unwrap();
        handle.hinstance = NonZeroIsize::new(module.0 as isize);
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: Some(wgpu::rwh::RawDisplayHandle::Windows(
                    wgpu::rwh::WindowsDisplayHandle::new(),
                )),
                raw_window_handle: wgpu::rwh::RawWindowHandle::Win32(handle),
            })
        }
        .expect("surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .expect("adapter");
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let errors = Arc::new(AtomicU64::new(0));
        let first_error = Arc::new(Mutex::new(None::<String>));
        device.on_uncaptured_error({
            let errors = errors.clone();
            let first_error = first_error.clone();
            Arc::new(move |error: wgpu::Error| {
                errors.fetch_add(1, Ordering::Relaxed);
                first_error
                    .lock()
                    .unwrap()
                    .get_or_insert_with(|| error.to_string());
            })
        });

        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities.formats[0];
        let present_mode = if capabilities
            .present_modes
            .contains(&wgpu::PresentMode::Mailbox)
        {
            wgpu::PresentMode::Mailbox
        } else {
            wgpu::PresentMode::Fifo
        };
        let mut configuration = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: Default::default(),
            width: 800,
            height: 600,
            present_mode,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        let (width, height) = client_size(window);
        configuration.width = width;
        configuration.height = height;
        surface.configure(&device, &configuration);

        let gpu = Gpu::new(options.frame_context, &adapter, &device, &queue);

        let gate = Arc::new(RwLock::new(()));
        let stop = Arc::new(AtomicBool::new(false));
        let producer = options.producer.then(|| {
            let device = device.clone();
            let queue = queue.clone();
            let gate = gate.clone();
            let stop = stop.clone();
            let (guard, load) = (options.guard, options.load);
            let gpu = gpu.clone();
            std::thread::spawn(move || produce(&device, &queue, &gpu, &gate, &stop, guard, load))
        });
        // Let the producer reach its steady state.
        std::thread::sleep(Duration::from_millis(300));

        let mut configure = Samples::default();
        let mut frame = Samples::default();
        let mut retries = 0u64;
        let errors_before = errors.load(Ordering::Relaxed);
        let started = Instant::now();
        let mut completed = 0;
        for step in 0..options.resizes {
            if started.elapsed() > WALL_CLOCK_CAP {
                println!("aborted: wall-clock cap reached after {step} resizes");
                break;
            }
            completed = step + 1;
            let width = 640 + (step % 23) * 17;
            let height = 480 + (step % 17) * 13;
            unsafe {
                SetWindowPos(
                    window,
                    None,
                    0,
                    0,
                    width as i32,
                    height as i32,
                    SWP_NOMOVE | SWP_NOZORDER,
                )
                .unwrap()
            };
            pump();
            let (width, height) = client_size(window);
            configuration.width = width;
            configuration.height = height;

            let frame_start = Instant::now();
            loop {
                let before = errors.load(Ordering::Relaxed);
                let t = Instant::now();
                {
                    let _gate = options.guard.then(|| gate.write().unwrap());
                    surface.configure(&device, &configuration);
                }
                configure.push(t.elapsed());
                if errors.load(Ordering::Relaxed) == before {
                    break;
                }
                retries += 1;
                assert!(retries < 10_000, "configure never succeeded");
            }
            let texture = match surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(texture)
                | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
                other => panic!("acquire failed: {other:?}"),
            };
            let view = texture.texture.create_view(&Default::default());
            let mut recording = gpu.begin(&device, false);
            recording
                .encoder()
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("resize frame"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: f64::from(step % 7) / 7.0,
                                g: 0.3,
                                b: 0.5,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });
            {
                let _gate = options.guard.then(|| gate.read().unwrap());
                recording.submit(&queue);
            }
            queue.present(texture);
            frame.push(frame_start.elapsed());
        }
        let elapsed = started.elapsed();
        stop.store(true, Ordering::Relaxed);
        let producer = producer.map(|producer| producer.join().unwrap());
        let _ = device.poll(wgpu::PollType::wait_indefinitely());

        let info = adapter.get_info();
        println!(
            "backend={:?} adapter={:?} present={:?} resizes={} load={} producer={} guard={} frame_context={}",
            info.backend,
            info.name,
            present_mode,
            options.resizes,
            options.load,
            options.producer,
            options.guard,
            options.frame_context
        );
        println!("configure: {}", configure.summary());
        println!("frame:     {}", frame.summary());
        println!(
            "total: {:.0}ms for {} resizes; rejected configures: {} (errors: {})",
            elapsed.as_secs_f64() * 1000.0,
            completed,
            retries,
            errors.load(Ordering::Relaxed) - errors_before
        );
        if let Some(error) = first_error.lock().unwrap().as_ref() {
            println!("first error: {}", error.lines().next().unwrap_or_default());
        }
        if let Some((submits, stalls)) = producer {
            println!(
                "producer submits: {submits}; begin+submit call: {}",
                stalls.summary()
            );
        }
        gpu.report();
        unsafe {
            let _ = DestroyWindow(window);
        }
    }

    /// The probe's two submit paths: raw WGPU, or `nana-gpu` frames.
    #[derive(Clone)]
    enum Gpu {
        Raw,
        #[cfg(feature = "wgpu-interop")]
        Frames(nana_gpu::GpuContext),
    }

    enum Recording {
        Raw(wgpu::CommandEncoder),
        #[cfg(feature = "wgpu-interop")]
        Frame(nana_gpu::FrameContext),
    }

    impl Gpu {
        #[cfg_attr(not(feature = "wgpu-interop"), allow(unused_variables))]
        fn new(
            frame_context: bool,
            adapter: &wgpu::Adapter,
            device: &wgpu::Device,
            queue: &wgpu::Queue,
        ) -> Self {
            #[cfg(feature = "wgpu-interop")]
            if frame_context {
                return Self::Frames(nana_gpu::GpuContext::from_wgpu(
                    adapter.clone(),
                    device.clone(),
                    queue.clone(),
                ));
            }
            Self::Raw
        }

        #[cfg_attr(not(feature = "wgpu-interop"), allow(unused_variables))]
        fn begin(&self, device: &wgpu::Device, producer: bool) -> Recording {
            match self {
                Self::Raw => Recording::Raw(device.create_command_encoder(&Default::default())),
                #[cfg(feature = "wgpu-interop")]
                Self::Frames(gpu) => Recording::Frame(if producer {
                    gpu.begin_producer_frame("probe producer")
                } else {
                    gpu.begin_frame("probe window")
                }),
            }
        }

        fn report(&self) {
            #[cfg(feature = "wgpu-interop")]
            if let Self::Frames(gpu) = self {
                let stats = gpu.policy().stats();
                println!(
                    "producer budget {}: peak in flight {}, producer waits {}, window waits {}, \
                     frame slot waits {} stalls {}",
                    gpu.policy().producer_submission_budget(),
                    stats.producer_submissions_peak,
                    stats.producer_submission_waits,
                    stats.presentation_submission_waits,
                    stats.frame_slot_waits,
                    stats.frame_slot_stalls,
                );
            }
        }
    }

    impl Recording {
        fn encoder(&mut self) -> &mut wgpu::CommandEncoder {
            match self {
                Self::Raw(encoder) => encoder,
                #[cfg(feature = "wgpu-interop")]
                Self::Frame(frame) => frame.wgpu_encoder(),
            }
        }

        /// Submits. The raw path returns its index for the producer's own
        /// one-in-flight wait; the frame path none (its budget bounds it).
        fn submit(self, queue: &wgpu::Queue) -> Option<wgpu::SubmissionIndex> {
            match self {
                Self::Raw(encoder) => Some(queue.submit([encoder.finish()])),
                #[cfg(feature = "wgpu-interop")]
                Self::Frame(frame) => {
                    frame.submit();
                    None
                }
            }
        }
    }

    fn produce(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        gpu: &Gpu,
        gate: &RwLock<()>,
        stop: &AtomicBool,
        guard: bool,
        load: u32,
    ) -> (u64, Samples) {
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("producer target"),
            size: wgpu::Extent3d {
                width: OFFSCREEN,
                height: OFFSCREEN,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("producer load"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("producer load"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(FORMAT.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let mut submits = 0;
        let mut stalls = Samples::default();
        while !stop.load(Ordering::Relaxed) {
            let t = Instant::now();
            let mut recording = gpu.begin(device, true);
            {
                let mut pass = recording
                    .encoder()
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("producer load"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        ..Default::default()
                    });
                pass.set_pipeline(&pipeline);
                for _ in 0..load {
                    pass.draw(0..3, 0..1);
                }
            }
            let index = {
                let _gate = guard.then(|| gate.read().unwrap());
                recording.submit(queue)
            };
            stalls.push(t.elapsed());
            submits += 1;
            // Raw path: keep one submission in flight, waiting for it before
            // the next, so the producer never queues ahead of the desktop
            // compositor. The frame path relies on the producer budget alone.
            if let Some(index) = index {
                let _ = device.poll(wgpu::PollType::Wait {
                    submission_index: Some(index),
                    timeout: Some(Duration::from_secs(2)),
                });
            }
        }
        (submits, stalls)
    }
}
