//! Operating system integration.

pub mod clipboard;
pub mod dpapi;
pub mod elevation;
pub mod firewall;
pub mod gpu;
pub mod open_apps;
pub mod power;
pub mod procs;
pub mod schedtask;
pub mod session_end;
pub mod shell;

/// The signed-in user's first name for greetings ("Sam" from "Sam Rivera"),
/// falling back to the account name.
pub fn user_display_name() -> String {
    #[cfg(windows)]
    {
        use windows::core::PWSTR;
        use windows::Win32::Security::Authentication::Identity::{GetUserNameExW, NameDisplay};
        let mut buf = [0u16; 256];
        let mut len = buf.len() as u32;
        // SAFETY: the buffer and its length are valid for the call.
        if unsafe { GetUserNameExW(NameDisplay, Some(PWSTR(buf.as_mut_ptr())), &mut len) } {
            let full = String::from_utf16_lossy(&buf[..len as usize]);
            if let Some(first) = full.split_whitespace().next() {
                return first.to_string();
            }
        }
    }
    let account = std::env::var(if cfg!(windows) { "USERNAME" } else { "USER" }).unwrap_or_default();
    let mut chars = account.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}
