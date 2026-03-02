# Design

This crate is intentionally small:

- **Upstream Draco** is included as a submodule under `./draco/`.
- `./draco_wrapper_cpp/` provides a thin C-compatible wrapper around Draco's C++ APIs.
- The Rust crate exposes a safe API on top and optionally exports a minimal C ABI.

## Why a C++ wrapper?

Draco is a C++ library with a fairly large surface area. Direct bindgen bindings tend to:

- pull in large parts of the C++ standard library,
- be brittle across toolchains/platforms, and
- force `libclang` as a build dependency.

Instead, we keep the boundary small:

- a small set of C-linkage functions, and
- plain structs/enums with stable layout.

This also makes it much easier to offer a stable Rust-level C ABI.

## Rust API principles

- **Validate inputs** early (lengths, null pointers, mismatched arrays).
- **No hidden global state**.
- **Explicit configuration** via `EncodeConfig`.
- Convenience wrappers (e.g. `encode_draco`) delegate to the configurable variant (`encode_draco_with_config`) with defaults.

## Configuration

`EncodeConfig` controls the parameters that matter most for point-cloud encoding:

- position quantization bits
- color quantization bits
- encoder/decoder speed tradeoff (Draco `SetSpeedOptions`)

The C++ wrapper validates these values and fails with a readable error message if they are out of range.

## Build configuration

The build is driven by CMake via `build.rs`. A few environment variables can be used to override defaults:

- `SPATIAL_DRACO_OUT_DIR`: where to place CMake build artifacts (useful in monorepos).
- `DRACO_CMAKE_GENERATOR`: e.g. `Ninja`.
- `DRACO_CMAKE_TOOLCHAIN_FILE`: toolchain file for cross-compiling.
- `DRACO_CMAKE_C_COMPILER` / `DRACO_CMAKE_CXX_COMPILER`: explicit compiler paths.
- `DRACO_ENABLE_NATIVE_OPTIMIZATIONS=1`: enable `-march=native`.
- `DRACO_STATIC_STDLIB=1`: (MinGW) statically link libstdc++/libgcc.
