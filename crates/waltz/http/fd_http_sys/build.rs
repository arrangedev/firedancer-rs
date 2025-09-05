use std::env;
use std::path::PathBuf;

fn main() {
    let firedancer_path = PathBuf::from("../../../../vendor");
    let waltz_path = firedancer_path.join("waltz");
    let http_path = waltz_path.join("http");
    let ballet_path = firedancer_path.join("ballet");
    let util_path = firedancer_path.join("util");

    // Add rerun-if-changed for all relevant files
    println!(
        "cargo:rerun-if-changed={}",
        http_path.join("fd_http_server.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        http_path.join("fd_http_server.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        http_path.join("fd_url.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        http_path.join("fd_url.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        http_path.join("picohttpparser.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        http_path.join("picohttpparser.c").display()
    );

    let target = env::var("TARGET").unwrap();
    let is_x86_64 = target.contains("x86_64");
    let is_aarch64 = target.contains("aarch64") || target.contains("arm");
    let is_macos = target.contains("apple");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("http_wrapper.h");
    let compat_path = out_path.join("macos_compat.c");

    let wrapper_content = if is_macos {
        format!(
            r#"
// macOS compatibility layer
#define _GNU_SOURCE 1

#ifndef SOCK_NONBLOCK
#define SOCK_NONBLOCK 0x40000000
#endif

#ifndef SOCK_CLOEXEC
#define SOCK_CLOEXEC 0x80000000
#endif

#ifndef ENONET
#define ENONET 64
#endif

// Forward declaration for accept4
struct sockaddr;
typedef unsigned int socklen_t;
int accept4(int sockfd, struct sockaddr *addr, socklen_t *addrlen, int flags);

#include "{}/fd_http_server.h"
#include "{}/fd_url.h"
#include "{}/picohttpparser.h"
"#,
            http_path.canonicalize().unwrap().display(),
            http_path.canonicalize().unwrap().display(),
            http_path.canonicalize().unwrap().display(),
        )
    } else {
        format!(
            r#"
#include "{}/fd_http_server.h"
#include "{}/fd_url.h"
#include "{}/picohttpparser.h"
"#,
            http_path.canonicalize().unwrap().display(),
            http_path.canonicalize().unwrap().display(),
            http_path.canonicalize().unwrap().display(),
        )
    };

    std::fs::write(&wrapper_path, wrapper_content).expect("Failed to write wrapper header");

    // Create macOS compatibility layer if needed
    if is_macos {
        std::fs::write(
            &compat_path,
            r#"
#define _GNU_SOURCE
#include <sys/socket.h>
#include <fcntl.h>
#include <errno.h>
#include <unistd.h>

#ifndef SOCK_NONBLOCK
#define SOCK_NONBLOCK 0x40000000
#endif

#ifndef SOCK_CLOEXEC
#define SOCK_CLOEXEC 0x80000000
#endif

#ifndef ENONET
#define ENONET 64
#endif

// Compatibility wrapper for accept4 on macOS
int accept4(int sockfd, struct sockaddr *addr, socklen_t *addrlen, int flags) {
    int fd = accept(sockfd, addr, addrlen);
    if (fd == -1) return -1;
    
    if (flags & SOCK_NONBLOCK) {
        int fcntl_flags = fcntl(fd, F_GETFL, 0);
        if (fcntl_flags == -1 || fcntl(fd, F_SETFL, fcntl_flags | O_NONBLOCK) == -1) {
            close(fd);
            return -1;
        }
    }
    
    if (flags & SOCK_CLOEXEC) {
        int fcntl_flags = fcntl(fd, F_GETFD, 0);
        if (fcntl_flags == -1 || fcntl(fd, F_SETFD, fcntl_flags | FD_CLOEXEC) == -1) {
            close(fd);
            return -1;
        }
    }
    
    return fd;
}
"#,
        )
        .expect("Failed to write macOS compatibility file");
    }

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
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .allowlist_function("fd_http_.*")
        .allowlist_type("fd_http_.*")
        .allowlist_var("FD_HTTP_.*")
        .allowlist_function("fd_url_.*")
        .allowlist_type("fd_url_.*")
        .allowlist_var("FD_URL_.*")
        .allowlist_function("phr_.*")
        .allowlist_type("phr_.*")
        .allowlist_var("PH_.*")
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
        // HTTP server sources
        .file(http_path.join("fd_http_server.c"))
        .file(http_path.join("fd_url.c"))
        .file(http_path.join("picohttpparser.c"))
        // Missing dependencies - utility sources
        .file(util_path.join("tile").join("fd_tile.c"))
        .file(util_path.join("io").join("fd_io.c"))
        .file(util_path.join("log").join("fd_log.c"))
        .file(util_path.join("fd_util.c"))  // Contains fd_syscall_poll
        .file(util_path.join("cstr").join("fd_cstr.c"))  // Contains fd_cstr_printf_check
        // Missing dependencies - ballet sources
        .file(ballet_path.join("base64").join("fd_base64.c"))  // Contains fd_base64_encode
        .file(ballet_path.join("sha1").join("fd_sha1.c"))  // Contains fd_sha1_hash
        .file(ballet_path.join("siphash13").join("fd_siphash13.c"))
        .include(&waltz_path)
        .include(&ballet_path)
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
        build
            .define("FD_HAS_X86", "1")
            .define("FD_HAS_SSE", "1")
            .define("FD_HAS_AVX", "1");
    } else if is_aarch64 {
        build.define("FD_HAS_ARM", "1");
    }

    if is_macos {
        build
            .define("SIGPOLL", "SIGIO")
            .define("_GNU_SOURCE", "1")
            .define("SOCK_NONBLOCK", "0x40000000")
            .define("SOCK_CLOEXEC", "0x80000000")
            .define("ENONET", "64")
            .file(&compat_path);
    }

    build.compile("fdhttp");
}
