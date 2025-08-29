use std::env;
use std::path::PathBuf;

fn main() {
    let firedancer_path = PathBuf::from("../../../vendor");
    let util_path = firedancer_path.join("util");
    let net_path = util_path.join("net");
    let bits_path = util_path.join("bits");
    let cstr_path = util_path.join("cstr");
    let _log_path = util_path.join("log");

    println!(
        "cargo:rerun-if-changed={}",
        net_path.join("fd_eth.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        net_path.join("fd_ip4.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        net_path.join("fd_pcap.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        net_path.join("fd_pcapng.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        bits_path.join("fd_bits.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        cstr_path.join("fd_cstr.c").display()
    );

    println!(
        "cargo:rerun-if-changed={}",
        net_path.join("fd_net_headers.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        net_path.join("fd_eth.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        net_path.join("fd_ip4.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        net_path.join("fd_udp.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        net_path.join("fd_pcap.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        net_path.join("fd_pcapng.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        bits_path.join("fd_bits.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        cstr_path.join("fd_cstr.h").display()
    );

    let target = env::var("TARGET").unwrap();
    let is_x86_64 = target.contains("x86_64");
    let is_aarch64 = target.contains("aarch64") || target.contains("arm");
    let is_macos = target.contains("apple");

    let mut build = cc::Build::new();
    build
        .file(net_path.join("fd_eth.c"))
        .file(net_path.join("fd_ip4.c"))
        .file(net_path.join("fd_pcap.c"))
        .file(net_path.join("fd_pcapng.c"))
        .file(bits_path.join("fd_bits.c"))
        .file(cstr_path.join("fd_cstr.c"))
        .include(&util_path)
        .define("FD_HAS_HOSTED", "1")
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC");

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

    build.compile("fdnet");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("net_wrapper.h");

    std::fs::write(
        &wrapper_path,
        format!(
            r#"
#include "{}/fd_bits.h"
#include "{}/fd_cstr.h"
#include "{}/fd_net_headers.h"
#include "{}/fd_eth.h"
#include "{}/fd_ip4.h"
#include "{}/fd_udp.h"
#include "{}/fd_pcap.h"
#include "{}/fd_pcapng.h"
#include "{}/fd_igmp.h"
#include "{}/fd_ip6.h"
#include "{}/fd_gre.h"
"#,
            bits_path.canonicalize().unwrap().display(),
            cstr_path.canonicalize().unwrap().display(),
            net_path.canonicalize().unwrap().display(),
            net_path.canonicalize().unwrap().display(),
            net_path.canonicalize().unwrap().display(),
            net_path.canonicalize().unwrap().display(),
            net_path.canonicalize().unwrap().display(),
            net_path.canonicalize().unwrap().display(),
            net_path.canonicalize().unwrap().display(),
            net_path.canonicalize().unwrap().display(),
            net_path.canonicalize().unwrap().display(),
        ),
    )
    .expect("Failed to write wrapper header");

    let mut bindgen = bindgen::Builder::default()
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-std=c17")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

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
}
