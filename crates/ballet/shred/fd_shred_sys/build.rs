use std::env;
use std::path::PathBuf;

fn main() {
    let firedancer_path = PathBuf::from("../../../../vendor");
    let ballet_path = firedancer_path.join("ballet");
    let shred_path = ballet_path.join("shred");
    let bmtree_path = ballet_path.join("bmtree");
    let sha256_path = ballet_path.join("sha256");
    let util_path = firedancer_path.join("util");

    // Add rerun-if-changed for all relevant files
    println!(
        "cargo:rerun-if-changed={}",
        shred_path.join("fd_shred.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        shred_path.join("fd_shred.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        shred_path.join("fd_deshredder.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        shred_path.join("fd_deshredder.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        bmtree_path.join("fd_bmtree.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        bmtree_path.join("fd_bmtree.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        sha256_path.join("fd_sha256.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        sha256_path.join("fd_sha256.c").display()
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
    let wrapper_path = out_path.join("shred_wrapper.h");

    std::fs::write(
        &wrapper_path,
        format!(
            r#"
#include "{}/fd_shred.h"
#include "{}/fd_deshredder.h"
#include "{}/fd_bmtree.h"
"#,
            shred_path.canonicalize().unwrap().display(),
            shred_path.canonicalize().unwrap().display(),
            bmtree_path.canonicalize().unwrap().display(),
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
            .clang_arg("-DFD_HAS_AVX=1")
            .clang_arg("-msse")
            .clang_arg("-msse2")
            .clang_arg("-mavx")
            .clang_arg("-mavx2");
    } else if is_aarch64 {
        bindgen = bindgen.clang_arg("-DFD_HAS_ARM=1");
    }

    if is_macos {
        bindgen = bindgen.clang_arg("-DSIGPOLL=SIGIO");
    }

    bindgen = bindgen
        .allowlist_function("fd_shred_.*")
        .allowlist_function("fd_deshredder_.*")
        .allowlist_function("fd_bmtree_.*")
        .allowlist_type("fd_shred.*")
        .allowlist_type("fd_deshredder.*")
        .allowlist_type("fd_bmtree.*")
        .allowlist_var("FD_SHRED_.*")
        .allowlist_var("FD_BMTREE_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    let bindings = bindgen.generate().expect("Unable to generate bindings");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    let wrapper_c_path = out_path.join("shred_wrapper.c");

    let mut build = cc::Build::new();
    build
        .file(shred_path.join("fd_shred.c"))
        .file(shred_path.join("fd_deshredder.c"))
        .file(bmtree_path.join("fd_bmtree.c"))
        .file(sha256_path.join("fd_sha256.c"))
        .file(&wrapper_c_path)
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
            .define("FD_HAS_AVX", "1")
            .flag("-msse")
            .flag("-msse2")
            .flag("-mavx")
            .flag("-mavx2");
    } else if is_aarch64 {
        build.define("FD_HAS_ARM", "1");
    }

    if is_macos {
        build.define("SIGPOLL", "SIGIO");
    }

    build.compile("fdshred");
}
