use std::env;
use std::path::PathBuf;

use firedancer_rs_common::{
    is_emulated, rerun_if_changed, TargetSpec, VendorPaths, _pipeline_finalize,
};

fn main() {
    let target_info = TargetSpec::new();
    let paths = VendorPaths::new();

    setup_rerun(&paths);

    let wrapper_path = _gen_header(&paths);
    let mut bindgen = _init_bindgen(&wrapper_path, &paths);
    let mut build = _init_cc(&paths);

    target_spec(&target_info, &mut bindgen, &mut build, &paths);

    _pipeline_finalize(bindgen, build, "fded25519", None);
}

fn setup_rerun(paths: &VendorPaths) {
    let ed25519_path = paths.ballet_module("ed25519");
    let sha512_path = paths.ballet_module("sha512");

    rerun_if_changed![
        ed25519_path.join("fd_ed25519.h"),
        ed25519_path.join("fd_ed25519_user.c"),
        ed25519_path.join("fd_curve25519.c"),
        ed25519_path.join("fd_curve25519_secure.c"),
        ed25519_path.join("fd_curve25519_scalar.c"),
        ed25519_path.join("fd_curve25519_tables.c"),
        ed25519_path.join("fd_f25519.c"),
        ed25519_path.join("fd_x25519.c"),
        ed25519_path.join("fd_ristretto255.c"),
        sha512_path.join("fd_sha512.h"),
        sha512_path.join("fd_sha512.c"),
        paths.util_module("fd_util_base.h"),
    ];
}

fn _gen_header(paths: &VendorPaths) -> PathBuf {
    let ed25519_path = paths.ballet_module("ed25519");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("ed25519_wrapper.h");

    std::fs::write(
        &wrapper_path,
        format!(
            r#"
#include "{}/fd_ed25519.h"
#include "{}/fd_f25519.h"
#include "{}/fd_curve25519.h"
#include "{}/fd_ristretto255.h"
#include "{}/fd_x25519.h"
"#,
            ed25519_path.canonicalize().unwrap().display(),
            ed25519_path.canonicalize().unwrap().display(),
            ed25519_path.canonicalize().unwrap().display(),
            ed25519_path.canonicalize().unwrap().display(),
            ed25519_path.canonicalize().unwrap().display(),
        ),
    )
    .expect("Failed to write wrapper header");

    wrapper_path
}

fn _init_bindgen(wrapper_path: &PathBuf, paths: &VendorPaths) -> bindgen::Builder {
    bindgen::Builder::default()
        .wrap_static_fns(true)
        .wrap_static_fns_path(wrapper_path)
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", paths.ballet_module("ballet").display()))
        .clang_arg(format!("-I{}", paths.util_module("util").display()))
        .clang_arg(format!("-I{}", paths.vendor.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .allowlist_function("fd_ed25519_.*")
        .allowlist_function("fd_x25519_.*")
        .allowlist_function("fd_ristretto255_.*")
        .allowlist_function("fd_sha512_.*")
        .allowlist_type("fd_ed25519_.*")
        .allowlist_type("fd_x25519_.*")
        .allowlist_type("fd_ristretto255_.*")
        .allowlist_type("fd_sha512_.*")
        .allowlist_var("FD_ED25519_.*")
        .allowlist_var("FD_X25519_.*")
        .allowlist_var("FD_RISTRETTO255_.*")
        .allowlist_var("FD_SHA512_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
}

fn _init_cc(paths: &VendorPaths) -> cc::Build {
    let ed25519_path = paths.ballet_module("ed25519");
    let sha512_path = paths.ballet_module("sha512");

    let mut build = cc::Build::new();
    build
        .file(ed25519_path.join("fd_ed25519_user.c"))
        .file(ed25519_path.join("fd_curve25519.c"))
        .file(ed25519_path.join("fd_curve25519_secure.c"))
        .file(ed25519_path.join("fd_curve25519_scalar.c"))
        .file(ed25519_path.join("fd_curve25519_tables.c"))
        .file(ed25519_path.join("fd_f25519.c"))
        .file(ed25519_path.join("fd_x25519.c"))
        .file(ed25519_path.join("fd_ristretto255.c"))
        .file(sha512_path.join("fd_sha512.c"))
        .include(&paths.ballet_module("ballet"))
        .include(&paths.util_module("util"))
        .include(&paths.vendor)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_LOG_STYLE", "0")
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC")
        .flag("-Wno-error=implicit-function-declaration");

    build
}

fn target_spec(
    target_info: &TargetSpec,
    bindgen: &mut bindgen::Builder,
    build: &mut cc::Build,
    paths: &VendorPaths,
) {
    println!("cargo:warning=Target architecture: {:?}", target_info.arch);
    println!("cargo:warning=Target platform: {:?}", target_info.os_target);

    if target_info.is_x86_64() {
        println!("cargo:warning=Configuring for x86_64");
        cfg_x86_64(target_info, bindgen, build, paths);
    } else if target_info.is_aarch64() {
        println!("cargo:warning=Configuring for AArch64/ARM");
        cfg_aarch64(bindgen, build);
    } else {
        println!("cargo:warning=Configuring for unknown/other architecture");
        _cfg_unknown(build, paths);
    }

    if target_info.is_macos() {
        _cfg_macos(bindgen, build);
    }
}

fn cfg_x86_64(
    _target_info: &TargetSpec,
    bindgen: &mut bindgen::Builder,
    build: &mut cc::Build,
    paths: &VendorPaths,
) {
    let emulated = is_emulated();
    println!("cargo:warning=x86_64 build - emulated: {}", emulated);

    if emulated {
        println!("cargo:warning=Using emulated x86_64 build (SIMD disabled)");
        cfg_x86_64_emu(bindgen, build);
    } else {
        println!("cargo:warning=Using native x86_64 build (SIMD enabled)");
        cfg_x86_64_native(bindgen, build, paths);
    }
}

fn cfg_x86_64_emu(bindgen: &mut bindgen::Builder, build: &mut cc::Build) {
    println!("cargo:warning=Configuring emulated x86_64 (all SIMD disabled)");

    *bindgen = std::mem::take(bindgen)
        .clang_arg("-DFD_HAS_X86=0")
        .clang_arg("-DFD_HAS_SSE=0")
        .clang_arg("-DFD_HAS_AVX=0")
        .clang_arg("-DFD_HAS_AVX512=0");

    build
        .define("FD_HAS_X86", "0")
        .define("FD_HAS_SSE", "0")
        .define("FD_HAS_AVX", "0")
        .define("FD_HAS_AVX512", "0");
}

fn cfg_x86_64_native(bindgen: &mut bindgen::Builder, build: &mut cc::Build, paths: &VendorPaths) {
    println!("cargo:warning=Configuring native x86_64 with AVX512 always enabled");

    // Enable x86_64 features including AVX512 (always enabled for header compatibility)
    *bindgen = std::mem::take(bindgen)
        .clang_arg("-DFD_HAS_X86=1")
        .clang_arg("-DFD_HAS_SSE=1")
        .clang_arg("-DFD_HAS_AVX=1")
        .clang_arg("-DFD_HAS_AVX512=1")
        .clang_arg("-msse")
        .clang_arg("-msse2")
        .clang_arg("-mavx")
        .clang_arg("-mavx2")
        .clang_arg("-mavx512f")
        .clang_arg("-mavx512bw")
        .clang_arg("-mavx512dq")
        .clang_arg("-mavx512vl")
        .clang_arg("-mavx512ifma")
        .clang_arg("-mavx512vbmi");

    println!("cargo:warning=Bindgen configured with FD_HAS_AVX512=1");

    build
        .define("FD_HAS_X86", "1")
        .define("FD_HAS_SSE", "1")
        .define("FD_HAS_AVX", "1")
        .define("FD_HAS_AVX512", "1")
        .flag("-msse")
        .flag("-msse2")
        .flag("-mavx")
        .flag("-mavx2")
        .flag("-mavx512f")
        .flag("-mavx512bw")
        .flag("-mavx512dq")
        .flag("-mavx512vl")
        .flag("-mavx512ifma")
        .flag("-mavx512vbmi")
        .file(paths.ballet_module("sha512").join("fd_sha512_core_avx2.S"));

    println!("cargo:warning=CC Build configured with FD_HAS_AVX512=1");

    _link_avx512_maybe(build, paths);
}

fn _link_avx512_maybe(build: &mut cc::Build, paths: &VendorPaths) {
    let avx512_path = paths.ballet_module("ed25519").join("avx512");

    if avx512_path.exists() {
        let required_avx512_files = [
            "fd_curve25519.c",
            "fd_curve25519_secure.c",
            "fd_f25519.c",
            "fd_r43x6.c",
            "fd_r43x6_ge.c",
        ];

        let all_files_exist = required_avx512_files
            .iter()
            .all(|file| avx512_path.join(file).exists());

        if all_files_exist {
            println!("cargo:warning=Adding AVX512 source files");
            for file in &required_avx512_files {
                build.file(avx512_path.join(file));
            }
        } else {
            println!("cargo:warning=Some AVX512 files missing, using reference implementations");
        }
    }
}

fn cfg_aarch64(bindgen: &mut bindgen::Builder, build: &mut cc::Build) {
    *bindgen = std::mem::take(bindgen).clang_arg("-DFD_HAS_ARM=1");
    build.define("FD_HAS_ARM", "1");
}

fn _cfg_unknown(build: &mut cc::Build, paths: &VendorPaths) {
    let ref_path = paths.ballet_module("ed25519").join("ref");
    if ref_path.exists() {
        build
            .file(ref_path.join("fd_curve25519.c"))
            .file(ref_path.join("fd_curve25519_secure.c"))
            .file(ref_path.join("fd_f25519.c"));
    }
}

fn _cfg_macos(bindgen: &mut bindgen::Builder, build: &mut cc::Build) {
    *bindgen = std::mem::take(bindgen).clang_arg("-DSIGPOLL=SIGIO");
    build.define("SIGPOLL", "SIGIO");
}
