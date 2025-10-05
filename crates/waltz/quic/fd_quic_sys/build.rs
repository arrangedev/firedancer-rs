use std::env;
use std::path::PathBuf;

use firedancer_rs_common::{TargetInfo, _pipeline_finalize};

fn main() {
    let target_info = TargetInfo::new();

    let (vendor_path, waltz_path) =
        find_vendor().expect("Failed to find vendor directory with submodules");

    let quic_path = waltz_path.join("quic");
    let util_path = vendor_path.join("util");
    let ballet_path = vendor_path.join("ballet");
    let disco_path = vendor_path.join("disco");

    setup_rerun(&quic_path, &util_path, &ballet_path, &disco_path);

    let wrapper_path = generate_header(&quic_path);
    let mut bindgen = init_bindgen(
        &wrapper_path,
        &waltz_path,
        &util_path,
        &ballet_path,
        &disco_path,
        &vendor_path,
    );
    let mut build = init_cc(
        &quic_path,
        &waltz_path,
        &util_path,
        &ballet_path,
        &disco_path,
        &vendor_path,
    );

    spec_target(&target_info, &mut bindgen, &mut build);
    _pipeline_finalize(build, bindgen, "fdquic", None);
}

fn setup_rerun(
    quic_path: &PathBuf,
    util_path: &PathBuf,
    ballet_path: &PathBuf,
    disco_path: &PathBuf,
) {
    let quic_sources = [
        "fd_quic.c",
        "fd_quic_ack_tx.c",
        "fd_quic_conn.c",
        "fd_quic_pkt_meta.c",
        "fd_quic_pretty_print.c",
        "fd_quic_proto.c",
        "fd_quic_retry.c",
        "fd_quic_stream.c",
        "fd_quic_stream_pool.c",
        "fd_quic_svc_q.c",
    ];

    for source in &quic_sources {
        println!(
            "cargo:rerun-if-changed={}",
            quic_path.join(source).display()
        );
    }

    println!(
        "cargo:rerun-if-changed={}",
        quic_path.join("fd_quic_pretty_print.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        quic_path.join("fd_quic_svc_q.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        quic_path.join("templ/fd_quic_frame.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        quic_path.join("templ/fd_quic_transport_params.c").display()
    );

    let quic_headers = [
        "fd_quic.h",
        "fd_quic_ack_tx.h",
        "fd_quic_common.h",
        "fd_quic_conn.h",
        "fd_quic_conn_id.h",
        "fd_quic_conn_map.h",
        "fd_quic_enum.h",
        "fd_quic_pkt_meta.h",
        "fd_quic_pretty_print.h",
        "fd_quic_private.h",
        "fd_quic_proto.h",
        "fd_quic_proto_structs.h",
        "fd_quic_retry.h",
        "fd_quic_retry_private.h",
        "fd_quic_stream.h",
        "fd_quic_stream_pool.h",
        "fd_quic_svc_q.h",
        "fd_quic_types.h",
    ];

    for header in &quic_headers {
        println!(
            "cargo:rerun-if-changed={}",
            quic_path.join(header).display()
        );
    }

    let crypto_sources = ["crypto/fd_quic_crypto_suites.c"];
    let crypto_headers = ["crypto/fd_quic_crypto_suites.h"];

    for source in &crypto_sources {
        println!(
            "cargo:rerun-if-changed={}",
            quic_path.join(source).display()
        );
    }
    for header in &crypto_headers {
        println!(
            "cargo:rerun-if-changed={}",
            quic_path.join(header).display()
        );
    }

    let tls_sources = ["tls/fd_quic_tls.c"];
    let tls_headers = ["tls/fd_quic_tls.h"];

    for source in &tls_sources {
        println!(
            "cargo:rerun-if-changed={}",
            quic_path.join(source).display()
        );
    }
    for header in &tls_headers {
        println!(
            "cargo:rerun-if-changed={}",
            quic_path.join(header).display()
        );
    }

    let log_sources = ["log/fd_quic_log.c"];
    let log_headers = ["log/fd_quic_log.h"];

    for source in &log_sources {
        println!(
            "cargo:rerun-if-changed={}",
            quic_path.join(source).display()
        );
    }
    for header in &log_headers {
        println!(
            "cargo:rerun-if-changed={}",
            quic_path.join(header).display()
        );
    }

    let templ_sources = ["templ/fd_quic_frame.c", "templ/fd_quic_transport_params.c"];

    for source in &templ_sources {
        println!(
            "cargo:rerun-if-changed={}",
            quic_path.join(source).display()
        );
    }

    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("fd_util_base.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ballet_path.join("ed25519").join("fd_ed25519.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ballet_path.join("sha512").join("fd_sha512.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        disco_path.join("fd_disco_base.h").display()
    );
}

fn generate_header(quic_path: &PathBuf) -> PathBuf {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("quic_wrapper.h");

    let header_content = format!(
        r#"
#include <stdio.h>
#include "{}/fd_quic.h"
"#,
        quic_path.canonicalize().unwrap().display()
    );

    std::fs::write(&wrapper_path, header_content).expect("Failed to write wrapper header");

    wrapper_path
}

fn init_bindgen(
    wrapper_path: &PathBuf,
    waltz_path: &PathBuf,
    util_path: &PathBuf,
    ballet_path: &PathBuf,
    disco_path: &PathBuf,
    vendor_path: &PathBuf,
) -> bindgen::Builder {
    let wrapper_c_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("wrapper.c");

    bindgen::Builder::default()
        .wrap_static_fns(true)
        .wrap_static_fns_path(&wrapper_c_path)
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", waltz_path.display()))
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg(format!("-I{}", ballet_path.display()))
        .clang_arg(format!("-I{}", disco_path.display()))
        .clang_arg(format!("-I{}", vendor_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-D_GNU_SOURCE")
        .clang_arg("-DFD_HAS_ATOMIC=1")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .wrap_unsafe_ops(true)
        .allowlist_function("fd_quic_.*")
        .allowlist_type("fd_quic_.*")
        .allowlist_var("FD_QUIC_.*")
        .allowlist_var("fd_quic_.*")
        .allowlist_var("FD_.*")
        .allowlist_recursively(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
}

fn init_cc(
    quic_path: &PathBuf,
    waltz_path: &PathBuf,
    util_path: &PathBuf,
    ballet_path: &PathBuf,
    disco_path: &PathBuf,
    vendor_path: &PathBuf,
) -> cc::Build {
    let mut build = cc::Build::new();

    let quic_sources = [
        "fd_quic.c",
        "fd_quic_ack_tx.c",
        "fd_quic_conn.c",
        "fd_quic_pkt_meta.c",
        // "fd_quic_pretty_print.c",
        "fd_quic_proto.c",
        "fd_quic_retry.c",
        "fd_quic_stream.c",
        "fd_quic_stream_pool.c",
        // "fd_quic_svc_q.c",
    ];

    let crypto_sources = ["crypto/fd_quic_crypto_suites.c"];
    let tls_sources = ["tls/fd_quic_tls.c"];
    let log_sources = ["log/fd_quic_log.c"];
    let templ_sources = ["templ/fd_quic_transport_params.c"];

    let util_sources = [
        "util/log/fd_log.c",
        "tango/dcache/fd_dcache.c",
        "tango/mcache/fd_mcache.c",
        "ballet/aes/fd_aes_gcm_ref.c",
        "ballet/aes/fd_aes_base_ref.c",
        "ballet/aes/fd_aes_gcm_ref_ghash.c",
        "ballet/sha256/fd_sha256.c",
        "ballet/sha512/fd_sha512.c",
        "ballet/hmac/fd_hmac.c",
        "ballet/ed25519/fd_curve25519.c",
        "ballet/ed25519/fd_curve25519_scalar.c",
        "ballet/ed25519/fd_f25519.c",
        "ballet/ed25519/fd_x25519.c",
        "ballet/ed25519/fd_ed25519_user.c",
        "ballet/hex/fd_hex.c",
        "util/rng/fd_rng.c",
        "util/rng/fd_rng_secure.c",
        "util/io/fd_io.c",
        "util/env/fd_env.c",
        "util/cstr/fd_cstr.c",
        "util/bits/fd_bits.c",
        "waltz/tls/fd_tls.c",
        "waltz/tls/fd_tls_proto.c",
        "waltz/tls/fd_tls_asn1.c",
        "ballet/x509/fd_x509_mock.c",
        "util/shmem/fd_numa_stub.c",
    ];

    #[cfg(target_os = "linux")]
    let xdp_sources = [
        "waltz/xdp/fd_xdp1.c",
        "waltz/xdp/fd_xdp_redirect_user.c",
        "waltz/xdp/fd_xsk.c",
    ];

    #[cfg(not(target_os = "linux"))]
    let xdp_sources: [&str; 0] = [];

    for source in &quic_sources {
        build.file(quic_path.join(source));
    }
    for source in &crypto_sources {
        build.file(quic_path.join(source));
    }
    for source in &tls_sources {
        build.file(quic_path.join(source));
    }
    for source in &log_sources {
        build.file(quic_path.join(source));
    }
    for source in &templ_sources {
        build.file(quic_path.join(source));
    }
    for source in &util_sources {
        build.file(vendor_path.join(source));
    }
    for source in &xdp_sources {
        build.file(vendor_path.join(source));
    }

    build
        .include(waltz_path)
        .include(util_path)
        .include(ballet_path)
        .include(disco_path)
        .include(vendor_path)
        .include(vendor_path.join("tango"))
        .define("FD_HAS_HOSTED", "1")
        .define("FD_LOG_STYLE", "0")
        .define("_GNU_SOURCE", "1")
        .define("FD_HAS_ATOMIC", "1")
        .flag("-Wall")
        .flag("-Wextra")
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC")
        .flag("-Wno-error=implicit-function-declaration");

    // macOS so i can test
    let stub_c_content = r#"
// Stub implementations for macOS compatibility
#include <errno.h>

int fd_cpuset_getaffinity(int pid, int cpusetsize, void *mask) {
    // Stub implementation - just return success
    (void)pid;
    (void)cpusetsize;
    (void)mask;
    return 0;
}
"#;

    let stub_c_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("fd_stubs.c");
    std::fs::write(&stub_c_path, stub_c_content).expect("Failed to write stub file");
    build.file(&stub_c_path);

    println!("cargo:rustc-link-lib=pthread");

    let wrapper_c_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("wrapper.c");
    if wrapper_c_path.exists() {
        let mut wrapper_build = cc::Build::new();
        wrapper_build
            .file(&wrapper_c_path)
            .include(waltz_path)
            .include(util_path)
            .include(ballet_path)
            .include(disco_path)
            .include(vendor_path)
            .define("FD_HAS_HOSTED", "1")
            .define("FD_LOG_STYLE", "0")
            .define("_GNU_SOURCE", "1")
            .define("FD_HAS_ATOMIC", "1")
            .flag("-std=c17")
            .flag("-O3")
            .flag("-fPIC")
            .flag("-Wno-error=implicit-function-declaration");

        wrapper_build.compile("fdquicwrapper");
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
    } else {
        cfg_linux_crypto(bindgen, build);
    }
}

fn cfg_x86_64(bindgen: &mut bindgen::Builder, build: &mut cc::Build) {
    *bindgen = std::mem::take(bindgen)
        .clang_arg("-DFD_HAS_X86=1")
        .clang_arg("-msse4.2")
        .clang_arg("-mavx2");

    build
        .define("FD_HAS_X86", "1")
        .flag("-msse4.2")
        .flag("-mavx2");
}

fn cfg_aarch64(bindgen: &mut bindgen::Builder, build: &mut cc::Build) {
    *bindgen = std::mem::take(bindgen).clang_arg("-DFD_HAS_ARM=1");
    build.define("FD_HAS_ARM", "1");
}

fn cfg_macos(bindgen: &mut bindgen::Builder, build: &mut cc::Build) {
    *bindgen = std::mem::take(bindgen).clang_arg("-DSIGPOLL=SIGIO");
    build.define("SIGPOLL", "SIGIO");

    // skip linking libopenssl on macOS
    println!("cargo:warning=no libopenssl, limited TLS capabilities on macOS");
}

fn cfg_linux_crypto(bindgen: &mut bindgen::Builder, build: &mut cc::Build) {
    println!("cargo:rustc-link-lib=ssl");
    println!("cargo:rustc-link-lib=crypto");

    if let Ok(output) = std::process::Command::new("pkg-config")
        .args(&["--cflags", "openssl"])
        .output()
    {
        let flags = String::from_utf8_lossy(&output.stdout);
        for flag in flags.split_whitespace() {
            if flag.starts_with("-I") {
                *bindgen = std::mem::take(bindgen).clang_arg(flag);
                build.include(&flag[2..]);
            }
        }
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
        let waltz_dir = vendor_path.join("waltz");
        if waltz_dir.exists() {
            eprintln!("Found waltz at: {}", waltz_dir.display());
            return Ok((vendor_path, waltz_dir));
        }

        let src_waltz = vendor_path.join("src").join("waltz");
        if src_waltz.exists() {
            eprintln!("Found waltz at: {}", src_waltz.display());
            return Ok((vendor_path, src_waltz));
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    Err(format!(
        "Failed to find vendor directory with waltz subdirectory. Started search from: {}",
        manifest_dir.display()
    ))
}
