#ifndef SPATIAL_CODEC_DRACO_WRAPPER_H
#define SPATIAL_CODEC_DRACO_WRAPPER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Draco point-cloud encoding methods.
 *
 * Keep values stable: they are part of the C ABI.
 */
typedef enum DracoWrapperPcEncodingMethod {
    DRACO_WRAPPER_POINT_CLOUD_SEQUENTIAL_ENCODING = 0,
    DRACO_WRAPPER_POINT_CLOUD_KD_TREE_ENCODING = 1,
} DracoWrapperPcEncodingMethod;

/**
 * Encoding configuration.
 *
 * - Quantization bits must be in [1, 31].
 * - Speed values must be in [0, 10] (Draco convention).
 */
typedef struct DracoWrapperEncodeConfig {
    uint32_t position_quantization_bits;
    uint32_t color_quantization_bits;
    uint8_t encoding_speed;
    uint8_t decoding_speed;
} DracoWrapperEncodeConfig;

/** Result of an encode operation. */
typedef struct DracoWrapperEncodeResult {
    bool success;
    size_t size;
    const uint8_t *data;
    char *error_msg; /* owned by the result; free via draco_wrapper_free_encode_result */
} DracoWrapperEncodeResult;

/** Result of a decode operation. */
typedef struct DracoWrapperDecodeResult {
    bool success;
    size_t num_points;
    float *coords;     /* len = num_points * 3 */
    uint8_t *colors;   /* len = num_points * 3 */
    char *error_msg;   /* owned by the result; free via draco_wrapper_free_decode_result */
} DracoWrapperDecodeResult;

/**
 * Encode a point cloud to Draco.
 *
 * - `coords` points to `num_points * 3` floats.
 * - `colors` points to `num_points * 3` bytes (RGB).
 * - `config` may be NULL to use defaults.
 *
 * Returns an allocated result that must be freed via `draco_wrapper_free_encode_result`.
 */
DracoWrapperEncodeResult *draco_wrapper_encode_points_to_draco(
    const float *coords,
    const uint8_t *colors,
    size_t num_points,
    DracoWrapperPcEncodingMethod encoding_method,
    const DracoWrapperEncodeConfig *config);

/**
 * Decode Draco bytes to point cloud coordinates and colors.
 *
 * Returns an allocated result that must be freed via `draco_wrapper_free_decode_result`.
 */
DracoWrapperDecodeResult *draco_wrapper_decode_draco_data(const uint8_t *encoded_data, size_t encoded_size);

/** Free an encode result returned by `draco_wrapper_encode_points_to_draco`. */
void draco_wrapper_free_encode_result(DracoWrapperEncodeResult *result);

/** Free a decode result returned by `draco_wrapper_decode_draco_data`. */
void draco_wrapper_free_decode_result(DracoWrapperDecodeResult *result);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* SPATIAL_CODEC_DRACO_WRAPPER_H */
