use std::env;
use std::path::PathBuf;

use firedancer_rs_common::{_pipeline_finalize, fd_log_stub_path, TargetInfo};

fn main() {
    let target_info = TargetInfo::new();

    let (vendor_path, util_path) =
        find_vendor().expect("Failed to find vendor directory with util subdirectory");

    let tmpl_path = util_path.join("tmpl");
    let bits_path = util_path.join("bits");

    setup_rerun(&tmpl_path, &bits_path, &util_path);

    let wrapper_path = generate_header(&tmpl_path);
    let mut bindgen = init_bindgen(&wrapper_path, &util_path, &vendor_path);
    let mut build = init_cc(&bits_path, &wrapper_path, &util_path, &vendor_path);

    spec_target(&target_info, &mut bindgen, &mut build);

    _pipeline_finalize(build, bindgen, "fdtmpl", None);
}

fn setup_rerun(tmpl_path: &PathBuf, bits_path: &PathBuf, util_path: &PathBuf) {
    let tmpl_files = [
        "fd_map.h", "fd_map.c", "fd_deque.c", "fd_heap.c", "fd_pool.c", "fd_queue.c",
        "fd_set.c", "fd_stack.c", "fd_vec.c",
    ];
    for f in &tmpl_files {
        println!("cargo:rerun-if-changed={}", tmpl_path.join(f).display());
    }
    println!("cargo:rerun-if-changed={}", bits_path.join("fd_bits.c").display());
    println!("cargo:rerun-if-changed={}", bits_path.join("fd_bits.h").display());
    println!("cargo:rerun-if-changed={}", util_path.join("fd_util_base.h").display());
}

fn generate_header(tmpl_path: &PathBuf) -> PathBuf {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("tmpl_wrapper.h");

    let tmpl_path_str = tmpl_path.canonicalize().unwrap().display().to_string();
    std::fs::write(
        &wrapper_path,
        format!(
            r#"
#include "{0}/fd_map.h"

struct fd_ulong_map_ele {{
    ulong key;
    uint hash;
    ulong value;
}};
typedef struct fd_ulong_map_ele fd_ulong_map_ele_t;

#define MAP_NAME fd_ulong_map
#define MAP_T fd_ulong_map_ele_t
#define MAP_LG_SLOT_CNT 8
#include "{0}/fd_map.c"

#define DEQUE_NAME fd_cstr_deque
#define DEQUE_T char*
#define DEQUE_MAX 64UL
#include "{0}/fd_deque.c"

struct fd_ulong_heap_ele {{
    ulong left;
    ulong right;
    ulong value;
}};
typedef struct fd_ulong_heap_ele fd_ulong_heap_ele_t;

#define HEAP_NAME fd_ulong_heap
#define HEAP_T fd_ulong_heap_ele_t
#define HEAP_LT(a,b) ((a)->value < (b)->value)
#include "{0}/fd_heap.c"

struct fd_ulong_pool_ele {{
    ulong next;
    ulong value;
}};
typedef struct fd_ulong_pool_ele fd_ulong_pool_ele_t;

#define POOL_NAME fd_ulong_pool
#define POOL_T fd_ulong_pool_ele_t
#define POOL_IDX_T uint
#define POOL_LG_SLOT_CNT 8
#include "{0}/fd_pool.c"

#define QUEUE_NAME fd_ulong_queue
#define QUEUE_T ulong
#define QUEUE_MAX 64UL
#include "{0}/fd_queue.c"

struct fd_ulong_set_ele {{
    ulong key;
    uint hash;
}};
typedef struct fd_ulong_set_ele fd_ulong_set_ele_t;

#define SET_NAME fd_ulong_set
#define SET_T fd_ulong_set_ele_t
#define SET_LG_SLOT_CNT 8
#define SET_MAX 256UL
#include "{0}/fd_set.c"

#define STACK_NAME fd_ulong_stack
#define STACK_T ulong
#define STACK_MAX 64UL
#include "{0}/fd_stack.c"

#define VEC_NAME fd_ulong_vec
#define VEC_T ulong
#include "{0}/fd_vec.c"
"#,
            tmpl_path_str
        ),
    )
    .expect("Failed to write wrapper header");

    wrapper_path
}

fn init_bindgen(
    wrapper_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
) -> bindgen::Builder {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_c_path = out_path.join("tmpl_wrapper.c");

    bindgen::Builder::default()
        .wrap_static_fns(true)
        .wrap_static_fns_path(&wrapper_c_path)
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg(format!("-I{}", vendor_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
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
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
}

fn init_cc(
    bits_path: &PathBuf,
    wrapper_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
) -> cc::Build {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_c_path = out_path.join("tmpl_wrapper.c");

    let mut build = cc::Build::new();
    build
        .file(bits_path.join("fd_bits.c"))
        .file(fd_log_stub_path())
        .file(&wrapper_c_path)
        .include(wrapper_path.parent().unwrap())
        .include(util_path)
        .include(vendor_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_LOG_STYLE", "0")
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC")
        .flag("-Wno-error=implicit-function-declaration");

    build
}

fn spec_target(
    target_info: &TargetInfo,
    bindgen: &mut bindgen::Builder,
    build: &mut cc::Build,
) {
    if target_info.is_x86_64() {
        *bindgen = std::mem::take(bindgen)
            .clang_arg("-DFD_HAS_X86=1")
            .clang_arg("-DFD_HAS_SSE=1")
            .clang_arg("-DFD_HAS_AVX=1");

        build
            .define("FD_HAS_X86", "1")
            .define("FD_HAS_SSE", "1")
            .define("FD_HAS_AVX", "1");
    } else if target_info.is_aarch64() {
        *bindgen = std::mem::take(bindgen).clang_arg("-DFD_HAS_ARM=1");
        build.define("FD_HAS_ARM", "1");
    }

    if target_info.is_macos() {
        *bindgen = std::mem::take(bindgen).clang_arg("-DSIGPOLL=SIGIO");
        build.define("SIGPOLL", "SIGIO");
    }
}

fn find_vendor() -> Result<(PathBuf, PathBuf), String> {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR")
            .map_err(|e| format!("Failed to get CARGO_MANIFEST_DIR: {}", e))?,
    );

    let mut current = manifest_dir.as_path();

    loop {
        let vendor_path = current.join("vendor");

        let util_dir = vendor_path.join("util");
        if util_dir.exists() {
            return Ok((vendor_path, util_dir));
        }

        let src_util = vendor_path.join("src").join("util");
        if src_util.exists() {
            return Ok((vendor_path, src_util));
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    Err(format!(
        "Failed to find vendor directory with util subdirectory. Started search from: {}",
        manifest_dir.display()
    ))
}
