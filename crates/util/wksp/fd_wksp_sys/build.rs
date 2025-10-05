use std::env;
use std::path::PathBuf;

use firedancer_rs_common::{TargetInfo, _pipeline_finalize};

fn main() {
    let target_info = TargetInfo::new();

    let (vendor_path, util_path) =
        find_vendor().expect("Failed to find vendor directory with submodules");

    let wksp_path = util_path.join("wksp");
    let valloc_path = util_path.join("valloc");
    let bits_path = util_path.join("bits");
    let log_path = util_path.join("log");
    let checkpt_path = util_path.join("checkpt");
    let tpool_path = util_path.join("tpool");

    setup_rerun(
        &wksp_path,
        &valloc_path,
        &bits_path,
        &log_path,
        &checkpt_path,
        &tpool_path,
        &util_path,
    );

    let wrapper_path = generate_header(&wksp_path);
    let mut bindgen = init_bindgen(&wrapper_path, &util_path, &vendor_path);
    let mut build = init_cc(
        &wksp_path,
        &valloc_path,
        &bits_path,
        &log_path,
        &checkpt_path,
        &tpool_path,
        &util_path,
        &vendor_path,
    );

    spec_target(&target_info, &mut bindgen, &mut build);

    _pipeline_finalize(build, bindgen, "fdwksp", None);
}

fn setup_rerun(
    wksp_path: &PathBuf,
    valloc_path: &PathBuf,
    bits_path: &PathBuf,
    log_path: &PathBuf,
    checkpt_path: &PathBuf,
    tpool_path: &PathBuf,
    util_path: &PathBuf,
) {
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp_admin.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp_user.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp_helper.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp_io.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp_free_treap.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp_used_treap.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp_checkpt_v1.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp_checkpt_v2.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp_restore_v1.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp_restore_v2.c").display()
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
        checkpt_path.join("fd_checkpt.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        checkpt_path.join("fd_checkpt.h").display()
    );
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
        util_path.join("fd_util_base.h").display()
    );
}

fn generate_header(wksp_path: &PathBuf) -> PathBuf {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("wksp_wrapper.h");

    let header_content = format!(
        r#"
#include "{}/fd_wksp.h"
"#,
        wksp_path.canonicalize().unwrap().display()
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
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-D_GNU_SOURCE")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .wrap_unsafe_ops(true)
        .allowlist_function("fd_wksp_.*")
        .allowlist_type("fd_wksp_.*")
        .allowlist_type("fd_valloc_.*")
        .allowlist_type("fd_tpool_.*")
        .allowlist_type("fd_checkpt_.*")
        .allowlist_var("FD_WKSP_.*")
        .allowlist_var("fd_wksp_.*")
        .allowlist_var("FD_.*")
        .allowlist_recursively(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
}

fn init_cc(
    wksp_path: &PathBuf,
    valloc_path: &PathBuf,
    bits_path: &PathBuf,
    log_path: &PathBuf,
    checkpt_path: &PathBuf,
    tpool_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
) -> cc::Build {
    let mut build = cc::Build::new();

    // Add C files
    build
        .file(wksp_path.join("fd_wksp_admin.c"))
        .file(wksp_path.join("fd_wksp_user.c"))
        .file(wksp_path.join("fd_wksp_helper.c"))
        .file(wksp_path.join("fd_wksp_io.c"))
        .file(wksp_path.join("fd_wksp_free_treap.c"))
        .file(wksp_path.join("fd_wksp_used_treap.c"))
        .file(wksp_path.join("fd_wksp_checkpt_v1.c"))
        .file(wksp_path.join("fd_wksp_checkpt_v2.c"))
        .file(wksp_path.join("fd_wksp_restore_v1.c"))
        .file(wksp_path.join("fd_wksp_restore_v2.c"))
        .file(valloc_path.join("fd_valloc.c"))
        .file(bits_path.join("fd_bits.c"))
        .file(log_path.join("fd_log.c"))
        .file(checkpt_path.join("fd_checkpt.c"))
        .include(util_path)
        .include(vendor_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_LOG_STYLE", "0")
        .define("_GNU_SOURCE", "1")
        .define("FD_HAS_ATOMIC", "1")
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC")
        .flag("-Wno-error=implicit-function-declaration");

    // Build tpool separately as C++
    let mut tpool_build = cc::Build::new();
    tpool_build
        .cpp(true)
        .file(tpool_path.join("fd_tpool.cxx"))
        .include(util_path)
        .include(vendor_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_HAS_THREADS", "1")
        .define("FD_HAS_ALLOCA", "1")
        .define("FD_LOG_STYLE", "0")
        .define("_GNU_SOURCE", "1")
        .flag("-std=c++17")
        .flag("-O3")
        .flag("-fPIC")
        .flag("-Wno-error=implicit-function-declaration");

    // Link pthread for tpool
    println!("cargo:rustc-link-lib=pthread");

    // Compile tpool separately
    tpool_build.compile("fdtpool");

    // Handle wrapper.c if it exists
    let wrapper_c_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("wrapper.c");
    if wrapper_c_path.exists() {
        let mut wrapper_build = cc::Build::new();
        wrapper_build
            .file(&wrapper_c_path)
            .include(util_path)
            .include(vendor_path)
            .define("FD_HAS_HOSTED", "1")
            .define("FD_LOG_STYLE", "0")
            .define("_GNU_SOURCE", "1")
            .flag("-std=c17")
            .flag("-O3")
            .flag("-fPIC")
            .flag("-Wno-error=implicit-function-declaration");

        wrapper_build.compile("fdwkspwrapper");
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
    *bindgen = std::mem::take(bindgen).clang_arg("-DSIGPOLL=SIGIO");
    build.define("SIGPOLL", "SIGIO");
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
