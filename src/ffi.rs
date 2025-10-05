use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use crate::client::NeonClient;
use crate::host::NeonHost;

#[repr(C)]
pub struct NeonClientHandle {
    _private: [u8; 0],
}

/// Create a new Neon client
/// Returns null on failure
#[unsafe(no_mangle)]
pub extern "C" fn neon_client_new(name: *const c_char) -> *mut NeonClientHandle {
    if name.is_null() {
        return ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(name) };
    let name_str = match c_str.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return ptr::null_mut(),
    };

    match NeonClient::new(name_str) {
        Ok(client) => Box::into_raw(Box::new(client)) as *mut NeonClientHandle,
        Err(_) => ptr::null_mut(),
    }
}

/// Connect the client to a session
/// Returns true on success, false on failure
#[unsafe(no_mangle)]
pub extern "C" fn neon_client_connect(
    client: *mut NeonClientHandle,
    session_id: u32,
    relay_addr: *const c_char,
) -> bool {
    if client.is_null() || relay_addr.is_null() {
        return false;
    }

    let client = unsafe { &mut *(client as *mut NeonClient) };
    let c_str = unsafe { CStr::from_ptr(relay_addr) };
    let addr = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };

    client.connect(session_id, addr).is_ok()
}

/// Process incoming packets (call this regularly, e.g. in your game tick)
/// Returns true on success, false on failure
#[unsafe(no_mangle)]
pub extern "C" fn neon_client_process_packets(client: *mut NeonClientHandle) -> bool {
    if client.is_null() {
        return false;
    }

    let client = unsafe { &mut *(client as *mut NeonClient) };
    client.process_packets().is_ok()
}

/// Get the client's assigned ID (returns 0 if not connected)
#[unsafe(no_mangle)]
pub extern "C" fn neon_client_get_id(client: *mut NeonClientHandle) -> u8 {
    if client.is_null() {
        return 0;
    }

    let client = unsafe { &*(client as *const NeonClient) };
    client.client_id().unwrap_or(0)
}

/// Get the session ID (returns 0 if not connected)
#[unsafe(no_mangle)]
pub extern "C" fn neon_client_get_session_id(client: *mut NeonClientHandle) -> u32 {
    if client.is_null() {
        return 0;
    }

    let client = unsafe { &*(client as *const NeonClient) };
    client.session_id().unwrap_or(0)
}

/// Check if the client is connected
#[unsafe(no_mangle)]
pub extern "C" fn neon_client_is_connected(client: *mut NeonClientHandle) -> bool {
    if client.is_null() {
        return false;
    }

    let client = unsafe { &*(client as *const NeonClient) };
    client.client_id().is_some()
}

/// Manually send a ping
#[unsafe(no_mangle)]
pub extern "C" fn neon_client_send_ping(client: *mut NeonClientHandle) -> bool {
    if client.is_null() {
        return false;
    }

    let client = unsafe { &*(client as *const NeonClient) };
    client.send_ping().is_ok()
}

/// Set auto-ping enabled/disabled
#[unsafe(no_mangle)]
pub extern "C" fn neon_client_set_auto_ping(client: *mut NeonClientHandle, enabled: bool) {
    if client.is_null() {
        return;
    }

    let client = unsafe { &mut *(client as *mut NeonClient) };
    client.set_auto_ping(enabled);
}

/// Free the client (call when done)
#[unsafe(no_mangle)]
pub extern "C" fn neon_client_free(client: *mut NeonClientHandle) {
    if !client.is_null() {
        unsafe {
            Box::from_raw(client as *mut NeonClient);
        }
    }
}

#[repr(C)]
pub struct NeonHostHandle {
    _private: [u8; 0],
}

/// Create a new Neon host
/// Returns null on failure
#[unsafe(no_mangle)]
pub extern "C" fn neon_host_new(session_id: u32, relay_addr: *const c_char) -> *mut NeonHostHandle {
    if relay_addr.is_null() {
        return ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(relay_addr) };
    let addr = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match NeonHost::new(session_id, addr) {
        Ok(host) => Box::into_raw(Box::new(host)) as *mut NeonHostHandle,
        Err(_) => ptr::null_mut(),
    }
}

/// Get the host's session ID
#[unsafe(no_mangle)]
pub extern "C" fn neon_host_get_session_id(host: *mut NeonHostHandle) -> u32 {
    if host.is_null() {
        return 0;
    }

    let host = unsafe { &*(host as *const NeonHost) };
    host.session_id()
}

/// Get the number of connected clients
#[unsafe(no_mangle)]
pub extern "C" fn neon_host_get_client_count(host: *mut NeonHostHandle) -> usize {
    if host.is_null() {
        return 0;
    }

    let host = unsafe { &*(host as *const NeonHost) };
    host.client_count()
}

/// Start the host (this blocks! Run in a separate thread)
/// Returns true on success, false on failure
#[unsafe(no_mangle)]
pub extern "C" fn neon_host_start(host: *mut NeonHostHandle) -> bool {
    if host.is_null() {
        return false;
    }

    let host = unsafe { &mut *(host as *mut NeonHost) };
    host.start().is_ok()
}

/// Free the host (call when done)
#[unsafe(no_mangle)]
pub extern "C" fn neon_host_free(host: *mut NeonHostHandle) {
    if !host.is_null() {
        unsafe {
            Box::from_raw(host as *mut NeonHost);
        }
    }
}

thread_local! {
    static LAST_ERROR: std::cell::RefCell<Option<CString>> = std::cell::RefCell::new(None);
}

/// Get the last error message (or null if no error)
/// The returned string is valid until the next error or until this thread exits
#[unsafe(no_mangle)]
pub extern "C" fn neon_get_last_error() -> *const c_char {
    LAST_ERROR.with(|e| {
        e.borrow()
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(ptr::null())
    })
}

fn set_last_error(err: &str) {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = CString::new(err).ok();
    });
}