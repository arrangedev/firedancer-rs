use std::env;
use std::path::PathBuf;

fn main() {
    let firedancer_path = PathBuf::from("../../../../vendor");
    let util_path = firedancer_path.join("util");
    let tpool_path = util_path.join("tpool");

    println!(
        "cargo:rerun-if-changed={}",
        tpool_path.join("fd_tpool.cxx").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tpool_path.join("fd_tpool.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tpool_path.join("fd_map_reduce.h").display()
    );

    let target = env::var("TARGET").unwrap();
    let is_x86_64 = target.contains("x86_64");
    let is_aarch64 = target.contains("aarch64") || target.contains("arm");
    let is_macos = target.contains("apple");

    let mut build = cc::Build::new();
    build
        .cpp(true) // C++ file
        .file(tpool_path.join("fd_tpool.cxx"))
        .include(&util_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_HAS_THREADS", "1") // requires threading
        .define("FD_HAS_ALLOCA", "1")  // uses alloca
        .flag("-std=c++17")
        .flag("-O3")
        .flag("-fPIC");

    if is_x86_64 {
        build
            .define("FD_HAS_X86", "1")
            .define("FD_HAS_SSE", "1")
            .define("FD_HAS_AVX", "1");
    } else if is_aarch64 {
        build.define("FD_HAS_ARM", "1");
    }

    if is_macos {
        build.define("SIGPOLL", "SIGIO");
    }

    println!("cargo:rustc-link-lib=pthread");

    build.compile("fdtpool");

    let mut bindgen = bindgen::Builder::default()
        .header(tpool_path.join("fd_tpool.h").to_string_lossy())
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_HAS_THREADS=1")
        .clang_arg("-DFD_HAS_ALLOCA=1")
        .clang_arg("-DFD_TILE_MAX=2048")
        .clang_arg("-std=c17") // C17 > C++17 (better compat)
        .allowlist_function("fd_tpool_.*")
        .allowlist_type("fd_tpool_.*")
        .allowlist_var("FD_TPOOL_.*")
        .blocklist_type("std::.*") // no std types
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

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
