use std::env;
use std::fs::{metadata, read_to_string};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Target architecture and platform detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    X86_64,
    AArch64,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
    MacOS,
    Windows,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetInfo {
    pub arch: Architecture,
    pub platform: Platform,
    pub is_emulated: bool,
}

impl TargetInfo {
    pub fn new() -> Self {
        let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
        let is_emulated = is_emulated();

        let arch = if target.contains("x86_64") {
            Architecture::X86_64
        } else if target.contains("aarch64") || target.contains("arm") {
            Architecture::AArch64
        } else {
            Architecture::Other
        };

        let platform = if target.contains("apple") {
            Platform::MacOS
        } else if target.contains("linux") {
            Platform::Linux
        } else if target.contains("windows") {
            Platform::Windows
        } else {
            Platform::Other
        };

        Self {
            arch,
            platform,
            is_emulated,
        }
    }

    pub fn is_x86_64(&self) -> bool {
        matches!(self.arch, Architecture::X86_64)
    }

    pub fn is_aarch64(&self) -> bool {
        matches!(self.arch, Architecture::AArch64)
    }

    pub fn is_macos(&self) -> bool {
        matches!(self.platform, Platform::MacOS)
    }

    pub fn is_linux(&self) -> bool {
        matches!(self.platform, Platform::Linux)
    }

    pub fn is_windows(&self) -> bool {
        matches!(self.platform, Platform::Windows)
    }
}

impl Default for TargetInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct FiredancerPaths {
    pub vendor: PathBuf,
    pub ballet: PathBuf,
    pub util: PathBuf,
}

impl FiredancerPaths {
    pub fn new() -> Self {
        let vendor = PathBuf::from("../../../../vendor");
        let ballet = vendor.join("ballet");
        let util = vendor.join("util");

        Self {
            vendor,
            ballet,
            util,
        }
    }

    pub fn with_vendor_path<P: AsRef<Path>>(vendor_path: P) -> Self {
        let vendor = vendor_path.as_ref().to_path_buf();
        let ballet = vendor.join("ballet");
        let util = vendor.join("util");

        Self {
            vendor,
            ballet,
            util,
        }
    }

    pub fn ballet_module<P: AsRef<Path>>(&self, module: P) -> PathBuf {
        self.ballet.join(module)
    }

    pub fn util_module<P: AsRef<Path>>(&self, module: P) -> PathBuf {
        self.util.join(module)
    }
}

impl Default for FiredancerPaths {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RerunHelper;

impl RerunHelper {
    pub fn add_file<P: AsRef<Path>>(path: P) {
        println!("cargo:rerun-if-changed={}", path.as_ref().display());
    }

    pub fn add_files<P: AsRef<Path>, I: IntoIterator<Item = P>>(paths: I) {
        for path in paths {
            Self::add_file(path);
        }
    }

    pub fn add_directory_with_extensions<P: AsRef<Path>>(
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
        Self::add_directory_with_extensions(dir, &["c", "cpp", "cxx", "cc", "h", "hpp", "hxx"])
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

pub struct CCBuildHelper {
    build: cc::Build,
    target_info: TargetInfo,
}

impl CCBuildHelper {
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

    pub fn with_firedancer_includes(mut self, paths: &FiredancerPaths) -> Self {
        self.build
            .include(&paths.ballet)
            .include(&paths.util)
            .include(&paths.vendor);
        self
    }

    pub fn with_arch_optimizations(mut self) -> Self {
        match self.target_info.arch {
            Architecture::X86_64 => {
                if self.target_info.is_emulated {
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
            Architecture::AArch64 => {
                self.build.define("FD_HAS_ARM", "1");
            }
            Architecture::Other => {}
        }
        self
    }

    pub fn with_platform_config(mut self) -> Self {
        match self.target_info.platform {
            Platform::MacOS => {
                self.build.define("SIGPOLL", "SIGIO");
            }
            Platform::Linux => {}
            Platform::Windows => {}
            Platform::Other => {}
        }
        self
    }

    pub fn with_logging(mut self, style: u32) -> Self {
        self.build
            .define("FD_LOG_STYLE", style.to_string().as_str());
        self
    }

    pub fn with_warning_suppressions(mut self) -> Self {
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

    pub fn with_avx512_files<P: AsRef<Path>>(mut self, avx512_dir: P) -> Self {
        if self.target_info.is_x86_64() && !self.target_info.is_emulated {
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

    pub fn compile(self, lib_name: &str) {
        self.build.compile(lib_name);
    }

    pub fn inner_mut(&mut self) -> &mut cc::Build {
        &mut self.build
    }

    pub fn target_info(&self) -> &TargetInfo {
        &self.target_info
    }
}

impl Default for CCBuildHelper {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BindgenHelper {
    builder: bindgen::Builder,
    target_info: TargetInfo,
}

impl BindgenHelper {
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

    pub fn with_firedancer_includes(mut self, paths: &FiredancerPaths) -> Self {
        self.builder = self
            .builder
            .clang_arg(format!("-I{}", paths.ballet.display()))
            .clang_arg(format!("-I{}", paths.util.display()))
            .clang_arg(format!("-I{}", paths.vendor.display()));
        self
    }

    pub fn with_firedancer_clang_args(mut self) -> Self {
        self.builder = self
            .builder
            .clang_arg("-DFD_HAS_HOSTED=1")
            .clang_arg("-std=c17");
        self
    }

    pub fn with_arch_clang_args(mut self) -> Self {
        match self.target_info.arch {
            Architecture::X86_64 => {
                if self.target_info.is_emulated {
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
            Architecture::AArch64 => {
                self.builder = self.builder.clang_arg("-DFD_HAS_ARM=1");
            }
            Architecture::Other => {}
        }
        self
    }

    pub fn with_platform_clang_args(mut self) -> Self {
        match self.target_info.platform {
            Platform::MacOS => {
                self.builder = self.builder.clang_arg("-DSIGPOLL=SIGIO");
            }
            Platform::Linux => {}
            Platform::Windows => {}
            Platform::Other => {}
        }
        self
    }

    pub fn with_logging(mut self, style: u32) -> Self {
        self.builder = self.builder.clang_arg(&format!("-DFD_LOG_STYLE={}", style));
        self
    }

    pub fn with_warning_suppressions(mut self) -> Self {
        self.builder = self
            .builder
            .clang_arg("-Wno-error=implicit-function-declaration");
        self
    }

    pub fn with_static_fn_wrapping<P: AsRef<Path>>(mut self, wrapper_path: P) -> Self {
        self.builder = self
            .builder
            .wrap_static_fns(true)
            .wrap_static_fns_path(wrapper_path);
        self
    }

    pub fn allowlist_functions(mut self, patterns: &[&str]) -> Self {
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

    pub fn with_cargo_callbacks(mut self) -> Self {
        self.builder = self
            .builder
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));
        self
    }

    pub fn generate_and_write(self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let bindings = self.builder.generate()?;

        let out_path = PathBuf::from(env::var("OUT_DIR")?);
        bindings.write_to_file(out_path.join(filename))?;

        Ok(())
    }

    pub fn inner_mut(&mut self) -> &mut bindgen::Builder {
        &mut self.builder
    }

    pub fn target_info(&self) -> &TargetInfo {
        &self.target_info
    }
}

impl Default for BindgenHelper {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_wrapper_header(
    filename: &str,
    includes: &[&str],
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let out_path = PathBuf::from(env::var("OUT_DIR")?);
    let wrapper_path = out_path.join(filename);

    let content = includes
        .iter()
        .map(|include| format!("#include \"{}\"", include))
        .collect::<Vec<_>>()
        .join("\n");

    std::fs::write(&wrapper_path, content)?;
    Ok(wrapper_path)
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
            $crate::RerunHelper::add_file($path);
        )*
    };
}
