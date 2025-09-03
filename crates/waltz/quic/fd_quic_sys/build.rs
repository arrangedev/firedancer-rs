use std::env;
use std::path::PathBuf;

fn main() {
    let firedancer_path = PathBuf::from("../../../../vendor");
    let waltz_path = firedancer_path.join("waltz");
    let quic_path = waltz_path.join("quic");
    let util_path = firedancer_path.join("util");
    let ballet_path = firedancer_path.join("ballet");
    let disco_path = firedancer_path.join("disco");

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

    for source in &crypto_sources {
        println!(
            "cargo:rerun-if-changed={}",
            quic_path.join(source).display()
        );
    }

    let crypto_headers = ["crypto/fd_quic_crypto_suites.h"];

    for header in &crypto_headers {
        println!(
            "cargo:rerun-if-changed={}",
            quic_path.join(header).display()
        );
    }

    let tls_sources = ["tls/fd_quic_tls.c"];

    for source in &tls_sources {
        println!(
            "cargo:rerun-if-changed={}",
            quic_path.join(source).display()
        );
    }

    let tls_headers = ["tls/fd_quic_tls.h"];

    for header in &tls_headers {
        println!(
            "cargo:rerun-if-changed={}",
            quic_path.join(header).display()
        );
    }

    let log_sources = ["log/fd_quic_log.c"];

    for source in &log_sources {
        println!(
            "cargo:rerun-if-changed={}",
            quic_path.join(source).display()
        );
    }

    let log_headers = ["log/fd_quic_log.h"];

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

    let target = env::var("TARGET").unwrap();
    let is_x86_64 = target.contains("x86_64");
    let is_aarch64 = target.contains("aarch64") || target.contains("arm");
    let is_macos = target.contains("apple");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("quic_wrapper.h");
    let wrapper_c_path = out_path.join("wrapper.c");

    std::fs::write(
        &wrapper_path,
        format!(
            r#"
#include <stdio.h>
#include "{}/fd_quic.h"
"#,
            quic_path.canonicalize().unwrap().display()
        ),
    )
    .expect("Failed to write wrapper header");

    let mut bindgen = bindgen::Builder::default()
        .wrap_static_fns(true)
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", waltz_path.display()))
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg(format!("-I{}", ballet_path.display()))
        .clang_arg(format!("-I{}", disco_path.display()))
        .clang_arg(format!("-I{}", firedancer_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .wrap_static_fns_path(&wrapper_c_path)
        .allowlist_function("fd_quic_.*")
        .allowlist_type("fd_quic_.*")
        .allowlist_var("FD_QUIC_.*")
        .allowlist_var("fd_quic_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    if is_x86_64 {
        bindgen = bindgen
            .clang_arg("-DFD_HAS_X86=1")
            .clang_arg("-msse4.2")
            .clang_arg("-mavx2");
    } else if is_aarch64 {
        bindgen = bindgen.clang_arg("-DFD_HAS_ARM=1");
    }

    if is_macos {
        bindgen = bindgen.clang_arg("-DSIGPOLL=SIGIO");
    }

    let bindings = bindgen.generate().expect("Unable to generate bindings");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    let mut build = cc::Build::new();
    let mut all_sources = Vec::new();
    for source in &quic_sources {
        all_sources.push(quic_path.join(source));
    }
    for source in &crypto_sources {
        all_sources.push(quic_path.join(source));
    }
    for source in &tls_sources {
        all_sources.push(quic_path.join(source));
    }
    for source in &log_sources {
        all_sources.push(quic_path.join(source));
    }
    for source in &templ_sources {
        all_sources.push(quic_path.join(source));
    }

    build
        .files(&all_sources)
        .include(&waltz_path)
        .include(&util_path)
        .include(&ballet_path)
        .include(&disco_path)
        .include(&firedancer_path)
        .flag("-Wall")
        .flag("-Wextra")
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC")
        .flag("-Wno-error=implicit-function-declaration")
        .define("FD_HAS_HOSTED", "1")
        .define("FD_LOG_STYLE", "0");

    if is_x86_64 {
        build
            .define("FD_HAS_X86", "1")
            .flag("-msse4.2")
            .flag("-mavx2");
    } else if is_aarch64 {
        build.define("FD_HAS_ARM", "1");
    }

    if is_macos {
        build.define("SIGPOLL", "SIGIO");
    } else {
        println!("cargo:rustc-link-lib=ssl");
        println!("cargo:rustc-link-lib=crypto");
    }

    println!("cargo:rustc-link-lib=pthread");

    build.compile("fdquic");
}
