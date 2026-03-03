cmake_minimum_required(VERSION 3.15 FATAL_ERROR)

# Silence the Draco warning about removed FindPython* modules on newer CMake,
# without impacting older versions.
if(POLICY CMP0148)
  cmake_policy(SET CMP0148 NEW)
endif()

project(draco_wrapper_cpp LANGUAGES C CXX)

# -------------------------
# User-facing configuration
# -------------------------
option(SPATIAL_DRACO_ENABLE_NATIVE_OPTIMIZATIONS "Enable native CPU tuning flags where supported" OFF)
option(SPATIAL_DRACO_STATIC_STDLIB "For Windows GNU builds: request static libstdc++/libgcc at final link" OFF)
option(SPATIAL_DRACO_MSVC_STATIC_RUNTIME "For MSVC: prefer /MT over /MD (must match Rust CRT selection)" OFF)
option(SPATIAL_DRACO_WERROR "Treat warnings as errors for the wrapper (not Draco)" OFF)

# Select MSVC runtime early so it affects all downstream targets (including Draco).
#if (MSVC)
#  if (SPATIAL_DRACO_MSVC_STATIC_RUNTIME)
#    set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>")
#  else()
#    set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreadedDLL$<$<CONFIG:Debug>:Debug>")
#  endif()
#endif()

# Standards / PIC
set(CMAKE_C_STANDARD 17)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# -------------------------
# Draco configuration
# -------------------------
# Keep Draco lean-ish; exact option names may differ across Draco versions, but
# these are safe in the upstream project we vend as a submodule.
set(DRACO_FAST ON)
set(DRACO_TESTS OFF)
set(DRACO_WASM OFF)
set(DRACO_INSTALL OFF)

# SIMD toggles
if (CMAKE_SYSTEM_PROCESSOR MATCHES "^(aarch64|ARM|arm)")
  set(DRACO_ENABLE_NEON ON)
else()
  set(DRACO_ENABLE_NEON OFF)
endif()

if (CMAKE_SYSTEM_PROCESSOR MATCHES "^(x86_64|AMD64|i[3-6]86)$")
  set(DRACO_ENABLE_SSE4_1 ON)
else()
  set(DRACO_ENABLE_SSE4_1 OFF)
endif()

# Important: build Draco into a dedicated subdir of the *top-level* build dir.
# Draco generates draco/draco_features.h under ${CMAKE_BINARY_DIR}/draco/...
add_subdirectory(../draco ${CMAKE_BINARY_DIR}/draco)

# Draco target naming differs across versions/build options.
# Prefer static if available, otherwise fall back to whatever Draco exports.
set(SPATIAL_DRACO_TARGET "")
if(TARGET draco_static)
  set(SPATIAL_DRACO_TARGET draco_static)
elseif(TARGET draco)
  set(SPATIAL_DRACO_TARGET draco)
elseif(TARGET draco_shared)
  set(SPATIAL_DRACO_TARGET draco_shared)
else()
  message(FATAL_ERROR "Could not find a Draco CMake target (expected draco_static, draco, or draco_shared).")
endif()

# -------------------------
# Wrapper library
# -------------------------
add_library(draco_wrapper_cpp_static STATIC)
target_sources(draco_wrapper_cpp_static
  PRIVATE
    ${CMAKE_CURRENT_LIST_DIR}/src/wrapper.cpp
  PUBLIC
    ${CMAKE_CURRENT_LIST_DIR}/include/wrapper.h
)

# Our own header is "normal" includes.
target_include_directories(draco_wrapper_cpp_static
  PUBLIC
    ${CMAKE_CURRENT_LIST_DIR}/include
)

# Draco headers + generated headers should not poison the wrapper with warnings.
# NOTE: draco/draco_features.h is generated into ${CMAKE_BINARY_DIR}/draco/...
target_include_directories(draco_wrapper_cpp_static
  SYSTEM PUBLIC
    ${CMAKE_CURRENT_LIST_DIR}/../draco/src
    ${CMAKE_BINARY_DIR}          # <-- required for "draco/draco_features.h"
    ${CMAKE_BINARY_DIR}/draco    # harmless; keeps other generated includes working
)

add_dependencies(draco_wrapper_cpp_static ${SPATIAL_DRACO_TARGET})
target_link_libraries(draco_wrapper_cpp_static PUBLIC ${SPATIAL_DRACO_TARGET})

# -------------------------
# Toolchain-conditional flags
# -------------------------
if (MSVC)
  target_compile_options(draco_wrapper_cpp_static PRIVATE /W4 /permissive- /EHsc)
  if (SPATIAL_DRACO_WERROR)
    target_compile_options(draco_wrapper_cpp_static PRIVATE /WX)
  endif()
  target_compile_definitions(draco_wrapper_cpp_static PRIVATE _CRT_SECURE_NO_WARNINGS)
else()
  target_compile_options(draco_wrapper_cpp_static PRIVATE -Wall -Wextra -Wpedantic)
  if (SPATIAL_DRACO_WERROR)
    target_compile_options(draco_wrapper_cpp_static PRIVATE -Werror)
  endif()

  if (SPATIAL_DRACO_ENABLE_NATIVE_OPTIMIZATIONS)
    target_compile_options(draco_wrapper_cpp_static PRIVATE -march=native -mtune=native)
  endif()
endif()

# NOTE: SPATIAL_DRACO_STATIC_STDLIB affects the final Rust link step (cargo),
# not this static library. build.rs emits the relevant rustc link args.