use std::env;
use std::path::PathBuf;

use firedancer_rs_common::{is_emulated, rerun_if_changed, FiredancerPaths, TargetInfo};

fn main() {
    let target_info = TargetInfo::new();

    let paths = FiredancerPaths::new();
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

    let mut bindgen = bindgen::Builder::default()
        .wrap_static_fns(true)
        .wrap_static_fns_path(&wrapper_path)
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", paths.ballet_module("ballet").display()))
        .clang_arg(format!("-I{}", paths.util_module("util").display()))
        .clang_arg(format!("-I{}", paths.vendor.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration");

    if target_info.is_x86_64() {
        if is_emulated() {
            bindgen = bindgen
                .clang_arg("-DFD_HAS_X86=0")
                .clang_arg("-DFD_HAS_SSE=0")
                .clang_arg("-DFD_HAS_AVX=0")
                .clang_arg("-DFD_HAS_AVX512=0");
        } else {
            bindgen = bindgen
                .clang_arg("-DFD_HAS_X86=1")
                .clang_arg("-DFD_HAS_SSE=1")
                .clang_arg("-DFD_HAS_AVX=1")
                .clang_arg("-msse")
                .clang_arg("-msse2")
                .clang_arg("-mavx")
                .clang_arg("-mavx2")
                .clang_arg("-DFD_HAS_AVX512=1")
                .clang_arg("-mavx512f")
                .clang_arg("-mavx512bw")
                .clang_arg("-mavx512dq")
                .clang_arg("-mavx512vl")
                .clang_arg("-mavx512ifma")
                .clang_arg("-mavx512vbmi");
        }
    } else if target_info.is_aarch64() {
        bindgen = bindgen.clang_arg("-DFD_HAS_ARM=1");
    }

    if target_info.is_macos() {
        bindgen = bindgen.clang_arg("-DSIGPOLL=SIGIO");
    }

    bindgen = bindgen
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
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    let bindings = bindgen.generate().expect("Unable to generate bindings");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

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

    if target_info.is_x86_64() {
        if is_emulated() {
            build
                .define("FD_HAS_X86", "0")
                .define("FD_HAS_SSE", "0")
                .define("FD_HAS_AVX", "0")
                .define("FD_HAS_AVX512", "0");
        } else {
            build
                .define("FD_HAS_X86", "1")
                .define("FD_HAS_SSE", "1")
                .define("FD_HAS_AVX", "1")
                .flag("-msse")
                .flag("-msse2")
                .flag("-mavx")
                .flag("-mavx2")
                .file(sha512_path.join("fd_sha512_core_avx2.S"));

            let avx512_path = ed25519_path.join("avx512");
            if avx512_path.exists() {
                build
                    .define("FD_HAS_AVX512", "1")
                    .flag("-mavx512f")
                    .flag("-mavx512bw")
                    .flag("-mavx512dq")
                    .flag("-mavx512vl")
                    .flag("-mavx512ifma")
                    .flag("-mavx512vbmi")
                    .file(avx512_path.join("fd_curve25519.c"))
                    .file(avx512_path.join("fd_curve25519_secure.c"))
                    .file(avx512_path.join("fd_f25519.c"))
                    .file(avx512_path.join("fd_r43x6.c"))
                    .file(avx512_path.join("fd_r43x6_ge.c"));
            }
        }
    } else if target_info.is_aarch64() {
        build.define("FD_HAS_ARM", "1");
    } else {
        let ref_path = ed25519_path.join("ref");
        if ref_path.exists() {
            build
                .file(ref_path.join("fd_curve25519.c"))
                .file(ref_path.join("fd_curve25519_secure.c"))
                .file(ref_path.join("fd_f25519.c"));
        }
    }

    if target_info.is_macos() {
        build.define("SIGPOLL", "SIGIO");
    }

    build.compile("fded25519");
}
