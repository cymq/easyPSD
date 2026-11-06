//! Minimal C standard library shims for `wasm32-unknown-unknown` when linking `lcms2-sys`
//! compiled with WASI SDK.
//!
//! Routes C heap allocations (`malloc`, `free`, `calloc`, `realloc`) through Rust's global
//! allocator so that `wasi-libc`'s `dlmalloc` is never linked (preventing heap collisions on
//! `__heap_base`), and stubs out `time`/`gmtime` and any WASI preview1 imports so the resulting
//! `.wasm` module has zero external `wasi_snapshot_preview1` imports in the browser.

use std::alloc::{Layout, alloc, alloc_zeroed, dealloc, realloc as rust_realloc};
use std::ffi::c_void;
use std::ptr;

const ALIGN: usize = 16;
const HEADER_SIZE: usize = 16;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    let payload = size.max(1);
    let Some(total) = payload.checked_add(HEADER_SIZE) else {
        return ptr::null_mut();
    };
    let Ok(layout) = Layout::from_size_align(total, ALIGN) else {
        return ptr::null_mut();
    };
    let base = unsafe { alloc(layout) };
    if base.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        *(base as *mut usize) = total;
        base.add(HEADER_SIZE) as *mut c_void
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let base = (ptr as *mut u8).sub(HEADER_SIZE);
        let total = *(base as *const usize);
        let layout = Layout::from_size_align_unchecked(total, ALIGN);
        dealloc(base, layout);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn calloc(nmemb: usize, size: usize) -> *mut c_void {
    let Some(bytes) = nmemb.checked_mul(size) else {
        return ptr::null_mut();
    };
    let payload = bytes.max(1);
    let Some(total) = payload.checked_add(HEADER_SIZE) else {
        return ptr::null_mut();
    };
    let Ok(layout) = Layout::from_size_align(total, ALIGN) else {
        return ptr::null_mut();
    };
    let base = unsafe { alloc_zeroed(layout) };
    if base.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        *(base as *mut usize) = total;
        base.add(HEADER_SIZE) as *mut c_void
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void {
    if ptr.is_null() {
        return unsafe { malloc(new_size) };
    }
    if new_size == 0 {
        unsafe { free(ptr) };
        return ptr::null_mut();
    }
    let Some(new_total) = new_size.checked_add(HEADER_SIZE) else {
        return ptr::null_mut();
    };
    unsafe {
        let base = (ptr as *mut u8).sub(HEADER_SIZE);
        let old_total = *(base as *const usize);
        let old_layout = Layout::from_size_align_unchecked(old_total, ALIGN);
        let new_base = rust_realloc(base, old_layout, new_total);
        if new_base.is_null() {
            return ptr::null_mut();
        }
        *(new_base as *mut usize) = new_total;
        new_base.add(HEADER_SIZE) as *mut c_void
    }
}

#[repr(C)]
pub struct Tm {
    pub tm_sec: i32,
    pub tm_min: i32,
    pub tm_hour: i32,
    pub tm_mday: i32,
    pub tm_mon: i32,
    pub tm_year: i32,
    pub tm_wday: i32,
    pub tm_yday: i32,
    pub tm_isdst: i32,
    pub tm_gmtoff: isize,
    pub tm_zone: *const u8,
}

static mut STATIC_TM: Tm = Tm {
    tm_sec: 0,
    tm_min: 0,
    tm_hour: 0,
    tm_mday: 1,
    tm_mon: 0,
    tm_year: 125, // 2025
    tm_wday: 3,
    tm_yday: 0,
    tm_isdst: 0,
    tm_gmtoff: 0,
    tm_zone: b"UTC\0".as_ptr(),
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn time(tloc: *mut i64) -> i64 {
    let now: i64 = 1_735_689_600;
    if !tloc.is_null() {
        unsafe {
            *tloc = now;
        }
    }
    now
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gmtime(_timer: *const i64) -> *mut Tm {
    &raw mut STATIC_TM
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ctime(_timer: *const i64) -> *mut u8 {
    b"Wed Jan  1 00:00:00 2025\n\0".as_ptr() as *mut u8
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fopen(_filename: *const u8, _mode: *const u8) -> *mut c_void {
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fclose(_stream: *mut c_void) -> i32 {
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fread(
    _ptr: *mut c_void,
    _size: usize,
    _nmemb: usize,
    _stream: *mut c_void,
) -> usize {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fseek(_stream: *mut c_void, _offset: isize, _whence: i32) -> i32 {
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ftell(_stream: *mut c_void) -> isize {
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn feof(_stream: *mut c_void) -> i32 {
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fgetc(_stream: *mut c_void) -> i32 {
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn remove(_pathname: *const u8) -> i32 {
    -1
}

// Satisfy any residual WASI preview1 syscall references from `wasi-libc` (`stdio` / `abort`)
// locally so that `wasm-ld` emits zero `"wasi_snapshot_preview1"` imports.

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __imported_wasi_snapshot_preview1_clock_time_get(
    _id: i32,
    _precision: i64,
    time_out: *mut u64,
) -> i32 {
    if !time_out.is_null() {
        unsafe {
            *time_out = 0;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __imported_wasi_snapshot_preview1_fd_close(_fd: i32) -> i32 {
    8 // EBADF
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __imported_wasi_snapshot_preview1_fd_fdstat_get(
    _fd: i32,
    _stat: *mut u8,
) -> i32 {
    8 // EBADF
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __imported_wasi_snapshot_preview1_fd_prestat_get(
    _fd: i32,
    _buf: *mut u8,
) -> i32 {
    8 // EBADF
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __imported_wasi_snapshot_preview1_fd_prestat_dir_name(
    _fd: i32,
    _path: *mut u8,
    _path_len: usize,
) -> i32 {
    8 // EBADF
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __imported_wasi_snapshot_preview1_fd_read(
    _fd: i32,
    _iovs: *const u8,
    _iovs_len: usize,
    _nread: *mut usize,
) -> i32 {
    8 // EBADF
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __imported_wasi_snapshot_preview1_fd_seek(
    _fd: i32,
    _offset: i64,
    _whence: i32,
    _newoffset: *mut u64,
) -> i32 {
    8 // EBADF
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __imported_wasi_snapshot_preview1_fd_write(
    _fd: i32,
    _iovs: *const u8,
    _iovs_len: usize,
    nwritten: *mut usize,
) -> i32 {
    if !nwritten.is_null() {
        unsafe {
            *nwritten = 0;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __imported_wasi_snapshot_preview1_path_open(
    _fd: i32,
    _dirflags: i32,
    _path: *const u8,
    _path_len: usize,
    _oflags: i32,
    _fs_rights_base: i64,
    _fs_rights_inheriting: i64,
    _fdflags: i32,
    _opened_fd: *mut i32,
) -> i32 {
    44 // ENOENT
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __imported_wasi_snapshot_preview1_path_remove_directory(
    _fd: i32,
    _path: *const u8,
    _path_len: usize,
) -> i32 {
    44 // ENOENT
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __imported_wasi_snapshot_preview1_path_unlink_file(
    _fd: i32,
    _path: *const u8,
    _path_len: usize,
) -> i32 {
    44 // ENOENT
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __imported_wasi_snapshot_preview1_proc_exit(_rval: i32) -> ! {
    std::process::abort()
}
