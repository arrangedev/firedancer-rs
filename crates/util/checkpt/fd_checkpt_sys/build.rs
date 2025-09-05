use std::env;
use std::path::PathBuf;

fn main() {
    let firedancer_path = PathBuf::from("../../../../vendor");
    let util_path = firedancer_path.join("util");
    let checkpt_path = util_path.join("checkpt");
    let log_path = util_path.join("log");
    let tile_path = util_path.join("tile");
    let io_path = util_path.join("io");

    println!(
        "cargo:rerun-if-changed={}",
        checkpt_path.join("fd_checkpt.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        checkpt_path.join("fd_checkpt.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        checkpt_path.join("fd_restore.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        log_path.join("fd_log.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tile_path.join("fd_tile.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        io_path.join("fd_io.c").display()
    );

    let target = env::var("TARGET").unwrap();
    let is_x86_64 = target.contains("x86_64");
    let is_aarch64 = target.contains("aarch64") || target.contains("arm");
    let is_macos = target.contains("apple");

    let mut build = cc::Build::new();
    build
        .file(checkpt_path.join("fd_checkpt.c"))
        .file(checkpt_path.join("fd_restore.c"))
        .file(log_path.join("fd_log.c"))
        .file(tile_path.join("fd_tile.c"))
        .file(io_path.join("fd_io.c"))
        .include(&util_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_HAS_LZ4", "1")  // Enable LZ4 compression support
        .define("_GNU_SOURCE", "1")
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC");

    // Add system include paths for LZ4 on macOS
    if is_macos {
        build.include("/opt/homebrew/include");
        println!("cargo:rustc-link-search=/opt/homebrew/lib");
    }

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

    // Link against liblz4 for compression support
    println!("cargo:rustc-link-lib=lz4");

    build.compile("fdcheckpt");

    let mut bindgen = bindgen::Builder::default()
        .header(checkpt_path.join("fd_checkpt.h").to_string_lossy())
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_HAS_LZ4=1")
        .clang_arg("-std=c17")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    // Add system include paths for LZ4 on macOS for bindgen
    if is_macos {
        bindgen = bindgen.clang_arg("-I/opt/homebrew/include");
    }

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
