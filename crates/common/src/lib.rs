use std::collections::HashMap;
use std::env;
use std::fs::{self, metadata, read_to_string};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    AArch64,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsTarget {
    Linux,
    MacOS,
    Windows,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetInfo {
    pub arch: Arch,
    pub platform: OsTarget,
    pub emulated: bool,
}

impl TargetInfo {
    pub fn new() -> Self {
        let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
        let emulated = is_emulated();

        let arch = if target.contains("x86_64") {
            Arch::X86_64
        } else if target.contains("aarch64") || target.contains("arm") {
            Arch::AArch64
        } else {
            Arch::Other
        };

        let platform = if target.contains("apple") {
            OsTarget::MacOS
        } else if target.contains("linux") {
            OsTarget::Linux
        } else if target.contains("windows") {
            OsTarget::Windows
        } else {
            OsTarget::Other
        };

        Self {
            arch,
            platform,
            emulated,
        }
    }

    #[inline]
    pub fn is_x86_64(&self) -> bool {
        matches!(self.arch, Arch::X86_64)
    }

    #[inline]
    pub fn is_aarch64(&self) -> bool {
        matches!(self.arch, Arch::AArch64)
    }

    #[inline]
    pub fn is_macos(&self) -> bool {
        matches!(self.platform, OsTarget::MacOS)
    }

    #[inline]
    pub fn is_linux(&self) -> bool {
        matches!(self.platform, OsTarget::Linux)
    }

    /// we generally don't care about windows
    #[inline]
    #[allow(dead_code)]
    pub fn is_windows(&self) -> bool {
        matches!(self.platform, OsTarget::Windows)
    }
}

impl Default for TargetInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct VendorPaths {
    pub vendor: PathBuf,
    pub ballet: PathBuf,
    pub util: PathBuf,
    pub misc: HashMap<String, PathBuf>,
}

impl VendorPaths {
    pub fn new() -> Self {
        let vendor = PathBuf::from("../../../../vendor");
        let ballet = vendor.join("ballet");
        let util = vendor.join("util");

        Self {
            vendor,
            ballet,
            util,
            misc: HashMap::new(),
        }
    }

    pub fn with_vendor<P: AsRef<Path>>(vendorpath: P) -> Self {
        let vendor = vendorpath.as_ref().to_path_buf();
        let ballet = vendor.join("ballet");
        let util = vendor.join("util");

        Self {
            vendor,
            ballet,
            util,
            misc: HashMap::new(),
        }
    }

    pub fn with_modpaths(mut self, modpaths: HashMap<String, PathBuf>) -> Self {
        self.misc = modpaths;
        self
    }

    pub fn ballet_module<P: AsRef<Path>>(&self, module: P) -> PathBuf {
        self.ballet.join(module)
    }

    pub fn util_module<P: AsRef<Path>>(&self, module: P) -> PathBuf {
        self.util.join(module)
    }

    pub fn find_vendor_directory() -> Option<PathBuf> {
        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        let possible_paths = vec![
            manifest_dir.join("../../../../vendor"),
            manifest_dir
                .parent()?
                .parent()?
                .parent()?
                .parent()?
                .join("vendor"),
            manifest_dir.join("vendor"),
        ];

        for path in possible_paths {
            if path.exists() && path.join("util").exists() {
                return Some(path);
            }
        }

        None
    }

    pub fn ensure_vendor_files(required_files: &[(&str, &str)]) -> Result<PathBuf, String> {
        if let Some(vendor_path) = Self::find_vendor_directory() {
            for (src_path, _) in required_files {
                if !vendor_path.join(src_path).exists() {
                    return Err(format!(
                        "Required file not found: {}",
                        vendor_path.join(src_path).display()
                    ));
                }
            }
            return Ok(vendor_path);
        }

        let out_dir = PathBuf::from(env::var("OUT_DIR").map_err(|_| "OUT_DIR not set")?);
        let vendor_out = out_dir.join("vendor");

        let all_files_exist = required_files
            .iter()
            .all(|(_, dst_path)| vendor_out.join(dst_path).exists());

        if all_files_exist {
            return Ok(vendor_out);
        }

        if let Some(original_vendor) = Self::find_vendor_directory() {
            for (_, dst_path) in required_files {
                let dst_full_path = vendor_out.join(dst_path);
                if let Some(parent) = dst_full_path.parent() {
                    fs::create_dir_all(parent).map_err(|e| {
                        format!("Failed to create directory {}: {}", parent.display(), e)
                    })?;
                }
            }

            for (src_path, dst_path) in required_files {
                let src_full_path = original_vendor.join(src_path);
                let dst_full_path = vendor_out.join(dst_path);

                if !src_full_path.exists() {
                    return Err(format!(
                        "Required source file not found: {}",
                        src_full_path.display()
                    ));
                }

                fs::copy(&src_full_path, &dst_full_path).map_err(|e| {
                    format!(
                        "Failed to copy {} to {}: {}",
                        src_full_path.display(),
                        dst_full_path.display(),
                        e
                    )
                })?;
            }

            return Ok(vendor_out);
        }

        Err("Cannot find vendor directory with required source files. This crate requires access to the libfiredancer vendor directory.".to_string())
    }
}

impl Default for VendorPaths {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RerunDef;

impl RerunDef {
    pub fn add_file<P: AsRef<Path>>(path: P) {
        println!("cargo:rerun-if-changed={}", path.as_ref().display());
    }

    pub fn add_files<P: AsRef<Path>, I: IntoIterator<Item = P>>(paths: I) {
        for path in paths {
            Self::add_file(path);
        }
    }

    pub fn add_dir_with_exts<P: AsRef<Path>>(
        dir: P,
        extensions: &[&str],
    ) -> Result<(), std::io::Error> {
        let dir = dir.as_ref();
        if !dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if extensions.contains(&ext) {
                        Self::add_file(&path);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn add_c_files<P: AsRef<Path>>(dir: P) -> Result<(), std::io::Error> {
        Self::add_dir_with_exts(dir, &["c", "cpp", "cxx", "cc", "h", "hpp", "hxx"])
    }

    pub fn add_files_matching<P: AsRef<Path>>(dir: P, pattern: &str) -> Result<(), std::io::Error> {
        let dir = dir.as_ref();
        if !dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                    if filename.contains(pattern) {
                        Self::add_file(&path);
                    }
                }
            }
        }
        Ok(())
    }
}

pub struct CCBuilder {
    build: cc::Build,
    target_info: TargetInfo,
}

impl CCBuilder {
    pub fn new() -> Self {
        let mut build = cc::Build::new();
        let target_info = TargetInfo::new();

        build
            .define("FD_HAS_HOSTED", "1")
            .flag("-std=c17")
            .flag("-O3")
            .flag("-fPIC");

        Self { build, target_info }
    }

    pub fn with_fd_includes(mut self, paths: &VendorPaths) -> Self {
        self.build
            .include(&paths.ballet)
            .include(&paths.util)
            .include(&paths.vendor);
        self
    }

    pub fn with_target_opts(mut self) -> Self {
        match self.target_info.arch {
            Arch::X86_64 => {
                if self.target_info.emulated {
                    self.build
                        .define("FD_HAS_X86", "0")
                        .define("FD_HAS_SSE", "0")
                        .define("FD_HAS_AVX", "0")
                        .define("FD_HAS_AVX512", "0");
                } else {
                    self.build
                        .define("FD_HAS_X86", "1")
                        .define("FD_HAS_SSE", "1")
                        .define("FD_HAS_AVX", "1")
                        .flag("-msse")
                        .flag("-msse2")
                        .flag("-mavx")
                        .flag("-mavx2");
                }
            }
            Arch::AArch64 => {
                self.build.define("FD_HAS_ARM", "1");
            }
            Arch::Other => {}
        }

        match self.target_info.platform {
            OsTarget::MacOS => {
                self.build.define("SIGPOLL", "SIGIO");
            }
            OsTarget::Linux => {}
            OsTarget::Windows => {}
            OsTarget::Other => {}
        }

        self
    }

    pub fn with_logs(mut self, style: u32) -> Self {
        self.build
            .define("FD_LOG_STYLE", style.to_string().as_str());
        self
    }

    pub fn suppress_warnings(mut self) -> Self {
        self.build.flag("-Wno-error=implicit-function-declaration");
        self
    }

    pub fn files<P: AsRef<Path>, I: IntoIterator<Item = P>>(mut self, files: I) -> Self {
        for file in files {
            self.build.file(file);
        }
        self
    }

    pub fn file<P: AsRef<Path>>(mut self, file: P) -> Self {
        self.build.file(file);
        self
    }

    pub fn with_avx512<P: AsRef<Path>>(mut self, avx512_dir: P) -> Self {
        if self.target_info.is_x86_64() && !self.target_info.emulated {
            let avx512_path = avx512_dir.as_ref();
            if avx512_path.exists() {
                self.build
                    .define("FD_HAS_AVX512", "1")
                    .flag("-mavx512f")
                    .flag("-mavx512bw")
                    .flag("-mavx512dq")
                    .flag("-mavx512vl")
                    .flag("-mavx512ifma")
                    .flag("-mavx512vbmi");

                let avx512_files = [
                    "fd_curve25519.c",
                    "fd_curve25519_secure.c",
                    "fd_f25519.c",
                    "fd_r43x6.c",
                    "fd_r43x6_ge.c",
                ];

                for file in &avx512_files {
                    let file_path = avx512_path.join(file);
                    if file_path.exists() {
                        self.build.file(file_path);
                    }
                }
            }
        }
        self
    }

    pub fn compile(self, libname: &str) {
        self.build.compile(libname);
    }

    pub fn inner_mut(&mut self) -> &mut cc::Build {
        &mut self.build
    }

    pub fn target_info(&self) -> &TargetInfo {
        &self.target_info
    }
}

impl Default for CCBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BindgenBuilder {
    builder: bindgen::Builder,
    target_info: TargetInfo,
}

impl BindgenBuilder {
    pub fn new() -> Self {
        let builder = bindgen::Builder::default();
        let target_info = TargetInfo::new();

        Self {
            builder,
            target_info,
        }
    }

    pub fn header<P: AsRef<Path>>(mut self, header: P) -> Self {
        self.builder = self.builder.header(header.as_ref().to_string_lossy());
        self
    }

    pub fn with_includes(mut self, paths: &VendorPaths) -> Self {
        self.builder = self
            .builder
            .clang_arg(format!("-I{}", paths.ballet.display()))
            .clang_arg(format!("-I{}", paths.util.display()))
            .clang_arg(format!("-I{}", paths.vendor.display()));
        self
    }

    pub fn with_clang_args(mut self) -> Self {
        self.builder = self
            .builder
            .clang_arg("-DFD_HAS_HOSTED=1")
            .clang_arg("-std=c17");
        self
    }

    pub fn with_clang_opts(mut self) -> Self {
        match self.target_info.arch {
            Arch::X86_64 => {
                if self.target_info.emulated {
                    self.builder = self
                        .builder
                        .clang_arg("-DFD_HAS_X86=0")
                        .clang_arg("-DFD_HAS_SSE=0")
                        .clang_arg("-DFD_HAS_AVX=0")
                        .clang_arg("-DFD_HAS_AVX512=0");
                } else {
                    self.builder = self
                        .builder
                        .clang_arg("-DFD_HAS_X86=1")
                        .clang_arg("-DFD_HAS_SSE=1")
                        .clang_arg("-DFD_HAS_AVX=1")
                        .clang_arg("-msse")
                        .clang_arg("-msse2")
                        .clang_arg("-mavx")
                        .clang_arg("-mavx2")
                        .clang_arg("-DFD_HAS_AVX512=1")
                        .clang_arg("-mavx512f")
                        .clang_arg("-mavx512bw")
                        .clang_arg("-mavx512dq")
                        .clang_arg("-mavx512vl")
                        .clang_arg("-mavx512ifma")
                        .clang_arg("-mavx512vbmi");
                }
            }
            Arch::AArch64 => {
                self.builder = self.builder.clang_arg("-DFD_HAS_ARM=1");
            }
            Arch::Other => {}
        }

        match self.target_info.platform {
            OsTarget::MacOS => {
                self.builder = self.builder.clang_arg("-DSIGPOLL=SIGIO");
            }
            OsTarget::Linux => {}
            OsTarget::Windows => {}
            OsTarget::Other => {}
        }

        self
    }

    pub fn with_logs(mut self, style: u32) -> Self {
        self.builder = self.builder.clang_arg(format!("-DFD_LOG_STYLE={style}"));
        self
    }

    pub fn suppress_warnings(mut self) -> Self {
        self.builder = self
            .builder
            .clang_arg("-Wno-error=implicit-function-declaration");
        self
    }

    pub fn with_static_fn_wrap<P: AsRef<Path>>(mut self, wrapper_path: P) -> Self {
        self.builder = self
            .builder
            .wrap_static_fns(true)
            .wrap_static_fns_path(wrapper_path);
        self
    }

    pub fn allowlist_fns(mut self, patterns: &[&str]) -> Self {
        for pattern in patterns {
            self.builder = self.builder.allowlist_function(pattern);
        }
        self
    }

    pub fn allowlist_types(mut self, patterns: &[&str]) -> Self {
        for pattern in patterns {
            self.builder = self.builder.allowlist_type(pattern);
        }
        self
    }

    pub fn allowlist_vars(mut self, patterns: &[&str]) -> Self {
        for pattern in patterns {
            self.builder = self.builder.allowlist_var(pattern);
        }
        self
    }

    pub fn with_cargo_cb(mut self) -> Self {
        self.builder = self
            .builder
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));
        self
    }

    pub fn finalize(self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let bindings = self.builder.generate()?;

        let out_path = PathBuf::from(env::var("OUT_DIR")?);
        bindings.write_to_file(out_path.join(filename))?;

        Ok(())
    }

    pub fn inner_mut(&mut self) -> &mut bindgen::Builder {
        &mut self.builder
    }

    pub fn targetinfo(&self) -> &TargetInfo {
        &self.target_info
    }
}

impl Default for BindgenBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub fn _gen_header(
    filename: &str,
    includes: &[&str],
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let out_path = PathBuf::from(env::var("OUT_DIR")?);
    let wrapper_path = out_path.join(filename);

    let content = header_includes!(includes);

    std::fs::write(&wrapper_path, content)?;
    Ok(wrapper_path)
}

pub fn _gen_header_single_mod(
    filename: &str,
    modpath: PathBuf,
    includes: &[&str],
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let out_path = PathBuf::from(env::var("OUT_DIR")?);
    let wrapper_path = out_path.join(filename);

    let stmts = indir_files!(modpath, includes);
    let content = header_includes!(stmts);

    std::fs::write(&wrapper_path, content)?;
    Ok(wrapper_path)
}

pub fn _pipeline_finalize(
    cc: cc::Build,
    bindgen: bindgen::Builder,
    libname: &str,
    outfile: Option<&str>,
) {
    let bindings = bindgen.generate().expect("Unable to generate bindings");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    bindings
        .write_to_file(out_path.join(outfile.unwrap_or("bindings.rs")))
        .expect("Couldn't write bindings!");

    cc.compile(libname);
}

pub fn is_emulated() -> bool {
    if env::var("QEMU_CPU").is_ok()
        || read_to_string("/proc/version")
            .unwrap_or_default()
            .contains("qemu")
    {
        return true;
    }

    if let Ok(cpuinfo) = read_to_string("/proc/cpuinfo") {
        if cpuinfo.contains("processor") && !cpuinfo.contains("flags") {
            return true;
        }
        if cpuinfo.contains("asimd") || cpuinfo.contains("neon") {
            return true;
        }
    }

    if let Ok(val) = env::var("FD_FORCE_REFERENCE_IMPL") {
        return val == "1" || val.to_lowercase() == "true";
    }

    if metadata("/.dockerenv").is_ok() {
        if let Ok(arch_output) = Command::new("uname").arg("-m").output() {
            let reported_arch = String::from_utf8_lossy(&arch_output.stdout);
            if reported_arch.trim() == "x86_64" {
                if let Ok(lscpu_output) = Command::new("lscpu").output() {
                    let lscpu_str = String::from_utf8_lossy(&lscpu_output.stdout);
                    if lscpu_str.contains("aarch64") || lscpu_str.contains("ARM") {
                        return true;
                    }
                }
            }
        }
    }

    false
}

#[macro_export]
macro_rules! fd_allowlist {
    ($prefix:literal) => {
        &[
            concat!($prefix, "_.*"),
            concat!($prefix, ".*"),
            concat!(stringify!($prefix), "_.*"),
        ]
    };
}

#[macro_export]
macro_rules! rerun_if_changed {
    ($($path:expr),* $(,)?) => {
        $(
            $crate::RerunDef::add_file($path);
        )*
    };
}

#[macro_export]
macro_rules! header_includes {
    ($includes:expr) => {{
        $includes
            .iter()
            .map(|include| format!("#include \"{}\"", include))
            .collect::<Vec<_>>()
            .join("\n")
    }};
}

/// generate formatted paths for `_gen_header`
///
/// pass in:
/// path: PathBuf (i.e ed25519_path)
/// array of filenames: &[&str]
#[macro_export]
macro_rules! indir_files {
    ($dir:expr, $exts:expr) => {{
        let mut paths = vec![];
        for ext in $exts {
            paths.push(format!(
                "{}/{}",
                $dir.canonicalize().unwrap().display(),
                ext
            ));
        }
        paths
    }};
}
