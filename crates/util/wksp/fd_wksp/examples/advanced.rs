use fd_wksp::WorkspaceBuilder;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Advanced Firedancer Workspace Features ===\n");

    // Create a larger workspace for advanced operations
    let wksp = WorkspaceBuilder::new()
        .name("advanced-workspace")
        .page_size(4096)
        .page_count(512) // 2MB total
        .cpu_index(0)
        .seed(12345)
        .part_max(200) // Explicit partition limit
        .build_anonymous()?;

    println!("Created advanced workspace:");
    println!("  Name: {}", wksp.name());
    println!("  Seed: {}", wksp.seed());
    println!("  Part Max: {}", wksp.part_max());
    println!("  Data Max: {} MB", wksp.data_max() / (1024 * 1024));
    println!();

    // Create a complex allocation pattern
    println!("=== Complex Allocation Pattern ===");
    let mut allocations = Vec::new();

    // Allocate different sizes with different tags
    for i in 1..=10 {
        let size = 1024 * i; // 1KB, 2KB, 3KB, ... 10KB
        let align = if i % 2 == 0 { 64 } else { 32 };
        let tag = ((i % 3) + 1) as u64; // Tags 1, 2, 3

        let mut alloc = wksp.allocate(size, align, tag)?;

        // Write a pattern to each allocation
        let data = alloc.as_mut_slice();
        for j in 0..data.len() {
            data[j] = ((i * 37 + j) % 256) as u8;
        }

        println!(
            "  Allocation {}: {} bytes, align={}, tag={}, gaddr={}",
            i,
            size,
            align,
            tag,
            alloc.global_address().as_u64()
        );

        allocations.push(alloc);
    }
    println!();

    // Analyze allocation patterns
    println!("=== Allocation Analysis ===");
    for tag in 1..=3 {
        let tag_allocs = wksp.query_by_tag(&[tag])?;
        let total_size: u64 = tag_allocs
            .iter()
            .map(|info| info.gaddr_hi.as_u64() - info.gaddr_lo.as_u64())
            .sum();
        println!(
            "Tag {}: {} allocations, {} bytes total",
            tag,
            tag_allocs.len(),
            total_size
        );
    }
    println!();

    // Demonstrate workspace verification
    println!("=== Workspace Integrity ===");
    match wksp.verify() {
        Ok(()) => println!("Workspace integrity check: PASSED"),
        Err(e) => println!("Workspace integrity check: FAILED - {}", e),
    }
    println!();

    // Checkpointing demonstration
    println!("=== Workspace Checkpointing ===");
    let checkpoint_path = "/tmp/wksp_checkpoint.bin";

    // Remove any existing checkpoint file
    let _ = fs::remove_file(checkpoint_path);

    match wksp.checkpoint(
        checkpoint_path,
        0o644, // File permissions
        0,     // Use default checkpoint style
        Some("Advanced example checkpoint"),
    ) {
        Ok(()) => {
            println!("Successfully created checkpoint at: {}", checkpoint_path);

            // Check file size
            if let Ok(metadata) = fs::metadata(checkpoint_path) {
                println!("Checkpoint file size: {} bytes", metadata.len());
            }
        }
        Err(e) => println!("Checkpoint failed: {}", e),
    }
    println!();

    // Demonstrate rebuilding with different seed
    println!("=== Workspace Rebuilding ===");
    let old_seed = wksp.seed();
    match wksp.rebuild(54321) {
        Ok(()) => {
            println!("Workspace rebuilt successfully");
            println!("  Old seed: {}", old_seed);
            println!("  New seed: {}", wksp.seed());
        }
        Err(e) => println!("Rebuild failed: {}", e),
    }
    println!();

    // Large allocation test
    println!("=== Large Allocation Test ===");
    let large_size = 128 * 1024; // 128KB
    match wksp.allocate(large_size, 4096, 100) {
        Ok(mut large_alloc) => {
            println!("Successfully allocated {} KB", large_size / 1024);
            println!(
                "  Global address: {}",
                large_alloc.global_address().as_u64()
            );
            println!(
                "  Alignment: {} bytes",
                large_alloc.as_ptr() as usize % 4096
            );

            // Fill with a pattern and verify
            {
                let data = large_alloc.as_mut_slice();
                for (i, byte) in data.iter_mut().enumerate() {
                    *byte = (i % 251) as u8; // Use a prime to avoid patterns
                }

                // Verify first and last few bytes
                println!("  First 8 bytes: {:02X?}", &data[0..8]);
                println!("  Last 8 bytes: {:02X?}", &data[data.len() - 8..]);
            }
        }
        Err(e) => println!("Large allocation failed: {}", e),
    }
    println!();

    // Detailed usage analysis
    println!("=== Detailed Usage Analysis ===");
    let detailed_usage = wksp.usage(&[]);
    println!("Workspace utilization:");
    println!(
        "  Partitions: {}/{} ({:.1}%)",
        detailed_usage.used_cnt,
        detailed_usage.total_max,
        100.0 * detailed_usage.used_cnt as f64 / detailed_usage.total_max as f64
    );
    println!(
        "  Memory: {}/{} bytes ({:.1}%)",
        detailed_usage.used_sz,
        detailed_usage.total_sz,
        100.0 * detailed_usage.used_sz as f64 / detailed_usage.total_sz as f64
    );

    // Fragmentation analysis
    let avg_used_size = if detailed_usage.used_cnt > 0 {
        detailed_usage.used_sz / detailed_usage.used_cnt
    } else {
        0
    };
    let avg_free_size = if detailed_usage.free_cnt > 0 {
        detailed_usage.free_sz / detailed_usage.free_cnt
    } else {
        0
    };

    println!("  Average used partition: {} bytes", avg_used_size);
    println!("  Average free partition: {} bytes", avg_free_size);
    println!();

    // Test allocation limits
    println!("=== Allocation Limits Test ===");
    let mut stress_allocations = Vec::new();
    let mut allocation_count = 0;

    // Try to allocate many small blocks until we run out of partitions
    loop {
        match wksp.allocate(64, 8, 200) {
            Ok(alloc) => {
                stress_allocations.push(alloc);
                allocation_count += 1;
            }
            Err(_) => break,
        }

        // Safety limit to avoid infinite loop
        if allocation_count > 1000 {
            break;
        }
    }

    println!(
        "Created {} additional small allocations before limit",
        allocation_count
    );

    let final_usage = wksp.usage(&[]);
    println!(
        "Final partition usage: {}/{}",
        final_usage.used_cnt, final_usage.total_max
    );
    println!();

    // Cleanup by tag
    println!("=== Selective Cleanup ===");
    println!(
        "Before cleanup: {} total allocations",
        wksp.query_by_tag(&[1, 2, 3, 100, 200])?.len()
    );

    // Free all stress test allocations (tag 200)
    wksp.free_by_tag(&[200]);
    println!(
        "After freeing tag 200: {} allocations",
        wksp.query_by_tag(&[1, 2, 3, 100, 200])?.len()
    );

    // Free tag 1 and 3, keep tag 2
    wksp.free_by_tag(&[1, 3]);
    let remaining = wksp.query_by_tag(&[1, 2, 3, 100])?;
    println!("After freeing tags 1&3: {} allocations", remaining.len());
    for info in &remaining {
        println!(
            "  Remaining: gaddr={}, tag={}",
            info.gaddr_lo.as_u64(),
            info.tag
        );
    }
    println!();

    // Restore from checkpoint (if available)
    if fs::metadata(checkpoint_path).is_ok() {
        println!("=== Checkpoint Restoration ===");

        // First reset the workspace
        wksp.reset(99999);
        println!(
            "Workspace reset - used partitions: {}",
            wksp.usage(&[]).used_cnt
        );

        // Restore from checkpoint
        match wksp.restore(checkpoint_path, 67890) {
            Ok(()) => {
                println!("Successfully restored from checkpoint");
                println!("  New seed: {}", wksp.seed());

                let restored_usage = wksp.usage(&[]);
                println!("  Restored partitions: {}", restored_usage.used_cnt);

                // Verify some allocations were restored
                let restored_allocs = wksp.query_by_tag(&[1, 2, 3])?;
                println!("  Restored tagged allocations: {}", restored_allocs.len());

                // Verify data integrity in restored allocations
                for info in restored_allocs.iter().take(3) {
                    if let Ok(laddr) = wksp.gaddr_to_laddr(info.gaddr_lo) {
                        let size = (info.gaddr_hi.as_u64() - info.gaddr_lo.as_u64()) as usize;
                        if size >= 8 {
                            let data = unsafe { std::slice::from_raw_parts(laddr, 8) };
                            println!(
                                "    gaddr {} first 8 bytes: {:02X?}",
                                info.gaddr_lo.as_u64(),
                                data
                            );
                        }
                    }
                }
            }
            Err(e) => println!("Restore failed: {}", e),
        }

        // Clean up checkpoint file
        let _ = fs::remove_file(checkpoint_path);
        println!();
    }

    // Performance timing example
    println!("=== Performance Timing ===");
    use std::time::Instant;

    let start = Instant::now();
    let mut perf_allocs = Vec::new();

    for _i in 0..100 {
        if let Ok(alloc) = wksp.allocate(1024, 32, 300) {
            perf_allocs.push(alloc);
        }
    }

    let alloc_time = start.elapsed();
    println!(
        "100 allocations took: {:?} ({:.2} μs/alloc)",
        alloc_time,
        alloc_time.as_micros() as f64 / 100.0
    );

    let start = Instant::now();
    wksp.free_by_tag(&[300]);
    let free_time = start.elapsed();
    println!("Bulk free took: {:?}", free_time);
    println!();

    // Final workspace state
    println!("=== Final Workspace State ===");
    let final_state = wksp.usage(&[]);
    println!("Final usage:");
    println!(
        "  Partitions: {}/{}",
        final_state.used_cnt, final_state.total_max
    );
    println!(
        "  Memory: {} bytes ({:.1}% of {} bytes)",
        final_state.used_sz,
        100.0 * final_state.used_sz as f64 / final_state.total_sz as f64,
        final_state.total_sz
    );
    println!("  Owner: {}", wksp.owner());

    println!("\n=== Advanced Example Completed Successfully! ===");

    Ok(())
}
