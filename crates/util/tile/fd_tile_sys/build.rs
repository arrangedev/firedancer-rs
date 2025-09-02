use std::env;
use std::path::PathBuf;

fn main() {
    let firedancer_path = PathBuf::from("../../../../vendor");
    let util_path = firedancer_path.join("util");
    let tile_path = util_path.join("tile");
    let valloc_path = util_path.join("valloc");
    let bits_path = util_path.join("bits");
    let log_path = util_path.join("log");
    let shmem_path = util_path.join("shmem");

    // Add rerun-if-changed for all relevant files
    println!(
        "cargo:rerun-if-changed={}",
        tile_path.join("fd_tile.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tile_path.join("fd_tile.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tile_path.join("fd_tile_threads.cxx").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tile_path.join("fd_tile_nothreads.cxx").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tile_path.join("fd_tile_private.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        shmem_path.join("fd_shmem.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        valloc_path.join("fd_valloc.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        valloc_path.join("fd_valloc.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        bits_path.join("fd_bits.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        bits_path.join("fd_bits.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        log_path.join("fd_log.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        log_path.join("fd_log.h").display()
    );

    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("fd_util_base.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("fd_util.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("fd_util.h").display()
    );

    let target = env::var("TARGET").unwrap();
    let is_x86_64 = target.contains("x86_64");
    let is_aarch64 = target.contains("aarch64") || target.contains("arm");
    let is_macos = target.contains("apple");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("tile_wrapper.h");
    let wrapper_c_path = out_path.join("wrapper.c");

    std::fs::write(
        &wrapper_path,
        format!(
            r#"
#include "{}/fd_tile.h"
"#,
            tile_path.canonicalize().unwrap().display()
        ),
    )
    .expect("Failed to write wrapper header");

    let mut bindgen = bindgen::Builder::default()
        .wrap_static_fns(true)
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg(format!("-I{}", firedancer_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-DFD_TILE_MAX=1024")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .wrap_static_fns_path(&wrapper_c_path)
        .allowlist_function("fd_tile_.*")
        .allowlist_type("fd_tile_.*")
        .allowlist_var("FD_TILE_.*")
        .allowlist_var("fd_tile_.*")
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

    // Build C files
    let mut build = cc::Build::new();
    build
        .file(tile_path.join("fd_tile.c"))
        .file(valloc_path.join("fd_valloc.c"))
        .file(bits_path.join("fd_bits.c"))
        .file(log_path.join("fd_log.c"))
        .file(util_path.join("fd_util.c"))
        .include(&util_path)
        .include(&firedancer_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_LOG_STYLE", "0")
        .define("FD_TILE_MAX", "1024")
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC")
        .flag("-Wno-error=implicit-function-declaration");

    // Build C++ files separately - choose implementation based on platform
    let mut tile_build = cc::Build::new();
    tile_build
        .cpp(true) // C++ file
        .include(&util_path)
        .include(&firedancer_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_LOG_STYLE", "0")
        .define("FD_TILE_MAX", "1024")
        .flag("-std=c++17")
        .flag("-O3")
        .flag("-fPIC")
        .flag("-Wno-error=implicit-function-declaration");

    // Choose the appropriate tile implementation based on platform
    if target.contains("linux") && !target.contains("android") {
        // Use full threading implementation on Linux
        tile_build
            .file(tile_path.join("fd_tile_threads.cxx"))
            .define("FD_HAS_THREADS", "1")
            .define("FD_HAS_ALLOCA", "1");
        println!("cargo:rustc-cfg=tile_threads");
    } else {
        // Use no-threads implementation on macOS and other platforms
        tile_build.file(tile_path.join("fd_tile_nothreads.cxx"));
        println!("cargo:rustc-cfg=tile_nothreads");
    }

    if is_x86_64 {
        build
            .define("FD_HAS_X86", "1")
            .define("FD_HAS_SSE", "1")
            .define("FD_HAS_AVX", "1");
        tile_build
            .define("FD_HAS_X86", "1")
            .define("FD_HAS_SSE", "1")
            .define("FD_HAS_AVX", "1");
    } else if is_aarch64 {
        build.define("FD_HAS_ARM", "1");
        tile_build.define("FD_HAS_ARM", "1");
    }

    if is_macos {
        build.define("SIGPOLL", "SIGIO");
        tile_build.define("SIGPOLL", "SIGIO");
    }

    // Link pthread for threading support
    println!("cargo:rustc-link-lib=pthread");

    build.compile("fdtile");
    tile_build.compile("fdtilecxx");

    if wrapper_c_path.exists() {
        let mut wrapper_build = cc::Build::new();
        wrapper_build
            .file(&wrapper_c_path)
            .include(&util_path)
            .include(&firedancer_path)
            .define("FD_HAS_HOSTED", "1")
            .define("FD_LOG_STYLE", "0")
            .define("FD_TILE_MAX", "1024")
            .flag("-std=c17")
            .flag("-O3")
            .flag("-fPIC")
            .flag("-Wno-error=implicit-function-declaration");

        if is_x86_64 {
            wrapper_build
                .define("FD_HAS_X86", "1")
                .define("FD_HAS_SSE", "1")
                .define("FD_HAS_AVX", "1");
        } else if is_aarch64 {
            wrapper_build.define("FD_HAS_ARM", "1");
        }

        if is_macos {
            wrapper_build.define("SIGPOLL", "SIGIO");
        }

        wrapper_build.compile("fdtilewrapper");
    }
}
