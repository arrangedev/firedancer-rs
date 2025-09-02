use std::env;
use std::path::PathBuf;

fn main() {
    let firedancer_path = PathBuf::from("../../../../vendor");
    let util_path = firedancer_path.join("util");
    let tmpl_path = util_path.join("tmpl");
    let bits_path = util_path.join("bits");
    let log_path = util_path.join("log");

    // Add rerun-if-changed for all relevant files
    println!(
        "cargo:rerun-if-changed={}",
        tmpl_path.join("fd_map.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tmpl_path.join("fd_map.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tmpl_path.join("fd_deque.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tmpl_path.join("fd_heap.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tmpl_path.join("fd_pool.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tmpl_path.join("fd_queue.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tmpl_path.join("fd_set.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tmpl_path.join("fd_stack.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tmpl_path.join("fd_vec.c").display()
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

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("tmpl_wrapper.h");
    let wrapper_c_path = out_path.join("wrapper.c");

    // Create a wrapper header that includes the main tmpl header
    let tmpl_path_str = tmpl_path.canonicalize().unwrap().display().to_string();
    std::fs::write(
        &wrapper_path,
        format!(
            r#"
#include "{}/fd_map.h"

// Define some basic template instantiations for common types
// These will be used by the bindings

// Basic ulong -> ulong map
struct fd_ulong_map_ele {{
    ulong key;
    uint hash;
    ulong value;
}};
typedef struct fd_ulong_map_ele fd_ulong_map_ele_t;

#define MAP_NAME fd_ulong_map
#define MAP_T fd_ulong_map_ele_t
#define MAP_LG_SLOT_CNT 8
#include "{}/fd_map.c"

// Basic string deque
#define DEQUE_NAME fd_cstr_deque
#define DEQUE_T char*
#define DEQUE_MAX 64UL
#include "{}/fd_deque.c"

// Basic ulong heap
struct fd_ulong_heap_ele {{
    ulong left;
    ulong right;
    ulong value;
}};
typedef struct fd_ulong_heap_ele fd_ulong_heap_ele_t;

#define HEAP_NAME fd_ulong_heap
#define HEAP_T fd_ulong_heap_ele_t
#define HEAP_LT(a,b) ((a)->value < (b)->value)
#include "{}/fd_heap.c"

// Basic ulong pool
struct fd_ulong_pool_ele {{
    ulong next;
    ulong value;
}};
typedef struct fd_ulong_pool_ele fd_ulong_pool_ele_t;

#define POOL_NAME fd_ulong_pool
#define POOL_T fd_ulong_pool_ele_t
#define POOL_IDX_T uint
#define POOL_LG_SLOT_CNT 8
#include "{}/fd_pool.c"

// Basic ulong queue
#define QUEUE_NAME fd_ulong_queue
#define QUEUE_T ulong
#define QUEUE_MAX 64UL
#include "{}/fd_queue.c"

// Basic ulong set
struct fd_ulong_set_ele {{
    ulong key;
    uint hash;
}};
typedef struct fd_ulong_set_ele fd_ulong_set_ele_t;

#define SET_NAME fd_ulong_set
#define SET_T fd_ulong_set_ele_t
#define SET_LG_SLOT_CNT 8
#define SET_MAX 256UL
#include "{}/fd_set.c"

// Basic ulong stack
#define STACK_NAME fd_ulong_stack
#define STACK_T ulong
#define STACK_MAX 64UL
#include "{}/fd_stack.c"

// Basic ulong vector
#define VEC_NAME fd_ulong_vec
#define VEC_T ulong
#include "{}/fd_vec.c"
"#,
            tmpl_path_str,
            tmpl_path_str,
            tmpl_path_str,
            tmpl_path_str,
            tmpl_path_str,
            tmpl_path_str,
            tmpl_path_str,
            tmpl_path_str,
            tmpl_path_str
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
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .wrap_static_fns_path(&wrapper_c_path)
        .allowlist_function("fd_.*_map_.*")
        .allowlist_function("fd_.*_deque_.*")
        .allowlist_function("fd_.*_heap_.*")
        .allowlist_function("fd_.*_pool_.*")
        .allowlist_function("fd_.*_queue_.*")
        .allowlist_function("fd_.*_set_.*")
        .allowlist_function("fd_.*_stack_.*")
        .allowlist_function("fd_.*_vec_.*")
        .allowlist_type("fd_.*_map_.*")
        .allowlist_type("fd_.*_deque_.*")
        .allowlist_type("fd_.*_heap_.*")
        .allowlist_type("fd_.*_pool_.*")
        .allowlist_type("fd_.*_queue_.*")
        .allowlist_type("fd_.*_set_.*")
        .allowlist_type("fd_.*_stack_.*")
        .allowlist_type("fd_.*_vec_.*")
        .allowlist_var("FD_MAP_.*")
        .allowlist_var(".*_MAX")
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

    // Build C files
    let mut build = cc::Build::new();
    build
        .file(bits_path.join("fd_bits.c"))
        .file(log_path.join("fd_log.c"))
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

    build.compile("fdtmpl");

    // Build the wrapper C file if it exists (generated by bindgen)
    if wrapper_c_path.exists() {
        let mut wrapper_build = cc::Build::new();
        wrapper_build
            .file(&wrapper_c_path)
            .include(&util_path)
            .include(&firedancer_path)
            .define("FD_HAS_HOSTED", "1")
            .define("FD_LOG_STYLE", "0")
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

        wrapper_build.compile("fdtmplwrapper");
    }
}
