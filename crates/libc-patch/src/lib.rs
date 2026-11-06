#![no_std]
#![allow(non_camel_case_types)]

pub use core::ffi::{
    c_char, c_double, c_float, c_int, c_long, c_longlong, c_schar, c_short, c_uchar, c_uint,
    c_ulong, c_ulonglong, c_ushort, c_void,
};

pub type int8_t = i8;
pub type int16_t = i16;
pub type int32_t = i32;
pub type int64_t = i64;
pub type uint8_t = u8;
pub type uint16_t = u16;
pub type uint32_t = u32;
pub type uint64_t = u64;

pub type size_t = usize;
pub type ssize_t = isize;
pub type intptr_t = isize;
pub type uintptr_t = usize;
pub type ptrdiff_t = isize;
pub type time_t = i64;

#[cfg(windows)]
pub type wchar_t = u16;
#[cfg(not(windows))]
pub type wchar_t = i32;

pub enum FILE {}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct tm {
    pub tm_sec: c_int,
    pub tm_min: c_int,
    pub tm_hour: c_int,
    pub tm_mday: c_int,
    pub tm_mon: c_int,
    pub tm_year: c_int,
    pub tm_wday: c_int,
    pub tm_yday: c_int,
    pub tm_isdst: c_int,
    #[cfg(not(windows))]
    pub tm_gmtoff: c_long,
    #[cfg(not(windows))]
    pub tm_zone: *const c_char,
}
