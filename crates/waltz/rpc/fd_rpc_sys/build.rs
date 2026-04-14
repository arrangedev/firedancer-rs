use std::env;
use std::path::PathBuf;

use firedancer_rs_common::TargetInfo;

fn main() {
    let target_info = TargetInfo::new();

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let csrc_path = manifest_dir.join("csrc");

    let (vendor_path, waltz_path) =
        find_vendor().expect("Failed to find vendor directory with waltz subdirectory");

    let http_path = waltz_path.join("http");
    let xdp_path = waltz_path.join("xdp");
    let ballet_path = vendor_path.join("ballet");
    let util_path = vendor_path.join("util");

    let xdp_enabled = cfg!(feature = "xdp") && target_info.is_linux();

    setup_rerun(&csrc_path, &http_path, &xdp_path, xdp_enabled);

    let (wrapper_h, wrapper_c) = generate_header(&csrc_path, &http_path, &xdp_path, xdp_enabled);

    let openssl_include = pkg_config_cflags("openssl");

    let bindgen = init_bindgen(
        &target_info,
        &wrapper_h,
        &wrapper_c,
        &csrc_path,
        &waltz_path,
        &ballet_path,
        &util_path,
        &vendor_path,
        &openssl_include,
        xdp_enabled,
    );

    let build = init_cc(
        &target_info,
        &csrc_path,
        &http_path,
        &xdp_path,
        &waltz_path,
        &ballet_path,
        &util_path,
        &vendor_path,
        &wrapper_c,
        &openssl_include,
        xdp_enabled,
    );

    let bindings = bindgen.generate().expect("Unable to generate bindings");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    build.compile("fdrpc");

    let openssl_libs = pkg_config_libs("openssl");
    for flag in openssl_libs.split_whitespace() {
        if flag.starts_with("-l") {
            println!("cargo:rustc-link-lib={}", &flag[2..]);
        } else if flag.starts_with("-L") {
            println!("cargo:rustc-link-search=native={}", &flag[2..]);
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
        if waltz_dir.exists() && vendor_path.join("util").exists() {
            eprintln!("Found waltz at: {}", waltz_dir.display());
            return Ok((vendor_path, waltz_dir));
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

fn setup_rerun(csrc_path: &PathBuf, http_path: &PathBuf, xdp_path: &PathBuf, xdp_enabled: bool) {
    let rerun = |p: PathBuf| println!("cargo:rerun-if-changed={}", p.display());

    rerun(csrc_path.join("fd_rpc_io.h"));
    rerun(csrc_path.join("fd_rpc_io.c"));
    rerun(http_path.join("picohttpparser.h"));
    rerun(http_path.join("picohttpparser.c"));
    rerun(http_path.join("fd_url.h"));
    rerun(http_path.join("fd_url.c"));

    if xdp_enabled {
        rerun(xdp_path.join("fd_xsk.h"));
        rerun(xdp_path.join("fd_xsk.c"));
        rerun(xdp_path.join("fd_xdp1.h"));
        rerun(xdp_path.join("fd_xdp1.c"));
        rerun(xdp_path.join("fd_xdp_redirect_user.h"));
        rerun(xdp_path.join("fd_xdp_redirect_user.c"));
    }
}

fn generate_header(
    csrc_path: &PathBuf,
    http_path: &PathBuf,
    xdp_path: &PathBuf,
    xdp_enabled: bool,
) -> (PathBuf, PathBuf) {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_h = out_path.join("rpc_wrapper.h");
    let wrapper_c = out_path.join("rpc_wrapper.c");

    let mut content = format!(
        r#"#define FD_HAS_OPENSSL 1
#include "{csrc}/fd_rpc_io.h"
#include "{http}/picohttpparser.h"
#include "{http}/fd_url.h"
"#,
        csrc = csrc_path.canonicalize().unwrap().display(),
        http = http_path.canonicalize().unwrap().display(),
    );

    if xdp_enabled {
        content.push_str(&format!(
            r#"#include "{xdp}/fd_xsk.h"
#include "{xdp}/fd_xdp1.h"
#include "{xdp}/fd_xdp_redirect_user.h"
"#,
            xdp = xdp_path.canonicalize().unwrap().display(),
        ));
    }

    std::fs::write(&wrapper_h, content).expect("Failed to write wrapper header");

    (wrapper_h, wrapper_c)
}

fn init_bindgen(
    target_info: &TargetInfo,
    wrapper_h: &PathBuf,
    wrapper_c: &PathBuf,
    csrc_path: &PathBuf,
    waltz_path: &PathBuf,
    ballet_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
    openssl_include: &str,
    xdp_enabled: bool,
) -> bindgen::Builder {
    let mut builder = bindgen::Builder::default()
        .wrap_static_fns(true)
        .wrap_static_fns_path(wrapper_c)
        .header(wrapper_h.to_string_lossy())
        .clang_arg(format!("-I{}", csrc_path.display()))
        .clang_arg(format!("-I{}", waltz_path.display()))
        .clang_arg(format!("-I{}", ballet_path.display()))
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg(format!("-I{}", vendor_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_HAS_OPENSSL=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-D_GNU_SOURCE")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .allowlist_function("fd_rpc_io_.*")
        .allowlist_type("fd_rpc_io_t")
        .allowlist_var("FD_RPC_IO_.*")
        .allowlist_function("fd_h2_rbuf_.*")
        .allowlist_type("fd_h2_rbuf_t")
        .allowlist_function("phr_.*")
        .allowlist_type("phr_.*")
        .allowlist_function("fd_url_.*")
        .allowlist_type("fd_url_t")
        .allowlist_var("FD_URL_.*")
        .blocklist_type("fd_h2_rbuf")
        .blocklist_type("fd_h2_rbuf_t")
        .blocklist_type("fd_rpc_io")
        .blocklist_type("fd_rpc_io_t")
        .blocklist_type("uchar")
        .blocklist_type("ulong")
        .blocklist_type("ushort")
        .blocklist_type("uint")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    if xdp_enabled {
        builder = builder
            .allowlist_function("fd_xsk_.*")
            .allowlist_type("fd_xsk_t")
            .allowlist_type("fd_xsk_params_t")
            .allowlist_type("fd_xdp_ring_t")
            .allowlist_var("FD_XSK_.*")
            .allowlist_function("fd_xdp_install")
            .allowlist_function("fd_xdp_gen_program")
            .allowlist_type("fd_xdp_fds_t");
    }

    for flag in openssl_include.split_whitespace() {
        if flag.starts_with("-I") {
            builder = builder.clang_arg(flag);
        }
    }

    if target_info.is_x86_64() {
        builder = builder
            .clang_arg("-DFD_HAS_X86=1")
            .clang_arg("-DFD_HAS_SSE=1")
            .clang_arg("-DFD_HAS_AVX=1");
    } else if target_info.is_aarch64() {
        builder = builder.clang_arg("-DFD_HAS_ARM=1");
    }

    if target_info.is_macos() {
        builder = builder.clang_arg("-DSIGPOLL=SIGIO");
    }

    builder
}

fn init_cc(
    target_info: &TargetInfo,
    csrc_path: &PathBuf,
    http_path: &PathBuf,
    xdp_path: &PathBuf,
    waltz_path: &PathBuf,
    ballet_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
    wrapper_c: &PathBuf,
    openssl_include: &str,
    xdp_enabled: bool,
) -> cc::Build {
    let mut build = cc::Build::new();
    build
        .file(csrc_path.join("fd_rpc_io.c"))
        .file(http_path.join("picohttpparser.c"))
        .file(http_path.join("fd_url.c"))
        .file(util_path.join("log").join("fd_log.c"))
        .file(util_path.join("tile").join("fd_tile.c"))
        .file(util_path.join("io").join("fd_io.c"))
        .file(ballet_path.join("siphash13").join("fd_siphash13.c"))
        .include(csrc_path)
        .include(waltz_path)
        .include(ballet_path)
        .include(util_path)
        .include(vendor_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_HAS_OPENSSL", "1")
        .define("FD_LOG_STYLE", "0")
        .define("_GNU_SOURCE", "1")
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC")
        .flag("-Wno-error=implicit-function-declaration");

    if xdp_enabled {
        build
            .file(xdp_path.join("fd_xsk.c"))
            .file(xdp_path.join("fd_xdp1.c"))
            .file(xdp_path.join("fd_xdp_redirect_user.c"));
    }

    build.file(wrapper_c).include(wrapper_c.parent().unwrap());

    for flag in openssl_include.split_whitespace() {
        if flag.starts_with("-I") {
            build.include(&flag[2..]);
        }
    }

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

    build
}

fn pkg_config_cflags(lib: &str) -> String {
    std::process::Command::new("pkg-config")
        .args(["--cflags", lib])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn pkg_config_libs(lib: &str) -> String {
    std::process::Command::new("pkg-config")
        .args(["--libs", lib])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}
