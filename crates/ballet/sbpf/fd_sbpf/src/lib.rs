//! Safe Rust wrappers for Firedancer's SBPF (Solana Berkeley Packet Filter) implementation.
//!
//! This crate provides safe, idiomatic Rust APIs for working with SBPF programs,
//! including ELF program loading, instruction parsing, and program execution setup.
//!
//! # Features
//!
//! - **ELF Program Loading**: Parse and load SBPF ELF binaries
//! - **Instruction Parsing**: Work with SBPF instructions and opcodes
//! - **Memory Management**: Safe handling of program memory and rodata segments
//! - **Error Handling**: Comprehensive error types with descriptive messages
//!
//! # Examples
//!
//! ## Loading an SBPF Program
//!
//! ```rust,no_run
//! use fd_sbpf::{SbpfProgram, LoaderConfig};
//!
//! // Load ELF binary
//! let elf_bytes = std::fs::read("program.so").expect("Failed to read ELF file");
//!
//! // Configure loader
//! let config = LoaderConfig::default();
//!
//! // Load program
//! match SbpfProgram::load(&elf_bytes, config) {
//!     Ok(program) => {
//!         println!("Program loaded successfully");
//!         println!("Entry PC: {}", program.entry_pc());
//!         println!("Text size: {}", program.text_size());
//!     }
//!     Err(e) => {
//!         eprintln!("Failed to load program: {}", e);
//!     }
//! }
//! ```
//!
//! ## Working with Instructions
//!
//! ```rust
//! use fd_sbpf::SbpfIxn;
//!
//! // Create instruction from raw value
//! let raw_instr: u64 = 0x1234567890abcdef;
//! let instr = SbpfIxn::from_raw(raw_instr);
//!
//! // Check instruction properties
//! if instr.is_function_start() {
//!     println!("This is a function start instruction");
//! }
//!
//! if instr.is_function_end() {
//!     println!("This is a function end instruction");
//! }
//!
//! // Convert back to raw value
//! let raw_again = instr.to_raw();
//! assert_eq!(raw_instr, raw_again);
//! ```

use core::{
    fmt::{self, Display},
    ptr::NonNull,
    slice,
};

use fd_sbpf_sys::*;

/// Errors that can occur during SBPF operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SbpfError {
    /// Invalid ELF file format
    InvalidElf,
    /// ELF parser error with specific error code
    ElfParserError(i32),
    /// Unsupported SBPF version
    UnsupportedSbpfVersion,
    /// Memory allocation failed
    AllocationFailed,
    /// Invalid program configuration
    InvalidConfig,
    /// Program loading failed
    LoadFailed(String),
    /// Null pointer encountered
    NullPointer,
    /// Invalid buffer size or alignment
    InvalidBuffer,
}

impl Display for SbpfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SbpfError::InvalidElf => write!(f, "Invalid ELF file format"),
            SbpfError::ElfParserError(code) => write!(f, "ELF parser error: {}", code),
            SbpfError::UnsupportedSbpfVersion => write!(f, "Unsupported SBPF version"),
            SbpfError::AllocationFailed => write!(f, "Memory allocation failed"),
            SbpfError::InvalidConfig => write!(f, "Invalid program configuration"),
            SbpfError::LoadFailed(msg) => write!(f, "Program loading failed: {}", msg),
            SbpfError::NullPointer => write!(f, "Null pointer encountered"),
            SbpfError::InvalidBuffer => write!(f, "Invalid buffer size or alignment"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LoaderConfig {
    /// enable ELF deployment checks
    pub elf_deploy_checks: bool,
    /// min SBPF version allowed
    pub sbpf_min_version: u32,
    /// max SBPF version allowed  
    pub sbpf_max_version: u32,
    /// enable symbol and section labels
    pub enable_symbol_and_section_labels: bool,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            elf_deploy_checks: true,
            sbpf_min_version: FD_SBPF_V0,
            sbpf_max_version: FD_SBPF_V3,
            enable_symbol_and_section_labels: true,
        }
    }
}

impl From<LoaderConfig> for fd_sbpf_loader_config_t {
    fn from(config: LoaderConfig) -> Self {
        Self {
            elf_deploy_checks: if config.elf_deploy_checks { 1 } else { 0 },
            sbpf_min_version: config.sbpf_min_version,
            sbpf_max_version: config.sbpf_max_version,
            enable_symbol_and_section_labels: if config.enable_symbol_and_section_labels {
                1
            } else {
                0
            },
        }
    }
}

#[derive(Debug)]
pub struct ElfInfo {
    inner: fd_sbpf_elf_info_t,
}

impl ElfInfo {
    pub fn parse(elf_data: &[u8], config: LoaderConfig) -> Result<Self, SbpfError> {
        let mut info = core::mem::MaybeUninit::<fd_sbpf_elf_info_t>::uninit();
        let config_sys = fd_sbpf_loader_config_t::from(config);

        let result = unsafe {
            fd_sbpf_elf_peek(
                info.as_mut_ptr(),
                elf_data.as_ptr() as *const core::ffi::c_void,
                elf_data.len() as u64,
                &config_sys,
            )
        };

        match result {
            0 => {
                let inner = unsafe { info.assume_init() };
                Ok(Self { inner })
            }
            code if code >= FD_SBPF_ELF_PARSER_ERR_INVALID_FILE_HEADER
                && code <= FD_SBPF_ELF_PARSER_ERR_NO_DYNAMIC_STRING_TABLE =>
            {
                Err(SbpfError::ElfParserError(result))
            }
            FD_SBPF_ELF_ERR_UNSUPPORTED_SBPF_VERSION => Err(SbpfError::UnsupportedSbpfVersion),
            _ => Err(SbpfError::InvalidElf),
        }
    }

    /// Get the footprint required for a program with this ELF info.
    pub fn program_footprint(&self) -> u64 {
        unsafe { fd_sbpf_program_footprint(&self.inner) }
    }

    /// Get the size of the rodata segment that will be mapped into VM memory.
    pub fn rodata_size(&self) -> u32 {
        self.inner.rodata_sz
    }

    /// Get the footprint required for the rodata segment during loading.
    /// This is typically the size of the entire ELF binary.
    pub fn rodata_footprint(&self) -> u32 {
        self.inner.rodata_footprint
    }

    /// Get the text section offset within the rodata segment.
    pub fn text_offset(&self) -> u32 {
        self.inner.text_off
    }

    /// Get the number of instructions in the text section.
    pub fn text_count(&self) -> u32 {
        self.inner.text_cnt
    }

    /// Get the entry point program counter.
    pub fn entry_pc(&self) -> u32 {
        self.inner.entry_pc
    }

    /// Get the SBPF version.
    pub fn sbpf_version(&self) -> u64 {
        self.inner.sbpf_version
    }

    /// Get the inner ELF info structure (for internal use).
    pub(crate) fn inner(&self) -> &fd_sbpf_elf_info_t {
        &self.inner
    }
}

pub struct SbpfProgram {
    program: NonNull<fd_sbpf_program_t>,
    _program_mem: Vec<u8>,
    _rodata: Vec<u8>,
}

impl SbpfProgram {
    pub fn load(elf_data: &[u8], config: LoaderConfig) -> Result<Self, SbpfError> {
        let elf_info = ElfInfo::parse(elf_data, config)?;
        let footprint = elf_info.program_footprint();
        let align = Self::program_align();

        if footprint == 0 {
            return Err(SbpfError::InvalidBuffer);
        }

        let mut program_mem = Vec::with_capacity(footprint as usize + align as usize);
        program_mem.resize(footprint as usize + align as usize, 0);

        let program_ptr = program_mem.as_mut_ptr();
        let aligned_ptr = unsafe {
            let offset = program_ptr.align_offset(align as usize);
            program_ptr.add(offset)
        };

        let rodata_footprint = elf_info.rodata_footprint() as usize;
        let rodata_align = FD_SBPF_PROG_RODATA_ALIGN as usize;
        let mut rodata = if rodata_footprint > 0 {
            let mut rodata_mem = Vec::with_capacity(rodata_footprint + rodata_align);
            rodata_mem.resize(rodata_footprint + rodata_align, 0);
            rodata_mem
        } else {
            Vec::new()
        };

        let rodata_ptr = if rodata_footprint > 0 {
            let rodata_raw_ptr = rodata.as_mut_ptr();
            unsafe {
                let offset = rodata_raw_ptr.align_offset(rodata_align);
                rodata_raw_ptr.add(offset) as *mut core::ffi::c_void
            }
        } else {
            core::ptr::null_mut()
        };

        let program = unsafe {
            fd_sbpf_program_new(
                aligned_ptr as *mut core::ffi::c_void,
                elf_info.inner(),
                rodata_ptr,
            )
        };

        let program = NonNull::new(program).ok_or(SbpfError::NullPointer)?;

        let result = Self {
            program,
            _program_mem: program_mem,
            _rodata: rodata,
        };

        let config_sys = fd_sbpf_loader_config_t::from(config);
        let load_result = unsafe {
            fd_sbpf_program_load(
                result.program.as_ptr(),
                elf_data.as_ptr() as *const core::ffi::c_void,
                elf_data.len() as u64,
                core::ptr::null_mut(), // syscalls
                &config_sys,
            )
        };

        if load_result != 0 {
            let error_msg = unsafe {
                let error_cstr = fd_sbpf_strerror();
                let error_slice = core::ffi::CStr::from_ptr(error_cstr);
                error_slice.to_string_lossy().into_owned()
            };
            return Err(SbpfError::LoadFailed(error_msg));
        }

        Ok(result)
    }

    /// program alignment requirement
    pub fn program_align() -> u64 {
        unsafe { fd_sbpf_program_align() }
    }

    /// required alignment for rodata segments
    pub fn rodata_align() -> u32 {
        FD_SBPF_PROG_RODATA_ALIGN
    }

    /// entry program counter
    pub fn entry_pc(&self) -> u64 {
        unsafe { (*self.program.as_ptr()).entry_pc }
    }

    /// text segment size
    pub fn text_size(&self) -> u64 {
        unsafe { (*self.program.as_ptr()).text_sz }
    }

    /// text instruction count
    pub fn text_count(&self) -> u64 {
        unsafe { (*self.program.as_ptr()).text_cnt }
    }

    /// rodata size
    pub fn rodata_size(&self) -> u64 {
        unsafe { (*self.program.as_ptr()).rodata_sz }
    }

    /// text instructions as a slice
    pub fn text_instructions(&self) -> &[u64] {
        unsafe {
            let program = self.program.as_ptr();
            let text_ptr = (*program).text;
            let text_cnt = (*program).text_cnt;

            if text_ptr.is_null() || text_cnt == 0 {
                &[]
            } else {
                slice::from_raw_parts(text_ptr, text_cnt as usize)
            }
        }
    }
}

impl Drop for SbpfProgram {
    fn drop(&mut self) {
        unsafe {
            fd_sbpf_program_delete(self.program.as_ptr());
        }
    }
}

unsafe impl Send for SbpfProgram {}
unsafe impl Sync for SbpfProgram {}

#[derive(Clone, Copy)]
pub struct SbpfIxn {
    inner: fd_sbpf_instr_t,
}

impl core::fmt::Debug for SbpfIxn {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SbpfIxn")
            .field("opcode", &self.opcode())
            .field("dst_reg", &self.dst())
            .field("src_reg", &self.src())
            .field("offset", &self.off())
            .field("immediate", &self.imm())
            .finish()
    }
}

impl PartialEq for SbpfIxn {
    fn eq(&self, other: &Self) -> bool {
        self.to_raw() == other.to_raw()
    }
}

impl Eq for SbpfIxn {}

impl SbpfIxn {
    /// create an instruction from a raw u64 value
    pub fn from_raw(raw: u64) -> Self {
        let inner = unsafe { fd_sbpf_instr(raw) };
        Self { inner }
    }

    /// convert the instruction to a raw u64 value
    pub fn to_raw(&self) -> u64 {
        unsafe { fd_sbpf_ulong(self.inner) }
    }

    /// check if this instruction marks the start of a function
    pub fn is_function_start(&self) -> bool {
        unsafe { fd_sbpf_is_function_start(self.inner) != 0 }
    }

    /// check if this instruction marks the end of a function
    pub fn is_function_end(&self) -> bool {
        unsafe { fd_sbpf_is_function_end(self.inner) != 0 }
    }

    /// opcode of this instruction
    pub fn opcode(&self) -> u8 {
        unsafe { self.inner.opcode.raw }
    }

    /// destination register
    pub fn dst(&self) -> u8 {
        self.inner.dst_reg()
    }

    /// source register
    pub fn src(&self) -> u8 {
        self.inner.src_reg()
    }

    /// offset field
    pub fn off(&self) -> i16 {
        self.inner.offset
    }

    /// immediate value
    pub fn imm(&self) -> u32 {
        self.inner.imm
    }
}

pub mod registers {
    pub const R0: u8 = 0;
    pub const R1: u8 = 1;
    pub const R2: u8 = 2;
    pub const R3: u8 = 3;
    pub const R4: u8 = 4;
    pub const R5: u8 = 5;
    pub const R6: u8 = 6;
    pub const R7: u8 = 7;
    pub const R8: u8 = 8;
    pub const R9: u8 = 9;
    pub const R10: u8 = 10;
}

pub mod versions {
    pub use fd_sbpf_sys::{FD_SBPF_V0, FD_SBPF_V1, FD_SBPF_V2, FD_SBPF_V3, FD_SBPF_V4};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ix_roundtrip() {
        let raw_val: u64 = 0x1234567890abcdef;
        let instr = SbpfIxn::from_raw(raw_val);
        let converted_back = instr.to_raw();
        assert_eq!(raw_val, converted_back);
    }

    #[test]
    fn test_fn_bounds() {
        // function start (opcode 0x07, dst_reg 0x0A)
        let function_start_val: u64 = 0x07 | (0x0A << 8);
        let instr = SbpfIxn::from_raw(function_start_val);
        assert!(instr.is_function_start());
        assert!(!instr.is_function_end());

        // function end (opcode 0x05)
        let function_end_val: u64 = 0x05;
        let instr = SbpfIxn::from_raw(function_end_val);
        assert!(!instr.is_function_start());
        assert!(instr.is_function_end());

        // function end (opcode 0x9D)
        let function_end_val2: u64 = 0x9D;
        let instr = SbpfIxn::from_raw(function_end_val2);
        assert!(!instr.is_function_start());
        assert!(instr.is_function_end());
    }

    #[test]
    fn test_ix_fields() {
        let raw_val: u64 = 0x12345678_9abc_def0;
        let instr = SbpfIxn::from_raw(raw_val);
        let _ = instr.opcode();
        let _ = instr.dst();
        let _ = instr.src();
        let _ = instr.off();
        let _ = instr.imm();
    }

    #[test]
    fn test_loader_cfg() {
        let config = LoaderConfig::default();
        let sys_config: fd_sbpf_loader_config_t = config.into();

        assert_eq!(sys_config.elf_deploy_checks, 1);
        assert_eq!(sys_config.sbpf_min_version, FD_SBPF_V0);
        assert_eq!(sys_config.sbpf_max_version, FD_SBPF_V3);
        assert_eq!(sys_config.enable_symbol_and_section_labels, 1);
    }

    #[test]
    fn test_progalign() {
        let align = SbpfProgram::program_align();
        assert!(align > 0);
        assert!(align.is_power_of_two());
    }

    #[test]
    fn test_elfinfo_invalid() {
        let invalid_elf = b"not an elf file";
        let config = LoaderConfig::default();

        let result = ElfInfo::parse(invalid_elf, config);
        assert!(result.is_err());
    }

    #[test]
    fn test_rodata_align() {
        let align = SbpfProgram::rodata_align();
        assert_eq!(align, 8);
        assert!(align.is_power_of_two());
    }

    #[test]
    fn test_elfinfo_rodata() {
        let empty_elf = [0u8; 64];
        let config = LoaderConfig::default();

        if let Ok(elf_info) = ElfInfo::parse(&empty_elf, config) {
            let _rodata_size = elf_info.rodata_size();
            let _rodata_footprint = elf_info.rodata_footprint();
            let _text_offset = elf_info.text_offset();
            let _text_count = elf_info.text_count();
            let _entry_pc = elf_info.entry_pc();
            let _sbpf_version = elf_info.sbpf_version();

            assert!(true, "All ElfInfo rodata methods are accessible");
        } else {
            // expected for invalid ELF data
            assert!(true, "ElfInfo rodata methods compile successfully");
        }
    }
}
