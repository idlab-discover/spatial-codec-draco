# C ABI

When the `ffi` feature is enabled (default), the crate exports a minimal, stable C ABI and generates:

- `bindings/spatial_codec_draco.h`

## Goals

- Layout-stable types (`#[repr(C)]`).
- No Rust ownership types (`Vec`, `Box`, `String`) across the ABI.
- Clear memory rules: the library allocates and the library frees.

## Error handling

The exported functions return a `SpatialDracoStatus`.

For human-readable errors, each API optionally accepts an error buffer (`char* err`, `size_t err_len`).
When provided, errors are written as a NUL-terminated UTF-8 string.

## Memory rules

- Encoding returns `SpatialDracoBytes` (pointer + length). Free with `spatial_draco_bytes_free`.
- Decoding returns `SpatialDracoPointCloudF32Rgb8`. Free with `spatial_draco_point_cloud_free`.

Never free the returned pointers with `free()` from the caller; always call the corresponding API free function.
