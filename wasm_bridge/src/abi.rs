use std::alloc::{Layout, alloc as std_alloc, dealloc as std_dealloc};

/// Allocates memory for the WASM module.
///
/// # Safety
/// This function is unsafe because it allocates raw memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alloc(len: usize) -> *mut u8 {
    let layout = Layout::from_size_align(len, 1).unwrap();
    unsafe { std_alloc(layout) }
}

/// Deallocates memory for the WASM module.
///
/// # Safety
/// This function is unsafe because it deallocates raw memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    let layout = Layout::from_size_align(len, 1).unwrap();
    unsafe { std_dealloc(ptr, layout) }
}

/// Converts a pointer and length from the host into a Rust String.
///
/// # Safety
/// This function is unsafe because it creates a slice from a raw pointer.
pub unsafe fn from_host_string(ptr: *const u8, len: usize) -> String {
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).to_string()
}

/// Converts a Rust String into a raw pointer for the host.
pub fn to_host_string(s: String) -> *mut u8 {
    let mut bytes = s.into_bytes();
    let total_len = bytes.len() as u32;

    let mut buffer = total_len.to_le_bytes().to_vec();
    buffer.append(&mut bytes);

    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}
