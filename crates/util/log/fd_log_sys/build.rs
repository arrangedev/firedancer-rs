use std::env;
use std::path::PathBuf;

fn main() {
    let (firedancer_path, util_path) =
        find_vendor().expect("Failed to find vendor directory with submodules");

    let log_path = util_path.join("log");
    let tile_path = util_path.join("tile");
    let io_path = util_path.join("io");
    let cstr_path = util_path.join("cstr");

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
        tile_path.join("fd_tile.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        io_path.join("fd_io.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        cstr_path.join("fd_cstr.c").display()
    );

    let target = env::var("TARGET").unwrap();
    let is_x86_64 = target.contains("x86_64");
    let is_aarch64 = target.contains("aarch64") || target.contains("arm");
    let is_macos = target.contains("apple");

    let mut build = cc::Build::new();
    build
        .file(log_path.join("fd_log.c"))
        .file(tile_path.join("fd_tile.c"))
        .file(io_path.join("fd_io.c"))
        .file(cstr_path.join("fd_cstr.c"))
        .include(&util_path)
        .define("FD_HAS_HOSTED", "1")
        .define("_GNU_SOURCE", "1")
        .flag("-std=c17")
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

    build.compile("fdlog");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("log_wrapper.h");

    std::fs::write(
        &wrapper_path,
        format!(
            r#"
#include "{}"
"#,
            log_path.join("fd_log.h").to_string_lossy()
        ),
    )
    .expect("Failed to write wrapper header");

    let mut bindgen = bindgen::Builder::default()
        .wrap_static_fns(true)
        .wrap_static_fns_path(&wrapper_path)
        //.header(log_path.join("fd_log.h").to_string_lossy())
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-D_GNU_SOURCE")
        .clang_arg("-std=c17")
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

fn find_vendor() -> Result<(PathBuf, PathBuf), String> {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR")
            .map_err(|e| format!("Failed to get CARGO_MANIFEST_DIR: {}", e))?,
    );

    let mut current = manifest_dir.as_path();

    loop {
        let vendor_path = current.join("vendor");
        let util_dir = vendor_path.join("util");
        if util_dir.exists() {
            eprintln!("Found util at: {}", util_dir.display());
            return Ok((vendor_path, util_dir));
        }

        let src_util = vendor_path.join("src").join("util");
        if src_util.exists() {
            eprintln!("Found util at: {}", src_util.display());
            return Ok((vendor_path, src_util));
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    Err(format!(
        "Failed to find vendor directory with util subdirectory. Started search from: {}",
        manifest_dir.display()
    ))
}
