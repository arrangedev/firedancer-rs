use std::env;
use std::path::PathBuf;

use firedancer_rs_common::{TargetInfo, _pipeline_finalize};

fn main() {
    let target_info = TargetInfo::new();

    let (vendor_path, funk_path) =
        find_vendor().expect("Failed to find vendor directory with funk module");
    let util_path = vendor_path.join("util");

    setup_rerun(&funk_path, &util_path);

    let wrapper_path = generate_header(&funk_path);
    let mut bindgen = init_bindgen(&wrapper_path, &funk_path, &util_path, &vendor_path);
    let mut build = init_cc(&funk_path, &util_path, &vendor_path);

    spec_target(&target_info, &mut bindgen, &mut build, &util_path);

    _pipeline_finalize(build, bindgen, "fdfunk", None);
}

fn find_vendor() -> Result<(PathBuf, PathBuf), String> {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR")
            .map_err(|e| format!("Failed to get CARGO_MANIFEST_DIR: {}", e))?,
    );

    let mut current = manifest_dir.as_path();

    loop {
        let vendor_path = current.join("vendor");
        let funk_dir = vendor_path.join("funk");
        if funk_dir.exists() && funk_dir.join("fd_funk.h").exists() {
            eprintln!("Found funk at: {}", funk_dir.display());
            return Ok((vendor_path, funk_dir));
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    Err(format!(
        "Failed to find vendor directory with funk module. Started search from: {}",
        manifest_dir.display()
    ))
}

fn setup_rerun(funk_path: &PathBuf, util_path: &PathBuf) {
    println!(
        "cargo:rerun-if-changed={}",
        funk_path.join("fd_funk.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        funk_path.join("fd_funk.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        funk_path.join("fd_funk_base.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        funk_path.join("fd_funk_base.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        funk_path.join("fd_funk_txn.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        funk_path.join("fd_funk_txn.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        funk_path.join("fd_funk_rec.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        funk_path.join("fd_funk_rec.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        funk_path.join("fd_funk_val.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        funk_path.join("fd_funk_val.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("fd_util_base.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("fd_util.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("fd_util.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("valloc").join("fd_valloc.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("valloc").join("fd_valloc.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("alloc").join("fd_alloc.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("alloc").join("fd_alloc.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("wksp").join("fd_wksp.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("wksp").join("fd_wksp_admin.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("wksp").join("fd_wksp_user.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("wksp").join("fd_wksp_helper.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path
            .join("wksp")
            .join("fd_wksp_free_treap.c")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path
            .join("wksp")
            .join("fd_wksp_used_treap.c")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("wksp").join("fd_wksp_io.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("shmem").join("fd_shmem.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("shmem").join("fd_shmem_user.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("bits").join("fd_bits.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("bits").join("fd_bits.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("rng").join("fd_rng.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("rng").join("fd_rng.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("log").join("fd_log.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("log").join("fd_log.c").display()
    );
}

fn generate_header(funk_path: &PathBuf) -> PathBuf {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("funk_wrapper.h");

    let header_content = format!(
        r#"
#include "{}/fd_funk.h"
"#,
        funk_path.canonicalize().unwrap().display()
    );

    std::fs::write(&wrapper_path, header_content).expect("Failed to write wrapper header");
    wrapper_path
}

fn write_stub(stub_name: &str) {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let stub_path = out_path.join(format!("{}.c", stub_name));
    std::fs::write(&stub_path, FD_LOG_STUB).expect("Failed to write stub file");
}

fn init_bindgen(
    wrapper_path: &PathBuf,
    funk_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
) -> bindgen::Builder {
    bindgen::Builder::default()
        .wrap_static_fns(true)
        .wrap_static_fns_path(wrapper_path)
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", funk_path.display()))
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg(format!("-I{}", vendor_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .wrap_unsafe_ops(true)
        .allowlist_function("fd_funk_.*")
        .allowlist_type("fd_funk_.*")
        .allowlist_var("FD_FUNK_.*")
        .allowlist_var("FD_.*")
        .allowlist_recursively(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
}

fn init_cc(funk_path: &PathBuf, util_path: &PathBuf, vendor_path: &PathBuf) -> cc::Build {
    let mut build = cc::Build::new();
    build
        .file(funk_path.join("fd_funk.c"))
        .file(funk_path.join("fd_funk_base.c"))
        .file(funk_path.join("fd_funk_txn.c"))
        .file(funk_path.join("fd_funk_rec.c"))
        .file(funk_path.join("fd_funk_val.c"))
        .file(util_path.join("fd_util.c"))
        .file(util_path.join("valloc").join("fd_valloc.c"))
        .file(util_path.join("alloc").join("fd_alloc.c"))
        .file(util_path.join("wksp").join("fd_wksp_admin.c"))
        .file(util_path.join("wksp").join("fd_wksp_user.c"))
        .file(util_path.join("wksp").join("fd_wksp_helper.c"))
        .file(util_path.join("wksp").join("fd_wksp_free_treap.c"))
        .file(util_path.join("wksp").join("fd_wksp_used_treap.c"))
        .file(util_path.join("wksp").join("fd_wksp_io.c"))
        .file(util_path.join("bits").join("fd_bits.c"))
        .file(util_path.join("rng").join("fd_rng.c"))
        .include(funk_path)
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
    _bindgen: &mut bindgen::Builder,
    build: &mut cc::Build,
    util_path: &PathBuf,
) {
    if target_info.is_linux() {
        _cfg_x86_64(build, util_path);
    } else if target_info.is_macos() {
        _cfg_aarch64(build, util_path);
    } else {
        _cfg_catchall(build, util_path);
    }
}

fn _cfg_x86_64(build: &mut cc::Build, util_path: &PathBuf) {
    println!("cargo:warning=enabling NUMA opt (linux target)");

    build
        .file(util_path.join("shmem").join("fd_shmem_user.c"))
        .file(util_path.join("shmem").join("fd_shmem_admin.c"))
        .file(util_path.join("shmem").join("fd_shmem_ctl.c"));

    let numa_linux_path = util_path.join("shmem").join("fd_numa_linux.c");
    if numa_linux_path.exists() {
        build.file(numa_linux_path);
    } else {
        println!("cargo:warning=NUMA support not found");
        let numa_stub_path = util_path.join("shmem").join("fd_numa_stub.c");
        if numa_stub_path.exists() {
            build.file(numa_stub_path);
        }
    }

    add_util_files_if_exist(
        build,
        util_path,
        &[
            "log/fd_log.c",
            "env/fd_env.c",
            "cstr/fd_cstr.c",
            "io/fd_io.c",
        ],
    );
}

fn _cfg_aarch64(build: &mut cc::Build, util_path: &PathBuf) {
    println!("cargo:warning=no NUMA opt (non-linux target)");
    let numa_stub_path = util_path.join("shmem").join("fd_numa_stub.c");
    if numa_stub_path.exists() {
        build.file(numa_stub_path);
    }

    let stub_name = "log_stub";
    write_stub(stub_name);
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let log_stub_path = out_path.join(format!("{}.c", stub_name));
    build.file(log_stub_path);

    add_util_files_if_exist(
        build,
        util_path,
        &["env/fd_env.c", "cstr/fd_cstr.c", "io/fd_io.c"],
    );

    build.define("SIGPOLL", "SIGIO");
}

fn _cfg_catchall(build: &mut cc::Build, util_path: &PathBuf) {
    println!("cargo:warning=minimal feature set (catchall)");

    let numa_stub_path = util_path.join("shmem").join("fd_numa_stub.c");
    if numa_stub_path.exists() {
        build.file(numa_stub_path);
    }

    add_util_files_if_exist(build, util_path, &["env/fd_env.c", "cstr/fd_cstr.c"]);
}

fn add_util_files_if_exist(build: &mut cc::Build, util_path: &PathBuf, files: &[&str]) {
    for file in files {
        let file_path = util_path.join(file);
        if file_path.exists() {
            println!("cargo:warning={}", file_path.display());
            build.file(file_path);
        } else {
            println!("cargo:warning=skipping {}", file_path.display());
        }
    }
}

const FD_LOG_STUB: &str = r#"#include "fd_util_base.h"
#include "fd_funk.h"
#include <time.h>
#include <unistd.h>
#include <pthread.h>

ulong
fd_log_thread_id( void ) {
    return (ulong)pthread_self();
}

ulong
fd_log_cpu_id( void ) {
    return 0UL;
}

long
fd_log_wallclock( void ) {
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    return (long)(ts.tv_sec * 1000000000L + ts.tv_nsec);
}

ulong
fd_log_group_id( void ) {
    return 0UL;
}

ulong
fd_tile_idx( void ) {
    return 0UL;
}

char const *
fd_log_private_0( char const * fmt, ... ) __attribute__((format(printf,1,2))) {
    static char buf[1024];
    (void)fmt;
    return buf;
}

void
fd_log_private_1( int level, long now, char const * file, int line, char const * func, char const * msg ) {
    (void)level; (void)now; (void)file; (void)line; (void)func; (void)msg;
}

void
fd_log_private_2( int level, long now, char const * file, int line, char const * func, char const * msg ) {
    (void)level; (void)now; (void)file; (void)line; (void)func; (void)msg;
}

int
FD_ATOMIC_CAS( volatile void * ptr, void * expected, void * desired ) {
    (void)ptr; (void)expected; (void)desired;
    return 1; 
}

int
fd_shmem_join_query_by_addr( void const * addr, ulong sz, fd_shmem_join_info_t * opt_info ) {
    (void)addr; (void)sz; (void)opt_info;
    return -1; /* Not found */
}


fd_wksp_t *
fd_funk_wksp__extern( fd_funk_t const * funk ) {
    return fd_funk_wksp( funk );
}

fd_alloc_t *
fd_funk_alloc__extern( fd_funk_t * funk ) {
    return fd_funk_alloc( funk );
}

int
fd_funk_txn_is_full__extern( fd_funk_t * funk ) {
    return fd_funk_txn_is_full( funk );
}

int
fd_funk_txn_is_frozen__extern( fd_funk_txn_t const * txn ) {
    return fd_funk_txn_is_frozen( txn );
}

ulong
fd_funk_val_sz__extern( fd_funk_rec_t const * rec ) {
    return fd_funk_val_sz( rec );
}

void const *
fd_funk_val_const__extern( fd_funk_rec_t const * rec, fd_wksp_t const * wksp ) {
    return fd_funk_val_const( rec, wksp );
}"#;
