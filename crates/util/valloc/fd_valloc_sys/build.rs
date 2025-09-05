use std::env;
use std::path::PathBuf;

fn main() {
    let firedancer_path = PathBuf::from("../../../../vendor");
    let util_path = firedancer_path.join("util");
    let valloc_path = util_path.join("valloc");
    let bits_path = util_path.join("bits");
    let log_path = util_path.join("log");

    println!(
        "cargo:rerun-if-changed={}",
        valloc_path.join("fd_valloc.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        valloc_path.join("fd_valloc.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        bits_path.join("fd_bits.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        bits_path.join("fd_bits.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        log_path.join("fd_log.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        log_path.join("fd_log.h").display()
    );

    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("fd_util_base.h").display()
    );

    let target = env::var("TARGET").unwrap();
    let is_x86_64 = target.contains("x86_64");
    let is_aarch64 = target.contains("aarch64") || target.contains("arm");
    let is_macos = target.contains("apple");

    let mut build = cc::Build::new();
    build
        .file(valloc_path.join("fd_valloc.c"))
        .file(bits_path.join("fd_bits.c"))
        .file(log_path.join("fd_log.c"))
        .include(&util_path)
        .include(&firedancer_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_LOG_STYLE", "0")
        .define("_GNU_SOURCE", "1")
        .define("FD_HAS_ATOMIC", "1")
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC")
        .flag("-Wno-error=implicit-function-declaration");

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

    build.compile("fdvalloc");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("valloc_wrapper.h");

    std::fs::write(
        &wrapper_path,
        format!(
            r#"
#include "{}/fd_valloc.h"
"#,
            valloc_path.canonicalize().unwrap().display()
        ),
    )
    .expect("Failed to write wrapper header");

    let mut bindgen = bindgen::Builder::default()
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg(format!("-I{}", firedancer_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .allowlist_function("fd_valloc_.*")
        .allowlist_function("fd_libc_alloc_virtual")
        .allowlist_function("fd_null_alloc_virtual")
        .allowlist_function("fd_is_null_alloc_virtual")
        .allowlist_function("fd_backtracing_alloc_virtual")
        .allowlist_type("fd_valloc_.*")
        .allowlist_var("fd_.*_vtable")
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
