use std::env;
use std::path::PathBuf;

use firedancer_rs_common::{_pipeline_finalize, fd_log_stub_path, TargetInfo};

fn main() {
    let target_info = TargetInfo::new();

    let (vendor_path, ballet_path) =
        find_vendor().expect("Failed to find vendor directory with submodules");

    let zstd_path = ballet_path.join("zstd");
    let util_path = vendor_path.join("util");
    let log_path = util_path.join("log");

    let zstd_prefix = resolve_zstd_prefix(&target_info);

    let stub_c_path = fd_log_stub_path();

    setup_rerun(&zstd_path, &util_path, &log_path, &stub_c_path);

    let wrapper_path = generate_header(&zstd_path);
    let mut bindgen = init_bindgen(
        &wrapper_path,
        &ballet_path,
        &util_path,
        &vendor_path,
        &zstd_prefix,
    );
    let mut build = init_cc(
        &zstd_path,
        &stub_c_path,
        &ballet_path,
        &util_path,
        &vendor_path,
        &zstd_prefix,
    );

    spec_target(&target_info, &mut bindgen, &mut build);

    emit_link_directives(&target_info, &zstd_prefix);

    _pipeline_finalize(build, bindgen, "fdzstd", None);
}

fn resolve_zstd_prefix(target_info: &TargetInfo) -> String {
    env::var("ZSTD_PREFIX").unwrap_or_else(|_| {
        if target_info.is_macos() {
            "/opt/homebrew/opt/zstd".to_string()
        } else {
            "/usr".to_string()
        }
    })
}

fn setup_rerun(zstd_path: &PathBuf, util_path: &PathBuf, log_path: &PathBuf, stub_c_path: &PathBuf) {
    println!(
        "cargo:rerun-if-changed={}",
        stub_c_path.display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        zstd_path.join("fd_zstd.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        zstd_path.join("fd_zstd_private.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        zstd_path.join("fd_zstd.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("fd_util_base.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        log_path.join("fd_log.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        log_path.join("fd_log.c").display()
    );
}

fn generate_header(zstd_path: &PathBuf) -> PathBuf {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("zstd_wrapper.h");

    let header_content = format!(
        r#"
#include "{}/fd_zstd.h"
"#,
        zstd_path.canonicalize().unwrap().display(),
    );

    std::fs::write(&wrapper_path, header_content).expect("Failed to write wrapper header");

    wrapper_path
}

fn init_bindgen(
    wrapper_path: &PathBuf,
    ballet_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
    zstd_prefix: &str,
) -> bindgen::Builder {
    bindgen::Builder::default()
        .wrap_static_fns(true)
        .wrap_static_fns_path(wrapper_path)
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", ballet_path.display()))
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg(format!("-I{}", vendor_path.display()))
        .clang_arg(format!("-I{}/include", zstd_prefix))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_HAS_ZSTD=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-D_GNU_SOURCE")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .use_core()
        .ctypes_prefix("libc")
        .allowlist_function("fd_zstd_.*")
        .allowlist_type("fd_zstd_.*")
        .allowlist_var("FD_ZSTD_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
}

fn init_cc(
    zstd_path: &PathBuf,
    stub_c_path: &PathBuf,
    ballet_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
    zstd_prefix: &str,
) -> cc::Build {
    let mut build = cc::Build::new();
    build
        .file(zstd_path.join("fd_zstd.c"))
        .file(stub_c_path)
        .include(ballet_path)
        .include(util_path)
        .include(vendor_path)
        .include(format!("{}/include", zstd_prefix))
        .define("FD_HAS_HOSTED", "1")
        .define("FD_HAS_ZSTD", "1")
        .define("FD_LOG_STYLE", "0")
        .define("_GNU_SOURCE", "1")
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC")
        .flag("-Wno-error=implicit-function-declaration")
        .flag(&format!("-L{}/lib", zstd_prefix));

    build
}

fn spec_target(
    target_info: &TargetInfo,
    bindgen: &mut bindgen::Builder,
    build: &mut cc::Build,
) {
    if target_info.is_x86_64() {
        if target_info.emulated {
            *bindgen = std::mem::take(bindgen)
                .clang_arg("-DFD_HAS_X86=0")
                .clang_arg("-DFD_HAS_SSE=0")
                .clang_arg("-DFD_HAS_AVX=0");

            build
                .define("FD_HAS_X86", "0")
                .define("FD_HAS_SSE", "0")
                .define("FD_HAS_AVX", "0");
        } else {
            *bindgen = std::mem::take(bindgen)
                .clang_arg("-DFD_HAS_X86=1")
                .clang_arg("-DFD_HAS_SSE=1")
                .clang_arg("-DFD_HAS_AVX=1");

            build
                .define("FD_HAS_X86", "1")
                .define("FD_HAS_SSE", "1")
                .define("FD_HAS_AVX", "1");
        }
    } else if target_info.is_aarch64() {
        *bindgen = std::mem::take(bindgen).clang_arg("-DFD_HAS_ARM=1");
        build.define("FD_HAS_ARM", "1");
    }

    if target_info.is_macos() {
        *bindgen = std::mem::take(bindgen).clang_arg("-DSIGPOLL=SIGIO");
        build.define("SIGPOLL", "SIGIO");
    }
}

fn emit_link_directives(target_info: &TargetInfo, zstd_prefix: &str) {
    println!("cargo:rustc-link-search=native={}/lib", zstd_prefix);

    if target_info.is_linux() {
        println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu");
        println!("cargo:rustc-link-search=native=/usr/lib");
        println!("cargo:rustc-link-search=native=/lib/x86_64-linux-gnu");
    }

    println!("cargo:rustc-link-lib=dylib=zstd");
}

fn find_vendor() -> Result<(PathBuf, PathBuf), String> {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR")
            .map_err(|e| format!("Failed to get CARGO_MANIFEST_DIR: {}", e))?,
    );

    let mut current = manifest_dir.as_path();

    loop {
        let vendor_path = current.join("vendor");
        let ballet_dir = vendor_path.join("ballet");
        if ballet_dir.exists() {
            eprintln!("Found ballet at: {}", ballet_dir.display());
            return Ok((vendor_path, ballet_dir));
        }

        let src_ballet = vendor_path.join("src").join("ballet");
        if src_ballet.exists() {
            eprintln!("Found ballet at: {}", src_ballet.display());
            return Ok((vendor_path, src_ballet));
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    Err(format!(
        "Failed to find vendor directory with ballet subdirectory. Started search from: {}",
        manifest_dir.display()
    ))
}
