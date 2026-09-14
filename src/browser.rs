//! Minimal "open a URL in the user's default browser" helper via
//! `ShellExecuteW`, rather than pulling in an `open`/`webbrowser` crate --
//! consistent with the rest of the app's direct-Win32-call style.

use windows_sys::Win32::UI::Shell::ShellExecuteW;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Opens `url` with the OS-registered default handler (the default browser
/// for `https://` links). Best-effort and silent on failure -- this is a
/// convenience shortcut, never load-bearing.
pub fn open_url(url: &str) {
    let op = to_wide("open");
    let file = to_wide(url);
    const SW_SHOWNORMAL: i32 = 1;
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            op.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}
