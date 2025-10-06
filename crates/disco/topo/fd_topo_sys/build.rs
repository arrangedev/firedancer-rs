use std::env;
use std::path::PathBuf;

use firedancer_rs_common::{TargetInfo, _pipeline_finalize};

fn main() {
    let target_info = TargetInfo::new();

    let (vendor_path, disco_path) =
        find_vendor().expect("Failed to find vendor directory with submodules");

    let topo_path = disco_path.join("topo");
    let stem_path = disco_path.join("stem");
    let tango_path = vendor_path.join("tango");
    let waltz_path = vendor_path.join("waltz");
    let ballet_path = vendor_path.join("ballet");
    let disco_path = vendor_path.join("disco");
    let util_path = vendor_path.join("util");

    setup_rerun(
        &topo_path,
        &stem_path,
        &tango_path,
        &waltz_path,
        &ballet_path,
        &util_path,
    );

    let wrapper_path = generate_header(
        &topo_path,
        &stem_path,
        &tango_path,
        &waltz_path,
        &ballet_path,
        &util_path,
    );
    let mut bindgen = init_bindgen(
        &wrapper_path,
        &disco_path,
        &tango_path,
        &waltz_path,
        &ballet_path,
        &util_path,
        &vendor_path,
    );
    let mut build = init_cc(
        &target_info,
        &topo_path,
        &stem_path,
        &disco_path,
        &tango_path,
        &waltz_path,
        &ballet_path,
        &util_path,
        &vendor_path,
    );

    #[cfg(target_os = "linux")]
    {
        let wrapper_path = generate_wrappers(&disco_path);
        build.file(wrapper_path);
    }

    spec_target(&target_info, &mut bindgen, &mut build, &topo_path);

    _pipeline_finalize(build, bindgen, "fdtopo", None);
}

fn setup_rerun(
    topo_path: &PathBuf,
    stem_path: &PathBuf,
    tango_path: &PathBuf,
    waltz_path: &PathBuf,
    ballet_path: &PathBuf,
    util_path: &PathBuf,
) {
    println!(
        "cargo:rerun-if-changed={}",
        topo_path.join("fd_topo.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        topo_path.join("fd_topo.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        topo_path.join("fd_topob.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        topo_path.join("fd_topob.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        topo_path.join("fd_cpu_topo.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        topo_path.join("fd_cpu_topo.c").display()
    );

    #[cfg(target_os = "linux")]
    {
        println!(
            "cargo:rerun-if-changed={}",
            topo_path.join("fd_topo_run.c").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            stem_path.join("fd_stem.h").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            stem_path.join("fd_stem.c").display()
        );
    }

    println!(
        "cargo:rerun-if-changed={}",
        tango_path.join("fd_tango.h").display()
    );

    println!(
        "cargo:rerun-if-changed={}",
        waltz_path.join("xdp").join("fd_xdp1.h").display()
    );

    println!(
        "cargo:rerun-if-changed={}",
        ballet_path.join("base58").join("fd_base58.h").display()
    );

    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("fd_util_base.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        util_path.join("net").join("fd_net_headers.h").display()
    );
}

fn generate_header(
    topo_path: &PathBuf,
    stem_path: &PathBuf,
    tango_path: &PathBuf,
    waltz_path: &PathBuf,
    ballet_path: &PathBuf,
    util_path: &PathBuf,
) -> PathBuf {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("topo_wrapper.h");

    let mut header_content = format!(
        r#"
#include "{}/fd_topo.h"
#include "{}/fd_topob.h"
#include "{}/fd_cpu_topo.h"
#include "{}/fd_util.h"
"#,
        topo_path.canonicalize().unwrap().display(),
        topo_path.canonicalize().unwrap().display(),
        topo_path.canonicalize().unwrap().display(),
        util_path.canonicalize().unwrap().display()
    );

    #[cfg(target_os = "linux")]
    {
        header_content.push_str(&format!(
            "#include \"{}/fd_stem.h\"\n",
            stem_path.canonicalize().unwrap().display()
        ));
    }

    std::fs::write(&wrapper_path, header_content).expect("Failed to write wrapper header");

    wrapper_path
}

fn init_bindgen(
    wrapper_path: &PathBuf,
    disco_path: &PathBuf,
    tango_path: &PathBuf,
    waltz_path: &PathBuf,
    ballet_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
) -> bindgen::Builder {
    let mut builder = bindgen::Builder::default()
        .wrap_static_fns(true)
        .wrap_static_fns_path(wrapper_path)
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", disco_path.display()))
        .clang_arg(format!("-I{}", tango_path.display()))
        .clang_arg(format!("-I{}", waltz_path.display()))
        .clang_arg(format!("-I{}", ballet_path.display()))
        .clang_arg(format!("-I{}", util_path.display()))
        .clang_arg(format!("-I{}", vendor_path.display()))
        .clang_arg("-DFD_HAS_HOSTED=1")
        .clang_arg("-DFD_HAS_THREADS=1")
        .clang_arg("-DFD_LOG_STYLE=0")
        .clang_arg("-std=c17")
        .clang_arg("-Wno-error=implicit-function-declaration")
        .wrap_unsafe_ops(true)
        .allowlist_function("fd_topo_.*")
        .allowlist_function("fd_topob_.*")
        .allowlist_function("fd_cpu_topo_.*")
        .allowlist_function("fd_boot")
        .allowlist_function("fd_halt")
        .allowlist_type("fd_topo_.*")
        .allowlist_type("fd_topob_.*")
        .allowlist_type("fd_cpu_topo_.*")
        .allowlist_var("FD_TOPO_.*")
        .allowlist_var("FD_TOPOB_.*")
        .allowlist_var("FD_CPU_TOPO_.*")
        .allowlist_var("FD_.*")
        .allowlist_recursively(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    #[cfg(target_os = "linux")]
    {
        builder = builder
            .clang_arg("-DFD_HAS_LINUX=1")
            .clang_arg("-DPATH_MAX=4096")
            .clang_arg("-DFD_HAS_ALLOCA=1")
            // stub defs for stem template macros // fd_stem.h
            .clang_arg("-DSTEM_BURST=1UL")
            .clang_arg("-DSTEM_CALLBACK_CONTEXT_TYPE=void")
            .clang_arg("-DSTEM_CALLBACK_CONTEXT_ALIGN=8UL")
            .clang_arg("-include")
            .clang_arg("limits.h")
            .clang_arg("-include")
            .clang_arg("alloca.h");
    }

    #[cfg(not(target_os = "linux"))]
    {
        builder = builder.clang_arg("-DFD_HAS_LINUX=0");
    }

    builder
}

fn init_cc(
    target_info: &TargetInfo,
    topo_path: &PathBuf,
    stem_path: &PathBuf,
    disco_path: &PathBuf,
    tango_path: &PathBuf,
    waltz_path: &PathBuf,
    ballet_path: &PathBuf,
    util_path: &PathBuf,
    vendor_path: &PathBuf,
) -> cc::Build {
    let mut build = cc::Build::new();
    build
        .file(topo_path.join("fd_topo.c"))
        .file(topo_path.join("fd_topob.c"))
        .file(topo_path.join("fd_cpu_topo.c"))
        .include(vendor_path)
        .include(util_path)
        .define("FD_HAS_HOSTED", "1")
        .define("FD_HAS_THREADS", "1")
        .define("FD_LOG_STYLE", "0")
        .define("FD_HAS_ATOMIC", "1")
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC")
        .flag("-Wno-error=implicit-function-declaration");

    #[cfg(target_os = "linux")]
    {
        build
            .file(topo_path.join("fd_topo_run.c"))
            .file(util_path.join("log/fd_log.c"))
            .file(util_path.join("cstr/fd_cstr.c"))
            .file(util_path.join("io/fd_io.c"))
            .file(util_path.join("shmem/fd_shmem_user.c"))
            .file(util_path.join("shmem/fd_shmem_admin.c"))
            .file(util_path.join("shmem/fd_numa_linux.c"))
            .file(util_path.join("wksp/fd_wksp_admin.c"))
            .file(util_path.join("wksp/fd_wksp_user.c"))
            .file(util_path.join("pod/fd_pod.c"))
            .file(util_path.join("sandbox/fd_sandbox.c"))
            .file(util_path.join("fd_hash.c"))
            .file(util_path.join("fd_util.c"))
            .file(util_path.join("env/fd_env.c"))
            .file(util_path.join("tile/fd_tile.c"))
            .file(util_path.join("wksp/fd_wksp_helper.c"))
            .file(disco_path.join("metrics/fd_metrics.c"))
            .file(tango_path.join("mcache/fd_mcache.c"))
            .file(tango_path.join("dcache/fd_dcache.c"))
            .file(tango_path.join("fseq/fd_fseq.c"))
            .define("FD_HAS_LINUX", "1")
            .define("PATH_MAX", "4096")
            .define("FD_HAS_ALLOCA", "1")
            .define("FD_HAS_ATOMIC", "1")
            // stub defs for STEM template macros
            .define("STEM_BURST", "1UL")
            .define("STEM_CALLBACK_CONTEXT_TYPE", "void")
            .define("STEM_CALLBACK_CONTEXT_ALIGN", "8UL");

        let mut cpp_build = cc::Build::new();
        cpp_build
            .file(util_path.join("tile/fd_tile_threads.cxx"))
            .include(vendor_path)
            .include(util_path)
            .define("FD_HAS_HOSTED", "1")
            .define("FD_HAS_THREADS", "1")
            .define("FD_LOG_STYLE", "0")
            .define("FD_HAS_LINUX", "1")
            .define("PATH_MAX", "4096")
            .define("FD_HAS_ALLOCA", "1")
            .define("_GNU_SOURCE", "1")
            .define("FD_HAS_ATOMIC", "1")
            .define("FD_TILE_MAX", "1024");

        if target_info.is_x86_64() {
            cpp_build
                .define("FD_HAS_X86", "1")
                .define("FD_HAS_SSE", "1")
                .define("FD_HAS_AVX", "1");
        } else if target_info.is_aarch64() {
            cpp_build.define("FD_HAS_ARM", "1");
        }

        cpp_build
            .cpp(true)
            .flag("-std=c++17")
            .flag("-O3")
            .flag("-fPIC")
            .flag("-Wno-error=implicit-function-declaration")
            .compile("fdtopo_cpp");
    }

    #[cfg(not(target_os = "linux"))]
    {
        build.define("FD_HAS_LINUX", "0").file("src/stubs.c");
    }

    build
}

fn spec_target(
    target_info: &TargetInfo,
    _bindgen: &mut bindgen::Builder,
    build: &mut cc::Build,
    _topo_path: &PathBuf,
) {
    if target_info.is_x86_64() {
        cfg_x86_64(build);
    } else if target_info.is_aarch64() {
        cfg_aarch64(build);
    } else {
        cfg_catchall(build);
    }

    if target_info.is_macos() {
        cfg_arm64_mac(build);
    }
}

fn cfg_x86_64(build: &mut cc::Build) {
    build
        .define("FD_HAS_X86", "1")
        .define("FD_HAS_SSE", "1")
        .define("FD_HAS_AVX", "1")
        .define("FD_HAS_AVX512", "1")
        .flag("-msse")
        .flag("-msse2")
        .flag("-mavx")
        .flag("-mavx2");

    #[cfg(target_os = "linux")]
    {
        build
            .flag("-mavx512f")
            .flag("-mavx512bw")
            .flag("-mavx512dq")
            .flag("-mavx512vl")
            .flag("-mavx512ifma")
            .flag("-mavx512vbmi");
    }
}

fn cfg_aarch64(build: &mut cc::Build) {
    build.define("FD_HAS_ARM", "1");
}

fn cfg_catchall(_build: &mut cc::Build) {
    // ref impls
}

fn cfg_arm64_mac(build: &mut cc::Build) {
    build.define("SIGPOLL", "SIGIO");
}

fn generate_wrappers(disco_path: &PathBuf) -> PathBuf {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("topo_wrappers.c");

    let topo_header_path = disco_path.join("topo/fd_topo.h");

    let wrapper_content = format!(
        r#"#include "{}"

ulong fd_topo_find_wksp__extern(fd_topo_t const * topo, char const * name) {{
    return fd_topo_find_wksp(topo, name);
}}

ulong fd_topo_find_tile__extern(fd_topo_t const * topo, char const * name, ulong kind_id) {{
    return fd_topo_find_tile(topo, name, kind_id);
}}

ulong fd_topo_find_link__extern(fd_topo_t const * topo, char const * name, ulong kind_id) {{
    return fd_topo_find_link(topo, name, kind_id);
}}

ulong fd_topo_tile_name_cnt__extern(fd_topo_t const * topo, char const * name) {{
    return fd_topo_tile_name_cnt(topo, name);
}}
"#,
        topo_header_path.canonicalize().unwrap().display()
    );

    std::fs::write(&wrapper_path, wrapper_content).expect("Failed to write wrapper file");

    wrapper_path
}

fn find_vendor() -> Result<(PathBuf, PathBuf), String> {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR")
            .map_err(|e| format!("Failed to get CARGO_MANIFEST_DIR: {}", e))?,
    );

    let mut current = manifest_dir.as_path();

    loop {
        let vendor_path = current.join("vendor");
        let disco_dir = vendor_path.join("disco");
        if disco_dir.exists() {
            eprintln!("Found disco at: {}", disco_dir.display());
            return Ok((vendor_path, disco_dir));
        }

        let src_disco = vendor_path.join("src").join("disco");
        if src_disco.exists() {
            eprintln!("Found disco at: {}", src_disco.display());
            return Ok((vendor_path, src_disco));
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    Err(format!(
        "Failed to find vendor directory with disco subdirectory. Started search from: {}",
        manifest_dir.display()
    ))
}
