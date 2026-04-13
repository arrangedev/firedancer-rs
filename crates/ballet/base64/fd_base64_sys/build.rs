use std::env;
use std::path::PathBuf;

use firedancer_rs_common::{_pipeline_finalize, TargetInfo};

fn main() {
    let target_info = TargetInfo::new();

    let (vendor_path, ballet_path) =
        find_vendor().expect("Failed to find vendor directory with submodules");

    let base64_path = ballet_path.join("base64");

    setup_rerun(&base64_path);

    let bindgen = init_bindgen(&base64_path, &ballet_path, &vendor_path);
    let mut build = init_cc(&base64_path, &ballet_path, &vendor_path);

    spec_target(&target_info, &mut build);

    _pipeline_finalize(build, bindgen, "fdbase64", None);
}

fn setup_rerun(base64_path: &PathBuf) {
    println!(
        "cargo:rerun-if-changed={}",
        base64_path.join("fd_base64.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        base64_path.join("fd_base64.h").display()
    );
}

fn init_bindgen(
    base64_path: &PathBuf,
    ballet_path: &PathBuf,
    vendor_path: &PathBuf,
) -> bindgen::Builder {
    bindgen::Builder::default()
        .header(base64_path.join("fd_base64.h").to_string_lossy())
        .clang_arg(format!("-I{}", ballet_path.display()))
        .clang_arg(format!("-I{}", vendor_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-std=c17")
        .allowlist_function("fd_base64_.*")
        .allowlist_function("fd_cstr_append_base64")
        .allowlist_var("FD_BASE64_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
}

fn init_cc(base64_path: &PathBuf, ballet_path: &PathBuf, vendor_path: &PathBuf) -> cc::Build {
    let mut build = cc::Build::new();
    build
        .file(base64_path.join("fd_base64.c"))
        .include(ballet_path)
        .include(vendor_path)
        .define("FD_HAS_HOSTED", "1")
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC");

    build
}

fn spec_target(target_info: &TargetInfo, build: &mut cc::Build) {
    if target_info.is_x86_64() {
        build
            .define("FD_HAS_X86", "1")
            .define("FD_HAS_SSE", "1")
            .define("FD_HAS_AVX", "1");
    } else if target_info.is_aarch64() {
        build.define("FD_HAS_ARM", "1");
    }

    if target_info.is_macos() {
        build.define("SIGPOLL", "SIGIO");
    }
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
