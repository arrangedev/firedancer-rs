use std::env;
use std::path::PathBuf;

fn main() {
    let firedancer_path = PathBuf::from("../../../../vendor");
    let util_path = firedancer_path.join("util");
    let wksp_path = util_path.join("wksp");
    let valloc_path = util_path.join("valloc");
    let bits_path = util_path.join("bits");
    let log_path = util_path.join("log");
    let checkpt_path = util_path.join("checkpt");

    // Add rerun-if-changed for all relevant files
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp_admin.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp_user.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp_helper.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp_io.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp_free_treap.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp_used_treap.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp_checkpt_v1.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp_checkpt_v2.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp_restore_v1.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        wksp_path.join("fd_wksp_restore_v2.c").display()
    );
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
        checkpt_path.join("fd_checkpt.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        checkpt_path.join("fd_checkpt.h").display()
    );

    // tpool files
    let tpool_path = util_path.join("tpool");
    println!(
        "cargo:rerun-if-changed={}",
        tpool_path.join("fd_tpool.cxx").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tpool_path.join("fd_tpool.h").display()
    );

    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("fd_util_base.h").display()
    );

    let target = env::var("TARGET").unwrap();
    let is_x86_64 = target.contains("x86_64");
    let is_aarch64 = target.contains("aarch64") || target.contains("arm");
    let is_macos = target.contains("apple");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("wksp_wrapper.h");
    let wrapper_c_path = out_path.join("wrapper.c");

    std::fs::write(
        &wrapper_path,
        format!(
            r#"
#include "{}/fd_wksp.h"
"#,
            wksp_path.canonicalize().unwrap().display()
        ),
    )
    .expect("Failed to write wrapper header");

    let mut bindgen = bindgen::Builder::default()
        .wrap_static_fns(true)
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg(format!("-I{}", firedancer_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-D_GNU_SOURCE")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .wrap_static_fns_path(&wrapper_c_path)
        .allowlist_function("fd_wksp_.*")
        .allowlist_type("fd_wksp_.*")
        .allowlist_type("fd_valloc_.*")
        .allowlist_type("fd_tpool_.*")
        .allowlist_type("fd_checkpt_.*")
        .allowlist_var("FD_WKSP_.*")
        .allowlist_var("fd_wksp_.*")
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

    let mut build = cc::Build::new();
    build
        .file(wksp_path.join("fd_wksp_admin.c"))
        .file(wksp_path.join("fd_wksp_user.c"))
        .file(wksp_path.join("fd_wksp_helper.c"))
        .file(wksp_path.join("fd_wksp_io.c"))
        .file(wksp_path.join("fd_wksp_free_treap.c"))
        .file(wksp_path.join("fd_wksp_used_treap.c"))
        .file(wksp_path.join("fd_wksp_checkpt_v1.c"))
        .file(wksp_path.join("fd_wksp_checkpt_v2.c"))
        .file(wksp_path.join("fd_wksp_restore_v1.c"))
        .file(wksp_path.join("fd_wksp_restore_v2.c"))
        .file(valloc_path.join("fd_valloc.c"))
        .file(bits_path.join("fd_bits.c"))
        .file(log_path.join("fd_log.c"))
        .file(checkpt_path.join("fd_checkpt.c"))
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

    // Build tpool separately as C++
    let tpool_path = util_path.join("tpool");
    let mut tpool_build = cc::Build::new();
    tpool_build
        .cpp(true) // C++ file
        .file(tpool_path.join("fd_tpool.cxx"))
        .include(&util_path)
        .include(&firedancer_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_HAS_THREADS", "1") // requires threading
        .define("FD_HAS_ALLOCA", "1")  // uses alloca
        .define("FD_LOG_STYLE", "0")
        .define("_GNU_SOURCE", "1")
        .flag("-std=c++17")
        .flag("-O3")
        .flag("-fPIC")
        .flag("-Wno-error=implicit-function-declaration");

    if is_x86_64 {
        build
            .define("FD_HAS_X86", "1")
            .define("FD_HAS_SSE", "1")
            .define("FD_HAS_AVX", "1");
        tpool_build
            .define("FD_HAS_X86", "1")
            .define("FD_HAS_SSE", "1")
            .define("FD_HAS_AVX", "1");
    } else if is_aarch64 {
        build.define("FD_HAS_ARM", "1");
        tpool_build.define("FD_HAS_ARM", "1");
    }

    if is_macos {
        build.define("SIGPOLL", "SIGIO");
        tpool_build.define("SIGPOLL", "SIGIO");
    }

    // Link pthread for tpool
    println!("cargo:rustc-link-lib=pthread");

    build.compile("fdwksp");
    tpool_build.compile("fdtpool");

    if wrapper_c_path.exists() {
        let mut wrapper_build = cc::Build::new();
        wrapper_build
            .file(&wrapper_c_path)
            .include(&util_path)
            .include(&firedancer_path)
            .define("FD_HAS_HOSTED", "1")
            .define("FD_LOG_STYLE", "0")
            .define("_GNU_SOURCE", "1")
            .flag("-std=c17")
            .flag("-O3")
            .flag("-fPIC")
            .flag("-Wno-error=implicit-function-declaration");

        if is_x86_64 {
            wrapper_build
                .define("FD_HAS_X86", "1")
                .define("FD_HAS_SSE", "1")
                .define("FD_HAS_AVX", "1");
        } else if is_aarch64 {
            wrapper_build.define("FD_HAS_ARM", "1");
        }

        if is_macos {
            wrapper_build.define("SIGPOLL", "SIGIO");
        }

        wrapper_build.compile("fdwkspwrapper");
    }
}
