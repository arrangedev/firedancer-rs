use fd_wksp::WorkspaceBuilder;
use std::{fs, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wksp = WorkspaceBuilder::new()
        .name("workspace")
        .page_size(4096)
        .page_count(512)
        .cpu_index(0)
        .seed(12345)
        .part_max(200)
        .build_anonymous()?;

    println!("  name: {}", wksp.name());
    println!("  seed: {}", wksp.seed());
    println!("  part_max: {}", wksp.part_max());
    println!("  data_max: {} MB", wksp.data_max() / (1024 * 1024));

    let mut allocations = Vec::new();

    for i in 1..=10 {
        let size = 1024 * i;
        let align = if i % 2 == 0 { 64 } else { 32 };
        let tag = ((i % 3) + 1) as u64;

        let mut alloc = wksp.allocate(size, align, tag)?;

        let data = alloc.as_mut_slice();
        for j in 0..data.len() {
            data[j] = ((i * 37 + j) % 256) as u8;
        }

        println!(
            "  alloc-{}: size={}, align={}, tag={}, gaddr={}",
            i,
            size,
            align,
            tag,
            alloc.global_address().as_u64()
        );

        allocations.push(alloc);
    }
    println!();

    for tag in 1..=3 {
        let tag_allocs = wksp.query_by_tag(&[tag])?;
        let total_size: u64 = tag_allocs
            .iter()
            .map(|info| info.gaddr_hi.as_u64() - info.gaddr_lo.as_u64())
            .sum();
        println!(
            "tag-{}: allocs={}, bytes={}",
            tag,
            tag_allocs.len(),
            total_size
        );
    }
    println!();

    match wksp.verify() {
        Ok(()) => println!("wksp verified"),
        Err(e) => println!("wksp verification failed: {}", e),
    }
    println!();

    let checkpoint_path = "/tmp/wksp_checkpoint.bin";
    let _ = fs::remove_file(checkpoint_path);

    match wksp.checkpoint(checkpoint_path, 0o644, 0, Some("checkpoint")) {
        Ok(()) => {
            println!("checkpt_path={}", checkpoint_path);

            if let Ok(metadata) = fs::metadata(checkpoint_path) {
                println!("checkpt_size={}", metadata.len());
            }
        }
        Err(e) => println!("checkpt failed: {}", e),
    }
    println!();

    let old_seed = wksp.seed();
    match wksp.rebuild(54321) {
        Ok(()) => {
            println!("wksp rebuilt");
            println!("  old_seed={}", old_seed);
            println!("  new_seed={}", wksp.seed());
        }
        Err(e) => println!("rebuild failed: {}", e),
    }
    println!();

    let large_size = 128 * 1024;
    match wksp.allocate(large_size, 4096, 100) {
        Ok(mut large_alloc) => {
            println!("large_alloc={} KB", large_size / 1024);
            println!("  gaddr={}", large_alloc.global_address().as_u64());
            println!("  align={} bytes", large_alloc.as_ptr() as usize % 4096);

            {
                let data = large_alloc.as_mut_slice();
                for (i, byte) in data.iter_mut().enumerate() {
                    *byte = (i % 251) as u8;
                }

                println!("  first_8_bytes={:02X?}", &data[0..8]);
                println!("  last_8_bytes={:02X?}", &data[data.len() - 8..]);
            }
        }
        Err(e) => println!("large allocation failed: {}", e),
    }
    println!();

    let usage = wksp.usage(&[]);

    println!("utilization:");
    println!(
        "  partitions={}/{} ({:.1}%)",
        usage.used_cnt,
        usage.total_max,
        100.0 * usage.used_cnt as f64 / usage.total_max as f64
    );
    println!(
        "  memory={}/{} bytes ({:.1}%)",
        usage.used_sz,
        usage.total_sz,
        100.0 * usage.used_sz as f64 / usage.total_sz as f64
    );

    // Fragmentation analysis
    let avg_used_size = if usage.used_cnt > 0 {
        usage.used_sz / usage.used_cnt
    } else {
        0
    };
    let avg_free_size = if usage.free_cnt > 0 {
        usage.free_sz / usage.free_cnt
    } else {
        0
    };

    println!("  avg_used_partition={} bytes", avg_used_size);
    println!("  avg_free_partition={} bytes", avg_free_size);

    let mut stress_allocations = Vec::new();
    let mut allocation_count = 0;

    loop {
        match wksp.allocate(64, 8, 200) {
            Ok(alloc) => {
                stress_allocations.push(alloc);
                allocation_count += 1;
            }
            Err(_) => break,
        }

        if allocation_count > 1000 {
            break;
        }
    }

    let final_usage = wksp.usage(&[]);
    wksp.free_by_tag(&[200]);

    println!("allocs_before_limit={}", allocation_count);
    println!(
        "final_part_usage={}/{}",
        final_usage.used_cnt, final_usage.total_max
    );
    println!(
        "pre_cleanup={}",
        wksp.query_by_tag(&[1, 2, 3, 100, 200])?.len()
    );

    println!(
        "post_free_tag_200={}",
        wksp.query_by_tag(&[1, 2, 3, 100, 200])?.len()
    );

    wksp.free_by_tag(&[1, 3]);
    let remaining = wksp.query_by_tag(&[1, 2, 3, 100])?;
    println!("post_free_tags_1_3={}", remaining.len());

    for info in &remaining {
        println!(
            "  remaining_gaddr={}, tag={}",
            info.gaddr_lo.as_u64(),
            info.tag
        );
    }
    println!();

    if fs::metadata(checkpoint_path).is_ok() {
        wksp.reset(99999);
        println!("used_partitions_after_reset={}", wksp.usage(&[]).used_cnt);

        match wksp.restore(checkpoint_path, 67890) {
            Ok(()) => {
                let restored_usage = wksp.usage(&[]);
                let restored_allocs = wksp.query_by_tag(&[1, 2, 3])?;

                println!("  new_seed={}", wksp.seed());
                println!("  restored_partitions={}", restored_usage.used_cnt);
                println!("  restored_tagged_allocs={}", restored_allocs.len());

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
            Err(e) => println!("restore failed: {}", e),
        }

        let _ = fs::remove_file(checkpoint_path);
        println!();
    }

    let start = Instant::now();
    let mut perf_allocs = Vec::new();

    for _i in 0..100 {
        if let Ok(alloc) = wksp.allocate(1024, 32, 300) {
            perf_allocs.push(alloc);
        }
    }

    let alloc_time = start.elapsed();
    println!(
        "100_allocs_took={:?} ({:.2} μs/alloc)",
        alloc_time,
        alloc_time.as_micros() as f64 / 100.0
    );

    let start = Instant::now();
    wksp.free_by_tag(&[300]);
    let free_time = start.elapsed();
    println!("bulk free took: {:?}", free_time);
    println!();

    let final_state = wksp.usage(&[]);
    println!(
        "  partitions={}/{}",
        final_state.used_cnt, final_state.total_max
    );
    println!(
        "  memory={} ({:.1}% of {} bytes)",
        final_state.used_sz,
        100.0 * final_state.used_sz as f64 / final_state.total_sz as f64,
        final_state.total_sz
    );
    println!("  owner={}", wksp.owner());

    Ok(())
}
