use crate::Result;
use fd_topo_sys as sys;

#[derive(Debug, Clone)]
pub struct Cpu {
    pub idx: usize,
    pub online: bool,
    pub numa_node: usize,
    /// sibling for hyperthreading
    pub sibling: usize,
}

#[repr(C)]
pub struct CpuTopology {
    inner: sys::fd_topo_cpus_t,
}

impl CpuTopology {
    pub fn new() -> Result<Self> {
        unsafe {
            let mut argc = 0i32;
            let mut argv: *mut *mut i8 = std::ptr::null_mut();
            sys::fd_boot(&mut argc, &mut argv);
        }

        let mut cpus = unsafe { core::mem::zeroed::<sys::fd_topo_cpus_t>() };

        unsafe {
            sys::fd_topo_cpus_init(&mut cpus);
        }

        Ok(Self { inner: cpus })
    }

    pub fn numa_node_count(&self) -> usize {
        self.inner.numa_node_cnt as usize
    }

    pub fn cpu_count(&self) -> usize {
        self.inner.cpu_cnt as usize
    }

    pub fn cpu(&self, index: usize) -> Option<Cpu> {
        if index >= self.cpu_count() {
            return None;
        }

        let cpu = &self.inner.cpu[index];
        Some(Cpu {
            idx: cpu.idx as usize,
            online: cpu.online != 0,
            numa_node: cpu.numa_node as usize,
            sibling: cpu.sibling as usize,
        })
    }

    pub fn cpus(&self) -> Vec<Cpu> {
        (0..self.cpu_count()).filter_map(|i| self.cpu(i)).collect()
    }

    pub fn cpus_on_numa_node(&self, numa_node: usize) -> Vec<Cpu> {
        self.cpus()
            .into_iter()
            .filter(|cpu| cpu.numa_node == numa_node)
            .collect()
    }

    pub fn online_cpus(&self) -> Vec<Cpu> {
        self.cpus().into_iter().filter(|cpu| cpu.online).collect()
    }

    pub fn print(&mut self) {
        unsafe {
            sys::fd_topo_cpus_printf(&mut self.inner);
        }
    }
}

impl Drop for CpuTopology {
    fn drop(&mut self) {
        unsafe {
            sys::fd_halt();
        }
    }
}

impl Default for CpuTopology {
    fn default() -> Self {
        Self::new().expect("Failed to initialize CPU topology")
    }
}

unsafe impl Send for CpuTopology {}
unsafe impl Sync for CpuTopology {}
