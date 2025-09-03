use std::env;
use std::path::PathBuf;

fn main() {
    let firedancer_path = PathBuf::from("../../../../vendor");
    let ballet_path = firedancer_path.join("ballet");
    let ed25519_path = ballet_path.join("ed25519");
    let sha512_path = ballet_path.join("sha512");
    let util_path = firedancer_path.join("util");

    // Add rerun-if-changed for all relevant files
    println!(
        "cargo:rerun-if-changed={}",
        ed25519_path.join("fd_ed25519.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ed25519_path.join("fd_ed25519_user.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ed25519_path.join("fd_curve25519.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ed25519_path.join("fd_curve25519_secure.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ed25519_path.join("fd_curve25519_scalar.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ed25519_path.join("fd_curve25519_tables.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ed25519_path.join("fd_f25519.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ed25519_path.join("fd_x25519.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ed25519_path.join("fd_ristretto255.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        sha512_path.join("fd_sha512.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        sha512_path.join("fd_sha512.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("fd_util_base.h").display()
    );

    let target = env::var("TARGET").unwrap();
    let is_x86_64 = target.contains("x86_64");
    let is_aarch64 = target.contains("aarch64") || target.contains("arm");
    let is_macos = target.contains("apple");

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
        .clang_arg(format!("-I{}", ballet_path.display()))
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg(format!("-I{}", firedancer_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration");

    // Add architecture-specific defines for bindgen
    if is_x86_64 {
        bindgen = bindgen
            .clang_arg("-DFD_HAS_X86=1")
            .clang_arg("-DFD_HAS_SSE=1")
            .clang_arg("-DFD_HAS_AVX=1");
    } else if is_aarch64 {
        bindgen = bindgen.clang_arg("-DFD_HAS_ARM=1");
    }

    if is_macos {
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

    if is_x86_64 {
        bindgen = bindgen
            .clang_arg("-DFD_HAS_X86=1")
            .clang_arg("-DFD_HAS_SSE=1")
            .clang_arg("-DFD_HAS_AVX=1");
    } else if is_aarch64 {
        bindgen = bindgen.clang_arg("-DFD_HAS_ARM=1");
    }

    if is_macos {
        bindgen = bindgen.clang_arg("-DSIGPOLL=SIGIO");
    }

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
        .include(&ballet_path)
        .include(&util_path)
        .include(&firedancer_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_LOG_STYLE", "0")
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC")
        .flag("-Wno-error=implicit-function-declaration");

    // Add architecture-specific optimizations
    if is_x86_64 {
        build
            .define("FD_HAS_X86", "1")
            .define("FD_HAS_SSE", "1")
            .define("FD_HAS_AVX", "1");

        // Add AVX512 implementations if available
        let avx512_path = ed25519_path.join("avx512");
        if avx512_path.exists() {
            build
                .file(avx512_path.join("fd_curve25519.c"))
                .file(avx512_path.join("fd_curve25519_secure.c"))
                .file(avx512_path.join("fd_f25519.c"))
                .file(avx512_path.join("fd_r43x6.c"))
                .file(avx512_path.join("fd_r43x6_ge.c"));
        }

        // Add table implementations
        let table_path = ed25519_path.join("table");
        if table_path.exists() {
            build
                .file(table_path.join("fd_curve25519_table_avx512.c"))
                .file(table_path.join("fd_f25519_table_avx512.c"));
        }
    } else if is_aarch64 {
        build.define("FD_HAS_ARM", "1");

        // Skip table implementations for now - they have dependency issues
        // let table_path = ed25519_path.join("table");
        // if table_path.exists() {
        //     build
        //         .file(table_path.join("fd_curve25519_table_ref.c"))
        //         .file(table_path.join("fd_f25519_table_ref.c"));
        // }
    } else {
        // Add reference implementations for other architectures
        let ref_path = ed25519_path.join("ref");
        if ref_path.exists() {
            build
                .file(ref_path.join("fd_curve25519.c"))
                .file(ref_path.join("fd_curve25519_secure.c"))
                .file(ref_path.join("fd_f25519.c"));
        }

        // Skip table implementations for now - they have dependency issues
        // let table_path = ed25519_path.join("table");
        // if table_path.exists() {
        //     build
        //         .file(table_path.join("fd_curve25519_table_ref.c"))
        //         .file(table_path.join("fd_f25519_table_ref.c"));
        // }
    }

    if is_macos {
        build.define("SIGPOLL", "SIGIO");
    }

    build.compile("fded25519");
}
