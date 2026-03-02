//! Raw FFI bindings to the bundled C++ wrapper.
//!
//! This module is intentionally private: callers should use the safe Rust API
//! or the `ffi` module.

use std::os::raw::c_char;

use crate::types::{EncodeConfig, PointCloudEncodingMethod};

/// Mirror of `DracoWrapperPcEncodingMethod` in `draco_wrapper_cpp/include/wrapper.h`.
#[repr(C)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum DracoWrapperPcEncodingMethod {
    DracoWrapperPointCloudSequentialEncoding = 0,
    DracoWrapperPointCloudKdTreeEncoding = 1,
}

impl From<PointCloudEncodingMethod> for DracoWrapperPcEncodingMethod {
    fn from(v: PointCloudEncodingMethod) -> Self {
        match v {
            PointCloudEncodingMethod::Sequential => {
                DracoWrapperPcEncodingMethod::DracoWrapperPointCloudSequentialEncoding
            }
            PointCloudEncodingMethod::KdTree => {
                DracoWrapperPcEncodingMethod::DracoWrapperPointCloudKdTreeEncoding
            }
        }
    }
}

/// Mirror of `DracoWrapperEncodeConfig` in `draco_wrapper_cpp/include/wrapper.h`.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DracoWrapperEncodeConfig {
    pub position_quantization_bits: u32,
    pub color_quantization_bits: u32,
    pub encoding_speed: u8,
    pub decoding_speed: u8,
}

impl From<EncodeConfig> for DracoWrapperEncodeConfig {
    fn from(v: EncodeConfig) -> Self {
        Self {
            position_quantization_bits: v.position_quantization_bits,
            color_quantization_bits: v.color_quantization_bits,
            encoding_speed: v.encoding_speed,
            decoding_speed: v.decoding_speed,
        }
    }
}

/// Mirror of `DracoWrapperEncodeResult` in `draco_wrapper_cpp/include/wrapper.h`.
#[repr(C)]
#[derive(Debug)]
pub struct DracoWrapperEncodeResult {
    pub success: bool,
    pub size: usize,
    pub data: *const u8,
    pub error_msg: *mut c_char,
}

/// Mirror of `DracoWrapperDecodeResult` in `draco_wrapper_cpp/include/wrapper.h`.
#[repr(C)]
#[derive(Debug)]
pub struct DracoWrapperDecodeResult {
    pub success: bool,
    pub num_points: usize,
    pub coords: *mut f32,
    pub colors: *mut u8,
    pub error_msg: *mut c_char,
}

extern "C" {
    pub fn draco_wrapper_encode_points_to_draco(
        coords: *const f32,
        colors: *const u8,
        num_points: usize,
        encoding_method: DracoWrapperPcEncodingMethod,
        config: *const DracoWrapperEncodeConfig,
    ) -> *mut DracoWrapperEncodeResult;

    pub fn draco_wrapper_decode_draco_data(
        encoded_data: *const u8,
        encoded_size: usize,
    ) -> *mut DracoWrapperDecodeResult;

    pub fn draco_wrapper_free_encode_result(result: *mut DracoWrapperEncodeResult);

    pub fn draco_wrapper_free_decode_result(result: *mut DracoWrapperDecodeResult);
}
