//! Low-level FFI bindings to Firedancer's fd_sbpf module.
//!
//! This crate provides raw, unsafe bindings to the Firedancer SBPF (Solana Berkeley Packet Filter) API,
//! including ELF program loading, instruction parsing, and program management.
//!
//! For safe, idiomatic Rust wrappers, see the `fd_sbpf` crate.
//!
//! # Safety
//!
//! All functions in this crate are unsafe and require careful handling of:
//! - Memory management and lifetime guarantees  
//! - Proper initialization of program structures
//! - Buffer size requirements and alignment constraints
//! - Thread safety considerations
//!
//! # SBPF Program Operations
//!
//! The main SBPF operations available:
//! - `fd_sbpf_elf_peek`: Parse ELF header and extract basic information
//! - `fd_sbpf_program_new`: Create a new program object
//! - `fd_sbpf_program_load`: Load an ELF program for execution
//! - `fd_sbpf_program_delete`: Destroy a program object
//!
//! # Instruction Operations
//!
//! SBPF instruction utilities:
//! - `fd_sbpf_instr`: Convert ulong to instruction
//! - `fd_sbpf_ulong`: Convert instruction to ulong
//! - `fd_sbpf_is_function_start`: Check if instruction is function start
//! - `fd_sbpf_is_function_end`: Check if instruction is function end
//!
//! # Example
//!
//! ```rust,no_run
//! use fd_sbpf_sys::*;
//! use std::mem::MaybeUninit;
//!
//! unsafe {
//!     // load binary
//!     let elf_bytes: &[u8] = &[/* ... */];
//!     // parse elf header
//!     let mut info = MaybeUninit::<fd_sbpf_elf_info_t>::uninit();
//!     let config = fd_sbpf_loader_config_t {
//!         elf_deploy_checks: 1,
//!         sbpf_min_version: FD_SBPF_V0,
//!         sbpf_max_version: FD_SBPF_V3,
//!         enable_symbol_and_section_labels: 1,
//!     };
//!     
//!     let result = fd_sbpf_elf_peek(
//!         info.as_mut_ptr(),
//!         elf_bytes.as_ptr() as *const core::ffi::c_void,
//!         elf_bytes.len() as u64,
//!         &config
//!     );
//!     
//!     if result == 0 {
//!         let info = info.assume_init();
//!         // program footprint
//!         let footprint = fd_sbpf_program_footprint(&info);
//!         let align = fd_sbpf_program_align();
//!         // allocate program memory
//!         let prog_mem = std::alloc::alloc(
//!             std::alloc::Layout::from_size_align(footprint as usize, align as usize).unwrap()
//!         );
//!         
//!         // create a program object  
//!         let program = fd_sbpf_program_new(
//!             prog_mem as *mut core::ffi::c_void,
//!             &info,
//!             core::ptr::null_mut()
//!         );
//!         
//!         if !program.is_null() {
//!             // clenaup
//!             fd_sbpf_program_delete(program);
//!         }
//!         
//!         std::alloc::dealloc(
//!             prog_mem,
//!             std::alloc::Layout::from_size_align(footprint as usize, align as usize).unwrap()
//!         );
//!     } else {
//!         let err = core::ffi::CStr::from_ptr(fd_sbpf_strerror());
//!         println!("Failed to parse: {err:?}");
//!     }
//! }
//! ```

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::MaybeUninit;

    #[test]
    fn test_strerror() {
        unsafe {
            let error_msg = fd_sbpf_strerror();
            assert!(!error_msg.is_null());
        }
    }

    #[test]
    fn test_ixn_conversion() {
        unsafe {
            let test_val: u64 = 0x1234567890abcdef;
            let instr = fd_sbpf_instr(test_val);
            let converted_back = fd_sbpf_ulong(instr);
            assert_eq!(test_val, converted_back);
        }
    }

    #[test]
    fn test_fn_detect() {
        unsafe {
            // function start (opcode 0x07, dst_reg 0x0A)
            let function_start_val: u64 = 0x07 | (0x0A << 8); // opcode=0x07, dst_reg=0x0A
            let instr = fd_sbpf_instr(function_start_val);
            assert_eq!(fd_sbpf_is_function_start(instr), 1);
            // function end (opcode 0x05 or 0x9D)
            let function_end_val1: u64 = 0x05; // opcode=0x05
            let instr1 = fd_sbpf_instr(function_end_val1);
            assert_eq!(fd_sbpf_is_function_end(instr1), 1);
            let function_end_val2: u64 = 0x9D; // opcode=0x9D
            let instr2 = fd_sbpf_instr(function_end_val2);
            assert_eq!(fd_sbpf_is_function_end(instr2), 1);
            // non-function ixn
            let normal_val: u64 = 0x04; // opcode=0x04
            let normal_instr = fd_sbpf_instr(normal_val);
            assert_eq!(fd_sbpf_is_function_start(normal_instr), 0);
            assert_eq!(fd_sbpf_is_function_end(normal_instr), 0);
        }
    }

    #[test]
    fn test_progalign_and_footprint() {
        unsafe {
            let align = fd_sbpf_program_align();
            assert!(align > 0);
            assert!(align.is_power_of_two());
            let mut info = MaybeUninit::<fd_sbpf_elf_info_t>::uninit();
            let info_ptr = info.as_mut_ptr();
            core::ptr::write_bytes(info_ptr, 0, 1);
            let info = info.assume_init();
            let footprint = fd_sbpf_program_footprint(&info);
            assert!(footprint >= core::mem::size_of::<fd_sbpf_program_t>() as u64);
        }
    }
}
