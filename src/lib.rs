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
    coords: &[[f32; 3]],
    colors: &[[u8; 3]],
    encoding_method: PointCloudEncodingMethod,
) -> Result<Vec<u8>, DracoError> {
    encode_draco_with_config(coords, colors, encoding_method, &EncodeConfig::default())
}

/// Encode a point cloud to Draco with explicit configuration.
///
/// `coords` must contain `num_points * 3` floats (XYZ), and `colors` must contain
/// `num_points * 3` bytes (RGB).
pub fn encode_draco_with_config(
    coords: &[[f32; 3]],
    colors: &[[u8; 3]],
    encoding_method: PointCloudEncodingMethod,
    config: &EncodeConfig,
) -> Result<Vec<u8>, DracoError> {
    let mut out = Vec::new();
    encode_draco_with_config_into(coords, colors, encoding_method, config, &mut out)?;
    Ok(out)
}

/// Encode a point cloud to Draco, writing directly into `out`.
pub fn encode_draco_with_config_into(
    coords: &[[f32; 3]],
    colors: &[[u8; 3]],
    encoding_method: PointCloudEncodingMethod,
    config: &EncodeConfig,
    out: &mut Vec<u8>,
) -> Result<(), DracoError> {
    config.validate()?;

    if coords.is_empty() {
        return Err(DracoError::InvalidInput("coords must be non-empty".into()));
    }
    if colors.len() != coords.len() {
        return Err(DracoError::InvalidInput(
            "colors length must equal coords length".into(),
        ));
    }

    // encode using wrapper (see 3.2 for the high-throughput wrapper change)
    let cfg = DracoWrapperEncodeConfig::from(*config);
    let method = DracoWrapperPcEncodingMethod::from(encoding_method);

    unsafe {
        let ptr = draco_wrapper_encode_points_to_draco(
            coords.as_ptr() as *const f32,
            colors.as_ptr() as *const u8,
            coords.len(),
            method,
            &cfg,
        );

        if ptr.is_null() {
            return Err(DracoError::EncodeFailed("null encode result".into()));
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
                std::ffi::CStr::from_ptr(res.error_msg).to_string_lossy().into_owned()
            } else {
                "Unknown encode error".into()
            };
            return Err(DracoError::EncodeFailed(msg));
        }

        if res.data.is_null() || res.size == 0 {
            return Err(DracoError::EncodeFailed("empty encode payload".into()));
        }

        // write directly into caller buffer
        // TODO: instead of copying, we could consider a zero-copy API where the caller provides an output buffer and we write directly into it from C++. This would require more complex memory management (e.g. ensuring the buffer is large enough, handling ownership/lifetimes, etc.) but could be more efficient for large point clouds.
        out.clear();
        out.reserve(res.size);
        let dst = out.as_mut_ptr();
        std::ptr::copy_nonoverlapping(res.data, dst, res.size);
        out.set_len(res.size);

        Ok(())
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

pub fn decode_draco_compact(encoded_data: &[u8]) -> Result<(Vec<[f32; 3]>, Vec<[u8; 3]>), DracoError> {
    let (coords, colors) = decode_draco(encoded_data)?;

    if coords.len() % 3 != 0 || colors.len() % 3 != 0 || coords.len() != colors.len() {
        return Err(DracoError::DecodeFailed(
            "Decoded data has invalid length".to_string(),
        ));
    }

    let num_points = coords.len() / 3;
    let mut coords_out = Vec::with_capacity(num_points);
    let mut colors_out = Vec::with_capacity(num_points);

    for i in 0..num_points {
        coords_out.push([
            coords[i * 3],
            coords[i * 3 + 1],
            coords[i * 3 + 2],
        ]);
        colors_out.push([
            colors[i * 3],
            colors[i * 3 + 1],
            colors[i * 3 + 2],
        ]);
    }

    Ok((coords_out, colors_out))
}
