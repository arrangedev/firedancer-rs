use fd_cirq::CirqBuilder;
use std::alloc::{alloc, dealloc, Layout};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let size = 4096;
    let builder = CirqBuilder::new(size);

    let footprint = builder.footprint();
    let align = builder.align();

    unsafe {
        let layout = Layout::from_size_align(footprint, align)?;
        let memory = alloc(layout);
        if memory.is_null() {
            return Err("Failed to allocate".into());
        }

        let mut queue = builder.build(memory).ok_or("Failed to create cirq")?;

        println!(
            "[queue_created]: len={}, footprint={}, align={}, initial_cnt={}, initial_drop_cnt={}",
            size,
            footprint,
            align,
            queue.count(),
            queue.drop_count()
        );

        let message1 = b"Hello, World!";
        if let Some(buffer) = queue.push_back(1, message1.len()) {
            buffer.copy_from_slice(message1);
            println!("[pushed]: msg={:?}", core::str::from_utf8(message1)?);
        }

        let message2 = b"This is message 2";
        if let Some(buffer) = queue.push_back(1, message2.len()) {
            buffer.copy_from_slice(message2);
            println!("[pushed]: msg={:?}", core::str::from_utf8(message2)?);
        }

        println!("[queue_cnt_post]: cnt={}", queue.count());

        if let Some(data) = queue.pop_front() {
            let message = core::str::from_utf8(data)?;
            println!("[popped]: msg={:?} len={}", message, data.len());
        }

        if let Some(data) = queue.pop_front() {
            let message = core::str::from_utf8(data)?;
            println!("[popped]: msg={:?} len={}", message, data.len());
        }

        println!(
            "[final]: len={}, drop_cnt={}",
            queue.count(),
            queue.drop_count()
        );

        drop(queue);
        dealloc(memory, layout);
    }

    Ok(())
}
