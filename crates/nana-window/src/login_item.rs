//! Starting the application when the user signs in.
//!
//! Each platform keeps this in its own place — the per-user `Run` key on
//! Windows, the main-app login item on macOS 13 and later, an XDG autostart
//! entry on Linux — and the user can change it there too, so the stored
//! state is always read back from the system, never remembered.
//!
//! The application names the command to start: it may be the running
//! executable, or a launcher that starts it (a store client, say). macOS
//! registers the running application's bundle and ignores the command.

use std::path::PathBuf;

/// What to start at sign-in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginItem {
    /// A stable name for the entry: the registry value on Windows, the
    /// autostart file's stem on Linux. Letters, digits, `-`, `_` and `.`.
    pub id: String,
    /// What the user sees in the system's list where it shows one.
    pub name: String,
    pub program: PathBuf,
    pub arguments: Vec<String>,
}

/// Whether the system starts the application at sign-in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginItemState {
    Enabled,
    Disabled,
    /// This platform, or this way of running the application (outside an
    /// application bundle on macOS), has no login item.
    Unavailable,
}

/// Why the login item could not be changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginItemError {
    Unavailable,
    /// The id holds characters a registry value or file name cannot take.
    InvalidId,
    /// The system refused; its own description.
    System(String),
}

impl std::fmt::Display for LoginItemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => f.write_str("login items are unavailable here"),
            Self::InvalidId => f.write_str("the login item id is not a plain name"),
            Self::System(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for LoginItemError {}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// One argument the way the platform's command line reads it back whole.
#[cfg(any(test, target_os = "windows", target_os = "linux"))]
fn quoted(argument: &str) -> String {
    if !argument.is_empty() && !argument.contains([' ', '\t', '"']) {
        return argument.to_owned();
    }
    let mut out = String::from('"');
    let mut backslashes = 0;
    for c in argument.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                // Backslashes before a quote are doubled, then the quote is
                // escaped.
                out.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                backslashes = 0;
                out.push('"');
                continue;
            }
            _ => {
                out.extend(std::iter::repeat_n('\\', backslashes));
                backslashes = 0;
            }
        }
        if c != '\\' {
            out.push(c);
        }
    }
    // Trailing backslashes would escape the closing quote.
    out.extend(std::iter::repeat_n('\\', backslashes * 2));
    out.push('"');
    out
}

#[cfg(any(test, target_os = "windows", target_os = "linux"))]
fn command_line(item: &LoginItem) -> String {
    let mut line = quoted(&item.program.to_string_lossy());
    for argument in &item.arguments {
        line.push(' ');
        line.push_str(&quoted(argument));
    }
    line
}

/// Reads from the system whether `item` starts at sign-in.
pub fn login_item_state(item: &LoginItem) -> LoginItemState {
    if !valid_id(&item.id) {
        return LoginItemState::Unavailable;
    }
    platform::state(item)
}

/// Turns starting at sign-in on or off.
pub fn set_login_item(item: &LoginItem, enabled: bool) -> Result<(), LoginItemError> {
    if !valid_id(&item.id) {
        return Err(LoginItemError::InvalidId);
    }
    platform::set(item, enabled)
}

#[cfg(target_os = "windows")]
mod platform {
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ, RegCloseKey,
        RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    };

    use super::{LoginItem, LoginItemError, LoginItemState, command_line};
    use crate::win32::wide;

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

    fn open(access: u32) -> Result<HKEY, u32> {
        let path = wide(RUN_KEY);
        let mut key: HKEY = std::ptr::null_mut();
        // SAFETY: a NUL-terminated path and a valid out pointer.
        let status =
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, path.as_ptr(), 0, access, &mut key) };
        if status == ERROR_SUCCESS {
            Ok(key)
        } else {
            Err(status)
        }
    }

    pub(super) fn state(item: &LoginItem) -> LoginItemState {
        let Ok(key) = open(KEY_QUERY_VALUE) else {
            return LoginItemState::Disabled;
        };
        let name = wide(&item.id);
        // SAFETY: only asks whether the value exists.
        let status = unsafe {
            RegQueryValueExW(
                key,
                name.as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        // SAFETY: opened above.
        unsafe { RegCloseKey(key) };
        if status == ERROR_SUCCESS {
            LoginItemState::Enabled
        } else {
            LoginItemState::Disabled
        }
    }

    pub(super) fn set(item: &LoginItem, enabled: bool) -> Result<(), LoginItemError> {
        let failed = |status: u32| LoginItemError::System(format!("registry error {status}"));
        let key = open(KEY_SET_VALUE).map_err(failed)?;
        let name = wide(&item.id);
        let status = if enabled {
            let value = wide(&command_line(item));
            // SAFETY: `value` is NUL-terminated UTF-16 of the given byte size.
            unsafe {
                RegSetValueExW(
                    key,
                    name.as_ptr(),
                    0,
                    REG_SZ,
                    value.as_ptr().cast(),
                    (value.len() * 2) as u32,
                )
            }
        } else {
            // SAFETY: a NUL-terminated value name.
            match unsafe { RegDeleteValueW(key, name.as_ptr()) } {
                // Already absent is the state asked for.
                windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND => ERROR_SUCCESS,
                status => status,
            }
        };
        // SAFETY: opened above.
        unsafe { RegCloseKey(key) };
        if status == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(failed(status))
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use objc2::msg_send;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyClass, AnyObject};
    use objc2_foundation::NSError;

    use super::{LoginItem, LoginItemError, LoginItemState};

    // `SMAppService` lives here; nothing else of the framework is used.
    #[link(name = "ServiceManagement", kind = "framework")]
    unsafe extern "C" {}

    /// `SMAppServiceStatusEnabled`.
    const STATUS_ENABLED: isize = 1;

    /// The main application's login item: macOS 13 and later, and only for a
    /// process that runs from an application bundle.
    fn service() -> Option<Retained<AnyObject>> {
        let class = AnyClass::get(c"SMAppService")?;
        let bundled: Option<Retained<AnyObject>> = unsafe {
            let bundle: Retained<AnyObject> = msg_send![objc2::class!(NSBundle), mainBundle];
            msg_send![&*bundle, bundleIdentifier]
        };
        bundled?;
        // SAFETY: a class method returning the shared service object.
        unsafe { msg_send![class, mainAppService] }
    }

    pub(super) fn state(_item: &LoginItem) -> LoginItemState {
        let Some(service) = service() else {
            return LoginItemState::Unavailable;
        };
        // SAFETY: `status` is a plain NSInteger property.
        let status: isize = unsafe { msg_send![&*service, status] };
        if status == STATUS_ENABLED {
            LoginItemState::Enabled
        } else {
            LoginItemState::Disabled
        }
    }

    pub(super) fn set(_item: &LoginItem, enabled: bool) -> Result<(), LoginItemError> {
        let service = service().ok_or(LoginItemError::Unavailable)?;
        let mut error: Option<Retained<NSError>> = None;
        // SAFETY: both selectors take an NSError out-parameter and answer BOOL.
        let done: bool = unsafe {
            if enabled {
                msg_send![&*service, registerAndReturnError: &mut error]
            } else {
                msg_send![&*service, unregisterAndReturnError: &mut error]
            }
        };
        if done {
            Ok(())
        } else {
            Err(LoginItemError::System(
                error
                    .map(|error| error.localizedDescription().to_string())
                    .unwrap_or_else(|| "the login item was not changed".to_owned()),
            ))
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use std::path::PathBuf;

    use super::{LoginItem, LoginItemError, LoginItemState, command_line};

    fn entry(item: &LoginItem) -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
        Some(base.join("autostart").join(format!("{}.desktop", item.id)))
    }

    pub(super) fn state(item: &LoginItem) -> LoginItemState {
        match entry(item) {
            Some(path) if path.is_file() => LoginItemState::Enabled,
            Some(_) => LoginItemState::Disabled,
            None => LoginItemState::Unavailable,
        }
    }

    pub(super) fn set(item: &LoginItem, enabled: bool) -> Result<(), LoginItemError> {
        let path = entry(item).ok_or(LoginItemError::Unavailable)?;
        let failed = |error: std::io::Error| LoginItemError::System(error.to_string());
        if !enabled {
            return match std::fs::remove_file(&path) {
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(failed(error)),
                _ => Ok(()),
            };
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(failed)?;
        }
        // A desktop entry is one line per key; a name cannot span lines.
        let name: String = item.name.chars().filter(|c| !c.is_control()).collect();
        let body = format!(
            "[Desktop Entry]\nType=Application\nName={name}\nExec={}\nX-GNOME-Autostart-enabled=true\n",
            command_line(item).replace('%', "%%"),
        );
        std::fs::write(&path, body).map_err(failed)
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
mod platform {
    use super::{LoginItem, LoginItemError, LoginItemState};

    pub(super) fn state(_item: &LoginItem) -> LoginItemState {
        LoginItemState::Unavailable
    }

    pub(super) fn set(_item: &LoginItem, _enabled: bool) -> Result<(), LoginItemError> {
        Err(LoginItemError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(program: &str, arguments: &[&str]) -> LoginItem {
        LoginItem {
            id: "example".into(),
            name: "Example".into(),
            program: program.into(),
            arguments: arguments.iter().map(|a| (*a).to_owned()).collect(),
        }
    }

    #[test]
    fn the_command_line_quotes_what_would_split() {
        assert_eq!(
            command_line(&item("/opt/app", &["--quiet"])),
            "/opt/app --quiet"
        );
        assert_eq!(
            command_line(&item(r"C:\Program Files\App\app.exe", &["a b", ""])),
            r#""C:\Program Files\App\app.exe" "a b" """#
        );
        assert_eq!(quoted(r#"say "hi""#), r#""say \"hi\"""#);
        assert_eq!(quoted(r"dir with space\"), r#""dir with space\\""#);
    }

    #[test]
    fn an_id_that_is_not_a_plain_name_is_refused() {
        let mut bad = item("/opt/app", &[]);
        bad.id = "../escape".into();
        assert_eq!(login_item_state(&bad), LoginItemState::Unavailable);
        assert_eq!(set_login_item(&bad, true), Err(LoginItemError::InvalidId));
    }
}
