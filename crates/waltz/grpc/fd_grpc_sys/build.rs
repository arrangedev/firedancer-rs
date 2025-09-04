use std::env;
use std::path::PathBuf;

fn main() {
    let firedancer_path = PathBuf::from("../../../../vendor");
    let waltz_path = firedancer_path.join("waltz");
    let grpc_path = waltz_path.join("grpc");
    let h2_path = waltz_path.join("h2");
    let ballet_path = firedancer_path.join("ballet");
    let util_path = firedancer_path.join("util");

    println!(
        "cargo:rerun-if-changed={}",
        grpc_path.join("fd_grpc_client.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        grpc_path.join("fd_grpc_client_private.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        grpc_path.join("fd_grpc_codec.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        grpc_path.join("fd_grpc_client.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        grpc_path.join("fd_grpc_codec.c").display()
    );

    let target = env::var("TARGET").unwrap();
    let is_x86_64 = target.contains("x86_64");
    let is_aarch64 = target.contains("aarch64") || target.contains("arm");
    let is_macos = target.contains("apple");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("grpc_wrapper.h");

    std::fs::write(
        &wrapper_path,
        format!(
            r#"
#include "{}/fd_grpc_client.h"
#include "{}/fd_grpc_codec.h"
"#,
            grpc_path.canonicalize().unwrap().display(),
            grpc_path.canonicalize().unwrap().display(),
        ),
    )
    .expect("Failed to write wrapper header");

    let openssl_include = std::process::Command::new("pkg-config")
        .args(&["--cflags", "openssl"])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| String::new());

    let mut bindgen = bindgen::Builder::default()
        .wrap_static_fns(true)
        .wrap_static_fns_path(&wrapper_path)
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", waltz_path.display()))
        .clang_arg(format!("-I{}", ballet_path.display()))
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg(format!("-I{}", firedancer_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-DFD_HAS_OPENSSL=1")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .allowlist_function("fd_grpc_.*")
        .allowlist_type("fd_grpc_.*")
        .allowlist_var("FD_GRPC_.*")
        .allowlist_function("fd_h2_.*")
        .allowlist_type("fd_h2_.*")
        .allowlist_var("FD_H2_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    if !openssl_include.is_empty() {
        for flag in openssl_include.split_whitespace() {
            if flag.starts_with("-I") {
                bindgen = bindgen.clang_arg(flag);
            }
        }
    }

    if is_x86_64 {
        bindgen = bindgen
            .clang_arg("-DFD_HAS_X86=1")
            .clang_arg("-DFD_HAS_SSE=1")
            .clang_arg("-DFD_HAS_AVX=1");
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

    let openssl_libs = std::process::Command::new("pkg-config")
        .args(&["--libs", "openssl"])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| String::new());

    let mut build = cc::Build::new();
    build
        .file(grpc_path.join("fd_grpc_client.c"))
        .file(grpc_path.join("fd_grpc_codec.c"))
        .file(h2_path.join("fd_h2_callback.c"))
        .file(h2_path.join("fd_h2_conn.c"))
        .file(h2_path.join("fd_h2_hdr_match.c"))
        .file(h2_path.join("fd_h2_proto.c"))
        .file(h2_path.join("fd_h2_tx.c"))
        .file(h2_path.join("fd_hpack.c"))
        .file(h2_path.join("nghttp2_hd_huffman.c"))
        .file(h2_path.join("nghttp2_hd_huffman_data.c"))
        .file(util_path.join("log").join("fd_log.c"))
        // Utility sources for missing dependencies
        .file(util_path.join("tile").join("fd_tile.c"))
        .file(util_path.join("io").join("fd_io.c"))
        .file(ballet_path.join("siphash13").join("fd_siphash13.c"))
        .include(&waltz_path)
        .include(&ballet_path)
        .include(&util_path)
        .include(&firedancer_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_LOG_STYLE", "0")
        .define("FD_HAS_OPENSSL", "1")
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC")
        .flag("-Wno-error=implicit-function-declaration");

    if !openssl_include.is_empty() {
        for flag in openssl_include.split_whitespace() {
            if flag.starts_with("-I") {
                build.include(&flag[2..]);
            }
        }
    }

    if is_x86_64 {
        build
            .define("FD_HAS_X86", "1")
            .define("FD_HAS_SSE", "1")
            .define("FD_HAS_AVX", "1");
    } else if is_aarch64 {
        build.define("FD_HAS_ARM", "1");
    }

    if is_macos {
        build.define("SIGPOLL", "SIGIO");
    }

    build.compile("fdgrpc");

    if !openssl_libs.is_empty() {
        for flag in openssl_libs.split_whitespace() {
            if flag.starts_with("-l") {
                println!("cargo:rustc-link-lib={}", &flag[2..]);
            } else if flag.starts_with("-L") {
                println!("cargo:rustc-link-search=native={}", &flag[2..]);
            }
        }
    }
}
