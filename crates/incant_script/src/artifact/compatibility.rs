//! Closed, reviewed producer/consumer contract. This is compatibility, never
//! executable authentication; the unsafe embedded loader still requires a fresh
//! trusted cook. Default Android admission remains denied. The explicit candidate
//! feature is only for a trusted fresh-source numeric compatibility experiment.
use std::{
    ffi::CStr,
    mem::{align_of, size_of},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct BytecodeAbi {
    pub version: u8,
    pub pointer_bits: u8,
    pub little_endian: bool,
    pub value_bytes: u8,
    pub value_alignment: u8,
    pub atom_bytes: u8,
    pub float_mantissa: u8,
    pub float_max_exponent: i16,
    pub write_flags: u32,
    pub default_c_flags: bool,
}

impl BytecodeAbi {
    pub const fn current() -> Self {
        Self {
            version: 28,
            pointer_bits: usize::BITS as u8,
            little_endian: cfg!(target_endian = "little"),
            value_bytes: size_of::<rquickjs::qjs::JSValue>() as u8,
            value_alignment: align_of::<rquickjs::qjs::JSValue>() as u8,
            atom_bytes: size_of::<rquickjs::qjs::JSAtom>() as u8,
            float_mantissa: f64::MANTISSA_DIGITS as u8,
            float_max_exponent: f64::MAX_EXP as i16,
            write_flags: rquickjs::qjs::JS_WRITE_OBJ_BYTECODE
                | rquickjs::qjs::JS_WRITE_OBJ_STRIP_SOURCE,
            default_c_flags: matches!(env!("INCANT_SCRIPT_DEFAULT_C_FLAGS").as_bytes(), b"1"),
        }
    }

    pub(crate) fn reviewed_arm64(self) -> bool {
        self.version == 28
            && self.pointer_bits == 64
            && self.little_endian
            && self.value_bytes == 16
            && self.value_alignment == 8
            && self.atom_bytes == 4
            && self.float_mantissa == 53
            && self.float_max_exponent == 1024
            && self.default_c_flags
            && self.write_flags
                == rquickjs::qjs::JS_WRITE_OBJ_BYTECODE | rquickjs::qjs::JS_WRITE_OBJ_STRIP_SOURCE
    }
}

/// Supported fresh source-cook routes. No implicit architecture-family match.
pub fn admits(producer: &str, destination: &str, abi: BytecodeAbi) -> bool {
    producer == destination
        || (producer == "aarch64-apple-darwin"
            && (matches!(destination, "aarch64-apple-ios" | "aarch64-apple-ios-sim")
                || (cfg!(feature = "probe-android-bytecode")
                    && destination == "aarch64-linux-android"))
            && abi.reviewed_arm64())
}

pub(crate) fn linked_engine_matches() -> bool {
    // SAFETY: the pinned linked QuickJS API returns its static version string.
    let version = unsafe { CStr::from_ptr(rquickjs::qjs::JS_GetVersion()) };
    version.to_bytes() == b"0.16.2"
}
