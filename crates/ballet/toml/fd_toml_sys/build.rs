use std::env;
use std::path::PathBuf;

use firedancer_rs_common::{_pipeline_finalize, fd_log_stub_path, TargetInfo};

fn main() {
    let target_info = TargetInfo::new();

    let (vendor_path, ballet_path) =
        find_vendor().expect("Failed to find vendor directory with submodules");

    let toml_path = ballet_path.join("toml");
    let util_path = vendor_path.join("util");
    let pod_path = util_path.join("pod");
    let cstr_path = util_path.join("cstr");

    let stub_c_path = fd_log_stub_path();

    setup_rerun(&toml_path, &pod_path, &cstr_path, &util_path, &stub_c_path);

    let wrapper_path = generate_header(&toml_path);
    let mut bindgen = init_bindgen(&wrapper_path, &ballet_path, &util_path, &vendor_path);
    let mut build = init_cc(
        &toml_path,
        &pod_path,
        &cstr_path,
        &stub_c_path,
        &wrapper_path,
        &ballet_path,
        &util_path,
        &vendor_path,
    );

    spec_target(&target_info, &mut bindgen, &mut build);

    _pipeline_finalize(build, bindgen, "fdtoml", None);
}

fn setup_rerun(
    toml_path: &PathBuf,
    pod_path: &PathBuf,
    cstr_path: &PathBuf,
    util_path: &PathBuf,
    stub_c_path: &PathBuf,
) {
    println!("cargo:rerun-if-changed={}", stub_c_path.display());
    println!(
        "cargo:rerun-if-changed={}",
        toml_path.join("fd_toml.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        toml_path.join("fd_toml.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        pod_path.join("fd_pod.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        pod_path.join("fd_pod.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        cstr_path.join("fd_cstr.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        cstr_path.join("fd_cstr.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("fd_util_base.h").display()
    );
}

fn generate_header(toml_path: &PathBuf) -> PathBuf {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("toml_wrapper.h");

    let header_content = format!(
        r#"
#include "{}/fd_toml.h"
"#,
        toml_path.canonicalize().unwrap().display(),
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
        .clang_arg("-D_GNU_SOURCE")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .allowlist_function("fd_toml_.*")
        .allowlist_function("fd_pod_.*")
        .allowlist_type("fd_toml_.*")
        .allowlist_type("fd_pod_.*")
        .allowlist_var("FD_TOML_.*")
        .allowlist_var("FD_POD_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
}

fn init_cc(
    toml_path: &PathBuf,
    pod_path: &PathBuf,
    cstr_path: &PathBuf,
    stub_c_path: &PathBuf,
    wrapper_path: &PathBuf,
    ballet_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
) -> cc::Build {
    let mut build = cc::Build::new();
    build
        .file(toml_path.join("fd_toml.c"))
        .file(pod_path.join("fd_pod.c"))
        .file(cstr_path.join("fd_cstr.c"))
        .file(stub_c_path)
        .file(wrapper_path.with_extension("c"))
        .include(ballet_path)
        .include(util_path)
        .include(vendor_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_LOG_STYLE", "0")
        .define("_GNU_SOURCE", "1")
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC")
        .flag("-Wno-error=implicit-function-declaration");

    build
}

fn spec_target(target_info: &TargetInfo, bindgen: &mut bindgen::Builder, build: &mut cc::Build) {
    if target_info.is_x86_64() {
        *bindgen = std::mem::take(bindgen)
            .clang_arg("-DFD_HAS_X86=1")
            .clang_arg("-DFD_HAS_SSE=1")
            .clang_arg("-DFD_HAS_AVX=1")
            .clang_arg("-msse")
            .clang_arg("-msse2")
            .clang_arg("-mavx")
            .clang_arg("-mavx2");

        build
            .define("FD_HAS_X86", "1")
            .define("FD_HAS_SSE", "1")
            .define("FD_HAS_AVX", "1")
            .flag("-msse")
            .flag("-msse2")
            .flag("-mavx")
            .flag("-mavx2");
    } else if target_info.is_aarch64() {
        *bindgen = std::mem::take(bindgen).clang_arg("-DFD_HAS_ARM=1");
        build.define("FD_HAS_ARM", "1");
    }

    if target_info.is_macos() {
        *bindgen = std::mem::take(bindgen).clang_arg("-DSIGPOLL=SIGIO");
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
