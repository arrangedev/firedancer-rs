use std::env;
use std::path::PathBuf;

use firedancer_rs_common::{TargetInfo, _pipeline_finalize};

fn main() {
    let target_info = TargetInfo::new();

    let (vendor_path, flamenco_path) =
        find_vendor().expect("Failed to find vendor directory with submodules");

    let vm_path = flamenco_path.join("vm");
    let vm_syscall_path = vm_path.join("syscall");
    let ballet_path = vendor_path.join("ballet");
    let util_path = vendor_path.join("util");

    setup_rerun(&vm_path, &vm_syscall_path, &ballet_path, &util_path);

    let wrapper_path = generate_header(&vm_path, &ballet_path);
    let mut bindgen = init_bindgen(
        &wrapper_path,
        &flamenco_path,
        &ballet_path,
        &util_path,
        &vendor_path,
    );
    let mut build = init_cc(
        &vm_path,
        &vm_syscall_path,
        &flamenco_path,
        &ballet_path,
        &util_path,
        &vendor_path,
    );

    spec_target(&target_info, &mut bindgen, &mut build);

    _pipeline_finalize(build, bindgen, "fdsvm", None);
}

fn setup_rerun(
    vm_path: &PathBuf,
    vm_syscall_path: &PathBuf,
    ballet_path: &PathBuf,
    util_path: &PathBuf,
) {
    println!(
        "cargo:rerun-if-changed={}",
        vm_path.join("fd_vm.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        vm_path.join("fd_vm.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        vm_path.join("fd_vm_base.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        vm_path.join("fd_vm_private.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        vm_path.join("fd_vm_interp.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        vm_path.join("fd_vm_interp_core.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        vm_path.join("fd_vm_interp_jump_table.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        vm_path.join("fd_vm_trace.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        vm_path.join("fd_vm_disasm.c").display()
    );

    println!(
        "cargo:rerun-if-changed={}",
        vm_syscall_path.join("fd_vm_syscall.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        vm_syscall_path.join("fd_vm_syscall.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        vm_syscall_path.join("fd_vm_cpi.h").display()
    );

    println!(
        "cargo:rerun-if-changed={}",
        ballet_path.join("sha256/fd_sha256.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ballet_path.join("sha256/fd_sha256.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("fd_util_base.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("log/fd_log.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("log/fd_log.c").display()
    );
}

fn generate_header(vm_path: &PathBuf, ballet_path: &PathBuf) -> PathBuf {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("svm_wrapper.h");

    let header_content = format!(
        r#"
#include "{}/fd_vm.h"
#include "{}/fd_vm_base.h"
#include "{}/syscall/fd_vm_syscall.h"
#include "{}/syscall/fd_vm_cpi.h"
#include "{}/sha256/fd_sha256.h"
"#,
        vm_path.canonicalize().unwrap().display(),
        vm_path.canonicalize().unwrap().display(),
        vm_path.canonicalize().unwrap().display(),
        vm_path.canonicalize().unwrap().display(),
        ballet_path.canonicalize().unwrap().display()
    );

    std::fs::write(&wrapper_path, header_content).expect("Failed to write wrapper header");

    wrapper_path
}

fn init_bindgen(
    wrapper_path: &PathBuf,
    flamenco_path: &PathBuf,
    ballet_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
) -> bindgen::Builder {
    bindgen::Builder::default()
        .wrap_static_fns(true)
        .wrap_static_fns_path(wrapper_path)
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", flamenco_path.display()))
        .clang_arg(format!("-I{}", ballet_path.display()))
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg(format!("-I{}", vendor_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-DFD_HAS_INT128=1")
        .clang_arg("-DFD_HAS_SECP256K1=1")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .wrap_unsafe_ops(true)
        .allowlist_function("fd_vm_.*")
        .allowlist_function("fd_sbpf_.*")
        .allowlist_function("fd_sol_.*")
        .allowlist_type("fd_vm_.*")
        .allowlist_type("fd_sbpf_.*")
        .allowlist_var("FD_VM_.*")
        .allowlist_var("FD_SBPF_.*")
        .allowlist_var("FD_SOL_.*")
        .allowlist_var("FD_.*")
        .allowlist_recursively(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
}

fn init_cc(
    vm_path: &PathBuf,
    vm_syscall_path: &PathBuf,
    flamenco_path: &PathBuf,
    ballet_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
) -> cc::Build {
    let mut build = cc::Build::new();
    build
        .file(vm_path.join("fd_vm.c"))
        .file(vm_path.join("fd_vm_interp.c"))
        .file(vm_path.join("fd_vm_trace.c"))
        .file(vm_path.join("fd_vm_disasm.c"))
        .file(vm_syscall_path.join("fd_vm_syscall.c"))
        .file(vm_syscall_path.join("fd_vm_syscall_util.c"))
        .file(vm_syscall_path.join("fd_vm_syscall_runtime.c"))
        .file(vm_syscall_path.join("fd_vm_syscall_crypto.c"))
        .file(vm_syscall_path.join("fd_vm_syscall_hash.c"))
        .file(vm_syscall_path.join("fd_vm_syscall_curve.c"))
        .file(vm_syscall_path.join("fd_vm_syscall_pda.c"))
        // .file(vm_syscall_path.join("fd_vm_syscall_cpi.c"))
        // .file(vm_syscall_path.join("fd_vm_syscall_cpi_common.c"))
        .file(ballet_path.join("sha256/fd_sha256.c"))
        .file(util_path.join("log/fd_log.c"))
        .file(util_path.join("tile/fd_tile.c"))
        .file(util_path.join("io/fd_io.c"))
        .file(util_path.join("cstr/fd_cstr.c"))
        .include(flamenco_path)
        .include(ballet_path)
        .include(util_path)
        .include(vendor_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_LOG_STYLE", "0")
        .define("FD_HAS_INT128", "1")
        .define("FD_HAS_SECP256K1", "1")
        .define("_GNU_SOURCE", "1")
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
        .clang_arg("-DFD_HAS_AVX=1")
        .clang_arg("-DFD_HAS_AVX512=1");

    build
        .define("FD_HAS_X86", "1")
        .define("FD_HAS_SSE", "1")
        .define("FD_HAS_AVX", "1")
        .define("FD_HAS_AVX512", "1");
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
        let flamenco_dir = vendor_path.join("flamenco");
        if flamenco_dir.exists() {
            eprintln!("Found flamenco at: {}", flamenco_dir.display());
            return Ok((vendor_path, flamenco_dir));
        }

        let src_flamenco = vendor_path.join("src").join("flamenco");
        if src_flamenco.exists() {
            eprintln!("Found flamenco at: {}", src_flamenco.display());
            return Ok((vendor_path, src_flamenco));
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    Err(format!(
        "Failed to find vendor directory with flamenco subdirectory. Started search from: {}",
        manifest_dir.display()
    ))
}
