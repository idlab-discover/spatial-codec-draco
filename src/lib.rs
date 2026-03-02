#![doc = include_str!("../README.md")]

mod cpp;
mod error;
mod types;

#[cfg(feature = "ffi")]
pub mod ffi;

pub use error::DracoError;
pub use types::{EncodeConfig, PointCloudEncodingMethod};

use std::ffi::CStr;
use std::slice;

use crate::cpp::{
    draco_wrapper_decode_draco_data, draco_wrapper_encode_points_to_draco,
    draco_wrapper_free_decode_result, draco_wrapper_free_encode_result, DracoWrapperDecodeResult,
    DracoWrapperEncodeConfig, DracoWrapperEncodeResult, DracoWrapperPcEncodingMethod,
};

/// Encode a point cloud to Draco using [`EncodeConfig::default`].
///
/// `coords` must contain `num_points * 3` floats (XYZ), and `colors` must contain
/// `num_points * 3` bytes (RGB).
pub fn encode_draco(
    coords: &[f32],
    colors: &[u8],
    encoding_method: PointCloudEncodingMethod,
) -> Result<Vec<u8>, DracoError> {
    encode_draco_with_config(coords, colors, encoding_method, &EncodeConfig::default())
}

/// Encode a point cloud to Draco with explicit configuration.
///
/// `coords` must contain `num_points * 3` floats (XYZ), and `colors` must contain
/// `num_points * 3` bytes (RGB).
pub fn encode_draco_with_config(
    coords: &[f32],
    colors: &[u8],
    encoding_method: PointCloudEncodingMethod,
    config: &EncodeConfig,
) -> Result<Vec<u8>, DracoError> {
    config.validate()?;

    if coords.is_empty() {
        return Err(DracoError::InvalidInput(
            "coords must contain at least one point".to_string(),
        ));
    }
    if coords.len() % 3 != 0 {
        return Err(DracoError::InvalidInput(
            "coords length must be a multiple of 3".to_string(),
        ));
    }

    let num_points = coords.len() / 3;
    let expected_colors_len = num_points
        .checked_mul(3)
        .ok_or_else(|| DracoError::InvalidInput("num_points overflow".to_string()))?;

    if colors.len() != expected_colors_len {
        return Err(DracoError::InvalidInput(
            "colors length must equal num_points * 3".to_string(),
        ));
    }

    let cfg = DracoWrapperEncodeConfig::from(*config);
    let method = DracoWrapperPcEncodingMethod::from(encoding_method);

    unsafe {
        let ptr: *mut DracoWrapperEncodeResult = draco_wrapper_encode_points_to_draco(
            coords.as_ptr(),
            colors.as_ptr(),
            num_points,
            method,
            &cfg,
        );

        if ptr.is_null() {
            return Err(DracoError::EncodeFailed(
                "C++ wrapper returned a null result".to_string(),
            ));
        }

        struct Guard(*mut DracoWrapperEncodeResult);
        impl Drop for Guard {
            fn drop(&mut self) {
                unsafe { draco_wrapper_free_encode_result(self.0) }
            }
        }

        let guard = Guard(ptr);
        let res: &DracoWrapperEncodeResult = &*guard.0;

        if !res.success {
            let msg = if !res.error_msg.is_null() {
                CStr::from_ptr(res.error_msg)
                    .to_string_lossy()
                    .into_owned()
            } else {
                "Unknown encode error".to_string()
            };
            return Err(DracoError::EncodeFailed(msg));
        }

        if res.data.is_null() {
            return Err(DracoError::EncodeFailed(
                "C++ wrapper reported success but returned null data".to_string(),
            ));
        }

        // `size_t` -> usize is safe by definition.
        let size = res.size;
        if size == 0 {
            return Err(DracoError::EncodeFailed(
                "C++ wrapper returned an empty payload".to_string(),
            ));
        }

        let bytes = slice::from_raw_parts(res.data, size);
        Ok(bytes.to_vec())
    }
}

/// Decode Draco-encoded bytes into point coordinates (XYZ) and colors (RGB).
pub fn decode_draco(encoded_data: &[u8]) -> Result<(Vec<f32>, Vec<u8>), DracoError> {
    if encoded_data.is_empty() {
        return Err(DracoError::InvalidInput(
            "encoded_data must be non-empty".to_string(),
        ));
    }

    unsafe {
        let ptr: *mut DracoWrapperDecodeResult =
            draco_wrapper_decode_draco_data(encoded_data.as_ptr(), encoded_data.len());

        if ptr.is_null() {
            return Err(DracoError::DecodeFailed(
                "C++ wrapper returned a null result".to_string(),
            ));
        }

        struct Guard(*mut DracoWrapperDecodeResult);
        impl Drop for Guard {
            fn drop(&mut self) {
                unsafe { draco_wrapper_free_decode_result(self.0) }
            }
        }

        let guard = Guard(ptr);
        let res: &DracoWrapperDecodeResult = &*guard.0;

        if !res.success {
            let msg = if !res.error_msg.is_null() {
                CStr::from_ptr(res.error_msg)
                    .to_string_lossy()
                    .into_owned()
            } else {
                "Unknown decode error".to_string()
            };
            return Err(DracoError::DecodeFailed(msg));
        }

        if res.coords.is_null() || res.colors.is_null() {
            return Err(DracoError::DecodeFailed(
                "C++ wrapper reported success but returned null buffers".to_string(),
            ));
        }

        let n3 = res
            .num_points
            .checked_mul(3)
            .ok_or_else(|| DracoError::DecodeFailed("num_points overflow".to_string()))?;

        let coords = slice::from_raw_parts(res.coords, n3).to_vec();
        let colors = slice::from_raw_parts(res.colors, n3).to_vec();

        Ok((coords, colors))
    }
}
