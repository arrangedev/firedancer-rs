use std::env;
use std::path::PathBuf;

fn main() {
    let firedancer_path = PathBuf::from("../../../../vendor");
    let ballet_path = firedancer_path.join("ballet");
    let zstd_path = ballet_path.join("zstd");
    let util_path = firedancer_path.join("util");
    let log_path = util_path.join("log");

    println!(
        "cargo:rerun-if-changed={}",
        zstd_path.join("fd_zstd.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        zstd_path.join("fd_zstd_private.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        zstd_path.join("fd_zstd.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("fd_util_base.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        log_path.join("fd_log.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        log_path.join("fd_log.c").display()
    );

    let target = env::var("TARGET").unwrap();
    let is_x86_64 = target.contains("x86_64");
    let is_aarch64 = target.contains("aarch64") || target.contains("arm");
    let is_macos = target.contains("apple");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("zstd_wrapper.h");

    std::fs::write(
        &wrapper_path,
        format!(
            r#"
#include "{}/fd_zstd.h"
"#,
            zstd_path.canonicalize().unwrap().display(),
        ),
    )
    .expect("Failed to write wrapper header");

    let zstd_prefix = env::var("ZSTD_PREFIX").unwrap_or_else(|_| {
        if cfg!(target_os = "macos") {
            "/opt/homebrew/opt/zstd".to_string()
        } else {
            "/usr".to_string()
        }
    });

    let mut bindgen = bindgen::Builder::default()
        .wrap_static_fns(true)
        .wrap_static_fns_path(&wrapper_path)
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", ballet_path.display()))
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg(format!("-I{}", firedancer_path.display()))
        .clang_arg(format!("-I{}/include", zstd_prefix))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_HAS_ZSTD=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .use_core()
        .ctypes_prefix("libc")
        .allowlist_function("fd_zstd_.*")
        .allowlist_type("fd_zstd_.*")
        .allowlist_var("FD_ZSTD_.*")
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

    println!("cargo:rustc-link-search=native={}/lib", zstd_prefix);
    println!("cargo:rustc-link-lib=zstd");

    let stub_c_path = out_path.join("fd_log_stub.c");
    std::fs::write(
        &stub_c_path,
        r#"
#include <stdio.h>
#include <stdarg.h>
#include <stdlib.h>
#include <time.h>

char const *
fd_log_private_0( char const * fmt, ... ) {
  static char buf[1024];
  va_list ap;
  va_start(ap, fmt);
  vsnprintf(buf, sizeof(buf), fmt, ap);
  va_end(ap);
  return buf;
}

void
fd_log_private_1( int level, long now, char const * file, int line, char const * func, char const * msg ) {
  (void)now;
  if (level >= 3) {
    fprintf(stderr, "[%s:%d] %s: %s\n", file, line, func, msg);
  }
}

void
fd_log_private_2( int level, long now, char const * file, int line, char const * func, char const * msg ) {
  (void)level;
  (void)now;
  fprintf(stderr, "FATAL [%s:%d] %s: %s\n", file, line, func, msg);
  abort();
}

long
fd_log_wallclock( void ) {
  struct timespec ts;
  clock_gettime(CLOCK_REALTIME, &ts);
  return ((long)1e9)*((long)ts.tv_sec) + (long)ts.tv_nsec;
}
"#,
    )
    .expect("Failed to write stub C file");

    let mut build = cc::Build::new();
    build
        .file(zstd_path.join("fd_zstd.c"))
        .file(&stub_c_path)
        .include(&ballet_path)
        .include(&util_path)
        .include(&firedancer_path)
        .include(format!("{}/include", zstd_prefix))
        .define("FD_HAS_HOSTED", "1")
        .define("FD_HAS_ZSTD", "1")
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

    build.compile("fdzstd");
}
