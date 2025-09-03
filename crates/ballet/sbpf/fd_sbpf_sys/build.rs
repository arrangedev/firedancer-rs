use std::env;
use std::path::PathBuf;

fn main() {
    let firedancer_path = PathBuf::from("../../../../vendor");
    let ballet_path = firedancer_path.join("ballet");
    let sbpf_path = ballet_path.join("sbpf");
    let elf_path = ballet_path.join("elf");
    let murmur3_path = ballet_path.join("murmur3");
    let util_path = firedancer_path.join("util");

    println!(
        "cargo:rerun-if-changed={}",
        sbpf_path.join("fd_sbpf_loader.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        sbpf_path.join("fd_sbpf_loader.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        sbpf_path.join("fd_sbpf_instr.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        sbpf_path.join("fd_sbpf_opcodes.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        elf_path.join("fd_elf64.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        murmur3_path.join("fd_murmur3.h").display()
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
    let wrapper_path = out_path.join("sbpf_wrapper.h");

    std::fs::write(
        &wrapper_path,
        format!(
            r#"
#include "{}/fd_sbpf_loader.h"
#include "{}/fd_sbpf_instr.h"
#include "{}/fd_sbpf_opcodes.h"
"#,
            sbpf_path.canonicalize().unwrap().display(),
            sbpf_path.canonicalize().unwrap().display(),
            sbpf_path.canonicalize().unwrap().display(),
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
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration");

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

    bindgen = bindgen
        .allowlist_function("fd_sbpf_.*")
        .allowlist_type("fd_sbpf_.*")
        .allowlist_var("FD_SBPF_.*")
        .allowlist_var("SET_NAME")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    let bindings = bindgen.generate().expect("Unable to generate bindings");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    let murmur3_path = ballet_path.join("murmur3");

    let mut build = cc::Build::new();
    build
        .file(sbpf_path.join("fd_sbpf_loader.c"))
        .file(murmur3_path.join("fd_murmur3.c"))
        .file(&wrapper_path.with_extension("c"))
        .include(&ballet_path)
        .include(&util_path)
        .include(&firedancer_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_LOG_STYLE", "0")
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

    build.compile("fdsbpf");
}
