use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use cmake::Config;

fn bool_to_on_off(v: bool) -> &'static str {
    if v { "ON" } else { "OFF" }
}

fn env_bool(var: &str) -> bool {
    match env::var(var) {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            matches!(v.as_str(), "1" | "true" | "yes" | "on")
        }
        Err(_) => false,
    }
}

fn env_opt(var: &str) -> Option<String> {
    env::var(var)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn env_target_opt(prefix: &str, target: &str) -> Option<String> {
    // Cargo-compatible pattern: CC_x86_64_pc_windows_gnu
    let suffix = target.replace('-', "_");
    let key = format!("{prefix}_{suffix}");
    env_opt(&key).or_else(|| env_opt(prefix))
}

#[cfg(windows)]
fn strip_windows_verbatim_prefix(p: &Path) -> PathBuf {
    use std::path::{Component, Prefix};

    let mut comps = p.components();
    match comps.next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            // \\?\C:\...
            Prefix::VerbatimDisk(drive) => {
                let mut out = PathBuf::from(format!("{}:\\", drive as char));
                for c in comps {
                    out.push(c.as_os_str());
                }
                out
            }
            // \\?\UNC\server\share\...
            Prefix::VerbatimUNC(server, share) => {
                let mut out = PathBuf::from(r"\\");
                out.push(server);
                out.push(share);
                for c in comps {
                    out.push(c.as_os_str());
                }
                out
            }
            _ => p.to_path_buf(),
        },
        _ => p.to_path_buf(),
    }
}

#[cfg(not(windows))]
fn strip_windows_verbatim_prefix(p: &Path) -> PathBuf {
    p.to_path_buf()
}

fn fnv1a64(s: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut h = FNV_OFFSET;
    for &b in s.as_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

fn ensure_file(path: &Path, hint: &str) -> Result<(), String> {
    if path.is_file() {
        Ok(())
    } else {
        Err(format!("Missing required file: {}. {hint}", path.display()))
    }
}

fn ensure_dir(path: &Path, hint: &str) -> Result<(), String> {
    if path.is_dir() {
        Ok(())
    } else {
        Err(format!("Missing required directory: {}. {hint}", path.display()))
    }
}

fn resolve_out_dir(manifest_dir: &Path, target: &str, profile: &str) -> Result<PathBuf, String> {
    println!("cargo:rerun-if-env-changed=SPATIAL_DRACO_OUT_DIR");
    println!("cargo:rerun-if-env-changed=CARGO_TARGET_DIR");
    println!("cargo:rerun-if-env-changed=PROFILE");
    println!("cargo:rerun-if-env-changed=TARGET");

    let base = if let Some(p) = env::var_os("SPATIAL_DRACO_OUT_DIR") {
        PathBuf::from(p)
    } else if cfg!(windows) {
        // Keep build paths short on Windows to avoid MAX_PATH/MSBuild issues.
        manifest_dir.join(".cmake-out")
    } else if let Some(p) = env::var_os("CARGO_TARGET_DIR") {
        PathBuf::from(p).join("cmake-out")
    } else {
        PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR missing")?).join("cmake-out")
    };

    let base = if base.is_absolute() { base } else { manifest_dir.join(base) };

    // IMPORTANT: do NOT canonicalize on Windows (it introduces \\?\ paths).
    #[cfg(windows)]
    let base = strip_windows_verbatim_prefix(&base);
    #[cfg(not(windows))]
    let base = fs::canonicalize(&base).unwrap_or(base);

    // Shorten deep paths on Windows aggressively.
    let prof = if profile.eq_ignore_ascii_case("release") { "rel" } else { "dbg" };
    let tgt_hash = format!("{:016x}", fnv1a64(target));

    let out_dir = if cfg!(windows) {
        base.join("scd").join(tgt_hash).join(prof)
    } else {
        base.join("spatial_codec_draco").join(target).join(profile)
    };

    fs::create_dir_all(&out_dir)
        .map_err(|e| format!("Failed to create build output directory {}: {e}", out_dir.display()))?;

    Ok(out_dir)
}

fn build_cpp(manifest_dir: &Path, out_dir: &Path, target: &str) -> Result<PathBuf, String> {
    // Validate source tree early with actionable errors.
    ensure_dir(
        &manifest_dir.join("draco"),
        "Did you run `git submodule update --init --recursive`?",
    )?;
    ensure_file(&manifest_dir.join("draco/CMakeLists.txt"), "Draco submodule looks incomplete.")?;
    ensure_dir(&manifest_dir.join("draco_wrapper_cpp"), "Missing draco_wrapper_cpp directory.")?;
    ensure_file(
        &manifest_dir.join("draco_wrapper_cpp/CMakeLists.txt"),
        "Missing draco_wrapper_cpp CMake project.",
    )?;
    ensure_file(
        &manifest_dir.join("draco_wrapper_cpp/src/wrapper.cpp"),
        "Missing C++ wrapper source.",
    )?;
    ensure_file(
        &manifest_dir.join("draco_wrapper_cpp/include/wrapper.h"),
        "Missing C wrapper header.",
    )?;

    println!("cargo:rerun-if-changed=draco/CMakeLists.txt");
    println!("cargo:rerun-if-changed=draco/src");
    println!("cargo:rerun-if-changed=draco/cmake");
    println!("cargo:rerun-if-changed=draco_wrapper_cpp/CMakeLists.txt");
    println!("cargo:rerun-if-changed=draco_wrapper_cpp/src");
    println!("cargo:rerun-if-changed=draco_wrapper_cpp/include");
    println!("cargo:rerun-if-changed=build.rs");

    println!("cargo:rerun-if-env-changed=DRACO_CMAKE_GENERATOR");
    println!("cargo:rerun-if-env-changed=DRACO_CMAKE_TOOLCHAIN_FILE");
    println!("cargo:rerun-if-env-changed=DRACO_CMAKE_C_COMPILER");
    println!("cargo:rerun-if-env-changed=DRACO_CMAKE_CXX_COMPILER");
    println!("cargo:rerun-if-env-changed=DRACO_ENABLE_NATIVE_OPTIMIZATIONS");
    println!("cargo:rerun-if-env-changed=DRACO_STATIC_STDLIB");
    println!("cargo:rerun-if-env-changed=DRACO_MSVC_STATIC_RUNTIME");
    println!("cargo:rerun-if-env-changed=DRACO_WERROR");

    // Toolchain selection can also be driven by Cargo's conventional env vars.
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=CXX");

    let host = env::var("HOST").unwrap_or_default();
    let is_cross = !host.is_empty() && host != target;

    // Cross-compiling to MSVC from non-Windows hosts is not a supported setup for this crate.
    if is_cross && target.contains("windows") && target.contains("msvc") {
        return Err(
            "Cross-compiling to a `*-windows-msvc` target from a non-Windows host is not supported. \
             Build on a Windows host for MSVC targets, or use `*-windows-gnu` with a MinGW toolchain."
                .to_string(),
        );
    }

    let enable_native = env_bool("DRACO_ENABLE_NATIVE_OPTIMIZATIONS");
    let static_stdlib = env_bool("DRACO_STATIC_STDLIB");
    let msvc_static_runtime = env_bool("DRACO_MSVC_STATIC_RUNTIME");
    let werror = env_bool("DRACO_WERROR");

    let mut cfg = Config::new(manifest_dir.join("draco_wrapper_cpp"));
    cfg.out_dir(out_dir);

    // Reliability > clever caching: re-run configure when build script runs.
    // This avoids stale CMakeCache issues when tweaking include dirs / options.
    cfg.always_configure(true);

    if let Some(generator) = env_opt("DRACO_CMAKE_GENERATOR") {
        cfg.generator(generator);
    }

    if let Some(toolchain) = env_opt("DRACO_CMAKE_TOOLCHAIN_FILE") {
        cfg.define("CMAKE_TOOLCHAIN_FILE", toolchain);
    }

    // Prefer explicit overrides, but fall back to Cargo-style env vars when cross-compiling.
    let cc = env_opt("DRACO_CMAKE_C_COMPILER").or_else(|| env_target_opt("CC", target));
    let cxx = env_opt("DRACO_CMAKE_CXX_COMPILER").or_else(|| env_target_opt("CXX", target));

    if let Some(cc) = cc {
        cfg.define("CMAKE_C_COMPILER", cc);
    }
    if let Some(cxx) = cxx {
        cfg.define("CMAKE_CXX_COMPILER", cxx);
    }

    // If we are cross-compiling and no toolchain/compilers were provided, fail fast with
    // an actionable error. This prevents accidentally building host binaries.
    let toolchain_set = env_opt("DRACO_CMAKE_TOOLCHAIN_FILE").is_some();
    let compiler_set = env_opt("DRACO_CMAKE_C_COMPILER").is_some()
        || env_opt("DRACO_CMAKE_CXX_COMPILER").is_some()
        || env_target_opt("CC", target).is_some()
        || env_target_opt("CXX", target).is_some();

    if is_cross && !toolchain_set && !compiler_set {
        return Err(format!(
            "Cross-compilation detected (HOST={host}, TARGET={target}) but no C/C++ toolchain was provided. \
             Set DRACO_CMAKE_TOOLCHAIN_FILE, or set CC_{t} and CXX_{t} (or CC/CXX) to a cross compiler.",
            t = target.replace('-', "_")
        ));
    }

    // Drive compiler-specific behavior from CMake options (so MSVC and GNU can coexist cleanly).
    cfg.define("SPATIAL_DRACO_ENABLE_NATIVE_OPTIMIZATIONS", bool_to_on_off(enable_native));
    cfg.define("SPATIAL_DRACO_STATIC_STDLIB", bool_to_on_off(static_stdlib));
    cfg.define("SPATIAL_DRACO_MSVC_STATIC_RUNTIME", bool_to_on_off(msvc_static_runtime));
    cfg.define("SPATIAL_DRACO_WERROR", bool_to_on_off(werror));

    // Avoid position-independent code surprises when linking into shared objects.
    cfg.define("CMAKE_POSITION_INDEPENDENT_CODE", "ON");

    // Respect Cargo's build profile.
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    cfg.profile(if profile.eq_ignore_ascii_case("release") { "Release" } else { "Debug" });

    let dst = cfg.build_target("draco_wrapper_cpp_static").build();
    Ok(strip_windows_verbatim_prefix(&dst))
}

fn emit_link(dst: &Path, target: &str) -> Result<(), String> {
    let build_dir = dst.join("build");
    if !build_dir.is_dir() {
        return Err(format!(
            "Expected CMake build directory at {}, but it does not exist.",
            build_dir.display()
        ));
    }

    // On MSVC generators, libs can land in config subdirs (Release/Debug).
    // Emit a few safe candidates; rustc ignores non-existing ones.
    let mut search_dirs: Vec<PathBuf> = Vec::with_capacity(16);
    search_dirs.push(build_dir.clone());
    search_dirs.push(build_dir.join("Release"));
    search_dirs.push(build_dir.join("Debug"));
    search_dirs.push(build_dir.join("RelWithDebInfo"));
    search_dirs.push(build_dir.join("MinSizeRel"));

    let draco_dir = build_dir.join("draco");
    search_dirs.push(draco_dir.clone());
    search_dirs.push(draco_dir.join("Release"));
    search_dirs.push(draco_dir.join("Debug"));
    search_dirs.push(draco_dir.join("RelWithDebInfo"));
    search_dirs.push(draco_dir.join("MinSizeRel"));

    // De-dup while preserving order.
    search_dirs.dedup();

    for dir in search_dirs {
        if dir.is_dir() {
            println!("cargo:rustc-link-search=native={}", dir.display());
        }
    }

    println!("cargo:rustc-link-lib=static=draco_wrapper_cpp_static");
    println!("cargo:rustc-link-lib=static=draco");

    if target.contains("apple-darwin") {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else if target.contains("linux") {
        // Detect which linux gnu version is installed 
        println!("cargo:rustc-link-search=native=/usr/lib/gcc/x86_64-linux-gnu/11");
        println!("cargo:rustc-link-arg=-Wl,-l:libstdc++.so.6");

        println!("cargo:rustc-link-lib=dylib=stdc++");
        //println!("cargo:rustc-link-lib=dylib=:libstdc++.so.6");
    } else if target.contains("windows") && target.contains("gnu") {
        println!("cargo:rustc-link-lib=stdc++");
    }

    if target.contains("windows") && target.contains("gnu") && env_bool("DRACO_STATIC_STDLIB") {
        println!("cargo:rustc-link-arg=-static");
        println!("cargo:rustc-link-arg=-static-libgcc");
        println!("cargo:rustc-link-arg=-static-libstdc++");
    }

    Ok(())
}

fn generate_c_header(manifest_dir: &Path) -> Result<(), String> {
    // Cargo sets CARGO_FEATURE_<FEATURE> for enabled features.
    if env::var_os("CARGO_FEATURE_FFI").is_none() {
        return Ok(());
    }

    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rerun-if-changed=src/ffi.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");

    let bindings_dir = manifest_dir.join("bindings");
    fs::create_dir_all(&bindings_dir)
        .map_err(|e| format!("Failed to create bindings dir {}: {e}", bindings_dir.display()))?;

    let out_path = bindings_dir.join("spatial_codec_draco.h");

    let config_path = manifest_dir.join("cbindgen.toml");
    let cfg = cbindgen::Config::from_file(config_path)
        .map_err(|e| format!("Failed to load cbindgen.toml: {e}"))?;

    let generated = cbindgen::generate_with_config(manifest_dir, cfg)
        .map_err(|e| format!("cbindgen failed: {e}"))?;

    generated.write_to_file(&out_path);
    Ok(())
}

fn main() {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR missing"));
    let target = env::var("TARGET").expect("TARGET missing");
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

    let out_dir = match resolve_out_dir(&manifest_dir, &target, &profile) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    let dst = match build_cpp(&manifest_dir, &out_dir, &target) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = emit_link(&dst, &target) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }

    if let Err(e) = generate_c_header(&manifest_dir) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}