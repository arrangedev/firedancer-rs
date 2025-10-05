use fd_checkpt::{Checkpt, CheckptResult, FrameStyle, Restore};

const PREAMBLE: &'static [u8] = b"We the People of the United States, in Order to form a more perfect Union, establish Justice, insure domestic Tranquility, provide for the common defence, promote the general Welfare, and secure the Blessings of Liberty to ourselves and our Posterity, do ordain and establish this Constitution for the United States of America.";

fn main() -> CheckptResult<()> {
    mmio()?;

    if FrameStyle::Lz4.is_supported() {
        compression()?;
    } else {
        println!("\nlz4 not supported on this platform (don't use windows)");
    }

    multiple_frames()?;
    Ok(())
}

#[inline]
fn mmio() -> CheckptResult<()> {
    // raw frame
    let mut checkpt = Checkpt::new_mmio(1024 * 1024)?;
    let start_offset = checkpt.open_frame(FrameStyle::Raw)?;

    println!(
        "\nmmio: buf_sz={}, metadata_len={}, start_offset={}",
        checkpt.mmio_buffer().unwrap().len(),
        PREAMBLE.len() + 1024,
        start_offset
    );
    println!("---------------------------------\n");

    checkpt.checkpoint_meta(PREAMBLE)?;

    let data = vec![0x42u8; 1024]; // 1kb
    checkpt.checkpoint_data(&data)?;

    let end_offset = checkpt.close_frame()?;
    println!(
        "end_offset={} (total_frame_sz={})\n",
        end_offset,
        end_offset - start_offset
    );

    let checkpoint_data = checkpt
        .mmio_buffer()
        .expect("buffer should be available")
        .to_vec();
    let checkpoint_data = checkpoint_data[..end_offset as usize].to_vec();
    let mut restore = Restore::new_mmio(checkpoint_data)?;
    let restore_start = restore.open_frame(FrameStyle::Raw)?;
    println!("\nrestoring frame: opened_at={}", restore_start);
    println!("---------------------------------\n");

    let mut restored_metadata = vec![0u8; PREAMBLE.len()];
    restore.restore_meta(&mut restored_metadata)?;
    println!(
        "restored_metadata={:?}...",
        core::str::from_utf8(&restored_metadata)
            .unwrap_or("invalid UTF-8")
            .to_string()
            .chars()
            .take(34)
            .collect::<String>()
    );

    let mut restored_data = vec![0u8; data.len()];
    restore.restore_data(&mut restored_data)?;
    println!("bytes_restored={}", restored_data.len());

    let restore_end = restore.close_frame()?;
    println!("closed_frame_at={}\n", restore_end);

    assert_eq!(restored_metadata, PREAMBLE);
    assert_eq!(restored_data, data);
    println!("✓ Success");

    Ok(())
}

fn compression() -> CheckptResult<()> {
    println!("\nlz4");
    println!("---------------------------");

    let mut checkpt = Checkpt::new_mmio(1024 * 1024)?;
    let start_offset = checkpt.open_frame(FrameStyle::Lz4)?;
    println!("opened_at: {}", start_offset);

    let compressible_data = "This is a test string that repeats. ".repeat(100);
    let data_bytes = compressible_data.as_bytes();

    checkpt.checkpoint_meta(data_bytes)?;
    println!("checkpointed_sz: {}", data_bytes.len());

    let end_offset = checkpt.close_frame()?;
    let compressed_size = end_offset - start_offset;
    let compression_ratio = data_bytes.len() as f64 / compressed_size as f64;

    println!("closed_at: {}", end_offset);
    println!("  original_sz: {} bytes", data_bytes.len());
    println!("  compressed_sz: {} bytes", compressed_size);
    println!("  compression_ratio: {:.2}x", compression_ratio);

    let checkpoint_data = checkpt
        .mmio_buffer()
        .expect("MMIO buffer should be available")
        .to_vec();
    let checkpoint_data = checkpoint_data[..end_offset as usize].to_vec();

    let mut restore = Restore::new_mmio(checkpoint_data)?;
    restore.open_frame(FrameStyle::Lz4)?;

    let mut restored_data = vec![0u8; data_bytes.len()];
    restore.restore_meta(&mut restored_data)?;
    restore.close_frame()?;

    assert_eq!(restored_data, data_bytes);
    println!("✓ success");

    Ok(())
}

fn multiple_frames() -> CheckptResult<()> {
    println!("\nmultiple frames");
    println!("--------------------------");

    let mut checkpt = Checkpt::new_mmio(2 * 1024 * 1024)?;
    let mut frame_offsets = Vec::new();

    for i in 0..3 {
        let start_offset = checkpt.open_frame(FrameStyle::Raw)?;
        println!("frame_{}: opened_at: {}", i, start_offset);

        let frame_data = format!("frame_{} data", i);
        checkpt.checkpoint_meta(frame_data.as_bytes())?;

        let end_offset = checkpt.close_frame()?;
        println!(
            "frame_{}: closed_at: {} (size: {} bytes)",
            i,
            end_offset,
            end_offset - start_offset
        );

        frame_offsets.push((start_offset, end_offset));
    }

    let total_size = frame_offsets.last().unwrap().1;
    let checkpoint_data = checkpt
        .mmio_buffer()
        .expect("MMIO buffer should be available")
        .to_vec();
    let checkpoint_data = checkpoint_data[..total_size as usize].to_vec();

    let mut restore = Restore::new_mmio(checkpoint_data)?;

    for (i, &(start_offset, _end_offset)) in frame_offsets.iter().enumerate() {
        restore.seek(start_offset)?;

        restore.open_frame(FrameStyle::Raw)?;

        let expected_data = format!("frame_{} data", i);
        let mut restored_data = vec![0u8; expected_data.len()];
        restore.restore_meta(&mut restored_data)?;

        restore.close_frame()?;

        let restored_str = std::str::from_utf8(&restored_data).unwrap();
        println!("restored_frame_{}: '{}'", i, restored_str);

        assert_eq!(restored_str, expected_data);
    }

    println!("✓ success");
    Ok(())
}
