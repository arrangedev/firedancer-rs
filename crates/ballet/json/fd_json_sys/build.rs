use std::env;
use std::path::PathBuf;

fn main() {
    let firedancer_path = PathBuf::from("../../../../vendor");
    let ballet_path = firedancer_path.join("ballet");
    let json_path = ballet_path.join("json");

    println!(
        "cargo:rerun-if-changed={}",
        json_path.join("cJSON.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        json_path.join("cJSON.h").display()
    );

    let target = env::var("TARGET").unwrap();
    let is_x86_64 = target.contains("x86_64");
    let is_aarch64 = target.contains("aarch64") || target.contains("arm");
    let is_macos = target.contains("apple");

    let mut build = cc::Build::new();
    build
        .file(json_path.join("cJSON.c"))
        .include(&ballet_path)
        .include(&firedancer_path)
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

    build.compile("fdjson");

    let mut bindgen = bindgen::Builder::default()
        .header(json_path.join("cJSON.h").to_string_lossy())
        .clang_arg(format!("-I{}", ballet_path.display()))
        .clang_arg(format!("-I{}", firedancer_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-std=c17")
        .allowlist_function("cJSON_.*")
        .allowlist_type("cJSON.*")
        .allowlist_var("cJSON_.*")
        .allowlist_var("CJSON_.*")
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

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
