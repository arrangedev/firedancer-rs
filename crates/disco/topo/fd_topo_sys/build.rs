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
        &topo_path,
        &stem_path,
        &tango_path,
        &waltz_path,
        &ballet_path,
        &util_path,
        &vendor_path,
    );

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
    // Core topo files
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

    // Only track fd_topo_run.c and stem files on Linux platforms
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

    // Tango dependencies
    println!(
        "cargo:rerun-if-changed={}",
        tango_path.join("fd_tango.h").display()
    );

    // Waltz XDP dependencies
    println!(
        "cargo:rerun-if-changed={}",
        waltz_path.join("xdp").join("fd_xdp1.h").display()
    );

    // Ballet base58 dependencies
    println!(
        "cargo:rerun-if-changed={}",
        ballet_path.join("base58").join("fd_base58.h").display()
    );

    // Util dependencies
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
"#,
        topo_path.canonicalize().unwrap().display(),
        topo_path.canonicalize().unwrap().display(),
        topo_path.canonicalize().unwrap().display()
    );

    // Only include stem on Linux platforms
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
        .allowlist_type("fd_topo_.*")
        .allowlist_type("fd_topob_.*")
        .allowlist_type("fd_cpu_topo_.*")
        .allowlist_var("FD_TOPO_.*")
        .allowlist_var("FD_TOPOB_.*")
        .allowlist_var("FD_CPU_TOPO_.*")
        .allowlist_var("FD_.*")
        .allowlist_recursively(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    // Platform-specific defines
    #[cfg(target_os = "linux")]
    {
        builder = builder.clang_arg("-DFD_HAS_LINUX=1");
    }

    #[cfg(not(target_os = "linux"))]
    {
        builder = builder.clang_arg("-DFD_HAS_LINUX=0");
    }

    builder
}

fn init_cc(
    topo_path: &PathBuf,
    stem_path: &PathBuf,
    _tango_path: &PathBuf,
    _waltz_path: &PathBuf,
    _ballet_path: &PathBuf,
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
        .flag("-std=c17")
        .flag("-O3")
        .flag("-fPIC")
        .flag("-Wno-error=implicit-function-declaration");

    // Only include Linux-specific files on Linux platforms
    #[cfg(target_os = "linux")]
    {
        build
            .file(topo_path.join("fd_topo_run.c"))
            .file(stem_path.join("fd_stem.c"))
            .define("FD_HAS_LINUX", "1");
    }

    // On non-Linux platforms, don't define FD_HAS_LINUX and exclude stem
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

    // Only add AVX512 flags if we're on Linux x86_64 where they're more likely to be supported
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
    // Use reference implementations for unsupported architectures
}

fn cfg_arm64_mac(build: &mut cc::Build) {
    build.define("SIGPOLL", "SIGIO");
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
