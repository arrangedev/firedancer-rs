use fd_spad::{SpadAllocator, SpadError};
use std::alloc::{alloc, dealloc, Layout};

fn main() -> Result<(), SpadError> {
    let mem_max = 65536; // 64KB
    let footprint = SpadAllocator::footprint(mem_max)?;
    let layout = Layout::from_size_align(footprint, SpadAllocator::align()).unwrap();
    let shmem = unsafe { alloc(layout) };

    println!(
        "created shared memory: {} bytes footprint, {} bytes capacity",
        footprint, mem_max
    );

    let mut allocator = SpadAllocator::new(shmem, mem_max)?;

    println!(
        "spad status: {} frames max, {} bytes free",
        allocator.frame_max(),
        allocator.mem_free()
    );

    {
        allocator.push_frame()?;
        println!(
            "pushed frame: {} frames used, in_frame={}",
            allocator.frames_used(),
            allocator.in_frame()
        );

        let alloc1 = allocator.allocate(1024, 64)?;
        println!(
            "  allocated 1024 bytes at {:p} (64-byte aligned)",
            alloc1.as_ptr()
        );
        println!(
            "  memory usage: {} bytes used, {} bytes free",
            allocator.mem_used(),
            allocator.mem_free()
        );

        let slice = alloc1.as_slice();
        println!("  first 4 bytes: {:?}", &slice[..4.min(slice.len())]);

        unsafe { allocator.pop_frame()? };
        println!(
            "popped frame: {} frames used, {} bytes used",
            allocator.frames_used(),
            allocator.mem_used()
        );
    }

    {
        allocator.push_frame()?;
        let _alloc1 = allocator.allocate(512, 32)?;
        println!(
            "frame 1: allocated 512 bytes, {} frames used",
            allocator.frames_used()
        );

        {
            allocator.push_frame()?;
            let _alloc2 = allocator.allocate(256, 64)?;
            println!(
                "  frame 2: allocated 256 bytes, {} frames used",
                allocator.frames_used()
            );

            {
                allocator.push_frame()?;
                let _alloc3 = allocator.allocate(128, 16)?;
                println!(
                    "    frame 3: allocated 128 bytes, {} frames used",
                    allocator.frames_used()
                );

                println!("    total memory usage: {} bytes", allocator.mem_used());
                unsafe { allocator.pop_frame()? };
            }

            println!(
                "  back to frame 2: {} frames used, {} bytes used",
                allocator.frames_used(),
                allocator.mem_used()
            );
            unsafe { allocator.pop_frame()? };
        }

        println!(
            "back to frame 1: {} frames used, {} bytes used",
            allocator.frames_used(),
            allocator.mem_used()
        );
        unsafe { allocator.pop_frame()? };
    }

    println!(
        "all frames popped: {} frames used, {} bytes used",
        allocator.frames_used(),
        allocator.mem_used()
    );

    {
        allocator.push_frame()?;
        let mut dynamic = allocator.prepare_alloc(128, 2048)?;
        println!(
            "prepared allocation: max {} bytes available at {:p}",
            dynamic.max_size(),
            dynamic.as_ptr()
        );

        let data =
            b"Hello, shared scratchpad allocator! This is a test of the prepare/publish pattern.";
        let actual_size = data.len();

        let slice = dynamic.as_mut_slice();
        slice[..actual_size].copy_from_slice(data);

        println!("wrote {} bytes of data", actual_size);

        allocator.publish_alloc(actual_size)?;
        println!("published allocation: {} bytes used", allocator.mem_used());

        unsafe { allocator.pop_frame()? };
    }

    {
        allocator.push_frame()?;

        let _dynamic = allocator.prepare_alloc(64, 1024)?;
        println!(
            "prepared allocation: {} bytes memory used before cancel",
            allocator.mem_used()
        );

        allocator.cancel_alloc()?;
        println!(
            "cancelled allocation: {} bytes memory used after cancel",
            allocator.mem_used()
        );

        unsafe { allocator.pop_frame()? };
    }

    {
        allocator.push_frame()?;
        let alloc = allocator.allocate(2048, 64)?;
        println!(
            "allocated 2048 bytes: {} bytes total used",
            allocator.mem_used()
        );

        let used_size = 512;
        let trim_ptr = unsafe { alloc.as_ptr().add(used_size) };

        allocator.trim(trim_ptr)?;
        println!(
            "trimmed to {} bytes: {} bytes total used",
            used_size,
            allocator.mem_used()
        );

        let (frame_lo, frame_hi) = allocator.frame_bounds().unwrap();
        println!("frame bounds: {:p} to {:p}", frame_lo, frame_hi);

        unsafe { allocator.pop_frame()? };
    }

    {
        allocator.push_frame()?;
        allocator.allocate(256, 32)?;

        allocator.push_frame()?;
        allocator.allocate(512, 64)?;

        println!(
            "before reset: {} frames used, {} bytes used",
            allocator.frames_used(),
            allocator.mem_used()
        );

        allocator.reset();

        println!(
            "after reset: {} frames used, {} bytes used",
            allocator.frames_used(),
            allocator.mem_used()
        );
    }

    {
        let shmem_ptr = allocator.leave();
        println!("left allocator, got shared memory pointer: {:p}", shmem_ptr);

        let mut allocator2 = SpadAllocator::join(shmem_ptr)?;
        println!(
            "joined shared memory: {} bytes capacity, {} bytes used",
            allocator2.mem_max(),
            allocator2.mem_used()
        );

        allocator2.push_frame()?;
        let alloc = allocator2.allocate(1024, 128)?;
        println!(
            "allocated in rejoined spad: {} bytes at {:p}",
            alloc.size(),
            alloc.as_ptr()
        );
        unsafe { allocator2.pop_frame()? };

        let shmem_ptr = allocator2.delete();
        unsafe {
            dealloc(shmem_ptr, layout);
        }
    }

    {
        let shmem = unsafe { alloc(layout) };
        let allocator = SpadAllocator::new(shmem, mem_max)?;

        let valloc = allocator.as_valloc();
        unsafe {
            println!("virtual allocator vtable: {:p}", valloc.vt);
            let vtable = &*valloc.vt;
            println!("malloc function: {:?}", vtable.malloc);
            println!("free function: {:?}", vtable.free);
        }

        let shmem_ptr = allocator.delete();
        unsafe {
            dealloc(shmem_ptr, layout);
        }
    }

    println!("\n------------------");
    println!("spad example completed successfully!");

    Ok(())
}
