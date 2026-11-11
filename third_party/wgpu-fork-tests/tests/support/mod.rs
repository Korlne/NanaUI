//! A small Win32 window and the surface and clear helpers the window tests
//! share.
#![cfg(windows)]
#![allow(dead_code, reason = "each test binary uses a subset")]

use std::num::NonZeroIsize;

use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DispatchMessageW, MSG, PM_REMOVE, PeekMessageW,
            RegisterClassW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER, SetWindowPos,
            TranslateMessage, WNDCLASSW, WS_EX_NOACTIVATE, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
        },
    },
    core::w,
};

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

/// A small visible window that does not take focus.
pub fn create_window() -> HWND {
    unsafe {
        let module = GetModuleHandleW(None).unwrap();
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: module.into(),
            lpszClassName: w!("NanaWgpuForkTestWindow"),
            ..Default::default()
        };
        RegisterClassW(&class);
        CreateWindowExW(
            WS_EX_NOACTIVATE,
            w!("NanaWgpuForkTestWindow"),
            w!("wgpu fork test"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            40,
            40,
            200,
            160,
            None,
            None,
            Some(module.into()),
            None,
        )
        .unwrap()
    }
}

/// Dispatch the window's pending messages.
pub fn pump() {
    unsafe {
        let mut message = MSG::default();
        while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

/// Resize the window so its client area is about `width`×`height`.
pub fn resize(window: HWND, width: u32, height: u32) {
    unsafe {
        SetWindowPos(
            window,
            None,
            0,
            0,
            width as i32 + 16,
            height as i32 + 40,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
    }
    pump();
}

/// A surface for `window`, which must outlive it.
pub fn surface(instance: &wgpu::Instance, window: HWND) -> wgpu::Surface<'static> {
    let mut handle =
        wgpu::rwh::Win32WindowHandle::new(NonZeroIsize::new(window.0 as isize).unwrap());
    let module = unsafe { GetModuleHandleW(None) }.unwrap();
    handle.hinstance = NonZeroIsize::new(module.0 as isize);
    // SAFETY: the callers destroy the window after dropping the surface.
    unsafe {
        instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle: Some(wgpu::rwh::RawDisplayHandle::Windows(
                wgpu::rwh::WindowsDisplayHandle::new(),
            )),
            raw_window_handle: wgpu::rwh::RawWindowHandle::Win32(handle),
        })
    }
    .unwrap()
}

/// Record a clear of `view`.
pub fn clear(encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, red: f64) {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("fork test clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: red,
                    g: 0.3,
                    b: 0.5,
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    });
}
