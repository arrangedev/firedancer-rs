use core::ffi::CStr;

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
    /// initialize cpu topology with the desired config
    #[inline]
    pub fn new_custom(
        progname: &'static CStr,
        cpu_count: usize,
        numa_node_count: usize,
    ) -> Result<Self> {
        unsafe { crate::init(progname) };

        let mut cpus = unsafe { core::mem::zeroed::<sys::fd_topo_cpus_t>() };

        cpus.cpu_cnt = cpu_count as u64;
        cpus.numa_node_cnt = numa_node_count as u64;
        for i in 0..cpu_count.min(1024) {
            cpus.cpu[i].idx = i as u64;
            cpus.cpu[i].online = 1;
            cpus.cpu[i].numa_node = (i % numa_node_count) as u64;
            cpus.cpu[i].sibling = u64::MAX;
        }

        Ok(Self { inner: cpus })
    }

    /// initialize cpu topology with the default settings.
    ///
    /// uses `get_nprocs()` internally, which avoids parsing sysfs
    /// and might work better on older systems
    #[inline]
    pub fn new_simple(progname: &'static CStr) -> Result<Self> {
        unsafe { crate::init(progname) };

        let cpu_count = unsafe { sys::fd_numa_cpu_cnt() } as usize;
        let numa_node_count = unsafe { sys::fd_numa_node_cnt() } as usize;

        if cpu_count == 0 {
            return Err(crate::TopoError::SystemError);
        }

        Self::new_custom(progname, cpu_count, numa_node_count.max(1))
    }

    #[inline]
    pub fn numa_node_count(&self) -> usize {
        self.inner.numa_node_cnt as usize
    }

    #[inline]
    pub fn cpu_count(&self) -> usize {
        self.inner.cpu_cnt as usize
    }

    #[inline]
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

    #[inline]
    pub fn cpus(&self) -> Vec<Cpu> {
        (0..self.cpu_count()).filter_map(|i| self.cpu(i)).collect()
    }

    #[inline]
    pub fn cpus_on_numa_node(&self, numa_node: usize) -> Vec<Cpu> {
        self.cpus()
            .into_iter()
            .filter(|cpu| cpu.numa_node == numa_node)
            .collect()
    }

    #[inline]
    pub fn online_cpus(&self) -> Vec<Cpu> {
        self.cpus().into_iter().filter(|cpu| cpu.online).collect()
    }

    #[inline]
    pub fn print(&mut self) {
        unsafe {
            sys::fd_topo_cpus_printf(&mut self.inner);
        }
    }
}

impl Default for CpuTopology {
    fn default() -> Self {
        Self::new_simple(c"tch_fd_topo").expect("Failed to initialize CPU topology")
    }
}

unsafe impl Send for CpuTopology {}
unsafe impl Sync for CpuTopology {}
