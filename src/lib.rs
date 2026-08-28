mod font;
mod style;
mod ui;
mod window;

use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::ptr;

use window::{ModulePayload, SignalCb, SignalCallback, WindowHandle};

/// Opaque handle exposed to C# via P/Invoke.
#[repr(C)]
pub struct ULibWindow {
    _private: [u8; 0],
}

/// Opaque handle to a parsed .ulib module.
#[repr(C)]
pub struct ULibModule {
    _private: [u8; 0],
}

/// Create a new window with the given dimensions.
/// Returns a pointer to an opaque `ULibWindow` handle.
#[unsafe(no_mangle)]
pub extern "C" fn ulib_window_create(width: u32, height: u32) -> *mut ULibWindow {
    match WindowHandle::new(width, height) {
        Ok(handle) => Box::into_raw(Box::new(handle)) as *mut ULibWindow,
        Err(e) => {
            eprintln!("[ulib] failed to create window: {e}");
            ptr::null_mut()
        }
    }
}

/// Set the window title.
#[unsafe(no_mangle)]
pub extern "C" fn ulib_window_set_title(handle: *mut ULibWindow, title: *const c_char) {
    if handle.is_null() || title.is_null() {
        return;
    }
    let win = unsafe { &mut *(handle as *mut WindowHandle) };
    let Ok(title) = unsafe { CStr::from_ptr(title) }.to_str() else {
        return;
    };
    win.set_title(title);
}

/// Set whether the window is fullscreen.
#[unsafe(no_mangle)]
pub extern "C" fn ulib_window_set_fullscreen(handle: *mut ULibWindow, fullscreen: bool) {
    if handle.is_null() {
        return;
    }
    let win = unsafe { &mut *(handle as *mut WindowHandle) };
    win.set_fullscreen(fullscreen);
}

/// Set the window width.
#[unsafe(no_mangle)]
pub extern "C" fn ulib_window_set_width(handle: *mut ULibWindow, width: u32) {
    if handle.is_null() {
        return;
    }
    let win = unsafe { &mut *(handle as *mut WindowHandle) };
    win.set_size(width, win.height());
}

/// Set the window height.
#[unsafe(no_mangle)]
pub extern "C" fn ulib_window_set_height(handle: *mut ULibWindow, height: u32) {
    if handle.is_null() {
        return;
    }
    let win = unsafe { &mut *(handle as *mut WindowHandle) };
    win.set_size(win.width(), height);
}

/// Get the window width.
#[unsafe(no_mangle)]
pub extern "C" fn ulib_window_get_width(handle: *mut ULibWindow) -> u32 {
    if handle.is_null() {
        return 0;
    }
    let win = unsafe { &mut *(handle as *mut WindowHandle) };
    win.width()
}

/// Get the window height.
#[unsafe(no_mangle)]
pub extern "C" fn ulib_window_get_height(handle: *mut ULibWindow) -> u32 {
    if handle.is_null() {
        return 0;
    }
    let win = unsafe { &mut *(handle as *mut WindowHandle) };
    win.height()
}

/// Poll events. Returns 1 if the window should close, 0 otherwise.
#[unsafe(no_mangle)]
pub extern "C" fn ulib_window_poll(handle: *mut ULibWindow) -> i32 {
    if handle.is_null() {
        return 1;
    }
    let win = unsafe { &mut *(handle as *mut WindowHandle) };
    if win.poll_close() {
        1
    } else {
        0
    }
}

/// Close the window.
#[unsafe(no_mangle)]
pub extern "C" fn ulib_window_close(handle: *mut ULibWindow) {
    if handle.is_null() {
        return;
    }
    let win = unsafe { &mut *(handle as *mut WindowHandle) };
    win.close();
}

/// Destroy the window and free its memory.
#[unsafe(no_mangle)]
pub extern "C" fn ulib_window_destroy(handle: *mut ULibWindow) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle as *mut WindowHandle));
        }
    }
}

/// Load a `.ulib` module from a file path. Returns an opaque module handle
/// (or null on failure). The `Style("file.css")` directive inside is honored.
#[unsafe(no_mangle)]
pub extern "C" fn ulib_module_load(path: *const c_char) -> *mut ULibModule {
    if path.is_null() {
        return ptr::null_mut();
    }
    let Ok(path) = unsafe { CStr::from_ptr(path) }.to_str() else {
        return ptr::null_mut();
    };
    match load_module_from_path(path) {
        Ok(payload) => Box::into_raw(Box::new(payload)) as *mut ULibModule,
        Err(e) => {
            eprintln!("[ulib] failed to load module `{path}`: {e}");
            ptr::null_mut()
        }
    }
}

fn load_module_from_path(path: &str) -> Result<ModulePayload, String> {
    use std::fs;

    let src = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let module = ui::parse_module(&src)?;

    let sheet = match module.style_file {
        Some(css_file) => {
            let rel = std::path::Path::new(path).parent().unwrap_or(std::path::Path::new(""));
            let css_path = rel.join(&css_file);
            let css = fs::read_to_string(&css_path).map_err(|e| e.to_string())?;
            style::parse(&css)
        }
        None => style::StyleSheet::new(),
    };

    Ok(ModulePayload {
        tree: module.root,
        sheet,
    })
}

/// Free a module handle.
#[unsafe(no_mangle)]
pub extern "C" fn ulib_module_free(module: *mut ULibModule) {
    if !module.is_null() {
        unsafe {
            drop(Box::from_raw(module as *mut ModulePayload));
        }
    }
}

/// Attach a parsed module to a window (places its widget tree into the window).
#[unsafe(no_mangle)]
pub extern "C" fn ulib_window_load_module(handle: *mut ULibWindow, module: *mut ULibModule) {
    if handle.is_null() || module.is_null() {
        return;
    }
    let win = unsafe { &mut *(handle as *mut WindowHandle) };
    let payload = unsafe { &*(module as *const ModulePayload) }.clone();
    win.load_module(payload);
}

/// Set a callback invoked when any button signal fires. Passing (null, 0)
/// clears it. The callback is invoked on the native event-loop thread.
#[unsafe(no_mangle)]
pub extern "C" fn ulib_window_set_signal_callback(
    handle: *mut ULibWindow,
    cb: Option<SignalCallback>,
    userdata: *mut c_void,
) {
    if handle.is_null() {
        return;
    }
    let win = unsafe { &mut *(handle as *mut WindowHandle) };
    let registered = cb.map(|c| SignalCb(c, userdata));
    win.set_signal_callback(registered);
}

