use std::env;
use std::path::PathBuf;

fn main() {
    let firedancer_path = PathBuf::from("../../../../vendor");
    let util_path = firedancer_path.join("util");
    let shmem_path = util_path.join("shmem");
    let log_path = util_path.join("log");
    let tile_path = util_path.join("tile");
    let io_path = util_path.join("io");
    let cstr_path = util_path.join("cstr");

    println!(
        "cargo:rerun-if-changed={}",
        shmem_path.join("fd_shmem_user.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        shmem_path.join("fd_shmem_admin.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        shmem_path.join("fd_shmem.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        log_path.join("fd_log.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tile_path.join("fd_tile.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        io_path.join("fd_io.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        cstr_path.join("fd_cstr.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("fd_hash.c").display()
    );

    let target = env::var("TARGET").unwrap();
    let is_x86_64 = target.contains("x86_64");
    let is_aarch64 = target.contains("aarch64") || target.contains("arm");
    let is_macos = target.contains("apple");
    let is_linux = target.contains("linux");

    let mut build = cc::Build::new();
    build
        .file(shmem_path.join("fd_shmem_user.c"))
        .file(shmem_path.join("fd_shmem_admin.c"))
        .file(log_path.join("fd_log.c"))
        .file(tile_path.join("fd_tile.c"))
        .file(io_path.join("fd_io.c"))
        .file(cstr_path.join("fd_cstr.c"))
        .file(util_path.join("fd_hash.c"))
        .include(&util_path)
        .define("FD_HAS_HOSTED", "1")
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC");

    if is_linux {
        build.file(shmem_path.join("fd_numa_linux.c"));
        println!("cargo:rustc-link-lib=numa");
    } else {
        build.file(shmem_path.join("fd_numa_stub.c"));
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
        // no getrandom or MADV_DONTDUMP for macos
        build.define("_GNU_SOURCE", "1");
        build.define("MADV_DONTDUMP", "0"); // no-op on macOS
                                            // fallback for macOS using stdlib.h for arc4random_buf
        build.flag("-include").flag("stdlib.h");
        build.flag("-Dgetrandom(buf,len,flags)=(arc4random_buf(buf,len),(len))");
        // macos stubs for linux-specific stuff
        build.include(".");
    } else if is_linux {
        build.define("_GNU_SOURCE", "1");
    }

    build.compile("fdshmem");

    let mut bindgen = bindgen::Builder::default()
        .header(shmem_path.join("fd_shmem.h").to_string_lossy())
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

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
