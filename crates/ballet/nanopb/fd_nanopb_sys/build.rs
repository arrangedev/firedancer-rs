use std::env;
use std::path::PathBuf;

fn main() {
    let firedancer_path = PathBuf::from("../../../../vendor");
    let ballet_path = firedancer_path.join("ballet");
    let nanopb_path = ballet_path.join("nanopb");
    let util_path = firedancer_path.join("util");

    // Add rerun-if-changed for all relevant files
    println!(
        "cargo:rerun-if-changed={}",
        nanopb_path.join("pb_encode.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nanopb_path.join("pb_decode.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nanopb_path.join("pb_common.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nanopb_path.join("pb_firedancer.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nanopb_path.join("pb.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nanopb_path.join("pb_encode.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nanopb_path.join("pb_decode.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nanopb_path.join("pb_common.c").display()
    );

    let target = env::var("TARGET").unwrap();
    let is_x86_64 = target.contains("x86_64");
    let is_aarch64 = target.contains("aarch64") || target.contains("arm");
    let is_macos = target.contains("apple");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("nanopb_wrapper.h");

    std::fs::write(
        &wrapper_path,
        format!(
            r#"
#include "{}/pb_firedancer.h"
#include "{}/pb_encode.h"
#include "{}/pb_decode.h"
#include "{}/pb_common.h"
"#,
            nanopb_path.canonicalize().unwrap().display(),
            nanopb_path.canonicalize().unwrap().display(),
            nanopb_path.canonicalize().unwrap().display(),
            nanopb_path.canonicalize().unwrap().display(),
        ),
    )
    .expect("Failed to write wrapper header");

    let mut bindgen = bindgen::Builder::default()
        .wrap_static_fns(true)
        .wrap_static_fns_path(&wrapper_path)
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", ballet_path.display()))
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg(format!("-I{}", firedancer_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-DPB_FIELD_32BIT=1")
        .clang_arg("-DPB_ENABLE_MALLOC=1")
        .clang_arg("-DPB_BUFFER_ONLY=1")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .allowlist_function("pb_.*")
        .allowlist_type("pb_.*")
        .allowlist_var("PB_.*")
        .allowlist_var("NANOPB_.*")
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
        .file(nanopb_path.join("pb_encode.c"))
        .file(nanopb_path.join("pb_decode.c"))
        .file(nanopb_path.join("pb_common.c"))
        .include(&ballet_path)
        .include(&util_path)
        .include(&firedancer_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_LOG_STYLE", "0")
        .define("PB_FIELD_32BIT", "1")
        .define("PB_ENABLE_MALLOC", "1")
        .define("PB_BUFFER_ONLY", "1")
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

    build.compile("fdnanopb");
}
