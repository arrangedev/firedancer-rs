use std::env;
use std::path::PathBuf;

use firedancer_rs_common::{TargetInfo, _pipeline_finalize};

fn main() {
    let target_info = TargetInfo::new();

    let (vendor_path, disco_path) =
        find_vendor().expect("Failed to find vendor directory with submodules");

    let events_path = disco_path.join("events");
    let util_path = vendor_path.join("util");
    let tango_path = vendor_path.join("tango");
    let ballet_path = vendor_path.join("ballet");
    let flamenco_path = vendor_path.join("flamenco");

    setup_rerun(
        &events_path,
        &util_path,
        &tango_path,
        &ballet_path,
        &flamenco_path,
    );

    let wrapper_path = generate_header(&events_path, &disco_path, &util_path);
    let mut bindgen = init_bindgen(&wrapper_path, &disco_path, &util_path, &vendor_path);
    let mut build = init_cc(&events_path, &disco_path, &util_path, &vendor_path);

    spec_target(&target_info, &mut bindgen, &mut build);

    _pipeline_finalize(build, bindgen, "fdcirq", None);
}

fn setup_rerun(
    events_path: &PathBuf,
    util_path: &PathBuf,
    tango_path: &PathBuf,
    ballet_path: &PathBuf,
    flamenco_path: &PathBuf,
) {
    println!(
        "cargo:rerun-if-changed={}",
        events_path.join("fd_circq.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        events_path.join("fd_circq.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("fd_util_base.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tango_path.join("fd_tango.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ballet_path.join("shred").join("fd_shred.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ballet_path.join("txn").join("fd_txn.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        flamenco_path
            .join("types")
            .join("fd_types_custom.h")
            .display()
    );
}

fn generate_header(events_path: &PathBuf, _disco_path: &PathBuf, _util_path: &PathBuf) -> PathBuf {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("circq_wrapper.h");

    let header_content = format!(
        r#"
#include "{}/fd_circq.h"
"#,
        events_path.canonicalize().unwrap().display()
    );

    std::fs::write(&wrapper_path, header_content).expect("Failed to write wrapper header");

    wrapper_path
}

fn init_bindgen(
    wrapper_path: &PathBuf,
    disco_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
) -> bindgen::Builder {
    bindgen::Builder::default()
        .wrap_static_fns(true)
        .wrap_static_fns_path(wrapper_path)
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", disco_path.display()))
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg(format!("-I{}", vendor_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .wrap_unsafe_ops(true)
        .allowlist_function("fd_circq_.*")
        .allowlist_type("fd_circq_.*")
        .allowlist_var("FD_CIRCQ_.*")
        .allowlist_var("FD_.*")
        .allowlist_recursively(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
}

fn init_cc(
    events_path: &PathBuf,
    disco_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
) -> cc::Build {
    let mut build = cc::Build::new();

    // Add stub implementations for missing functions
    let stub_path = write_stubs();

    build
        .file(events_path.join("fd_circq.c"))
        .file(stub_path)
        .include(disco_path)
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

fn write_stubs() -> PathBuf {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let stub_path = std::path::PathBuf::from(&out_dir).join("stubs.c");

    let stub_content = r#"
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <unistd.h>

// Logging stubs for fd_circq
char const *
fd_log_private_0( char const * fmt, ... ) {
    static char buf[1024];
    (void)fmt;
    return buf;
}

void
fd_log_private_1( int level, long now, char const * file, int line, char const * func, char const * msg ) {
    (void)level; (void)now; (void)file; (void)line; (void)func; (void)msg;
    // Optional: uncomment for debug output
    // fprintf(stderr, "[LOG] %s:%d %s: %s\n", file, line, func, msg);
}

void
fd_log_private_2( int level, long now, char const * file, int line, char const * func, char const * msg ) {
    (void)level; (void)now; (void)file; (void)line; (void)func; (void)msg;
    // Optional: uncomment for debug output
    // fprintf(stderr, "[LOG] %s:%d %s: %s\n", file, line, func, msg);
}

long
fd_log_wallclock( void ) {
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    return (long)(ts.tv_sec * 1000000000L + ts.tv_nsec);
}
"#;

    std::fs::write(&stub_path, stub_content).expect("Failed to write stub file");
    stub_path
}

fn spec_target(target_info: &TargetInfo, _bindgen: &mut bindgen::Builder, build: &mut cc::Build) {
    if target_info.is_x86_64() {
        cfg_x86_64(build);
    } else if target_info.is_aarch64() {
        cfg_aarch64(build);
    }

    if target_info.is_macos() {
        cfg_macos(build);
    }
}

fn cfg_x86_64(build: &mut cc::Build) {
    build
        .define("FD_HAS_X86", "1")
        .define("FD_HAS_SSE", "1")
        .define("FD_HAS_AVX", "1");
}

fn cfg_aarch64(build: &mut cc::Build) {
    build.define("FD_HAS_ARM", "1");
}

fn cfg_macos(build: &mut cc::Build) {
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
        let disco_dir = vendor_path.join("disco");
        if disco_dir.exists() {
            eprintln!("Found disco at: {}", disco_dir.display());
            return Ok((vendor_path, disco_dir));
        }

        let src_disco = vendor_path.join("src").join("disco");
        if src_disco.exists() {
            eprintln!("Found disco at: {}", src_disco.display());
            return Ok((vendor_path, src_disco));
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    Err(format!(
        "Failed to find vendor directory with disco subdirectory. Started search from: {}",
        manifest_dir.display()
    ))
}
