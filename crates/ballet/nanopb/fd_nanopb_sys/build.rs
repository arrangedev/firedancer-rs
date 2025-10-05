use std::env;
use std::path::PathBuf;

use firedancer_rs_common::{TargetInfo, _pipeline_finalize};

fn main() {
    let target_info = TargetInfo::new();

    let (vendor_path, ballet_path) =
        find_vendor().expect("Failed to find vendor directory with submodules");

    let nanopb_path = ballet_path.join("nanopb");
    let util_path = vendor_path.join("util");

    setup_rerun(&nanopb_path);

    let wrapper_path = generate_header(&nanopb_path);
    let mut bindgen = init_bindgen(&wrapper_path, &ballet_path, &util_path, &vendor_path);
    let mut build = init_cc(&nanopb_path, &ballet_path, &util_path, &vendor_path);

    spec_target(&target_info, &mut bindgen, &mut build);

    _pipeline_finalize(build, bindgen, "fdnanopb", None);
}

fn setup_rerun(nanopb_path: &PathBuf) {
    println!(
        "cargo:rerun-if-changed={}",
        nanopb_path.join("pb_encode.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nanopb_path.join("pb_decode.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nanopb_path.join("pb_common.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nanopb_path.join("pb_firedancer.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nanopb_path.join("pb.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nanopb_path.join("pb_encode.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nanopb_path.join("pb_decode.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nanopb_path.join("pb_common.c").display()
    );
}

fn generate_header(nanopb_path: &PathBuf) -> PathBuf {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("nanopb_wrapper.h");

    let header_content = format!(
        r#"
#include "{}/pb_firedancer.h"
#include "{}/pb_encode.h"
#include "{}/pb_decode.h"
#include "{}/pb_common.h"
"#,
        nanopb_path.canonicalize().unwrap().display(),
        nanopb_path.canonicalize().unwrap().display(),
        nanopb_path.canonicalize().unwrap().display(),
        nanopb_path.canonicalize().unwrap().display(),
    );

    std::fs::write(&wrapper_path, header_content).expect("Failed to write wrapper header");

    wrapper_path
}

fn init_bindgen(
    wrapper_path: &PathBuf,
    ballet_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
) -> bindgen::Builder {
    bindgen::Builder::default()
        .wrap_static_fns(true)
        .wrap_static_fns_path(wrapper_path)
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", ballet_path.display()))
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg(format!("-I{}", vendor_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-DPB_FIELD_32BIT=1")
        .clang_arg("-DPB_ENABLE_MALLOC=1")
        .clang_arg("-DPB_BUFFER_ONLY=1")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .wrap_unsafe_ops(true)
        .allowlist_function("pb_.*")
        .allowlist_type("pb_.*")
        .allowlist_var("PB_.*")
        .allowlist_var("NANOPB_.*")
        .allowlist_var("FD_.*")
        .allowlist_recursively(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
}

fn init_cc(
    nanopb_path: &PathBuf,
    ballet_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
) -> cc::Build {
    let mut build = cc::Build::new();
    build
        .file(nanopb_path.join("pb_encode.c"))
        .file(nanopb_path.join("pb_decode.c"))
        .file(nanopb_path.join("pb_common.c"))
        .include(ballet_path)
        .include(util_path)
        .include(vendor_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_LOG_STYLE", "0")
        .define("PB_FIELD_32BIT", "1")
        .define("PB_ENABLE_MALLOC", "1")
        .define("PB_BUFFER_ONLY", "1")
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC")
        .flag("-Wno-error=implicit-function-declaration");

    build
}

fn spec_target(target_info: &TargetInfo, bindgen: &mut bindgen::Builder, build: &mut cc::Build) {
    if target_info.is_x86_64() {
        cfg_x86_64(bindgen, build);
    } else if target_info.is_aarch64() {
        cfg_aarch64(bindgen, build);
    }

    if target_info.is_macos() {
        cfg_arm64_mac(bindgen, build);
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

fn cfg_arm64_mac(bindgen: &mut bindgen::Builder, build: &mut cc::Build) {
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
