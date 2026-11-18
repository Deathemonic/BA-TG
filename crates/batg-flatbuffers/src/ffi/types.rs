use std::ffi::CString;
use std::fmt::Display;
use std::os::raw::{c_char, c_int};
use std::ptr;

use better_default::Default;

use super::strings::into_raw;
use crate::error::FlatBufferError;

#[repr(C)]
#[derive(Default)]
#[default(json: ptr::null_mut(), error: ptr::null_mut())]
pub struct FfiResult {
    pub json: *mut c_char,
    pub error: *mut c_char,
    pub success: c_int
}

impl FfiResult {
    pub fn ok(json: String) -> Self {
        match CString::new(json) {
            Ok(json) => Self {
                json: json.into_raw(),
                error: ptr::null_mut(),
                success: 1
            },
            Err(error) => Self::err(error)
        }
    }

    pub fn err(error: impl Display) -> Self {
        Self {
            json: ptr::null_mut(),
            error: into_raw(&error.to_string()),
            success: 0
        }
    }

    pub fn from(result: Result<String, FlatBufferError>) -> Self {
        result.map_or_else(Self::err, Self::ok)
    }

    pub fn done() -> Self {
        Self {
            json: ptr::null_mut(),
            error: ptr::null_mut(),
            success: 1
        }
    }

    pub fn from_unit(result: Result<(), FlatBufferError>) -> Self {
        result.map_or_else(Self::err, |()| Self::done())
    }
}
