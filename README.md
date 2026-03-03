# spatial_codec_draco

A small, robust wrapper around the upstream **Draco** point-cloud codec.

This crate vendors Draco as a git submodule (`./draco/`), builds a minimal C++ wrapper (`./draco_wrapper_cpp/`), and exposes:

- a **safe Rust API** for encoding/decoding point clouds, and
- an optional **C ABI** (generated header) for C/C++ (and other languages) to stay layout-compatible.

## Documentation

- Design notes and rationale: [`docs/design.md`](docs/design.md)
- C ABI types + memory/ownership rules: [`docs/ffi.md`](docs/ffi.md)

## Prerequisites

You typically only need:

- **CMake**
- a **C++17** compiler toolchain
- (**optional**) **Ninja** as a faster CMake generator

For Windows cross-compilation from Linux, **MinGW-w64** is commonly used.

## Getting started

Make sure the Draco submodule is present:

```bash
git submodule update --init --recursive
```

Build:

```bash
cargo build
```

## Rust usage

```rust
use spatial_codec_draco::{decode_draco, encode_draco, EncodeConfig, PointCloudEncodingMethod};

let coords: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
let colors: Vec<[u8; 3]> = vec![[255, 0, 0], [0, 255, 0]];

// Convenience: uses `EncodeConfig::default()`.
let encoded = encode_draco(&coords, &colors, PointCloudEncodingMethod::KdTree)?;

// Configurable: control quantization and speed/size tradeoffs.
let cfg = EncodeConfig {
    position_quantization_bits: 12,
    color_quantization_bits: 8,
    encoding_speed: 5,
    decoding_speed: 5,
};
let encoded = spatial_codec_draco::encode_draco_with_config(
    &coords,
    &colors,
    PointCloudEncodingMethod::KdTree,
    &cfg,
)?;

let (decoded_coords, decoded_colors) = decode_draco(&encoded)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## C ABI

When the `ffi` feature is enabled (default), the build produces a header at:

- `bindings/spatial_codec_draco.h`

This header exposes only **layout-stable** (`#[repr(C)]`) types and avoids Rust-specific ownership types (e.g. `Vec`, `Box`, `String`) across ABI boundaries. External projects are encouraged to base their native-side type definitions directly on this header to remain compatible.

See [`docs/ffi.md`](docs/ffi.md) for the exact API, error handling, and memory rules.

## Examples

- Combine multiple folders of Draco frames into a single stream:

```bash
cargo run --example combine_folders -- --help
```
