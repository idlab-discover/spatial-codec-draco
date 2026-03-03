//! C ABI exports.
//!
//! The generated header is written to `bindings/spatial_codec_draco.h`.

use crate::{decode_draco, encode_draco_with_config, DracoError, EncodeConfig, PointCloudEncodingMethod};

use core::ffi::c_char;
use core::ptr;

/// Status codes returned by the C ABI.
#[repr(C)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SpatialDracoStatus {
    /// Success.
    Ok = 0,
    /// A required pointer argument was NULL.
    NullPtr = 1,
    /// Input validation failed.
    InvalidInput = 2,
    /// Encode failed.
    EncodeFailed = 3,
    /// Decode failed.
    DecodeFailed = 4,
    /// A panic occurred inside the Rust wrapper (should be treated as fatal).
    Panic = 255,
}

/// A heap-allocated byte buffer returned by the C ABI.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SpatialDracoBytes {
    pub ptr: *mut u8,
    pub len: usize,
}

/// A decoded point cloud returned by the C ABI.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SpatialDracoPointCloudF32Rgb8 {
    pub coords: *mut f32,
    pub colors: *mut u8,
    pub num_points: usize,
}

/// C ABI mirror of [`EncodeConfig`].
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SpatialDracoEncodeConfig {
    pub position_quantization_bits: u32,
    pub color_quantization_bits: u32,
    pub encoding_speed: u8,
    pub decoding_speed: u8,
}

impl From<SpatialDracoEncodeConfig> for EncodeConfig {
    fn from(v: SpatialDracoEncodeConfig) -> Self {
        Self {
            position_quantization_bits: v.position_quantization_bits,
            color_quantization_bits: v.color_quantization_bits,
            encoding_speed: v.encoding_speed,
            decoding_speed: v.decoding_speed,
        }
    }
}

fn write_err(err: *mut c_char, err_len: usize, msg: &str) {
    if err.is_null() || err_len == 0 {
        return;
    }

    // Write a NUL-terminated UTF-8 string.
    let bytes = msg.as_bytes();
    let max = err_len.saturating_sub(1);
    let n = core::cmp::min(bytes.len(), max);

    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), err as *mut u8, n);
        *(err.add(n)) = 0;
    }
}

fn map_err(err: DracoError) -> (SpatialDracoStatus, String) {
    match err {
        DracoError::InvalidInput(m) => (SpatialDracoStatus::InvalidInput, m),
        DracoError::EncodeFailed(m) => (SpatialDracoStatus::EncodeFailed, m),
        DracoError::DecodeFailed(m) => (SpatialDracoStatus::DecodeFailed, m),
    }
}

unsafe fn malloc_bytes(len: usize) -> *mut u8 {
    if len == 0 {
        return ptr::null_mut();
    }
    libc::malloc(len) as *mut u8
}

unsafe fn malloc_f32(len: usize) -> *mut f32 {
    if len == 0 {
        return ptr::null_mut();
    }
    let bytes = match len.checked_mul(core::mem::size_of::<f32>()) {
        Some(b) => b,
        None => return ptr::null_mut(),
    };
    libc::malloc(bytes) as *mut f32
}

/// Encode a point cloud.
///
/// - `coords` points to `coords_len` floats and must be a multiple of 3.
/// - `colors` points to `colors_len` bytes and must equal `(coords_len/3)*3`.
/// - `out` must be non-null.
/// - `config` may be NULL to use defaults.
/// - `err` is optional; when provided, errors are written as a NUL-terminated UTF-8 string.
#[no_mangle]
pub extern "C" fn spatial_draco_encode_f32_rgb8(
    coords: *const f32,
    coords_len: usize,
    colors: *const u8,
    colors_len: usize,
    encoding_method: PointCloudEncodingMethod,
    config: *const SpatialDracoEncodeConfig,
    out: *mut SpatialDracoBytes,
    err: *mut c_char,
    err_len: usize,
) -> SpatialDracoStatus {
    // Never unwind across FFI.
    let res = std::panic::catch_unwind(|| {
        if out.is_null() {
            return Err((SpatialDracoStatus::NullPtr, "out is NULL".to_string()));
        }
        unsafe {
            (*out).ptr = ptr::null_mut();
            (*out).len = 0;
        }

        if coords.is_null() || colors.is_null() {
            return Err((
                SpatialDracoStatus::NullPtr,
                "coords/colors is NULL".to_string(),
            ));
        }
        if coords_len == 0 {
            return Err((
                SpatialDracoStatus::InvalidInput,
                "coords_len must be > 0".to_string(),
            ));
        }
        if coords_len % 3 != 0 {
            return Err((
                SpatialDracoStatus::InvalidInput,
                "coords_len must be a multiple of 3".to_string(),
            ));
        }
        let num_points = coords_len / 3;
        if colors_len != num_points * 3 {
            return Err((
                SpatialDracoStatus::InvalidInput,
                "colors_len must equal num_points * 3".to_string(),
            ));
        }

        let cfg = unsafe {
            if config.is_null() {
                EncodeConfig::default()
            } else {
                EncodeConfig::from(*config)
            }
        };

        let coords_s = unsafe {
            let temp = core::slice::from_raw_parts(coords, coords_len);
            // Compact Vec<f32> to Vec<[f32; 3]> to match the expected input of encode_draco_with_config.
            let mut compact = Vec::with_capacity(num_points);
            for chunk in temp.chunks_exact(3) {
                compact.push([chunk[0], chunk[1], chunk[2]]);
            }
            compact
        };
        let colors_s = unsafe { 
            let temp = core::slice::from_raw_parts(colors, colors_len);
            // Compact Vec<u8> to Vec<[u8; 3]> to match the expected input of encode_draco_with_config.
            let mut compact = Vec::with_capacity(num_points);
            for chunk in temp.chunks_exact(3) {
                compact.push([chunk[0], chunk[1], chunk[2]]);
            }
            compact
        };

        let encoded = encode_draco_with_config(coords_s, colors_s, encoding_method, &cfg)
            .map_err(map_err)?;

        unsafe {
            let ptr_out = malloc_bytes(encoded.len());
            if ptr_out.is_null() {
                return Err((
                    SpatialDracoStatus::EncodeFailed,
                    "allocation failed".to_string(),
                ));
            }
            ptr::copy_nonoverlapping(encoded.as_ptr(), ptr_out, encoded.len());
            (*out).ptr = ptr_out;
            (*out).len = encoded.len();
        }

        Ok(SpatialDracoStatus::Ok)
    });

    match res {
        Ok(Ok(code)) => code,
        Ok(Err((code, msg))) => {
            write_err(err, err_len, &msg);
            code
        }
        Err(_) => {
            write_err(err, err_len, "panic in spatial_draco_encode_f32_rgb8");
            SpatialDracoStatus::Panic
        }
    }
}

/// Decode Draco bytes.
///
/// - `data` points to `len` bytes.
/// - `out` must be non-null.
/// - `err` is optional; when provided, errors are written as a NUL-terminated UTF-8 string.
#[no_mangle]
pub extern "C" fn spatial_draco_decode_f32_rgb8(
    data: *const u8,
    len: usize,
    out: *mut SpatialDracoPointCloudF32Rgb8,
    err: *mut c_char,
    err_len: usize,
) -> SpatialDracoStatus {
    let res = std::panic::catch_unwind(|| {
        if out.is_null() {
            return Err((SpatialDracoStatus::NullPtr, "out is NULL".to_string()));
        }
        unsafe {
            (*out).coords = ptr::null_mut();
            (*out).colors = ptr::null_mut();
            (*out).num_points = 0;
        }

        if data.is_null() {
            return Err((SpatialDracoStatus::NullPtr, "data is NULL".to_string()));
        }
        if len == 0 {
            return Err((
                SpatialDracoStatus::InvalidInput,
                "len must be > 0".to_string(),
            ));
        }

        let bytes = unsafe { core::slice::from_raw_parts(data, len) };
        let (coords, colors) = decode_draco(bytes).map_err(map_err)?;

        if coords.len() % 3 != 0 {
            return Err((
                SpatialDracoStatus::DecodeFailed,
                "decoded coords length is not a multiple of 3".to_string(),
            ));
        }
        let num_points = coords.len() / 3;
        if colors.len() != num_points * 3 {
            return Err((
                SpatialDracoStatus::DecodeFailed,
                "decoded colors length mismatch".to_string(),
            ));
        }

        unsafe {
            let coords_ptr = malloc_f32(coords.len());
            let colors_ptr = malloc_bytes(colors.len());
            if coords_ptr.is_null() || colors_ptr.is_null() {
                if !coords_ptr.is_null() {
                    libc::free(coords_ptr as *mut libc::c_void);
                }
                if !colors_ptr.is_null() {
                    libc::free(colors_ptr as *mut libc::c_void);
                }
                return Err((
                    SpatialDracoStatus::DecodeFailed,
                    "allocation failed".to_string(),
                ));
            }

            ptr::copy_nonoverlapping(coords.as_ptr(), coords_ptr, coords.len());
            ptr::copy_nonoverlapping(colors.as_ptr(), colors_ptr, colors.len());

            (*out).coords = coords_ptr;
            (*out).colors = colors_ptr;
            (*out).num_points = num_points;
        }

        Ok(SpatialDracoStatus::Ok)
    });

    match res {
        Ok(Ok(code)) => code,
        Ok(Err((code, msg))) => {
            write_err(err, err_len, &msg);
            code
        }
        Err(_) => {
            write_err(err, err_len, "panic in spatial_draco_decode_f32_rgb8");
            SpatialDracoStatus::Panic
        }
    }
}

/// Free a byte buffer returned by `spatial_draco_encode_f32_rgb8`.
#[no_mangle]
pub extern "C" fn spatial_draco_bytes_free(bytes: SpatialDracoBytes) {
    if bytes.ptr.is_null() {
        return;
    }
    unsafe {
        libc::free(bytes.ptr as *mut libc::c_void);
    }
}

/// Free a point cloud returned by `spatial_draco_decode_f32_rgb8`.
#[no_mangle]
pub extern "C" fn spatial_draco_point_cloud_free(pc: SpatialDracoPointCloudF32Rgb8) {
    unsafe {
        if !pc.coords.is_null() {
            libc::free(pc.coords as *mut libc::c_void);
        }
        if !pc.colors.is_null() {
            libc::free(pc.colors as *mut libc::c_void);
        }
    }
}
