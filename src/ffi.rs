use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use crate::client::NeonClient;
use crate::host::NeonHost;

#[repr(C)]
pub struct NeonClientHandle {
    _private: [u8; 0],
}

#[repr(C)]
pub struct NeonHostHandle {
    _private: [u8; 0],
}

pub type PongCallbackC = extern "C" fn(response_time_ms: u64, timestamp: u64);
pub type SessionConfigCallbackC = extern "C" fn(version: u8, tick_rate: u16, max_packet_size: u16);
pub type PacketTypeRegistryCallbackC = extern "C" fn(count: usize, ids: *const u8, names: *const *const c_char, descriptions: *const *const c_char);
pub type UnhandledPacketCallbackC = extern "C" fn(packet_type: u8, from_client_id: u8);
pub type WrongDestinationCallbackC = extern "C" fn(my_id: u8, packet_destination_id: u8);

pub type ClientConnectCallbackC = extern "C" fn(client_id: u8, name: *const c_char, session_id: u32);
pub type ClientDenyCallbackC = extern "C" fn(name: *const c_char, reason: *const c_char);
pub type PingReceivedCallbackC = extern "C" fn(from_client_id: u8);
pub type HostUnhandledPacketCallbackC = extern "C" fn(packet_type: u8, from_client_id: u8);

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

/// Set callback for pong events
#[unsafe(no_mangle)]
pub extern "C" fn neon_client_set_pong_callback(
    client: *mut NeonClientHandle,
    callback: PongCallbackC,
) {
    if client.is_null() {
        return;
    }

    let client = unsafe { &mut *(client as *mut NeonClient) };
    client.on_pong(move |response_time, timestamp| {
        callback(response_time, timestamp);
    });
}

/// Set callback for session config events
#[unsafe(no_mangle)]
pub extern "C" fn neon_client_set_session_config_callback(
    client: *mut NeonClientHandle,
    callback: SessionConfigCallbackC,
) {
    if client.is_null() {
        return;
    }

    let client = unsafe { &mut *(client as *mut NeonClient) };
    client.on_session_config(move |version, tick_rate, max_packet_size| {
        callback(version, tick_rate, max_packet_size);
    });
}

/// Set callback for packet type registry events
#[unsafe(no_mangle)]
pub extern "C" fn neon_client_set_packet_type_registry_callback(
    client: *mut NeonClientHandle,
    callback: PacketTypeRegistryCallbackC,
) {
    if client.is_null() {
        return;
    }

    let client = unsafe { &mut *(client as *mut NeonClient) };
    
    client.on_packet_type_registry(move |entries| {
        let count = entries.len();
        let ids: Vec<u8> = entries.iter().map(|(id, _, _)| *id).collect();
        
        let names: Vec<*const c_char> = entries.iter()
            .map(|(_, name, _)| {
                CString::new(name.as_str()).unwrap().into_raw() as *const c_char
            })
            .collect();
        
        let descriptions: Vec<*const c_char> = entries.iter()
            .map(|(_, _, desc)| {
                CString::new(desc.as_str()).unwrap().into_raw() as *const c_char
            })
            .collect();
        
        callback(count, ids.as_ptr(), names.as_ptr(), descriptions.as_ptr());
        
        for name in names {
            unsafe { CString::from_raw(name as *mut c_char) };
        }
        for desc in descriptions {
            unsafe { CString::from_raw(desc as *mut c_char) };
        }
    });
}

/// Set callback for unhandled packet events
#[unsafe(no_mangle)]
pub extern "C" fn neon_client_set_unhandled_packet_callback(
    client: *mut NeonClientHandle,
    callback: UnhandledPacketCallbackC,
) {
    if client.is_null() {
        return;
    }

    let client = unsafe { &mut *(client as *mut NeonClient) };
    client.on_unhandled_packet(move |packet_type, from_client_id| {
        callback(packet_type, from_client_id);
    });
}

/// Set callback for wrong destination events
#[unsafe(no_mangle)]
pub extern "C" fn neon_client_set_wrong_destination_callback(
    client: *mut NeonClientHandle,
    callback: WrongDestinationCallbackC,
) {
    if client.is_null() {
        return;
    }

    let client = unsafe { &mut *(client as *mut NeonClient) };
    client.on_wrong_destination(move |my_id, packet_destination_id| {
        callback(my_id, packet_destination_id);
    });
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

/// Set callback for client connect events
#[unsafe(no_mangle)]
pub extern "C" fn neon_host_set_client_connect_callback(
    host: *mut NeonHostHandle,
    callback: ClientConnectCallbackC,
) {
    if host.is_null() {
        return;
    }

    let host = unsafe { &mut *(host as *mut NeonHost) };
    host.on_client_connect(move |client_id, name, session_id| {
        let c_name = CString::new(name.as_str()).unwrap();
        callback(client_id, c_name.as_ptr(), session_id);
    });
}

/// Set callback for client deny events
#[unsafe(no_mangle)]
pub extern "C" fn neon_host_set_client_deny_callback(
    host: *mut NeonHostHandle,
    callback: ClientDenyCallbackC,
) {
    if host.is_null() {
        return;
    }

    let host = unsafe { &mut *(host as *mut NeonHost) };
    host.on_client_deny(move |name, reason| {
        let c_name = CString::new(name.as_str()).unwrap();
        let c_reason = CString::new(reason.as_str()).unwrap();
        callback(c_name.as_ptr(), c_reason.as_ptr());
    });
}

/// Set callback for ping received events
#[unsafe(no_mangle)]
pub extern "C" fn neon_host_set_ping_received_callback(
    host: *mut NeonHostHandle,
    callback: PingReceivedCallbackC,
) {
    if host.is_null() {
        return;
    }

    let host = unsafe { &mut *(host as *mut NeonHost) };
    host.on_ping_received(move |from_client_id| {
        callback(from_client_id);
    });
}

/// Set callback for unhandled packet events
#[unsafe(no_mangle)]
pub extern "C" fn neon_host_set_unhandled_packet_callback(
    host: *mut NeonHostHandle,
    callback: HostUnhandledPacketCallbackC,
) {
    if host.is_null() {
        return;
    }

    let host = unsafe { &mut *(host as *mut NeonHost) };
    host.on_unhandled_packet(move |packet_type, from_client_id, _addr| {
        callback(packet_type, from_client_id);
    });
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