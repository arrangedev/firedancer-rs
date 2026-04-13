use std::env;
use std::path::PathBuf;

use firedancer_rs_common::{_pipeline_finalize, fd_log_stub_path, TargetInfo};

fn main() {
    let target_info = TargetInfo::new();

    let (vendor_path, util_path) =
        find_vendor().expect("Failed to find vendor directory with util subdirectory");

    let clock_path = util_path.join("clock");
    let bits_path = util_path.join("bits");

    setup_rerun(&clock_path, &bits_path, &util_path);

    let wrapper_path = generate_header(&clock_path);
    let mut bindgen = init_bindgen(&wrapper_path, &util_path, &vendor_path);
    let mut build = init_cc(&clock_path, &bits_path, &util_path, &vendor_path);

    spec_target(&target_info, &mut bindgen, &mut build);

    _pipeline_finalize(build, bindgen, "fdclock", None);
}

fn setup_rerun(clock_path: &PathBuf, bits_path: &PathBuf, util_path: &PathBuf) {
    println!(
        "cargo:rerun-if-changed={}",
        clock_path.join("fd_clock.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        clock_path.join("fd_clock.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        bits_path.join("fd_bits.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("fd_util_base.h").display()
    );
}

fn generate_header(clock_path: &PathBuf) -> PathBuf {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("clock_wrapper.h");

    let header_content = format!(
        "#include \"{}/fd_clock.h\"\n",
        clock_path.canonicalize().unwrap().display(),
    );

    std::fs::write(&wrapper_path, header_content).expect("Failed to write wrapper header");

    wrapper_path
}

fn init_bindgen(
    wrapper_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
) -> bindgen::Builder {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_c_path = out_path.join("clock_wrapper.c");

    bindgen::Builder::default()
        .wrap_static_fns(true)
        .wrap_static_fns_path(&wrapper_c_path)
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg(format!("-I{}", vendor_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_HAS_DOUBLE=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .allowlist_function("fd_clock_.*")
        .allowlist_type("fd_clock_.*")
        .allowlist_var("FD_CLOCK_.*")
        .allowlist_type("fd_clock_func_t")
        .wrap_unsafe_ops(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
}

fn init_cc(
    clock_path: &PathBuf,
    bits_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
) -> cc::Build {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_c_path = out_path.join("clock_wrapper.c");

    let mut build = cc::Build::new();
    build
        .file(clock_path.join("fd_clock.c"))
        .file(bits_path.join("fd_bits.c"))
        .file(fd_log_stub_path())
        .file(&wrapper_c_path)
        .include(wrapper_c_path.parent().unwrap())
        .include(util_path)
        .include(vendor_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_HAS_DOUBLE", "1")
        .define("FD_LOG_STYLE", "0")
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
            .clang_arg("-DFD_HAS_AVX=1");

        build
            .define("FD_HAS_X86", "1")
            .define("FD_HAS_SSE", "1")
            .define("FD_HAS_AVX", "1");
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

        let util_dir = vendor_path.join("util");
        if util_dir.exists() {
            return Ok((vendor_path, util_dir));
        }

        let src_util = vendor_path.join("src").join("util");
        if src_util.exists() {
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
