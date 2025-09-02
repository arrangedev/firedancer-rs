use fd_scratch::{ScratchAllocator, ScratchError};

fn main() -> Result<(), ScratchError> {
    let mut allocator = ScratchAllocator::new(64 * 1024, 32)?;

    println!(
        "created allocator: {} bytes free, {} frames available",
        allocator.free_bytes(),
        allocator.frames_free()
    );

    {
        let _frame = allocator.push_frame()?;
        let allocation = allocator.allocate(1024, 64)?;

        println!(
            "  allocated {} bytes at {:p} (64-byte aligned)",
            allocation.size(),
            allocation.as_ptr()
        );
        println!(
            "  scratch-usage: {} bytes used, {} bytes free",
            allocator.used_bytes(),
            allocator.free_bytes()
        );

        let mut_slice =
            unsafe { core::slice::from_raw_parts_mut(allocation.as_ptr(), allocation.size()) };
        for i in 0..1024 {
            mut_slice[i] = (i % 256) as u8;
        }
    }

    println!("  cleanup: {} bytes used", allocator.used_bytes());

    {
        let _frame = allocator.push_frame()?;

        let alloc1 = allocator.allocate(512, 32)?;
        let alloc2 = allocator.allocate(256, 64)?;
        let alloc3 = allocator.allocate(128, 16)?;

        println!(
            "  allocated 3 blocks: {}+{}+{} bytes = {} total",
            alloc1.size(),
            alloc2.size(),
            alloc3.size(),
            alloc1.size() + alloc2.size() + alloc3.size()
        );
        println!(
            "  alignments: {:p} (32), {:p} (64), {:p} (16)",
            alloc1.as_ptr(),
            alloc2.as_ptr(),
            alloc3.as_ptr()
        );
        println!("  scratch-usage: {} bytes used", allocator.used_bytes());
    }

    println!("  cleanup: {} bytes used", allocator.used_bytes());

    {
        let _frame1 = allocator.push_frame()?;
        let _alloc1 = allocator.allocate(1024, 64)?;
        println!(
            "  frame 1: {} frames used, {} bytes used",
            allocator.frames_used(),
            allocator.used_bytes()
        );

        {
            let _frame2 = allocator.push_frame()?;
            let _alloc2 = allocator.allocate(512, 32)?;
            println!(
                "  frame 2: {} frames used, {} bytes used",
                allocator.frames_used(),
                allocator.used_bytes()
            );

            {
                let _frame3 = allocator.push_frame()?;
                let _alloc3 = allocator.allocate(256, 16)?;
                println!(
                    "  frame 3: {} frames used, {} bytes used",
                    allocator.frames_used(),
                    allocator.used_bytes()
                );
            }

            println!(
                "  back to frame 2: {} frames used, {} bytes used",
                allocator.frames_used(),
                allocator.used_bytes()
            );
        }

        println!(
            "  back to frame 1: {} frames used, {} bytes used",
            allocator.frames_used(),
            allocator.used_bytes()
        );
    }

    println!(
        "  cleanup: {} frames used, {} bytes used",
        allocator.frames_used(),
        allocator.used_bytes()
    );

    {
        let _frame = allocator.push_frame()?;
        let mut dynamic = allocator.prepare_alloc(128)?;
        println!(
            "  dynamic-alloc: {} bytes available",
            dynamic.max_available()
        );

        let data_to_write = b"Hello, scratch alloc!";
        let final_size = data_to_write.len();

        let slice = dynamic.as_mut_slice();
        slice[..final_size].copy_from_slice(data_to_write);
        let allocation = dynamic.publish(final_size)?;

        let read_slice = allocation.as_slice();
        let read_string = std::str::from_utf8(read_slice).unwrap();
        println!("  data-written: \"{}\"", read_string);
    }

    println!("  cleanup: {} bytes used", allocator.used_bytes());

    {
        let _frame = allocator.push_frame()?;

        let mut allocation = allocator.allocate(2048, 64)?;
        println!("  allocation: {} bytes", allocation.size());
        println!(
            "  scratch-usage before trim: {} bytes used",
            allocator.used_bytes()
        );

        let used_size = 512;
        allocation.trim(used_size)?;

        println!(
            "  trimming to {} bytes: {} bytes",
            used_size,
            allocation.size()
        );
        println!(
            "  scratch-usage after trim: {} bytes used",
            allocator.used_bytes()
        );
    }

    let result: Result<String, ScratchError> = fd_scratch::scratch_scope!(allocator, {
        let allocation = allocator.allocate(64, 32)?;
        let slice =
            unsafe { core::slice::from_raw_parts_mut(allocation.as_ptr(), allocation.size()) };

        let message = b"allocation with macro";
        slice[..message.len()].copy_from_slice(message);

        Ok(std::str::from_utf8(&slice[..message.len()])
            .unwrap()
            .to_string())
    });

    match result {
        Ok(message) => println!("  result: \"{}\"", message),
        Err(e) => println!("  error: {}", e),
    }

    println!("  cleanup (macro): {} bytes used", allocator.used_bytes());

    println!("\n------------------");
    println!(
        "  total-mem: {} bytes",
        allocator.used_bytes() + allocator.free_bytes()
    );
    println!("  used-mem: {} bytes", allocator.used_bytes());
    println!("  free-mem: {} bytes", allocator.free_bytes());
    println!("  frames-used: {}", allocator.frames_used());
    println!("  frames-free: {}", allocator.frames_free());
    Ok(())
}
