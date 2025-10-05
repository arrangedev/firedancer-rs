use std::env;
use std::path::PathBuf;

use firedancer_rs_common::{TargetInfo, _pipeline_finalize};

fn main() {
    let target_info = TargetInfo::new();

    let (vendor_path, util_path) =
        find_vendor().expect("Failed to find vendor directory with submodules");

    let checkpt_path = util_path.join("checkpt");
    let log_path = util_path.join("log");
    let tile_path = util_path.join("tile");
    let io_path = util_path.join("io");

    setup_rerun(&checkpt_path, &log_path, &tile_path, &io_path);

    let wrapper_path = generate_header(&checkpt_path);
    let mut bindgen = init_bindgen(&wrapper_path, &util_path, &vendor_path);
    let mut build = init_cc(
        &checkpt_path,
        &log_path,
        &tile_path,
        &io_path,
        &util_path,
        &vendor_path,
    );

    spec_target(&target_info, &mut bindgen, &mut build);

    _pipeline_finalize(build, bindgen, "fdcheckpt", None);
}

fn setup_rerun(checkpt_path: &PathBuf, log_path: &PathBuf, tile_path: &PathBuf, io_path: &PathBuf) {
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
        log_path.join("fd_log.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tile_path.join("fd_tile.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tile_path.join("fd_tile.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        io_path.join("fd_io.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        io_path.join("fd_io.h").display()
    );
}

fn generate_header(checkpt_path: &PathBuf) -> PathBuf {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("checkpt_wrapper.h");

    let header_content = format!(
        r#"
#include "{}/fd_checkpt.h"
"#,
        checkpt_path.canonicalize().unwrap().display()
    );

    std::fs::write(&wrapper_path, header_content).expect("Failed to write wrapper header");

    wrapper_path
}

fn init_bindgen(
    wrapper_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
) -> bindgen::Builder {
    let wrapper_c_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("wrapper.c");

    bindgen::Builder::default()
        .wrap_static_fns(true)
        .wrap_static_fns_path(&wrapper_c_path)
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg(format!("-I{}", vendor_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_HAS_LZ4=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-D_GNU_SOURCE")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .wrap_unsafe_ops(true)
        .allowlist_function("fd_checkpt_.*")
        .allowlist_function("fd_restore_.*")
        .allowlist_type("fd_checkpt_.*")
        .allowlist_type("fd_restore_.*")
        .allowlist_var("FD_CHECKPT_.*")
        .allowlist_var("FD_RESTORE_.*")
        .allowlist_var("FD_.*")
        .allowlist_recursively(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
}

fn init_cc(
    checkpt_path: &PathBuf,
    log_path: &PathBuf,
    tile_path: &PathBuf,
    io_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
) -> cc::Build {
    let mut build = cc::Build::new();

    build
        .file(checkpt_path.join("fd_checkpt.c"))
        .file(checkpt_path.join("fd_restore.c"))
        .file(log_path.join("fd_log.c"))
        .file(tile_path.join("fd_tile.c"))
        .file(io_path.join("fd_io.c"))
        .include(util_path)
        .include(vendor_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_HAS_LZ4", "1")
        .define("FD_LOG_STYLE", "0")
        .define("_GNU_SOURCE", "1")
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC")
        .flag("-Wno-error=implicit-function-declaration");

    // link liblz4
    println!("cargo:rustc-link-lib=lz4");

    let wrapper_c_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("wrapper.c");
    if wrapper_c_path.exists() {
        let mut wrapper_build = cc::Build::new();
        wrapper_build
            .file(&wrapper_c_path)
            .include(util_path)
            .include(vendor_path)
            .define("FD_HAS_HOSTED", "1")
            .define("FD_HAS_LZ4", "1")
            .define("FD_LOG_STYLE", "0")
            .define("_GNU_SOURCE", "1")
            .flag("-std=c17")
            .flag("-O3")
            .flag("-fPIC")
            .flag("-Wno-error=implicit-function-declaration");

        wrapper_build.compile("fdcheckptwrapper");
    }

    build
}

fn spec_target(target_info: &TargetInfo, bindgen: &mut bindgen::Builder, build: &mut cc::Build) {
    if target_info.is_x86_64() {
        cfg_x86_64(bindgen, build);
    } else if target_info.is_aarch64() {
        cfg_aarch64(bindgen, build);
    }

    if target_info.is_macos() {
        cfg_macos(bindgen, build);
    }
}

fn cfg_x86_64(bindgen: &mut bindgen::Builder, build: &mut cc::Build) {
    *bindgen = std::mem::take(bindgen)
        .clang_arg("-DFD_HAS_X86=1")
        .clang_arg("-DFD_HAS_SSE=1")
        .clang_arg("-DFD_HAS_AVX=1");

    build
        .define("FD_HAS_X86", "1")
        .define("FD_HAS_SSE", "1")
        .define("FD_HAS_AVX", "1");
}

fn cfg_aarch64(bindgen: &mut bindgen::Builder, build: &mut cc::Build) {
    *bindgen = std::mem::take(bindgen).clang_arg("-DFD_HAS_ARM=1");
    build.define("FD_HAS_ARM", "1");
}

fn cfg_macos(bindgen: &mut bindgen::Builder, build: &mut cc::Build) {
    *bindgen = std::mem::take(bindgen)
        .clang_arg("-DSIGPOLL=SIGIO")
        .clang_arg("-I/opt/homebrew/include");

    build
        .define("SIGPOLL", "SIGIO")
        .include("/opt/homebrew/include");

    println!("cargo:rustc-link-search=/opt/homebrew/lib");
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
