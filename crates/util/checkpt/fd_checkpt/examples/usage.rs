//! Basic usage example for the Firedancer checkpoint system
//!
//! This example demonstrates how to use the checkpoint and restore functionality
//! for both raw and LZ4 compressed frames.

use fd_checkpt::{CheckptResult, FdCheckpt, FdRestore, FrameStyle};

fn main() -> CheckptResult<()> {
    println!("Firedancer Checkpoint Example");
    println!("=============================");

    // Example 1: Basic MMIO checkpoint with raw frame
    basic_mmio_example()?;

    // Example 2: LZ4 compressed frame (if supported)
    if FrameStyle::Lz4.is_supported() {
        lz4_compression_example()?;
    } else {
        println!("\nLZ4 compression is not supported on this platform");
    }

    // Example 3: Multiple frames in one checkpoint
    multiple_frames_example()?;

    println!("\nAll examples completed successfully!");
    Ok(())
}

fn basic_mmio_example() -> CheckptResult<()> {
    println!("\n1. Basic MMIO Checkpoint Example");
    println!("---------------------------------");

    // Create a checkpoint in memory
    let mut checkpt = FdCheckpt::new_mmio(1024 * 1024)?;
    println!("Created MMIO checkpoint with 1MB buffer");

    // Open a raw frame
    let start_offset = checkpt.open_frame(FrameStyle::Raw)?;
    println!("Opened raw frame at offset: {}", start_offset);

    // Checkpoint some metadata
    let metadata = b"Hello, Firedancer checkpoint!";
    checkpt.checkpoint_meta(metadata)?;
    println!("Checkpointed {} bytes of metadata", metadata.len());

    // Checkpoint some larger data
    let data = vec![0x42u8; 1024]; // 1KB of data
    checkpt.checkpoint_data(&data)?;
    println!("Checkpointed {} bytes of data", data.len());

    // Close the frame
    let end_offset = checkpt.close_frame()?;
    println!(
        "Closed frame at offset: {} (frame size: {} bytes)",
        end_offset,
        end_offset - start_offset
    );

    // Get the checkpoint data for restoration
    let checkpoint_data = checkpt
        .mmio_buffer()
        .expect("MMIO buffer should be available")
        .to_vec();
    let checkpoint_data = checkpoint_data[..end_offset as usize].to_vec();

    // Create a restore from the checkpoint data
    let mut restore = FdRestore::new_mmio(checkpoint_data)?;
    println!("Created restore from checkpoint data");

    // Open the frame for restoration
    let restore_start = restore.open_frame(FrameStyle::Raw)?;
    println!("Opened frame for restoration at offset: {}", restore_start);

    // Restore the metadata
    let mut restored_metadata = vec![0u8; metadata.len()];
    restore.restore_meta(&mut restored_metadata)?;
    println!(
        "Restored metadata: {:?}",
        std::str::from_utf8(&restored_metadata).unwrap_or("invalid UTF-8")
    );

    // Restore the data
    let mut restored_data = vec![0u8; data.len()];
    restore.restore_data(&mut restored_data)?;
    println!("Restored {} bytes of data", restored_data.len());

    // Close the restore frame
    let restore_end = restore.close_frame()?;
    println!("Closed restore frame at offset: {}", restore_end);

    // Verify the data matches
    assert_eq!(restored_metadata, metadata);
    assert_eq!(restored_data, data);
    println!("✓ Data verification successful!");

    Ok(())
}

fn lz4_compression_example() -> CheckptResult<()> {
    println!("\n2. LZ4 Compression Example");
    println!("---------------------------");

    // Create a checkpoint in memory
    let mut checkpt = FdCheckpt::new_mmio(1024 * 1024)?;
    println!("Created MMIO checkpoint with 1MB buffer");

    // Open an LZ4 compressed frame
    let start_offset = checkpt.open_frame(FrameStyle::Lz4)?;
    println!("Opened LZ4 compressed frame at offset: {}", start_offset);

    // Create some compressible data (lots of repeated patterns)
    let compressible_data = "This is a test string that repeats. ".repeat(100);
    let data_bytes = compressible_data.as_bytes();

    checkpt.checkpoint_meta(data_bytes)?;
    println!(
        "Checkpointed {} bytes of compressible data",
        data_bytes.len()
    );

    // Close the frame
    let end_offset = checkpt.close_frame()?;
    let compressed_size = end_offset - start_offset;
    let compression_ratio = data_bytes.len() as f64 / compressed_size as f64;

    println!("Closed LZ4 frame:");
    println!("  Original size: {} bytes", data_bytes.len());
    println!("  Compressed size: {} bytes", compressed_size);
    println!("  Compression ratio: {:.2}x", compression_ratio);

    // Restore and verify
    let checkpoint_data = checkpt
        .mmio_buffer()
        .expect("MMIO buffer should be available")
        .to_vec();
    let checkpoint_data = checkpoint_data[..end_offset as usize].to_vec();

    let mut restore = FdRestore::new_mmio(checkpoint_data)?;
    restore.open_frame(FrameStyle::Lz4)?;

    let mut restored_data = vec![0u8; data_bytes.len()];
    restore.restore_meta(&mut restored_data)?;
    restore.close_frame()?;

    assert_eq!(restored_data, data_bytes);
    println!("✓ LZ4 compression/decompression successful!");

    Ok(())
}

fn multiple_frames_example() -> CheckptResult<()> {
    println!("\n3. Multiple Frames Example");
    println!("--------------------------");

    let mut checkpt = FdCheckpt::new_mmio(2 * 1024 * 1024)?;
    println!("Created MMIO checkpoint with 2MB buffer");

    let mut frame_offsets = Vec::new();

    // Create multiple frames with different data
    for i in 0..3 {
        let start_offset = checkpt.open_frame(FrameStyle::Raw)?;
        println!("Frame {}: opened at offset {}", i, start_offset);

        let frame_data = format!("Frame {} data", i);
        checkpt.checkpoint_meta(frame_data.as_bytes())?;

        let end_offset = checkpt.close_frame()?;
        println!(
            "Frame {}: closed at offset {} (size: {} bytes)",
            i,
            end_offset,
            end_offset - start_offset
        );

        frame_offsets.push((start_offset, end_offset));
    }

    // Get the complete checkpoint data
    let total_size = frame_offsets.last().unwrap().1;
    let checkpoint_data = checkpt
        .mmio_buffer()
        .expect("MMIO buffer should be available")
        .to_vec();
    let checkpoint_data = checkpoint_data[..total_size as usize].to_vec();

    // Restore each frame individually
    let mut restore = FdRestore::new_mmio(checkpoint_data)?;

    for (i, &(start_offset, _end_offset)) in frame_offsets.iter().enumerate() {
        // Seek to the frame's start
        restore.seek(start_offset)?;

        // Open and restore the frame
        restore.open_frame(FrameStyle::Raw)?;

        let expected_data = format!("Frame {} data", i);
        let mut restored_data = vec![0u8; expected_data.len()];
        restore.restore_meta(&mut restored_data)?;

        restore.close_frame()?;

        let restored_str = std::str::from_utf8(&restored_data).unwrap();
        println!("Restored frame {}: '{}'", i, restored_str);

        assert_eq!(restored_str, expected_data);
    }

    println!("✓ Multiple frames example successful!");
    Ok(())
}
