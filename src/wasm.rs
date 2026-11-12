//! C-ABI WebAssembly exports backed by `TrekSession`.

use std::cell::RefCell;

use crate::session::TrekSession;

struct WasmRuntime {
    session: TrekSession,
    response_bytes: Vec<u8>,
}

thread_local! {
    static RUNTIME: RefCell<Option<WasmRuntime>> = const { RefCell::new(None) };
}

unsafe fn read_utf8_lossy(ptr: *const u8, len: usize) -> String {
    if len == 0 || ptr.is_null() {
        return String::new();
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).into_owned()
}

#[no_mangle]
pub extern "C" fn trek_alloc(len: usize) -> *mut u8 {
    if len == 0 {
        return std::ptr::NonNull::<u8>::dangling().as_ptr();
    }
    let mut boxed = vec![0u8; len].into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    std::mem::forget(boxed);
    ptr
}

#[no_mangle]
pub extern "C" fn trek_dealloc(ptr: *mut u8, len: usize) {
    if len == 0 || ptr.is_null() {
        return;
    }
    unsafe {
        let slice = std::ptr::slice_from_raw_parts_mut(ptr, len);
        drop(Box::from_raw(slice));
    }
}

#[no_mangle]
pub extern "C" fn trek_init(
    seed: u32,
    coord_fixed: u32,
    cmdline_ptr: *const u8,
    cmdline_len: usize,
) -> u32 {
    let cmdline = unsafe { read_utf8_lossy(cmdline_ptr, cmdline_len) };
    let session = TrekSession::new(seed, coord_fixed != 0, &cmdline);
    let response_bytes = session.last_response().to_json().into_bytes();
    RUNTIME.with(|cell| {
        *cell.borrow_mut() = Some(WasmRuntime {
            session,
            response_bytes,
        });
    });
    1
}

#[no_mangle]
pub extern "C" fn trek_step(input_ptr: *const u8, input_len: usize, is_key: u32) -> u32 {
    let input = unsafe { read_utf8_lossy(input_ptr, input_len) };
    RUNTIME.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let Some(rt) = borrow.as_mut() else {
            return 0;
        };
        let json = rt.session.step(&input, is_key != 0).to_json();
        rt.response_bytes = json.into_bytes();
        1
    })
}

#[no_mangle]
pub extern "C" fn trek_set_coord_fixed(coord_fixed: u32) -> u32 {
    RUNTIME.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let Some(rt) = borrow.as_mut() else {
            return 0;
        };
        let json = rt.session.set_coord_fixed(coord_fixed != 0).to_json();
        rt.response_bytes = json.into_bytes();
        1
    })
}

#[no_mangle]
pub extern "C" fn trek_import_save(
    name_ptr: *const u8,
    name_len: usize,
    data_ptr: *const u8,
    data_len: usize,
) -> u32 {
    let name = unsafe { read_utf8_lossy(name_ptr, name_len) };
    let data = unsafe { read_utf8_lossy(data_ptr, data_len) };
    RUNTIME.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let Some(rt) = borrow.as_mut() else {
            return 0;
        };
        if rt.session.import_save(&name, &data) {
            1
        } else {
            0
        }
    })
}

#[no_mangle]
pub extern "C" fn trek_response_ptr() -> *const u8 {
    RUNTIME.with(|cell| {
        let borrow = cell.borrow();
        match borrow.as_ref() {
            Some(rt) if !rt.response_bytes.is_empty() => rt.response_bytes.as_ptr(),
            _ => std::ptr::null(),
        }
    })
}

#[no_mangle]
pub extern "C" fn trek_response_len() -> usize {
    RUNTIME.with(|cell| {
        let borrow = cell.borrow();
        borrow.as_ref().map_or(0, |rt| rt.response_bytes.len())
    })
}
