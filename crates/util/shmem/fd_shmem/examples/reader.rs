use fd_shmem::{FdShmem, JoinMode, ShmemResult};
use std::time::{SystemTime, UNIX_EPOCH};

const REGION_NAME: &str = "fd_shmem_demo";

fn main() -> ShmemResult<()> {
    check_region_exists()?;
    read_region()?;
    cleanup_region()?;

    println!("\ndata read successfully");

    Ok(())
}

fn check_region_exists() -> ShmemResult<()> {
    match FdShmem::info(REGION_NAME, None) {
        Ok(info) => {
            println!("✓ Found region '{REGION_NAME}':");
            println!(
                "  - page-size: {} ({} bytes)",
                info.page_size.as_str(),
                info.page_size.size_bytes()
            );
            println!("  - page-count: {}", info.page_count);
            println!("  - total-size: {} bytes", info.total_size());
        }
        Err(e) => {
            println!("⚠ region '{REGION_NAME}' not found: {e}");
            println!("  make sure to run `cargo run --example writer -p fd_shmem`");
            return Ok(());
        }
    }

    match FdShmem::query_by_name(REGION_NAME) {
        Ok(join_info) => {
            println!("  - name: {}", join_info.name);
            println!("  - addr: {:p}", join_info.address);
            println!("  - refcount: {}", join_info.ref_count);
            println!("  - mode: {:?}", join_info.mode);
        }
        Err(_) => {
            println!("ℹ region exists but is not currently joined by this process");
        }
    }

    Ok(())
}

fn read_region() -> ShmemResult<()> {
    match FdShmem::join(REGION_NAME, JoinMode::ReadOnly, true) {
        Ok(shmem_join) => {
            println!("✓ joined region");

            let info = shmem_join.info();
            println!("info:");
            println!("  - name: {}", info.name);
            println!("  - addr: {:p}", info.address);
            println!("  - size: {} bytes", info.total_size());
            println!("  - mode: {:?}", info.mode);
            println!("  - refcount: {}", info.ref_count);

            let memory = shmem_join.as_slice();
            read_data(memory)?;

            println!("✓ data read");
        }
        Err(e) => {
            println!("⚠ failed to join region: {e}");
            println!("  make sure the writer example created the region first");
        }
    }

    Ok(())
}

fn read_data(memory: &[u8]) -> ShmemResult<()> {
    if memory.len() < 8 {
        return Err(fd_shmem::ShmemError::InvalidArgs(
            "region too small".to_string(),
        ));
    }

    let mut len_bytes = [0u8; 8];
    len_bytes.copy_from_slice(&memory[0..8]);
    let message_len = u64::from_le_bytes(len_bytes) as usize;

    println!("✓ read message length: {} bytes", message_len);

    if message_len > memory.len() - 8 {
        return Err(fd_shmem::ShmemError::InvalidArgs(
            "invalid message length in region".to_string(),
        ));
    }

    let message_bytes = &memory[8..8 + message_len];
    match std::str::from_utf8(message_bytes) {
        Ok(message) => {
            println!("✓ message: '{message}'");

            if let Some(timestamp_str) = message.split("Timestamp: ").nth(1) {
                if let Ok(timestamp) = timestamp_str.parse::<u64>() {
                    let current_time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    let age = current_time.saturating_sub(timestamp);
                    println!("  - age: {age} seconds");
                }
            }
        }
        Err(e) => {
            println!("⚠ failed to parse message: {e}");
        }
    }

    Ok(())
}

fn cleanup_region() -> ShmemResult<()> {
    println!("joined regions:");
    let mut join_count = 0;
    for join_info in fd_shmem::ShmemIterator::new() {
        join_count += 1;
        println!(
            "  {}. {} ({} bytes, ref_count: {})",
            join_count,
            join_info.name,
            join_info.total_size(),
            join_info.ref_count
        );
    }

    if join_count == 0 {
        println!("  no regions currently joined");
    }

    match FdShmem::unlink(REGION_NAME, fd_shmem::PageSize::Normal) {
        Ok(()) => println!("✓ region unlinked"),
        Err(e) => println!("⚠ failed to unlink region: {e}"),
    }

    Ok(())
}
