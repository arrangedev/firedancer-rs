use fd_shmem::{FdShmem, PageSize, ShmemResult};

fn main() -> ShmemResult<()> {
    system_info()?;
    page_size()?;
    name_validation()?;
    memory_allocation()?;
    Ok(())
}

fn system_info() -> ShmemResult<()> {
    let numa_count = FdShmem::numa_count();
    let cpu_count = FdShmem::cpu_count();

    println!("numa-nodes: {}", numa_count);
    println!("logical-cpus: {}", cpu_count);

    if numa_count > 0 && cpu_count > 0 {
        for cpu_idx in 0..core::cmp::min(cpu_count, 8) {
            if let Some(numa_idx) = FdShmem::numa_idx_for_cpu(cpu_idx) {
                println!("  cpu {} -> numa-node {}", cpu_idx, numa_idx);
            }
        }

        for numa_idx in 0..numa_count {
            if let Some(cpu_idx) = FdShmem::cpu_idx_for_numa(numa_idx) {
                println!("  numa-node {} -> cpu {} (first)", numa_idx, cpu_idx);
            }
        }
    } else {
        println!("  numa not supported");
    }

    Ok(())
}

fn page_size() -> ShmemResult<()> {
    let page_sizes = [PageSize::Normal, PageSize::Huge, PageSize::Gigantic];

    for page_size in &page_sizes {
        println!(
            "page-size: {} ({} bytes, log2: {})",
            page_size.as_str(),
            page_size.size_bytes(),
            page_size.log2_size()
        );
    }

    println!("page-size conversions:");
    if let Some(ps) = PageSize::from_str("normal") {
        println!("  'normal' -> {:?}", ps);
    }
    if let Some(ps) = PageSize::from_str("huge") {
        println!("  'huge' -> {:?}", ps);
    }
    if let Some(ps) = PageSize::from_str("gigantic") {
        println!("  'gigantic' -> {:?}", ps);
    }

    if let Some(ps) = PageSize::from_raw(4096) {
        println!("  4096 -> {:?}", ps);
    }
    if let Some(ps) = PageSize::from_raw(2097152) {
        println!("  2097152 -> {:?}", ps);
    }

    Ok(())
}

fn name_validation() -> ShmemResult<()> {
    let test_names = [
        "valid_region",
        "test123",
        "my-region.test",
        "",              // Invalid
        "invalid\0name", // Invalid
    ];

    for name in &test_names {
        let is_valid = FdShmem::validate_name(name);
        println!(
            "  '{}' -> {}",
            name,
            if is_valid { "valid" } else { "invalid" }
        );
    }

    Ok(())
}

fn memory_allocation() -> ShmemResult<()> {
    let page_size = PageSize::Normal;
    let page_count = 1; // 4KB
    let cpu_idx = 0;

    println!(
        "allocating {} page of {} bytes on CPU {}...",
        page_count,
        page_size.size_bytes(),
        cpu_idx
    );

    match FdShmem::acquire(page_size, page_count, cpu_idx) {
        Ok(ptr) => {
            println!("  allocated at address: {:p}", ptr);

            unsafe {
                let slice = core::slice::from_raw_parts_mut(ptr, page_size.size_bytes());
                slice[0] = 0x42;
                slice[page_size.size_bytes() - 1] = 0x24;

                println!(
                    "  wrote test pattern: first byte = 0x{:02x}, last byte = 0x{:02x}",
                    slice[0],
                    slice[page_size.size_bytes() - 1]
                );
            }

            match FdShmem::release(ptr, page_size, page_count) {
                Ok(()) => println!("  released memory"),
                Err(e) => println!("  warning: failed to release memory: {e}"),
            }
        }
        Err(e) => {
            println!("  failed to allocate: {e}");
        }
    }

    Ok(())
}
