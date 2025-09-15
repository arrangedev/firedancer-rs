use std::env;
use std::path::PathBuf;

use firedancer_rs_common::{TargetInfo, _pipeline_finalize};

fn main() {
    let target_info = TargetInfo::new();

    let (vendor_path, ballet_path) =
        find_vendor().expect("Failed to find vendor directory with submodules");

    let ed25519_path = ballet_path.join("ed25519");
    let sha512_path = ballet_path.join("sha512");
    let sha256_path = ballet_path.join("sha256");
    let util_path = vendor_path.join("util");

    setup_rerun(&ed25519_path, &sha512_path, &sha256_path, &util_path);

    let wrapper_path = generate_header(&ed25519_path);
    let mut bindgen = init_bindgen(&wrapper_path, &ballet_path, &util_path, &vendor_path);
    let mut build = init_cc(
        &ed25519_path,
        &sha512_path,
        &sha256_path,
        &ballet_path,
        &util_path,
        &vendor_path,
    );

    spec_target(
        &target_info,
        &mut bindgen,
        &mut build,
        &ed25519_path,
        &sha512_path,
    );

    _pipeline_finalize(build, bindgen, "fded25519", None);
}

fn setup_rerun(
    ed25519_path: &PathBuf,
    sha512_path: &PathBuf,
    sha256_path: &PathBuf,
    util_path: &PathBuf,
) {
    println!(
        "cargo:rerun-if-changed={}",
        ed25519_path.join("fd_ed25519.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ed25519_path.join("fd_ed25519_user.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ed25519_path.join("fd_curve25519.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ed25519_path.join("fd_curve25519_secure.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ed25519_path.join("fd_curve25519_scalar.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ed25519_path.join("fd_curve25519_tables.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ed25519_path.join("fd_f25519.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ed25519_path.join("fd_x25519.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ed25519_path.join("fd_ristretto255.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        sha512_path.join("fd_sha512.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        sha512_path.join("fd_sha512.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        sha256_path.join("fd_sha256.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        sha256_path.join("fd_sha256.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("fd_util_base.h").display()
    );
}

fn generate_header(ed25519_path: &PathBuf) -> PathBuf {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("ed25519_wrapper.h");
    let sha256_path = ed25519_path.parent().unwrap().join("sha256");

    std::fs::write(
        &wrapper_path,
        format!(
            r#"
#include "{}/fd_ed25519.h"
#include "{}/fd_f25519.h"
#include "{}/fd_curve25519.h"
#include "{}/fd_ristretto255.h"
#include "{}/fd_x25519.h"
#include "{}/fd_sha256.h"
"#,
            ed25519_path.canonicalize().unwrap().display(),
            ed25519_path.canonicalize().unwrap().display(),
            ed25519_path.canonicalize().unwrap().display(),
            ed25519_path.canonicalize().unwrap().display(),
            ed25519_path.canonicalize().unwrap().display(),
            sha256_path.canonicalize().unwrap().display(),
        ),
    )
    .expect("Failed to write wrapper header");

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
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .wrap_unsafe_ops(true)
        .allowlist_function("fd_ed25519_.*")
        .allowlist_function("fd_x25519_.*")
        .allowlist_function("fd_ristretto255_.*")
        .allowlist_function("fd_sha512_.*")
        .allowlist_function("fd_sha256_.*")
        .allowlist_type("fd_ed25519_.*")
        .allowlist_type("fd_x25519_.*")
        .allowlist_type("fd_ristretto255_.*")
        .allowlist_type("fd_sha512_.*")
        .allowlist_type("fd_sha256_.*")
        .allowlist_var("FD_ED25519_.*")
        .allowlist_var("FD_X25519_.*")
        .allowlist_var("FD_RISTRETTO255_.*")
        .allowlist_var("FD_SHA512_.*")
        .allowlist_var("FD_SHA256_.*")
        .allowlist_var("FD_.*")
        .allowlist_recursively(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
}

fn init_cc(
    ed25519_path: &PathBuf,
    sha512_path: &PathBuf,
    sha256_path: &PathBuf,
    ballet_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
) -> cc::Build {
    let mut build = cc::Build::new();
    build
        .file(ed25519_path.join("fd_ed25519_user.c"))
        .file(ed25519_path.join("fd_curve25519.c"))
        .file(ed25519_path.join("fd_curve25519_secure.c"))
        .file(ed25519_path.join("fd_curve25519_scalar.c"))
        .file(ed25519_path.join("fd_curve25519_tables.c"))
        .file(ed25519_path.join("fd_f25519.c"))
        .file(ed25519_path.join("fd_x25519.c"))
        .file(ed25519_path.join("fd_ristretto255.c"))
        .file(sha512_path.join("fd_sha512.c"))
        .file(sha256_path.join("fd_sha256.c"))
        .include(ballet_path)
        .include(util_path)
        .include(vendor_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_LOG_STYLE", "0")
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC")
        .flag("-Wno-error=implicit-function-declaration");

    build
}

fn spec_target(
    target_info: &TargetInfo,
    bindgen: &mut bindgen::Builder,
    build: &mut cc::Build,
    ed25519_path: &PathBuf,
    sha512_path: &PathBuf,
) {
    //if target_info.is_x86_64() {
    //    cfg_x86_64(target_info, bindgen, build, ed25519_path, sha512_path);
    //} else if target_info.is_aarch64() {
    //    cfg_aarch64(bindgen, build);
    //} else {
    cfg_catchall(build, ed25519_path);
    //}

    //if target_info.is_macos() {
    //    cfg_arm64_mac(bindgen, build);
    //}
}

fn cfg_x86_64(
    _target_info: &TargetInfo,
    bindgen: &mut bindgen::Builder,
    build: &mut cc::Build,
    ed25519_path: &PathBuf,
    sha512_path: &PathBuf,
) {
    cfg_x86_64_native(bindgen, build, ed25519_path, sha512_path);
}

fn cfg_x86_64_emu(bindgen: &mut bindgen::Builder, build: &mut cc::Build) {
    *bindgen = std::mem::take(bindgen)
        .clang_arg("-DFD_HAS_X86=0")
        .clang_arg("-DFD_HAS_SSE=0")
        .clang_arg("-DFD_HAS_AVX=0")
        .clang_arg("-DFD_HAS_AVX512=0");

    build
        .define("FD_HAS_X86", "0")
        .define("FD_HAS_SSE", "0")
        .define("FD_HAS_AVX", "0")
        .define("FD_HAS_AVX512", "0");
}

fn cfg_x86_64_native(
    bindgen: &mut bindgen::Builder,
    build: &mut cc::Build,
    ed25519_path: &PathBuf,
    sha512_path: &PathBuf,
) {
    *bindgen = std::mem::take(bindgen)
        .clang_arg("-DFD_HAS_X86=1")
        .clang_arg("-DFD_HAS_SSE=1")
        .clang_arg("-DFD_HAS_AVX=1")
        .clang_arg("-DFD_HAS_AVX512=1")
        .clang_arg("-msse")
        .clang_arg("-msse2")
        .clang_arg("-mavx")
        .clang_arg("-mavx2")
        .clang_arg("-mavx512f")
        .clang_arg("-mavx512bw")
        .clang_arg("-mavx512dq")
        .clang_arg("-mavx512vl")
        .clang_arg("-mavx512ifma")
        .clang_arg("-mavx512vbmi");

    build
        .define("FD_HAS_X86", "1")
        .define("FD_HAS_SSE", "1")
        .define("FD_HAS_AVX", "1")
        .define("FD_HAS_AVX512", "1")
        .flag("-msse")
        .flag("-msse2")
        .flag("-mavx")
        .flag("-mavx2")
        .flag("-mavx512f")
        .flag("-mavx512bw")
        .flag("-mavx512dq")
        .flag("-mavx512vl")
        .flag("-mavx512ifma")
        .flag("-mavx512vbmi")
        .file(sha512_path.join("fd_sha512_core_avx2.S"));

    link_avx512_maybe(build, ed25519_path);
}

fn link_avx512_maybe(build: &mut cc::Build, ed25519_path: &PathBuf) {
    let avx512_path = ed25519_path.join("avx512");
    if avx512_path.exists() {
        let required_avx512_files = [
            "fd_curve25519.c",
            "fd_curve25519_secure.c",
            "fd_f25519.c",
            "fd_r43x6.c",
            "fd_r43x6_ge.c",
        ];

        let all_files_exist = required_avx512_files
            .iter()
            .all(|file| avx512_path.join(file).exists());

        if all_files_exist {
            println!("cargo:warning=adding AVX512 src");
            for file in &required_avx512_files {
                build.file(avx512_path.join(file));
            }
        } else {
            println!("cargo:warning=Some AVX512 src missing, using refimpl");
        }
    }
}

fn cfg_aarch64(bindgen: &mut bindgen::Builder, build: &mut cc::Build) {
    *bindgen = std::mem::take(bindgen).clang_arg("-DFD_HAS_ARM=1");
    build.define("FD_HAS_ARM", "1");
}

fn cfg_catchall(build: &mut cc::Build, ed25519_path: &PathBuf) {
    let ref_path = ed25519_path.join("ref");
    if ref_path.exists() {
        build
            .file(ref_path.join("fd_curve25519.c"))
            .file(ref_path.join("fd_curve25519_secure.c"))
            .file(ref_path.join("fd_f25519.c"));
    }
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
