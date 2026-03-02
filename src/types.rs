use crate::error::DracoError;

/// Draco point cloud encoding method.
///
/// The numeric values are part of the C ABI surface (see `docs/ffi.md`).
#[repr(C)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PointCloudEncodingMethod {
    /// Sequential encoding.
    Sequential = 0,
    /// KD-tree encoding.
    KdTree = 1,
}

/// Encode configuration.
///
/// These parameters map to Draco's encoder settings.
#[repr(C)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct EncodeConfig {
    /// Quantization bits for positions (XYZ).
    ///
    /// Valid range: `[1, 31]`.
    pub position_quantization_bits: u32,

    /// Quantization bits for colors (RGB).
    ///
    /// Valid range: `[1, 31]`.
    pub color_quantization_bits: u32,

    /// Encoder speed in `[0, 10]`.
    ///
    /// Larger values prioritize speed over compression ratio.
    pub encoding_speed: u8,

    /// Decoder speed in `[0, 10]`.
    ///
    /// Larger values prioritize speed.
    pub decoding_speed: u8,
}

impl Default for EncodeConfig {
    fn default() -> Self {
        Self {
            position_quantization_bits: 11,
            color_quantization_bits: 8,
            encoding_speed: 5,
            decoding_speed: 5,
        }
    }
}

impl EncodeConfig {
    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), DracoError> {
        fn check_u32(v: u32, lo: u32, hi: u32, name: &str) -> Result<(), DracoError> {
            if (lo..=hi).contains(&v) {
                Ok(())
            } else {
                Err(DracoError::InvalidInput(format!(
                    "{name} must be in [{lo}, {hi}]"
                )))
            }
        }
        fn check_u8(v: u8, lo: u8, hi: u8, name: &str) -> Result<(), DracoError> {
            if (lo..=hi).contains(&v) {
                Ok(())
            } else {
                Err(DracoError::InvalidInput(format!(
                    "{name} must be in [{lo}, {hi}]"
                )))
            }
        }

        check_u32(
            self.position_quantization_bits,
            1,
            31,
            "position_quantization_bits",
        )?;
        check_u32(self.color_quantization_bits, 1, 31, "color_quantization_bits")?;
        check_u8(self.encoding_speed, 0, 10, "encoding_speed")?;
        check_u8(self.decoding_speed, 0, 10, "decoding_speed")?;
        Ok(())
    }
}
