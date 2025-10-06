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
    // /// initialize cpu topology using automatic detection from the operating system
    // pub fn new() -> Result<Self> {
    //     unsafe { crate::init() };

    //     let mut cpus = unsafe { core::mem::zeroed::<sys::fd_topo_cpus_t>() };
    //     unsafe { sys::fd_topo_cpus_init(&mut cpus) };

    //     Ok(Self { inner: cpus })
    // }

    /// initialize cpu topology with custom configuration
    ///
    /// For a 6-core system with 1 NUMA node:
    /// ```rust
    /// let t = CpuTopology::new_custom(6, 1)?;
    /// ```
    pub fn new_custom(cpu_count: usize, numa_node_count: usize) -> Result<Self> {
        unsafe { crate::init() };

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

    /// uses `get_nprocs()` internally, which avoids parsing sysfs
    pub fn new_simple() -> Result<Self> {
        unsafe { crate::init() };

        let cpu_count = unsafe { sys::fd_numa_cpu_cnt() } as usize;
        let numa_node_count = unsafe { sys::fd_numa_node_cnt() } as usize;

        if cpu_count == 0 {
            return Err(crate::TopoError::SystemError);
        }

        Self::new_custom(cpu_count, numa_node_count.max(1))
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

impl Default for CpuTopology {
    fn default() -> Self {
        Self::new().expect("Failed to initialize CPU topology")
    }
}

unsafe impl Send for CpuTopology {}
unsafe impl Sync for CpuTopology {}
