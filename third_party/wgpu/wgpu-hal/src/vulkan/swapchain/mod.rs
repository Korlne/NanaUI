use alloc::boxed::Box;
use core::{any::Any, fmt::Debug, time::Duration};

use crate::vulkan::{semaphore_list::SemaphoreType, DeviceShared};

pub(super) use native::*;

mod native;

/// Win32 `HWND`, handed to a Win32 [`NativeSurface`] at construction so it can
/// build its [`DxgiHdrSource`](crate::auxil::dxgi::hdr::DxgiHdrSource) for the
/// display-HDR query. Borrowed from the app's window.
///
/// A newtype rather than a bare `isize` so the cfg lives in one place: off Windows
/// it is an uninhabited enum, so the non-Windows surface paths can't construct one
/// and the constructor signatures stay cfg-free.
#[cfg(windows)]
#[derive(Clone, Copy)]
pub(crate) struct WindowHandle(pub(crate) windows::Win32::Foundation::HWND);

#[cfg(not(windows))]
#[derive(Clone, Copy)]
pub(crate) enum WindowHandle {}

pub(super) trait Surface: Send + Sync + 'static {
    /// Returns the surface capabilities for the given adapter.
    ///
    /// Returns `None` if the surface is not compatible with the adapter.
    fn surface_capabilities(&self, adapter: &super::Adapter) -> Option<crate::SurfaceCapabilities>;

    /// Creates a swapchain for the surface with the given configuration.
    ///
    /// If this is not the first swapchain created for the surface, the old swapchain
    /// must be provided. It is retired by this call, even if it fails; the caller
    /// keeps it until its presents are done, then calls
    /// [`Swapchain::release_resources`] and drops it.
    unsafe fn create_swapchain(
        &self,
        device: &super::Device,
        config: &crate::SurfaceConfiguration,
        provided_old_swapchain: Option<&dyn Swapchain>,
    ) -> Result<Box<dyn Swapchain>, crate::SurfaceError>;

    /// This surface's current display HDR info, if it can report it.
    ///
    /// `Some` only for Win32 surfaces (read through DXGI); `None` otherwise
    /// (Wayland / X11 / Android / Metal), which is the default.
    fn display_hdr_info(&self) -> Option<wgt::DisplayHdrInfo> {
        None
    }

    /// Allows downcasting to the concrete type.
    fn as_any(&self) -> &dyn Any;
}

pub(super) trait Swapchain: Send + Sync + 'static {
    /// Releases all resources associated with the swapchain, without
    /// destroying the swapchain itself. Must be called before calling
    /// either [`Surface::create_swapchain`] or dropping the swapchain.
    ///
    /// The swapchain must not be in use when this is called, including by
    /// presentation; this does not wait for anything.
    unsafe fn release_resources(&mut self);

    /// Block until the device is idle: the only portable way to know every
    /// present on this swapchain is done.
    unsafe fn wait_device_idle(&self);

    /// The fence value of the latest submission that drew to one of this
    /// swapchain's images, 0 if none has.
    fn last_used_fence_value(&self) -> crate::FenceValue;

    /// Whether `fence` has reached `value`, without blocking. A failed query
    /// reads as not reached.
    unsafe fn fence_reached(&self, fence: &super::Fence, value: crate::FenceValue) -> bool;

    /// Acquires the next available surface texture for rendering.
    ///
    /// `timeout` specifies the maximum time to wait for an image to become available.
    /// If `None` is specified, this function will wait indefinitely.
    ///
    /// Returns `Err(SurfaceError::Timeout)` if the timeout elapsed before an image became available.
    unsafe fn acquire(
        &mut self,
        timeout: Option<Duration>,
        fence: &super::Fence,
    ) -> Result<crate::AcquiredSurfaceTexture<crate::api::Vulkan>, crate::SurfaceError>;

    /// Tries to discard the acquired texture without presenting it.
    ///
    /// In practice, this doesn't really work in the current implementations.
    unsafe fn discard_texture(
        &mut self,
        texture: super::SurfaceTexture,
    ) -> Result<(), crate::SurfaceError>;

    /// Presents the given surface texture using the queue.
    unsafe fn present(
        &mut self,
        queue: &super::Queue,
        texture: crate::vulkan::SurfaceTexture,
    ) -> Result<(), crate::SurfaceError>;

    /// Allows downcasting to the concrete type.
    fn as_any(&self) -> &dyn Any;

    /// Allows downcasting to the concrete type mutably.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// A swapchain replaced by `Surface::configure` (NanaUI fork).
///
/// `vkCreateSwapchainKHR` retired it, but its presents may still be pending, and
/// there is no portable way to wait for them short of the device going idle.
/// Upstream waits for that on every reconfigure; the fork keeps the swapchain
/// here instead, until a submission that drew to its successor completes:
/// that submission was queued after its last present. Dropping it unreleased
/// falls back to waiting for the device.
pub(in crate::vulkan) struct RetiredSwapchain {
    swapchain: Option<Box<dyn Swapchain>>,
    /// The fence value that, once reached, proves the presents done; `None`
    /// until the successor has drawn a frame.
    pub(in crate::vulkan) after: Option<crate::FenceValue>,
}

impl RetiredSwapchain {
    pub(in crate::vulkan) fn new(swapchain: Box<dyn Swapchain>) -> Self {
        Self {
            swapchain: Some(swapchain),
            after: None,
        }
    }

    /// Free the swapchain.
    ///
    /// # Safety
    ///
    /// Its presents must be done.
    pub(in crate::vulkan) unsafe fn release(mut self) {
        if let Some(mut swapchain) = self.swapchain.take() {
            unsafe { swapchain.release_resources() };
        }
    }
}

impl Drop for RetiredSwapchain {
    fn drop(&mut self) {
        if let Some(mut swapchain) = self.swapchain.take() {
            unsafe {
                swapchain.wait_device_idle();
                swapchain.release_resources();
            }
        }
    }
}

/// Swapchain specific metadata associated with a surface texture.
pub(super) trait SurfaceTextureMetadata: Debug + Send + Sync + 'static {
    /// Returns a guard which can yield the semaphores needed for submission using this swapchain texture.
    fn get_semaphore_guard(&self) -> Box<dyn SwapchainSubmissionSemaphoreGuard + '_>;

    /// Allows downcasting to the concrete type.
    fn as_any(&self) -> &dyn Any;
}

/// Guard type for managing swapchain submission semaphores.
pub(super) trait SwapchainSubmissionSemaphoreGuard {
    /// Sets the Fence value for this submission.
    fn set_used_fence_value(&mut self, value: u64);

    /// Gets semaphores to wait on before doing GPU work for this swapchain texture.
    fn get_acquire_wait_semaphore(&mut self) -> Option<SemaphoreType>;

    /// Gets the semaphore to signal when GPU work for this swapchain texture is complete.
    fn get_submit_signal_semaphore(
        &mut self,
        device: &DeviceShared,
    ) -> Result<SemaphoreType, crate::DeviceError>;
}
