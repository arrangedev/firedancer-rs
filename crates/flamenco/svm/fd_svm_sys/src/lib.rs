#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_align_footprint() {
        unsafe {
            let align = fd_vm_align();
            let footprint = fd_vm_footprint();

            assert!(align > 0);
            assert!(footprint > 0);
            assert_eq!(align, FD_VM_ALIGN as u64);
            assert_eq!(footprint, FD_VM_FOOTPRINT as u64);
        }
    }

    #[test]
    fn test_vm_memory_allocation() {
        unsafe {
            let align = fd_vm_align() as usize;
            let footprint = fd_vm_footprint() as usize;

            let layout = std::alloc::Layout::from_size_align(footprint, align).unwrap();
            let mem = std::alloc::alloc(layout);
            assert!(!mem.is_null());

            let vm_mem = fd_vm_new(mem as *mut std::ffi::c_void);
            assert!(!vm_mem.is_null());

            fd_vm_delete(vm_mem);
            std::alloc::dealloc(mem, layout);
        }
    }
}
