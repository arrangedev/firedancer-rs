use fd_wksp::{WorkspaceBuilder, WorkspaceError};

fn main() -> Result<(), WorkspaceError> {
    println!("=== Firedancer Workspace (wksp) Usage Example ===\n");

    // Create an anonymous workspace for testing
    let wksp = WorkspaceBuilder::new()
        .name("test-workspace")
        .page_size(4096)
        .page_count(256) // 1MB total
        .cpu_index(0)
        .seed(42)
        .build_anonymous()?;

    println!("Created workspace:");
    println!("  Name: {}", wksp.name());
    println!("  Seed: {}", wksp.seed());
    println!("  Part Max: {}", wksp.part_max());
    println!("  Data Max: {} bytes", wksp.data_max());
    println!();

    // Basic allocation
    println!("=== Basic Allocation ===");
    let mut allocation1 = wksp.allocate(1024, 64, 1)?;
    println!("Allocated 1024 bytes with 64-byte alignment, tag=1");
    println!(
        "  Global address: {}",
        allocation1.global_address().as_u64()
    );
    println!("  Local pointer: {:p}", allocation1.as_ptr());
    println!("  Size: {} bytes", allocation1.size());
    println!("  Tag: {}", allocation1.tag());
    println!();

    // Write some data
    {
        let data = allocation1.as_mut_slice();
        data[0..12].copy_from_slice(b"Hello, wksp!");
        println!(
            "Wrote data: {:?}",
            std::str::from_utf8(&data[0..12]).unwrap()
        );
    }

    // Address conversion
    println!("=== Address Conversion ===");
    let gaddr = allocation1.global_address();
    let laddr = wksp.gaddr_to_laddr(gaddr)?;
    let gaddr_back = wksp.laddr_to_gaddr(laddr)?;
    println!(
        "Global -> Local -> Global: {} -> {:p} -> {}",
        gaddr.as_u64(),
        laddr,
        gaddr_back.as_u64()
    );
    println!();

    // Multiple allocations with different tags
    println!("=== Multiple Allocations ===");
    let allocation2 = wksp.allocate(512, 32, 2)?;
    let allocation3 = wksp.allocate(256, 16, 1)?; // Same tag as allocation1
    let allocation4 = wksp.allocate(128, 8, 3)?;

    println!("Allocated additional blocks:");
    println!(
        "  Block 2: {} bytes, tag={}",
        allocation2.size(),
        allocation2.tag()
    );
    println!(
        "  Block 3: {} bytes, tag={}",
        allocation3.size(),
        allocation3.tag()
    );
    println!(
        "  Block 4: {} bytes, tag={}",
        allocation4.size(),
        allocation4.tag()
    );
    println!();

    // Query allocations by tag
    println!("=== Tag Queries ===");
    let tag1_allocs = wksp.query_by_tag(&[1])?;
    println!("Allocations with tag 1: {}", tag1_allocs.len());
    for (i, info) in tag1_allocs.iter().enumerate() {
        println!(
            "  #{}: gaddr range [{}, {}), tag={}",
            i + 1,
            info.gaddr_lo.as_u64(),
            info.gaddr_hi.as_u64(),
            info.tag
        );
    }

    let tag2_allocs = wksp.query_by_tag(&[2])?;
    println!("Allocations with tag 2: {}", tag2_allocs.len());

    let all_allocs = wksp.query_by_tag(&[1, 2, 3])?;
    println!("Total allocations with tags 1,2,3: {}", all_allocs.len());
    println!();

    // Workspace usage statistics
    println!("=== Usage Statistics ===");
    let usage = wksp.usage(&[]);
    println!("Overall usage:");
    println!(
        "  Total partitions: {} / {}",
        usage.total_cnt, usage.total_max
    );
    println!("  Total size: {} bytes", usage.total_sz);
    println!(
        "  Used: {} partitions, {} bytes",
        usage.used_cnt, usage.used_sz
    );
    println!(
        "  Free: {} partitions, {} bytes",
        usage.free_cnt, usage.free_sz
    );

    let tag1_usage = wksp.usage(&[1]);
    println!("Tag 1 usage:");
    println!(
        "  Used: {} partitions, {} bytes",
        tag1_usage.used_cnt, tag1_usage.used_sz
    );
    println!();

    // Allocation manipulation
    println!("=== Allocation Manipulation ===");
    {
        let mut alloc = allocation2;
        println!(
            "Before clear: first 4 bytes = {:?}",
            &alloc.as_slice()[0..4]
        );

        // Fill with pattern
        alloc.fill(0xAB);
        println!(
            "After fill(0xAB): first 4 bytes = {:02X?}",
            &alloc.as_slice()[0..4]
        );

        // Clear to zeros
        alloc.clear();
        println!(
            "After clear: first 4 bytes = {:02X?}",
            &alloc.as_slice()[0..4]
        );
    }
    println!();

    // Free by tag
    println!("=== Free by Tag ===");
    println!(
        "Before free: {} total allocations",
        wksp.query_by_tag(&[1, 2, 3])?.len()
    );

    wksp.free_by_tag(&[2]); // Free allocation with tag 2

    let remaining = wksp.query_by_tag(&[1, 2, 3])?;
    println!(
        "After freeing tag 2: {} remaining allocations",
        remaining.len()
    );
    for info in &remaining {
        println!(
            "  Remaining: gaddr={}, tag={}",
            info.gaddr_lo.as_u64(),
            info.tag
        );
    }
    println!();

    // Allocation with detailed range
    println!("=== Allocation with Range Info ===");
    let (allocation5, lo, hi) = wksp.allocate_at_least(2048, 128, 5)?;
    println!("Allocated at least 2048 bytes:");
    println!(
        "  Returned address: {}",
        allocation5.global_address().as_u64()
    );
    println!("  Actual range: [{}, {})", lo.as_u64(), hi.as_u64());
    println!("  Actual size: {} bytes", hi.as_u64() - lo.as_u64());
    println!();

    // Manual memory management
    println!("=== Manual Memory Management ===");
    let raw_gaddr = allocation5.into_raw(); // Take ownership, prevent auto-cleanup
    println!("Converted allocation to raw gaddr: {}", raw_gaddr.as_u64());

    // Manually free it
    wksp.free_gaddr(raw_gaddr);
    println!("Manually freed allocation");

    // Verify it's gone
    let tag5_allocs = wksp.query_by_tag(&[5])?;
    println!("Tag 5 allocations after manual free: {}", tag5_allocs.len());
    println!();

    // Workspace verification
    println!("=== Workspace Verification ===");
    match wksp.verify() {
        Ok(()) => println!("Workspace verification: PASSED"),
        Err(e) => println!("Workspace verification: FAILED - {}", e),
    }
    println!();

    // Reset demonstration
    println!("=== Workspace Reset ===");
    let before_reset = wksp.usage(&[]);
    println!(
        "Before reset: {} used partitions, {} used bytes",
        before_reset.used_cnt, before_reset.used_sz
    );

    wksp.reset(123); // Reset with new seed

    let after_reset = wksp.usage(&[]);
    println!(
        "After reset: {} used partitions, {} used bytes",
        after_reset.used_cnt, after_reset.used_sz
    );
    println!("New seed: {}", wksp.seed());
    println!();

    // Multi-NUMA example (would require appropriate system setup in real usage)
    println!("=== Multi-NUMA Example ===");
    let multi_numa_wksp = WorkspaceBuilder::new()
        .name("multi-numa-test")
        .page_size(4096)
        .multi_numa(vec![64, 64], vec![0, 1]) // 64 pages on CPU 0 and 1
        .seed(456)
        .build_anonymous()?;

    println!("Created multi-NUMA workspace: {}", multi_numa_wksp.name());
    println!("  Seed: {}", multi_numa_wksp.seed());
    println!("  Data Max: {} bytes", multi_numa_wksp.data_max());

    // Allocate from the multi-NUMA workspace
    let numa_alloc = multi_numa_wksp.allocate(4096, 4096, 100)?;
    println!(
        "  Allocated 4KB page: gaddr={}",
        numa_alloc.global_address().as_u64()
    );
    println!();

    // Error handling examples
    println!("=== Error Handling ===");

    // Invalid size
    match wksp.allocate(0, 64, 1) {
        Err(e) => println!("Expected error for zero size: {}", e),
        Ok(_) => println!("Unexpected success for zero size"),
    }

    // Invalid tag
    match wksp.allocate(1024, 64, 0) {
        Err(e) => println!("Expected error for zero tag: {}", e),
        Ok(_) => println!("Unexpected success for zero tag"),
    }

    // Invalid alignment
    match wksp.allocate(1024, 63, 1) {
        Err(e) => println!("Expected error for non-power-of-2 alignment: {}", e),
        Ok(_) => println!("Unexpected success for invalid alignment"),
    }
    println!();

    // Utility functions
    println!("=== Utility Functions ===");
    let align = fd_wksp::utils::align();
    println!("Workspace alignment requirement: {} bytes", align);

    let footprint = fd_wksp::utils::footprint(1000, 1024 * 1024);
    println!(
        "Footprint for 1000 partitions, 1MB data: {} bytes",
        footprint
    );

    let part_max_est = fd_wksp::utils::part_max_est(16 * 1024 * 1024, 64 * 1024);
    println!(
        "Estimated max partitions for 16MB footprint, 64KB typical: {}",
        part_max_est
    );

    let data_max_est = fd_wksp::utils::data_max_est(16 * 1024 * 1024, part_max_est);
    println!(
        "Estimated max data for 16MB footprint, {} partitions: {} bytes",
        part_max_est, data_max_est
    );
    println!();

    println!("=== Final Statistics ===");
    let final_usage = wksp.usage(&[]);
    println!("Final workspace usage:");
    println!(
        "  Total: {} partitions, {} bytes",
        final_usage.total_cnt, final_usage.total_sz
    );
    println!(
        "  Used: {} partitions, {} bytes",
        final_usage.used_cnt, final_usage.used_sz
    );
    println!(
        "  Free: {} partitions, {} bytes",
        final_usage.free_cnt, final_usage.free_sz
    );

    println!("\n=== Example Completed Successfully! ===");

    // Workspaces are automatically cleaned up when dropped
    Ok(())
}
