//! Safe API for `fd_svm_sys`.

use core::fmt;
use fd_svm_sys as sys;
use std::alloc::{alloc, dealloc, Layout};
use std::ptr::NonNull;

pub const COMPUTE_UNIT_LIMIT: u64 = 1_400_000;
pub const HEAP_MAX: u64 = 256 * 1024; // 256KB
pub const STACK_FRAME_MAX: usize = 64;

#[derive(Debug, Clone, PartialEq)]
pub enum SvmError {
    InvalidInput(&'static str),
    AllocationFailed,
    InitializationFailed,
    ValidationFailed(i32),
    ExecutionFailed(i32),
    InvalidOpcode,
    InvalidRegister,
    JumpOutOfBounds,
    StackOverflow,
    IllegalInstruction,
    MemoryViolation,
    UnalignedAccess,
    ComputeBudgetExceeded,
    DivisionByZero,
}

impl fmt::Display for SvmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SvmError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            SvmError::AllocationFailed => write!(f, "Memory allocation failed"),
            SvmError::InitializationFailed => write!(f, "VM initialization failed"),
            SvmError::ValidationFailed(code) => write!(f, "Program validation failed: {}", code),
            SvmError::ExecutionFailed(code) => write!(f, "Program execution failed: {}", code),
            SvmError::InvalidOpcode => write!(f, "Invalid opcode encountered"),
            SvmError::InvalidRegister => write!(f, "Invalid register access"),
            SvmError::JumpOutOfBounds => write!(f, "Jump out of bounds"),
            SvmError::StackOverflow => write!(f, "Call stack overflow"),
            SvmError::IllegalInstruction => write!(f, "Illegal instruction"),
            SvmError::MemoryViolation => write!(f, "Memory access violation"),
            SvmError::UnalignedAccess => write!(f, "Unaligned memory access"),
            SvmError::ComputeBudgetExceeded => write!(f, "Compute budget exceeded"),
            SvmError::DivisionByZero => write!(f, "Division by zero"),
        }
    }
}

impl std::error::Error for SvmError {}

fn convert_vm_error(code: i32) -> SvmError {
    match code {
        code if code == sys::FD_VM_ERR_INVAL as i32 => SvmError::InvalidInput("Invalid parameter"),
        code if code == sys::FD_VM_ERR_INVALID_OPCODE as i32 => SvmError::InvalidOpcode,
        code if code == sys::FD_VM_ERR_INVALID_SRC_REG as i32 => SvmError::InvalidRegister,
        code if code == sys::FD_VM_ERR_INVALID_DST_REG as i32 => SvmError::InvalidRegister,
        code if code == sys::FD_VM_ERR_JMP_OUT_OF_BOUNDS as i32 => SvmError::JumpOutOfBounds,
        code if code == sys::FD_VM_ERR_SIGFPE as i32 => SvmError::DivisionByZero,
        _ => SvmError::ExecutionFailed(code),
    }
}

pub struct Vm {
    vm_ptr: NonNull<sys::fd_vm_t>,
    _memory: NonNull<u8>,
    layout: Layout,
}

impl Vm {
    pub fn new() -> Result<Self, SvmError> {
        unsafe {
            let align = sys::fd_vm_align() as usize;
            let footprint = sys::fd_vm_footprint() as usize;

            let layout = Layout::from_size_align(footprint, align)
                .map_err(|_| SvmError::InvalidInput("Invalid alignment or size"))?;

            let memory = alloc(layout);
            if memory.is_null() {
                return Err(SvmError::AllocationFailed);
            }

            let memory = NonNull::new_unchecked(memory);
            let vm_mem = sys::fd_vm_new(memory.as_ptr() as *mut std::ffi::c_void);
            if vm_mem.is_null() {
                dealloc(memory.as_ptr(), layout);
                return Err(SvmError::InitializationFailed);
            }

            let vm_ptr = sys::fd_vm_join(vm_mem);
            if vm_ptr.is_null() {
                sys::fd_vm_delete(vm_mem);
                dealloc(memory.as_ptr(), layout);
                return Err(SvmError::InitializationFailed);
            }

            let vm_ptr = NonNull::new_unchecked(vm_ptr);

            Ok(Vm {
                vm_ptr,
                _memory: memory,
                layout,
            })
        }
    }

    pub fn align() -> usize {
        unsafe { sys::fd_vm_align() as usize }
    }

    pub fn footprint() -> usize {
        unsafe { sys::fd_vm_footprint() as usize }
    }

    pub fn init(
        &mut self,
        heap_max: u64,
        entry_cu: u64,
        rodata: &[u8],
        text: &[u64],
        text_off: u64,
        entry_pc: u64,
        calldests: &[u64],
        sbpf_version: u64,
    ) -> Result<(), SvmError> {
        let config = VmConfig {
            heap_max,
            entry_cu,
            sbpf_version: match sbpf_version {
                0 => SbpfVersion::V0,
                1 => SbpfVersion::V1,
                2 => SbpfVersion::V2,
                _ => return Err(SvmError::InvalidInput("Invalid SBPF version")),
            },
            enable_direct_mapping: false,
            dump_syscall_to_pb: false,
            tracing: None,
        };

        self.init_with_config(
            config,
            rodata,
            text,
            text_off,
            entry_pc,
            calldests,
            &[],
            &[],
        )
    }

    pub fn init_with_config(
        &mut self,
        config: VmConfig,
        rodata: &[u8],
        text: &[u64],
        text_off: u64,
        entry_pc: u64,
        calldests: &[u64],
        memory_regions: &[MemoryRegion],
        account_regions: &[AccountRegionMeta],
    ) -> Result<(), SvmError> {
        if config.heap_max > HEAP_MAX {
            return Err(SvmError::InvalidInput("Heap size too large"));
        }

        if config.entry_cu > COMPUTE_UNIT_LIMIT {
            return Err(SvmError::InvalidInput("Compute units too large"));
        }

        let mut c_mem_regions: Vec<sys::fd_vm_input_region_t> = memory_regions
            .iter()
            .map(|r| sys::fd_vm_input_region_t {
                vaddr_offset: r.vaddr_offset,
                haddr: r.haddr,
                region_sz: r.size,
                is_writable: if r.is_writable { 1 } else { 0 },
                is_acct_data: if r.is_account_data { 1 } else { 0 },
            })
            .collect();

        let mut c_acc_regions: Vec<sys::fd_vm_acc_region_meta_t> = account_regions
            .iter()
            .map(|a| sys::fd_vm_acc_region_meta_t {
                region_idx: a.region_idx,
                has_data_region: if a.has_data_region { 1 } else { 0 },
                has_resizing_region: if a.has_resizing_region { 1 } else { 0 },
                metadata_region_offset: a.metadata_region_offset,
                original_data_len: a.original_data_len,
            })
            .collect();

        unsafe {
            let result = sys::fd_vm_init(
                self.vm_ptr.as_ptr(),
                std::ptr::null_mut(), // instr_ctx
                config.heap_max,
                config.entry_cu,
                rodata.as_ptr(),
                rodata.len() as u64,
                text.as_ptr(),
                text.len() as u64,
                text_off,
                text.len() as u64 * 8, // text_sz
                entry_pc,
                calldests.as_ptr() as *mut u64, // TODO: bitvec
                config.sbpf_version.into(),
                std::ptr::null_mut(), // syscalls
                std::ptr::null_mut(), // trace - TODO: implement tracing
                std::ptr::null_mut(), // sha
                if c_mem_regions.is_empty() {
                    std::ptr::null_mut()
                } else {
                    c_mem_regions.as_mut_ptr()
                },
                c_mem_regions.len() as u32,
                if c_acc_regions.is_empty() {
                    std::ptr::null_mut()
                } else {
                    c_acc_regions.as_mut_ptr()
                },
                0, // is_deprecated
                if config.enable_direct_mapping { 1 } else { 0 },
                if config.dump_syscall_to_pb { 1 } else { 0 },
            );

            if result.is_null() {
                return Err(SvmError::InitializationFailed);
            }
        }

        Ok(())
    }

    pub fn setup_for_execution(&mut self) -> Result<(), SvmError> {
        unsafe {
            let result = sys::fd_vm_setup_state_for_execution(self.vm_ptr.as_ptr());
            if result != 0 {
                return Err(SvmError::InitializationFailed);
            }
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), SvmError> {
        unsafe {
            let result = sys::fd_vm_validate(self.vm_ptr.as_ptr());
            if result != sys::FD_VM_SUCCESS as i32 {
                return Err(SvmError::ValidationFailed(result));
            }
        }
        Ok(())
    }

    /// Execute the loaded program
    pub fn execute(&mut self) -> Result<(), SvmError> {
        unsafe {
            let result = sys::fd_vm_exec(self.vm_ptr.as_ptr());
            if result != sys::FD_VM_SUCCESS as i32 {
                return Err(convert_vm_error(result));
            }
        }
        Ok(())
    }

    pub fn execute_trace(&mut self) -> Result<(), SvmError> {
        unsafe {
            let result = sys::fd_vm_exec_trace(self.vm_ptr.as_ptr());
            if result != sys::FD_VM_SUCCESS as i32 {
                return Err(convert_vm_error(result));
            }
        }
        Ok(())
    }

    pub fn execute_notrace(&mut self) -> Result<(), SvmError> {
        unsafe {
            let result = sys::fd_vm_exec_notrace(self.vm_ptr.as_ptr());
            if result != sys::FD_VM_SUCCESS as i32 {
                return Err(convert_vm_error(result));
            }
        }
        Ok(())
    }

    pub fn get_execution_state(&self) -> ExecutionResult {
        unsafe {
            let vm = self.vm_ptr.as_ptr();
            ExecutionResult {
                pc: (*vm).pc,
                instruction_count: (*vm).ic,
                compute_units_remaining: (*vm).cu,
                frame_count: (*vm).frame_cnt,
            }
        }
    }

    pub fn get_heap_size(&self) -> u64 {
        unsafe {
            let vm = self.vm_ptr.as_ptr();
            (*vm).heap_sz
        }
    }

    pub fn get_heap_max(&self) -> u64 {
        unsafe {
            let vm = self.vm_ptr.as_ptr();
            (*vm).heap_max
        }
    }

    pub fn is_direct_mapping_enabled(&self) -> bool {
        unsafe {
            let vm = self.vm_ptr.as_ptr();
            (*vm).direct_mapping != 0
        }
    }

    pub fn get_sbpf_version(&self) -> SbpfVersion {
        unsafe {
            let vm = self.vm_ptr.as_ptr();
            match (*vm).sbpf_version {
                0 => SbpfVersion::V0,
                1 => SbpfVersion::V1,
                2 => SbpfVersion::V2,
                _ => SbpfVersion::V0, // Default fallback
            }
        }
    }

    /// SAFETY: The caller must ensure that the returned pointer is not used after
    /// this VM instance is dropped, and that it's not used to violate
    /// Rust's aliasing rules.
    pub unsafe fn as_ptr(&self) -> *mut sys::fd_vm_t {
        self.vm_ptr.as_ptr()
    }
}

impl Drop for Vm {
    fn drop(&mut self) {
        unsafe {
            let vm_mem = sys::fd_vm_leave(self.vm_ptr.as_ptr());
            if !vm_mem.is_null() {
                sys::fd_vm_delete(vm_mem);
            }
            dealloc(self._memory.as_ptr(), self.layout);
        }
    }
}

// VM is not Sync because it contains mutable state
unsafe impl Send for Vm {}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// program counter
    pub pc: u64,
    /// instructions executed
    pub instruction_count: u64,
    /// CUs remaining
    pub compute_units_remaining: u64,
    /// stack frame count
    pub frame_count: u64,
}

#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// virtual address offset
    pub vaddr_offset: u64,
    /// host address
    pub haddr: u64,
    /// region size
    pub size: u32,
    /// whether the region is writable
    pub is_writable: bool,
    /// whether this is account data
    pub is_account_data: bool,
}

#[derive(Debug, Clone)]
pub struct AccountRegionMeta {
    /// region index in the input memory regions array
    pub region_idx: u32,
    /// whether this account has a data region
    pub has_data_region: bool,
    /// whether this account has a resizing region
    pub has_resizing_region: bool,
    /// metadata region offset relative to input region start
    pub metadata_region_offset: u64,
    /// original data length (needed for non-DM code path)
    pub original_data_len: u64,
}

#[derive(Debug, Clone)]
pub struct TracingConfig {
    pub enable_instruction_tracing: bool,
    pub enable_memory_tracing: bool,
    pub max_trace_entries: usize,
}

#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SbpfVersion {
    V0 = 0,
    /// dynamic stack frames
    V1 = 1,
    /// callx uses src reg, no lddw/le
    V2 = 2,
}

impl From<SbpfVersion> for u64 {
    fn from(version: SbpfVersion) -> Self {
        version as u64
    }
}

#[derive(Debug, Clone)]
pub struct VmConfig {
    pub heap_max: u64,
    pub entry_cu: u64,
    pub sbpf_version: SbpfVersion,
    pub enable_direct_mapping: bool,
    pub dump_syscall_to_pb: bool,
    pub tracing: Option<TracingConfig>,
}

#[cfg(target_os = "solana")]
mod _syscalls {
    use super::*;

    #[no_mangle]
    pub unsafe extern "C" fn _sol_log_64(arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) {
        fd_sol_log_64(arg1, arg2, arg3, arg4, arg5);
    }

    #[no_mangle]
    pub unsafe extern "C" fn _sol_log(message: *const u8, len: u64) {
        fd_sol_log(message, len);
    }

    #[no_mangle]
    pub unsafe extern "C" fn _sol_log_pubkey(pubkey_addr: *const u8) {
        fd_sol_log_pubkey(pubkey_addr);
    }

    #[no_mangle]
    pub unsafe extern "C" fn _sol_log_data(data: *const u8, data_len: u64) {
        fd_sol_log_data(data, data_len);
    }

    // Memory syscalls
    #[no_mangle]
    pub unsafe extern "C" fn _sol_memcpy(dst: *mut u8, src: *const u8, n: u64) {
        fd_sol_memcpy(dst, src, n);
    }

    #[no_mangle]
    pub unsafe extern "C" fn _sol_memmove(dst: *mut u8, src: *const u8, n: u64) {
        fd_sol_memmove(dst, src, n);
    }

    #[no_mangle]
    pub unsafe extern "C" fn _sol_memcmp(s1: *const u8, s2: *const u8, n: u64, result: *mut i32) {
        fd_sol_memcmp(s1, s2, n, result);
    }

    #[no_mangle]
    pub unsafe extern "C" fn _sol_memset(s: *mut u8, c: i32, n: u64) {
        fd_sol_memset(s, c, n);
    }

    #[no_mangle]
    pub unsafe extern "C" fn _sol_sha256(bytes: *const u8, bytes_len: u64, result: *mut u8) -> u64 {
        fd_sol_sha256(bytes, bytes_len, result)
    }

    #[no_mangle]
    pub unsafe extern "C" fn _sol_keccak256(
        bytes: *const u8,
        bytes_len: u64,
        result: *mut u8,
    ) -> u64 {
        fd_sol_keccak256(bytes, bytes_len, result)
    }

    #[no_mangle]
    pub unsafe extern "C" fn _sol_secp256k1_recover(
        hash: *const u8,
        recovery_id: u64,
        signature: *const u8,
        result: *mut u8,
    ) -> u64 {
        fd_sol_secp256k1_recover(hash, recovery_id, signature, result)
    }

    #[no_mangle]
    pub unsafe extern "C" fn _sol_blake3(bytes: *const u8, bytes_len: u64, result: *mut u8) -> u64 {
        fd_sol_blake3(bytes, bytes_len, result)
    }

    // Program syscalls
    #[no_mangle]
    pub unsafe extern "C" fn _sol_get_clock_sysvar(addr: *mut u8) -> u64 {
        fd_sol_get_clock_sysvar(addr)
    }

    #[no_mangle]
    pub unsafe extern "C" fn _sol_get_epoch_schedule_sysvar(addr: *mut u8) -> u64 {
        fd_sol_get_epoch_schedule_sysvar(addr)
    }

    #[no_mangle]
    pub unsafe extern "C" fn _sol_get_rent_sysvar(addr: *mut u8) -> u64 {
        fd_sol_get_rent_sysvar(addr)
    }

    #[no_mangle]
    pub unsafe extern "C" fn _sol_create_program_address(
        seeds_addr: *const u8,
        seeds_len: u64,
        program_id_addr: *const u8,
        address_bytes_addr: *mut u8,
    ) -> u64 {
        fd_sol_create_program_address(seeds_addr, seeds_len, program_id_addr, address_bytes_addr)
    }

    #[no_mangle]
    pub unsafe extern "C" fn _sol_try_find_program_address(
        seeds_addr: *const u8,
        seeds_len: u64,
        program_id_addr: *const u8,
        address_bytes_addr: *mut u8,
        bump_seed_addr: *mut u8,
    ) -> u64 {
        fd_sol_try_find_program_address(
            seeds_addr,
            seeds_len,
            program_id_addr,
            address_bytes_addr,
            bump_seed_addr,
        )
    }

    #[no_mangle]
    pub unsafe extern "C" fn _sol_get_processed_sibling_instruction(
        index: u64,
        meta: *mut u8,
        program_id: *mut u8,
        data: *mut u8,
        accounts: *mut u8,
    ) -> u64 {
        fd_sol_get_processed_sibling_instruction(index, meta, program_id, data, accounts)
    }

    #[no_mangle]
    pub unsafe extern "C" fn _sol_get_stack_height() -> u64 {
        fd_sol_get_stack_height()
    }

    #[no_mangle]
    pub unsafe extern "C" fn _sol_invoke_signed(
        instruction_addr: *const u8,
        account_infos_addr: *const u8,
        account_infos_len: u64,
        signers_seeds_addr: *const u8,
        signers_seeds_len: u64,
    ) -> u64 {
        fd_sol_invoke_signed(
            instruction_addr,
            account_infos_addr,
            account_infos_len,
            signers_seeds_addr,
            signers_seeds_len,
        )
    }

    #[no_mangle]
    pub unsafe extern "C" fn _sol_set_return_data(data: *const u8, length: u64) {
        fd_sol_set_return_data(data, length);
    }

    #[no_mangle]
    pub unsafe extern "C" fn _sol_get_return_data(
        data: *mut u8,
        length: u64,
        program_id: *mut u8,
    ) -> u64 {
        fd_sol_get_return_data(data, length, program_id)
    }
}

#[cfg(target_os = "solana")]
pub mod syscalls {
    use super::*;

    /// Runtime syscall for logging a UTF-8 string
    ///
    /// Args:
    /// - message: *const u8 - Pointer to the UTF-8 string to log
    /// - len: u64 - Length of the string in bytes
    #[macro_export]
    macro_rules! sol_log {
        ($message:expr, $len:expr) => {
            $crate::_syscalls::_sol_log($message.as_ptr(), $len)
        };
    }

    /// Runtime syscall for logging a 32-byte public key
    ///
    /// Args:
    /// - pubkey_addr: *const u8 - Pointer to the 32-byte public key to log
    #[macro_export]
    macro_rules! sol_log_pubkey {
        ($pubkey:expr) => {
            $crate::_syscalls::_sol_log_pubkey($pubkey.as_ptr())
        };
    }

    /// Runtime syscall for logging arbitrary data
    ///
    /// Args:
    /// - data: *const u8 - Pointer to the data to log
    /// - data_len: u64 - Length of the data in bytes
    #[macro_export]
    macro_rules! sol_log_data {
        ($data:expr, $data_len:expr) => {
            $crate::_syscalls::_sol_log_data($data.as_ptr(), $data_len)
        };
    }

    /// Runtime syscall for logging 64-bit values
    ///
    /// Args:
    /// - arg1: u64 - First 64-bit value
    /// - arg2: u64 - Second 64-bit value
    /// - arg3: u64 - Third 64-bit value
    /// - arg4: u64 - Fourth 64-bit value
    /// - arg5: u64 - Fifth 64-bit value
    ///
    /// Returns:
    /// - u64 - Result code
    #[macro_export]
    macro_rules! sol_log_64 {
        ($arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr) => {
            $crate::_syscalls::_sol_log_64($arg1, $arg2, $arg3, $arg4, $arg5)
        };
    }

    /// Runtime syscall for computing SHA256 hash
    ///
    /// Args:
    /// - bytes: *const u8 - Pointer to the data to hash
    /// - bytes_len: u64 - Length of the data in bytes
    /// - result: *mut u8 - Pointer to the result buffer
    ///
    /// Returns:
    /// - u64 - Result code
    #[macro_export]
    macro_rules! sol_sha256 {
        ($bytes:expr, $bytes_len:expr, $result:expr) => {
            $crate::_syscalls::_sol_sha256($bytes.as_ptr(), $bytes_len, $result.as_ptr())
        };
    }

    /// Runtime syscall for computing Keccak256 hashes
    ///
    /// Args:
    /// - bytes: *const u8 - Pointer to the data to hash
    /// - bytes_len: u64 - Length of the data in bytes
    /// - result: *mut u8 - Pointer to the result buffer
    ///
    /// Returns:
    /// - u64 - Result code
    #[macro_export]
    macro_rules! sol_keccak256 {
        ($bytes:expr, $bytes_len:expr, $result:expr) => {
            $crate::_syscalls::_sol_keccak256($bytes.as_ptr(), $bytes_len, $result.as_ptr())
        };
    }

    /// Runtime syscall for recovering secp256k1 public key from signature
    ///
    /// Args:
    /// - hash: *const u8 - Pointer to the hash to recover the public key from
    /// - recovery_id: u64 - Recovery ID
    /// - signature: *const u8 - Pointer to the signature
    /// - result: *mut u8 - Pointer to the result buffer
    ///
    /// Returns:
    /// - u64 - Result code
    #[macro_export]
    macro_rules! sol_secp256k1_recover {
        ($hash:expr, $recovery_id:expr, $signature:expr, $result:expr) => {
            $crate::_syscalls::_sol_secp256k1_recover(
                $hash.as_ptr(),
                $recovery_id,
                $signature.as_ptr(),
                $result.as_ptr(),
            )
        };
    }

    /// Runtime syscall for computing BLAKE3 hashes
    ///
    /// Args:
    /// - bytes: *const u8 - Pointer to the data to hash
    /// - bytes_len: u64 - Length of the data in bytes
    /// - result: *mut u8 - Pointer to the result buffer
    ///
    /// Returns:
    /// - u64 - Result code
    #[macro_export]
    macro_rules! sol_blake3 {
        ($bytes:expr, $bytes_len:expr, $result:expr) => {
            $crate::_syscalls::_sol_blake3($bytes.as_ptr(), $bytes_len, $result.as_ptr())
        };
    }

    /// Runtime syscall for getting clock sysvar
    ///
    /// Args:
    /// - addr: *mut u8 - Pointer to the result buffer
    ///
    /// Returns:
    /// - u64 - Result code
    #[macro_export]
    macro_rules! sol_get_clock_sysvar {
        ($addr:expr) => {
            $crate::_syscalls::_sol_get_clock_sysvar($addr.as_ptr())
        };
    }

    /// Runtime syscall for getting epoch schedule sysvar
    ///
    /// Args:
    /// - addr: *mut u8 - Pointer to the result buffer
    ///
    /// Returns:
    /// - u64 - Result code
    #[macro_export]
    macro_rules! sol_get_epoch_schedule_sysvar {
        ($addr:expr) => {
            $crate::_syscalls::_sol_get_epoch_schedule_sysvar($addr.as_ptr())
        };
    }

    /// Runtime syscall for getting rent sysvar
    ///
    /// Args:
    /// - addr: *mut u8 - Pointer to the result buffer
    ///
    /// Returns:
    /// - u64 - Result code
    #[macro_export]
    macro_rules! sol_get_rent_sysvar {
        ($addr:expr) => {
            $crate::_syscalls::_sol_get_rent_sysvar($addr.as_ptr())
        };
    }

    /// Runtime syscall for creating program address
    ///
    /// Args:
    /// - seeds_addr: *const u8 - Pointer to the seeds
    /// - seeds_len: u64 - Length of the seeds
    /// - program_id_addr: *const u8 - Pointer to the program ID
    /// - address_bytes_addr: *mut u8 - Pointer to the result buffer
    ///
    /// Returns:
    /// - u64 - Result code
    #[macro_export]
    macro_rules! sol_create_program_address {
        ($seeds:expr, $seeds_len:expr, $program_id:expr, $output:expr) => {
            $crate::_syscalls::_sol_create_program_address(
                $seeds.as_ptr(),
                $seeds_len,
                $program_id.as_ptr(),
                $output.as_ptr(),
            )
        };
    }

    /// Runtime syscall for trying to find program address
    ///
    /// Args:
    /// - seeds_addr: *const u8 - Pointer to the seeds
    /// - seeds_len: u64 - Length of the seeds
    /// - program_id_addr: *const u8 - Pointer to the program ID
    /// - address_bytes_addr: *mut u8 - Pointer to the result buffer
    /// - bump_seed_addr: *mut u8 - Pointer to the bump seed
    ///
    /// Returns:
    /// - u64 - Result code
    #[macro_export]
    macro_rules! sol_try_find_program_address {
        ($seeds:expr, $seeds_len:expr, $program_id:expr, $output:expr, $bump_seed:expr) => {
            $crate::_syscalls::_sol_try_find_program_address(
                $seeds.as_ptr(),
                $seeds_len,
                $program_id.as_ptr(),
                $output.as_ptr(),
                $bump_seed.as_ptr(),
            )
        };
    }

    /// Runtime syscall for getting processed sibling instruction
    ///
    /// Args:
    /// - index: u64 - Index of the instruction
    /// - meta: *mut u8 - Pointer to the metadata
    /// - program_id: *mut u8 - Pointer to the program ID
    /// - data: *mut u8 - Pointer to the data
    /// - accounts: *mut u8 - Pointer to the accounts
    ///
    /// Returns:
    /// - u64 - Result code
    #[macro_export]
    macro_rules! sol_get_processed_sibling_instruction {
        ($index:expr, $meta:expr, $program_id:expr, $data:expr, $accounts:expr) => {
            $crate::_syscalls::_sol_get_processed_sibling_instruction(
                $index,
                $meta.as_ptr(),
                $program_id.as_ptr(),
                $data.as_ptr(),
                $accounts.as_ptr(),
            )
        };
    }

    /// Runtime syscall for getting stack height
    ///
    /// Returns:
    /// - u64 - Result code
    #[macro_export]
    macro_rules! sol_get_stack_height {
        () => {
            $crate::_syscalls::_sol_get_stack_height()
        };
    }

    /// Runtime syscall for invoking signed
    ///
    /// Args:
    /// - instruction_addr: *const u8 - Pointer to the instruction
    /// - account_infos_addr: *const u8 - Pointer to the account infos
    /// - account_infos_len: u64 - Length of the account infos
    /// - signers_seeds_addr: *const u8 - Pointer to the signers seeds
    /// - signers_seeds_len: u64 - Length of the signers seeds
    ///
    /// Returns:
    /// - u64 - Result code
    #[macro_export]
    macro_rules! sol_invoke_signed {
        ($instruction:expr, $account_infos:expr, $account_infos_len:expr, $signers_seeds:expr, $signers_seeds_len:expr) => {
            $crate::_syscalls::_sol_invoke_signed(
                $instruction.as_ptr(),
                $account_infos.as_ptr(),
                $account_infos_len,
                $signers_seeds.as_ptr(),
                $signers_seeds_len,
            )
        };
    }

    /// Runtime syscall for setting return data
    ///
    /// Args:
    /// - data: *const u8 - Pointer to the data
    /// - length: u64 - Length of the data
    ///
    /// Returns:
    /// - u64 - Result code
    #[macro_export]
    macro_rules! sol_set_return_data {
        ($data:expr, $length:expr) => {
            $crate::_syscalls::_sol_set_return_data($data.as_ptr(), $length)
        };
    }

    /// Runtime syscall for getting return data
    ///
    /// Args:
    /// - data: *mut u8 - Pointer to the data
    /// - length: u64 - Length of the data
    /// - program_id: *mut u8 - Pointer to the program ID
    ///
    /// Returns:
    /// - u64 - Result code
    #[macro_export]
    macro_rules! sol_get_return_data {
        ($data:expr, $length:expr, $program_id:expr) => {
            $crate::_syscalls::_sol_get_return_data($data.as_ptr(), $length, $program_id.as_ptr())
        };
    }

    /// Runtime syscall for copying memory
    ///
    /// Args:
    /// - dst: *mut u8 - Pointer to the destination
    /// - src: *const u8 - Pointer to the source
    /// - n: u64 - Length of the data
    ///
    /// Returns:
    /// - u64 - Result code
    #[macro_export]
    macro_rules! sol_memcpy {
        ($dst:expr, $src:expr, $n:expr) => {
            $crate::_syscalls::_sol_memcpy($dst, $src, $n)
        };
    }

    /// Runtime syscall for moving memory
    ///
    /// Args:
    /// - dst: *mut u8 - Pointer to the destination
    /// - src: *const u8 - Pointer to the source
    /// - n: u64 - Length of the data
    ///
    /// Returns:
    /// - u64 - Result code
    #[macro_export]
    macro_rules! sol_memmove {
        ($dst:expr, $src:expr, $n:expr) => {
            $crate::_syscalls::_sol_memmove($dst, $src, $n)
        };
    }

    /// Runtime syscall for comparing memory
    ///
    /// Args:
    /// - s1: *const u8 - Pointer to the first source
    /// - s2: *const u8 - Pointer to the second source
    /// - n: u64 - Length of the data
    /// - result: *mut i32 - Pointer to the result
    ///
    /// Returns:
    /// - u64 - Result code
    #[macro_export]
    macro_rules! sol_memcmp {
        ($s1:expr, $s2:expr, $n:expr) => {
            $crate::_syscalls::_sol_memcmp($s1, $s2, $n)
        };
    }

    /// Runtime syscall for setting memory
    ///
    /// Args:
    /// - s: *mut u8 - Pointer to the memory
    /// - c: i32 - Value to set
    /// - n: u64 - Length of the memory
    ///
    /// Returns:
    /// - u64 - Result code
    #[macro_export]
    macro_rules! sol_memset {
        ($s:expr, $c:expr, $n:expr) => {
            $crate::_syscalls::_sol_memset($s, $c, $n)
        };
    }

    pub fn log_pubkey(pubkey: &[u8; 32]) {
        unsafe {
            sys::fd_sol_log_pubkey(pubkey.as_ptr());
        }
    }

    pub fn log_data(data: &[u8]) {
        unsafe {
            sys::fd_sol_log_data(data.as_ptr(), data.len() as u64);
        }
    }

    pub fn log_64(arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) {
        unsafe {
            sys::fd_sol_log_64(arg1, arg2, arg3, arg4, arg5);
        }
    }

    pub fn sha256(data: &[u8]) -> Result<[u8; 32], SvmError> {
        let mut result = [0u8; 32];
        unsafe {
            let ret = sys::fd_sol_sha256(data.as_ptr(), data.len() as u64, result.as_mut_ptr());
            if ret != 0 {
                return Err(SvmError::ExecutionFailed(ret as i32));
            }
        }
        Ok(result)
    }

    pub fn keccak256(data: &[u8]) -> Result<[u8; 32], SvmError> {
        let mut result = [0u8; 32];
        unsafe {
            let ret = sys::fd_sol_keccak256(data.as_ptr(), data.len() as u64, result.as_mut_ptr());
            if ret != 0 {
                return Err(SvmError::ExecutionFailed(ret as i32));
            }
        }
        Ok(result)
    }

    pub fn blake3(data: &[u8]) -> Result<[u8; 32], SvmError> {
        let mut result = [0u8; 32];
        unsafe {
            let ret = sys::fd_sol_blake3(data.as_ptr(), data.len() as u64, result.as_mut_ptr());
            if ret != 0 {
                return Err(SvmError::ExecutionFailed(ret as i32));
            }
        }
        Ok(result)
    }

    pub fn secp256k1_recover(
        hash: &[u8; 32],
        recovery_id: u64,
        signature: &[u8; 64],
    ) -> Result<[u8; 64], SvmError> {
        let mut result = [0u8; 64];
        unsafe {
            let ret = sys::fd_sol_secp256k1_recover(
                hash.as_ptr(),
                recovery_id,
                signature.as_ptr(),
                result.as_mut_ptr(),
            );
            if ret != 0 {
                return Err(SvmError::ExecutionFailed(ret as i32));
            }
        }
        Ok(result)
    }

    pub fn create_program_address(
        seeds: &[&[u8]],
        program_id: &[u8; 32],
    ) -> Result<[u8; 32], SvmError> {
        let mut serialized_seeds = Vec::new();
        for seed in seeds {
            serialized_seeds.extend_from_slice(&(seed.len() as u64).to_le_bytes());
            serialized_seeds.extend_from_slice(seed);
        }

        let mut result = [0u8; 32];
        unsafe {
            let ret = sys::fd_sol_create_program_address(
                serialized_seeds.as_ptr(),
                serialized_seeds.len() as u64,
                program_id.as_ptr(),
                result.as_mut_ptr(),
            );
            if ret != 0 {
                return Err(SvmError::ExecutionFailed(ret as i32));
            }
        }
        Ok(result)
    }

    pub fn try_find_program_address(
        seeds: &[&[u8]],
        program_id: &[u8; 32],
    ) -> Result<([u8; 32], u8), SvmError> {
        let mut serialized_seeds = Vec::new();
        for seed in seeds {
            serialized_seeds.extend_from_slice(&(seed.len() as u64).to_le_bytes());
            serialized_seeds.extend_from_slice(seed);
        }

        let mut address = [0u8; 32];
        let mut bump_seed = 0u8;
        unsafe {
            let ret = sys::fd_sol_try_find_program_address(
                serialized_seeds.as_ptr(),
                serialized_seeds.len() as u64,
                program_id.as_ptr(),
                address.as_mut_ptr(),
                &mut bump_seed,
            );
            if ret != 0 {
                return Err(SvmError::ExecutionFailed(ret as i32));
            }
        }
        Ok((address, bump_seed))
    }

    pub fn get_stack_height() -> u64 {
        unsafe { sys::fd_sol_get_stack_height() }
    }

    pub fn memcpy(dst: &mut [u8], src: &[u8]) -> Result<(), SvmError> {
        if dst.len() < src.len() {
            return Err(SvmError::InvalidInput("Destination buffer too small"));
        }

        unsafe {
            sys::fd_sol_memcpy(dst.as_mut_ptr(), src.as_ptr(), src.len() as u64);
        }
        Ok(())
    }

    pub fn memmove(dst: &mut [u8], src: &[u8]) -> Result<(), SvmError> {
        if dst.len() < src.len() {
            return Err(SvmError::InvalidInput("Destination buffer too small"));
        }

        unsafe {
            sys::fd_sol_memmove(dst.as_mut_ptr(), src.as_ptr(), src.len() as u64);
        }
        Ok(())
    }

    pub fn memcmp(s1: &[u8], s2: &[u8]) -> Result<i32, SvmError> {
        let len = s1.len().min(s2.len()) as u64;
        let mut result = 0i32;

        unsafe {
            sys::fd_sol_memcmp(s1.as_ptr(), s2.as_ptr(), len, &mut result);
        }
        Ok(result)
    }

    pub fn memset(buffer: &mut [u8], value: u8) {
        unsafe {
            sys::fd_sol_memset(buffer.as_mut_ptr(), value as i32, buffer.len() as u64);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heap_validation() {
        let mut vm = Vm::new().unwrap();
        let result = vm.init(
            HEAP_MAX + 1, // Too large
            1000,
            &[],
            &[],
            0,
            0,
            &[],
            0,
        );

        assert!(matches!(result, Err(SvmError::InvalidInput(_))));
    }

    #[test]
    fn test_cus_validation() {
        let mut vm = Vm::new().unwrap();
        let result = vm.init(1024, COMPUTE_UNIT_LIMIT + 1, &[], &[], 0, 0, &[], 0);

        assert!(matches!(result, Err(SvmError::InvalidInput(_))));
    }
}
