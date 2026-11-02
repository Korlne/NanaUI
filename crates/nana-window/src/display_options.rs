//! macOS accessibility display options: increase contrast, reduce motion and
//! reduce transparency.
//!
//! All three are `NSWorkspace` properties
//! (`accessibilityDisplayShouldIncreaseContrast`,
//! `…ShouldReduceMotion`, `…ShouldReduceTransparency`) and change together
//! under one notification,
//! `NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification`, posted on
//! the workspace notification center. The first read of any of them installs
//! one process-lifetime observer; each post marks every preference changed and
//! the host compares the fresh value with the one it holds, so a post that did
//! not move a flag reports nothing.
//!
//! The notification is delivered on the main run loop, which is the host's
//! event loop, so it needs no separate wake.

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2_foundation::NSString;

/// Which display option a read asks for.
#[derive(Clone, Copy)]
pub(crate) enum DisplayOption {
    IncreaseContrast,
    ReduceMotion,
    ReduceTransparency,
}

/// The current value of `option`, installing the change observer first.
pub(crate) fn display_option(option: DisplayOption) -> bool {
    observe_changes();
    let workspace = shared_workspace();
    // SAFETY: the three getters exist on NSWorkspace since macOS 10.10
    // (contrast, transparency) and 10.12 (motion), below the deployment floor.
    unsafe {
        match option {
            DisplayOption::IncreaseContrast => {
                objc2::msg_send![&*workspace, accessibilityDisplayShouldIncreaseContrast]
            }
            DisplayOption::ReduceMotion => {
                objc2::msg_send![&*workspace, accessibilityDisplayShouldReduceMotion]
            }
            DisplayOption::ReduceTransparency => {
                objc2::msg_send![&*workspace, accessibilityDisplayShouldReduceTransparency]
            }
        }
    }
}

fn shared_workspace() -> Retained<NSObject> {
    // SAFETY: `+[NSWorkspace sharedWorkspace]` returns the process singleton.
    unsafe { objc2::msg_send![objc2::class!(NSWorkspace), sharedWorkspace] }
}

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {
    static NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification: &'static NSString;
}

fn observe_changes() {
    static INSTALLED: std::sync::Once = std::sync::Once::new();
    INSTALLED.call_once(|| {
        let block = block2::RcBlock::new(|_notification: std::ptr::NonNull<AnyObject>| {
            crate::contrast_preference::mark_changed();
            crate::motion_preference::mark_changed();
            crate::transparency_preference::mark_changed();
        });
        let workspace = shared_workspace();
        // SAFETY: the workspace notification center outlives the process; a
        // nil queue runs the block on the posting (main) thread. The returned
        // observer token is kept for the life of the process on purpose: the
        // observer is never removed.
        unsafe {
            let center: Retained<NSObject> = objc2::msg_send![&*workspace, notificationCenter];
            let token: Option<Retained<NSObject>> = objc2::msg_send![
                &*center,
                addObserverForName: NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification,
                object: std::ptr::null::<AnyObject>(),
                queue: std::ptr::null::<AnyObject>(),
                usingBlock: &*block,
            ];
            std::mem::forget(token);
        }
    });
}

/// Post the display-options notification the way the system does, so a test
/// observes the installed observer run.
#[cfg(test)]
pub(crate) fn post_change_for_test() {
    let workspace = shared_workspace();
    // SAFETY: posting a named notification with no object or user info.
    unsafe {
        let center: Retained<NSObject> = objc2::msg_send![&*workspace, notificationCenter];
        let _: () = objc2::msg_send![
            &*center,
            postNotificationName: NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification,
            object: std::ptr::null::<AnyObject>(),
        ];
    }
}
