//! Out-of-process CPU stack sampler for Windows, for attributing a bench's
//! frame time to NanaUI, wgpu-core and wgpu-hal/driver without ETW (which
//! needs an elevated session). Measurement only: nothing here runs in a
//! product process.
//!
//! ```text
//! cargo run --release -p nana-ui --example stack-sampler -- \
//!     --delay-ms 3000 --duration-ms 10000 --interval-us 1000 --out stacks.folded \
//!     -- target/release/nana-gpu-scene-benchmark.exe --scenario ... --sample-seconds 30
//! ```
//!
//! It starts the command, waits `--delay-ms`, picks the child's busiest
//! thread, then every `--interval-us` suspends it just long enough to read
//! its registers and copy its stack, resumes it, and walks the copy with
//! dbghelp (inline frames included). Stacks are written as folded lines
//! (`root;...;leaf count`, frames as `module!symbol`) once the child exits;
//! give it a run length that covers delay + duration. `perf/stack_attribution.py` classifies them.
//!
//! The suspension time per sample is reported on stderr; shares are what
//! this measures, not absolute speed. Build the target with
//! `CARGO_PROFILE_RELEASE_DEBUG=limited` so its PDB has function names.

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("stack-sampler is Windows-only");
    std::process::exit(2);
}

#[cfg(target_os = "windows")]
fn main() {
    windows_sampler::run();
}

#[cfg(target_os = "windows")]
mod windows_sampler {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::io::Write;
    use std::os::windows::io::AsRawHandle;
    use std::path::PathBuf;
    use std::process::Command;
    use std::time::{Duration, Instant};

    use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
    use windows::Win32::System::Diagnostics::Debug::{
        AddrModeFlat, CONTEXT, CONTEXT_FULL_AMD64, GetThreadContext, IMAGEHLP_MODULEW64,
        ReadProcessMemory, STACKFRAME_EX, SYM_STKWALK_DEFAULT, SYMBOL_INFOW, SYMOPT_DEFERRED_LOADS,
        SYMOPT_FAIL_CRITICAL_ERRORS, SYMOPT_NO_PROMPTS, SYMOPT_UNDNAME, StackWalkEx, SymFromAddrW,
        SymFromInlineContextW, SymFunctionTableAccess64, SymGetModuleBase64, SymGetModuleInfoW64,
        SymInitializeW, SymSetOptions,
    };
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
    };
    use windows::Win32::System::Memory::{MEMORY_BASIC_INFORMATION, VirtualQueryEx};
    use windows::Win32::System::SystemInformation::IMAGE_FILE_MACHINE_AMD64;
    use windows::Win32::System::Threading::{
        GetThreadTimes, OpenThread, ResumeThread, SuspendThread, THREAD_GET_CONTEXT,
        THREAD_QUERY_INFORMATION, THREAD_SUSPEND_RESUME,
    };
    use windows::core::{BOOL, PCWSTR};

    const MAX_FRAMES: usize = 192;
    const MAX_STACK_COPY: u64 = 512 * 1024;

    #[repr(C, align(16))]
    struct AlignedContext(CONTEXT);

    struct Args {
        delay: Duration,
        duration: Duration,
        interval: Duration,
        out: PathBuf,
        command: Vec<String>,
    }

    fn parse() -> Args {
        let mut delay = Duration::from_millis(3000);
        let mut duration = Duration::from_millis(10_000);
        let mut interval = Duration::from_micros(1000);
        let mut out = PathBuf::from("stacks.folded");
        let mut args = std::env::args().skip(1);
        let mut command = Vec::new();
        while let Some(arg) = args.next() {
            let mut value = || args.next().expect("option value");
            match arg.as_str() {
                "--delay-ms" => delay = Duration::from_millis(value().parse().unwrap()),
                "--duration-ms" => duration = Duration::from_millis(value().parse().unwrap()),
                "--interval-us" => interval = Duration::from_micros(value().parse().unwrap()),
                "--out" => out = PathBuf::from(value()),
                "--" => {
                    command = args.collect();
                    break;
                }
                other => panic!("unknown option {other}"),
            }
        }
        assert!(!command.is_empty(), "missing `-- <command>`");
        Args {
            delay,
            duration,
            interval,
            out,
            command,
        }
    }

    fn filetime(value: FILETIME) -> u64 {
        (u64::from(value.dwHighDateTime) << 32) | u64::from(value.dwLowDateTime)
    }

    fn thread_cpu_100ns(thread: HANDLE) -> u64 {
        let mut times = [FILETIME::default(); 4];
        let [creation, exit, kernel, user] = &mut times;
        unsafe { GetThreadTimes(thread, creation, exit, kernel, user) }.ok();
        filetime(times[2]) + filetime(times[3])
    }

    /// The child's thread with the most CPU time so far: the bench's main loop.
    fn busiest_thread(pid: u32) -> (u32, HANDLE) {
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }.unwrap();
        let mut entry = THREADENTRY32 {
            dwSize: size_of::<THREADENTRY32>() as u32,
            ..Default::default()
        };
        let mut best: Option<(u64, u32, HANDLE)> = None;
        let mut more = unsafe { Thread32First(snapshot, &mut entry) }.is_ok();
        while more {
            if entry.th32OwnerProcessID == pid
                && let Ok(thread) = unsafe {
                    OpenThread(
                        THREAD_SUSPEND_RESUME | THREAD_GET_CONTEXT | THREAD_QUERY_INFORMATION,
                        false,
                        entry.th32ThreadID,
                    )
                }
            {
                let cpu = thread_cpu_100ns(thread);
                if best.is_some_and(|(most, _, _)| most >= cpu) {
                    unsafe { CloseHandle(thread) }.ok();
                } else {
                    if let Some((_, _, previous)) = best {
                        unsafe { CloseHandle(previous) }.ok();
                    }
                    best = Some((cpu, entry.th32ThreadID, thread));
                }
            }
            more = unsafe { Thread32Next(snapshot, &mut entry) }.is_ok();
        }
        unsafe { CloseHandle(snapshot) }.ok();
        let (_, id, thread) = best.expect("child has a thread");
        (id, thread)
    }

    unsafe extern "system" fn function_table(
        process: HANDLE,
        address: u64,
    ) -> *mut core::ffi::c_void {
        unsafe { SymFunctionTableAccess64(process, address) }
    }

    unsafe extern "system" fn module_base(process: HANDLE, address: u64) -> u64 {
        unsafe { SymGetModuleBase64(process, address) }
    }

    /// One sample: registers plus a copy of the live part of the stack, so
    /// the walk runs after the thread resumed.
    struct Snapshot {
        context: Box<AlignedContext>,
        base: u64,
        bytes: Vec<u8>,
    }

    thread_local! {
        static STACK: RefCell<(u64, Vec<u8>)> = const { RefCell::new((0, Vec::new())) };
    }

    /// Stack reads come from the copy; code and unwind data (which do not
    /// change) from the live process.
    unsafe extern "system" fn read_memory(
        process: HANDLE,
        address: u64,
        buffer: *mut core::ffi::c_void,
        size: u32,
        read: *mut u32,
    ) -> BOOL {
        let served = STACK.with(|stack| {
            let (base, bytes) = &*stack.borrow();
            let end = base + bytes.len() as u64;
            if address >= *base && address + u64::from(size) <= end {
                let offset = (address - base) as usize;
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        bytes.as_ptr().add(offset),
                        buffer.cast::<u8>(),
                        size as usize,
                    );
                    *read = size;
                }
                true
            } else {
                false
            }
        });
        if served {
            return true.into();
        }
        let mut done = 0usize;
        let ok = unsafe {
            ReadProcessMemory(
                process,
                address as *const _,
                buffer,
                size as usize,
                Some(&mut done),
            )
        }
        .is_ok();
        unsafe { *read = done as u32 };
        ok.into()
    }

    fn capture(process: HANDLE, thread: HANDLE) -> Option<Snapshot> {
        let mut context = Box::new(AlignedContext(CONTEXT::default()));
        context.0.ContextFlags = CONTEXT_FULL_AMD64;
        unsafe { GetThreadContext(thread, &mut context.0) }.ok()?;
        let rsp = context.0.Rsp;
        let mut region = MEMORY_BASIC_INFORMATION::default();
        let queried = unsafe {
            VirtualQueryEx(
                process,
                Some(rsp as *const _),
                &mut region,
                size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        };
        if queried == 0 {
            return None;
        }
        let end = region.BaseAddress as u64 + region.RegionSize as u64;
        let len = (end - rsp).min(MAX_STACK_COPY) as usize;
        let mut bytes = vec![0u8; len];
        let mut done = 0usize;
        unsafe {
            ReadProcessMemory(
                process,
                rsp as *const _,
                bytes.as_mut_ptr().cast(),
                len,
                Some(&mut done),
            )
        }
        .ok()?;
        bytes.truncate(done);
        Some(Snapshot {
            context,
            base: rsp,
            bytes,
        })
    }

    fn walk(process: HANDLE, thread: HANDLE, snapshot: Snapshot, frames: &mut Vec<(u64, u32)>) {
        frames.clear();
        let Snapshot {
            mut context,
            base,
            bytes,
        } = snapshot;
        STACK.with(|stack| *stack.borrow_mut() = (base, bytes));
        let mut frame = STACKFRAME_EX {
            StackFrameSize: size_of::<STACKFRAME_EX>() as u32,
            ..Default::default()
        };
        frame.AddrPC.Offset = context.0.Rip;
        frame.AddrPC.Mode = AddrModeFlat;
        frame.AddrFrame.Offset = context.0.Rbp;
        frame.AddrFrame.Mode = AddrModeFlat;
        frame.AddrStack.Offset = context.0.Rsp;
        frame.AddrStack.Mode = AddrModeFlat;
        let leaf = context.0.Rip;
        while frames.len() < MAX_FRAMES {
            let ok = unsafe {
                StackWalkEx(
                    u32::from(IMAGE_FILE_MACHINE_AMD64.0),
                    process,
                    thread,
                    &mut frame,
                    (&mut context.0 as *mut CONTEXT).cast(),
                    Some(read_memory),
                    Some(function_table),
                    Some(module_base),
                    None,
                    SYM_STKWALK_DEFAULT,
                )
            };
            if !ok.as_bool() || frame.AddrPC.Offset == 0 {
                break;
            }
            // A return address points after the call; look the caller up by
            // the call instruction instead. The leaf is the interrupted PC,
            // and the leaf's inline frames share it.
            let pc = if frame.AddrPC.Offset == leaf {
                frame.AddrPC.Offset
            } else {
                frame.AddrPC.Offset - 1
            };
            frames.push((pc, frame.InlineFrameContext));
        }
    }

    fn wide(text: &[u16]) -> String {
        let end = text.iter().position(|&c| c == 0).unwrap_or(text.len());
        String::from_utf16_lossy(&text[..end])
    }

    fn module_info(process: HANDLE, pc: u64) -> Option<IMAGEHLP_MODULEW64> {
        let mut module = IMAGEHLP_MODULEW64 {
            SizeOfStruct: size_of::<IMAGEHLP_MODULEW64>() as u32,
            ..Default::default()
        };
        unsafe { SymGetModuleInfoW64(process, pc, &mut module) }
            .ok()
            .map(|_| module)
    }

    fn symbolize(process: HANDLE, pc: u64, inline: u32) -> String {
        let module = module_info(process, pc).map_or_else(|| "?".into(), |m| wide(&m.ModuleName));
        const NAME: usize = 1024;
        let mut buffer = vec![0u64; (size_of::<SYMBOL_INFOW>() + NAME * 2) / 8 + 1];
        let symbol = buffer.as_mut_ptr().cast::<SYMBOL_INFOW>();
        unsafe {
            (*symbol).SizeOfStruct = size_of::<SYMBOL_INFOW>() as u32;
            (*symbol).MaxNameLen = NAME as u32;
        }
        let found = unsafe { SymFromInlineContextW(process, pc, inline, None, symbol) }.is_ok()
            || unsafe { SymFromAddrW(process, pc, None, symbol) }.is_ok();
        let name = if found {
            unsafe {
                let len = ((*symbol).NameLen as usize).min(NAME);
                String::from_utf16_lossy(std::slice::from_raw_parts((*symbol).Name.as_ptr(), len))
            }
        } else {
            format!("0x{pc:x}")
        };
        // Folded format uses ';' and ' ' as separators.
        format!("{module}!{name}").replace([';', ' '], "_")
    }

    pub fn run() {
        let args = parse();
        let mut child = Command::new(&args.command[0])
            .args(&args.command[1..])
            .spawn()
            .expect("start target");
        let process = HANDLE(child.as_raw_handle());
        std::thread::sleep(args.delay);
        let exe_dir = std::path::Path::new(&args.command[0])
            .canonicalize()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .map(|d| windows::core::HSTRING::from(d.as_os_str()));
        unsafe {
            SymSetOptions(
                SYMOPT_UNDNAME
                    | SYMOPT_DEFERRED_LOADS
                    | SYMOPT_FAIL_CRITICAL_ERRORS
                    | SYMOPT_NO_PROMPTS,
            );
            match &exe_dir {
                Some(dir) => SymInitializeW(process, dir, true),
                None => SymInitializeW(process, PCWSTR::null(), true),
            }
            .expect("SymInitialize");
        }
        let (thread_id, thread) = busiest_thread(child.id());
        let cpu_start = thread_cpu_100ns(thread);
        let mut stacks: HashMap<Vec<(u64, u32)>, u64> = HashMap::new();
        let mut frames = Vec::with_capacity(MAX_FRAMES);
        let mut walk_time = Duration::ZERO;
        let mut suspend_time = Duration::ZERO;
        let mut samples = 0u64;
        let mut failed = 0u64;
        let started = Instant::now();
        let mut next = started;
        while started.elapsed() < args.duration {
            if child.try_wait().ok().flatten().is_some() {
                break;
            }
            while Instant::now() < next {
                std::hint::spin_loop();
            }
            next += args.interval;
            let suspended = Instant::now();
            if unsafe { SuspendThread(thread) } == u32::MAX {
                break;
            }
            let snapshot = capture(process, thread);
            unsafe { ResumeThread(thread) };
            suspend_time += suspended.elapsed();
            let walk_started = Instant::now();
            match snapshot {
                Some(snapshot) => {
                    walk(process, thread, snapshot, &mut frames);
                    samples += 1;
                    *stacks.entry(frames.clone()).or_default() += 1;
                }
                None => failed += 1,
            }
            walk_time += walk_started.elapsed();
            if Instant::now() > next {
                next = Instant::now();
            }
        }
        let wall = started.elapsed();
        let cpu = Duration::from_nanos((thread_cpu_100ns(thread) - cpu_start) * 100);
        let mut names: HashMap<(u64, u32), String> = HashMap::new();
        let mut folded: HashMap<String, u64> = HashMap::new();
        let mut modules = std::collections::BTreeMap::new();
        for (stack, count) in &stacks {
            let line = stack
                .iter()
                .rev()
                .map(|&key| {
                    names
                        .entry(key)
                        .or_insert_with(|| {
                            if let Some(m) = module_info(process, key.0) {
                                modules.entry(wide(&m.ModuleName)).or_insert_with(|| {
                                    serde_json::json!({
                                        "sym_type": m.SymType.0,
                                        "pdb": wide(&m.LoadedPdbName),
                                    })
                                });
                            }
                            symbolize(process, key.0, key.1)
                        })
                        .clone()
                })
                .collect::<Vec<_>>()
                .join(";");
            *folded.entry(line).or_default() += count;
        }
        // The command ends by itself (a bench with `--sample-seconds`), so
        // its own report covers the sampled window.
        let _ = child.wait();
        let mut out = std::fs::File::create(&args.out).expect("create output");
        let mut lines = folded.into_iter().collect::<Vec<_>>();
        lines.sort_by_key(|line| std::cmp::Reverse(line.1));
        for (line, count) in lines {
            writeln!(out, "{line} {count}").unwrap();
        }
        let per_sample =
            |time: Duration| time.as_secs_f64() * 1e6 / (samples + failed).max(1) as f64;
        eprintln!(
            "{}",
            serde_json::json!({
                "thread_id": thread_id,
                "samples": samples,
                "failed_captures": failed,
                "wall_ms": wall.as_secs_f64() * 1e3,
                "thread_cpu_ms": cpu.as_secs_f64() * 1e3,
                "mean_suspended_us": per_sample(suspend_time),
                "mean_walk_us": per_sample(walk_time),
                "modules": modules,
                "out": args.out,
            })
        );
    }
}
