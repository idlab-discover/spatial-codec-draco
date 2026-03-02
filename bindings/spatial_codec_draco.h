#ifndef SPATIAL_CODEC_DRACO_H
#define SPATIAL_CODEC_DRACO_H

#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Mirror of `DracoWrapperPcEncodingMethod` in `draco_wrapper_cpp/include/wrapper.h`.
 */
typedef enum DracoWrapperPcEncodingMethod {
  DracoWrapperPointCloudSequentialEncoding = 0,
  DracoWrapperPointCloudKdTreeEncoding = 1,
} DracoWrapperPcEncodingMethod;

/**
 * Draco point cloud encoding method.
 *
 * The numeric values are part of the C ABI surface (see `docs/ffi.md`).
 */
typedef enum PointCloudEncodingMethod {
  /**
   * Sequential encoding.
   */
  Sequential = 0,
  /**
   * KD-tree encoding.
   */
  KdTree = 1,
} PointCloudEncodingMethod;

/**
 * Status codes returned by the C ABI.
 */
typedef enum SpatialDracoStatus {
  /**
   * Success.
   */
  Ok = 0,
  /**
   * A required pointer argument was NULL.
   */
  NullPtr = 1,
  /**
   * Input validation failed.
   */
  InvalidInput = 2,
  /**
   * Encode failed.
   */
  EncodeFailed = 3,
  /**
   * Decode failed.
   */
  DecodeFailed = 4,
  /**
   * A panic occurred inside the Rust wrapper (should be treated as fatal).
   */
  Panic = 255,
} SpatialDracoStatus;

/**
 * Mirror of `DracoWrapperEncodeResult` in `draco_wrapper_cpp/include/wrapper.h`.
 */
typedef struct DracoWrapperEncodeResult {
  bool success;
  uintptr_t size;
  const uint8_t *data;
  char *error_msg;
} DracoWrapperEncodeResult;

/**
 * Mirror of `DracoWrapperEncodeConfig` in `draco_wrapper_cpp/include/wrapper.h`.
 */
typedef struct DracoWrapperEncodeConfig {
  uint32_t position_quantization_bits;
  uint32_t color_quantization_bits;
  uint8_t encoding_speed;
  uint8_t decoding_speed;
} DracoWrapperEncodeConfig;

/**
 * Mirror of `DracoWrapperDecodeResult` in `draco_wrapper_cpp/include/wrapper.h`.
 */
typedef struct DracoWrapperDecodeResult {
  bool success;
  uintptr_t num_points;
  float *coords;
  uint8_t *colors;
  char *error_msg;
} DracoWrapperDecodeResult;

/**
 * C ABI mirror of [`EncodeConfig`].
 */
typedef struct SpatialDracoEncodeConfig {
  uint32_t position_quantization_bits;
  uint32_t color_quantization_bits;
  uint8_t encoding_speed;
  uint8_t decoding_speed;
} SpatialDracoEncodeConfig;

/**
 * A heap-allocated byte buffer returned by the C ABI.
 */
typedef struct SpatialDracoBytes {
  uint8_t *ptr;
  uintptr_t len;
} SpatialDracoBytes;

/**
 * A decoded point cloud returned by the C ABI.
 */
typedef struct SpatialDracoPointCloudF32Rgb8 {
  float *coords;
  uint8_t *colors;
  uintptr_t num_points;
} SpatialDracoPointCloudF32Rgb8;

extern struct DracoWrapperEncodeResult *draco_wrapper_encode_points_to_draco(const float *coords,
                                                                             const uint8_t *colors,
                                                                             uintptr_t num_points,
                                                                             enum DracoWrapperPcEncodingMethod encoding_method,
                                                                             const struct DracoWrapperEncodeConfig *config);

extern struct DracoWrapperDecodeResult *draco_wrapper_decode_draco_data(const uint8_t *encoded_data,
                                                                        uintptr_t encoded_size);

extern void draco_wrapper_free_encode_result(struct DracoWrapperEncodeResult *result);

extern void draco_wrapper_free_decode_result(struct DracoWrapperDecodeResult *result);

/**
 * Encode a point cloud.
 *
 * - `coords` points to `coords_len` floats and must be a multiple of 3.
 * - `colors` points to `colors_len` bytes and must equal `(coords_len/3)*3`.
 * - `out` must be non-null.
 * - `config` may be NULL to use defaults.
 * - `err` is optional; when provided, errors are written as a NUL-terminated UTF-8 string.
 */
enum SpatialDracoStatus spatial_draco_encode_f32_rgb8(const float *coords,
                                                      uintptr_t coords_len,
                                                      const uint8_t *colors,
                                                      uintptr_t colors_len,
                                                      enum PointCloudEncodingMethod encoding_method,
                                                      const struct SpatialDracoEncodeConfig *config,
                                                      struct SpatialDracoBytes *out,
                                                      char *err,
                                                      uintptr_t err_len);

/**
 * Decode Draco bytes.
 *
 * - `data` points to `len` bytes.
 * - `out` must be non-null.
 * - `err` is optional; when provided, errors are written as a NUL-terminated UTF-8 string.
 */
enum SpatialDracoStatus spatial_draco_decode_f32_rgb8(const uint8_t *data,
                                                      uintptr_t len,
                                                      struct SpatialDracoPointCloudF32Rgb8 *out,
                                                      char *err,
                                                      uintptr_t err_len);

/**
 * Free a byte buffer returned by `spatial_draco_encode_f32_rgb8`.
 */
void spatial_draco_bytes_free(struct SpatialDracoBytes bytes);

/**
 * Free a point cloud returned by `spatial_draco_decode_f32_rgb8`.
 */
void spatial_draco_point_cloud_free(struct SpatialDracoPointCloudF32Rgb8 pc);

#endif  /* SPATIAL_CODEC_DRACO_H */
