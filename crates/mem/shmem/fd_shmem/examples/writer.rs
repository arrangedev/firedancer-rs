use fd_shmem::{FdShmem, JoinMode, PageSize, ShmemResult};
use std::time::{SystemTime, UNIX_EPOCH};

const REGION_NAME: &str = "fd_shmem_demo";
const REGION_SIZE_PAGES: u64 = 1; // 4KB
const REGION_MODE: u64 = 0o666;

fn main() -> ShmemResult<()> {
    create_region()?;
    write_region()?;

    println!("region created; run `cargo run --example reader -p fd_shmem`");

    Ok(())
}

fn create_region() -> ShmemResult<()> {
    let page_size = PageSize::Normal;
    let cpu_idx = 0; // cpu = 0

    println!("creating '{REGION_NAME}' with:");
    println!(
        "  - page-size: {} ({} bytes)",
        page_size.as_str(),
        page_size.size_bytes()
    );
    println!("  - page-count: {}", REGION_SIZE_PAGES);
    println!(
        "  - total-size: {} bytes",
        page_size.size_bytes() as u64 * REGION_SIZE_PAGES
    );
    println!("  - cpu_idx: {}", cpu_idx);
    println!("  - mode: {:o}", REGION_MODE);

    // Try to create the region
    match FdShmem::create(
        REGION_NAME,
        page_size,
        REGION_SIZE_PAGES,
        cpu_idx,
        REGION_MODE,
    ) {
        Ok(()) => {
            println!("✓ Shmem region created");
        }
        Err(e) => {
            println!("⚠ failed to create shmem region: {e}");
            return Ok(());
        }
    }

    match FdShmem::info(REGION_NAME, Some(page_size)) {
        Ok(info) => {
            println!("✓ region info verified:");
            println!(
                "  - page-size: {} ({} bytes)",
                info.page_size.as_str(),
                info.page_size.size_bytes()
            );
            println!("  - page-count: {}", info.page_count);
            println!("  - Total size: {} bytes", info.total_size());
        }
        Err(e) => {
            println!("⚠ failed to query region info: {}", e);
        }
    }

    Ok(())
}

fn write_region() -> ShmemResult<()> {
    match FdShmem::join(REGION_NAME, JoinMode::ReadWrite, true) {
        Ok(mut shmem_join) => {
            println!("✓ joined shmem region");

            let info = shmem_join.info();
            println!("info:");
            println!("  - name: {}", info.name);
            println!("  - addr: {:p}", info.address);
            println!("  - size: {} bytes", info.total_size());
            println!("  - mode: {:?}", info.mode);
            println!("  - refcount: {}", info.ref_count);

            match shmem_join.as_mut_slice() {
                Ok(memory) => {
                    write_data(memory)?;
                }
                Err(e) => {
                    println!("⚠ failed to get mut access: {e}");
                }
            }

            println!("✓ data written");
        }
        Err(e) => {
            println!("⚠ failed to join shmem region: {e}");
        }
    }

    Ok(())
}

fn write_data(memory: &mut [u8]) -> ShmemResult<()> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let message = format!("Hello from fd_shmem writer - timestamp: {timestamp}");
    let message_bytes = message.as_bytes();
    if message_bytes.len() > memory.len() - 8 {
        return Err(fd_shmem::ShmemError::InvalidArgs(
            "message too large for shmem".to_string(),
        ));
    }

    let len_bytes = (message_bytes.len() as u64).to_le_bytes();
    memory[0..8].copy_from_slice(&len_bytes);
    memory[8..8 + message_bytes.len()].copy_from_slice(message_bytes);
    for i in (8 + message_bytes.len())..memory.len() {
        memory[i] = (i % 256) as u8;
    }

    println!("✓ wrote {} bytes:", message_bytes.len() + 8);
    println!("  - message: '{}'", message);
    println!("  - raw: {:?}", memory);
    println!();

    Ok(())
}
