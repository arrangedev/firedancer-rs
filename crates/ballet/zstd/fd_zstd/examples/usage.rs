use fd_zstd::{DecompressionStream, FramePeek};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let window = 64 * 1024;
    println!("decompression stream with {}KB window", window / 1024);

    let _dstream = DecompressionStream::new(window).unwrap();
    match FramePeek::new(&[0, 1, 2, 3]) {
        Ok(peek) => {
            println!("✓ frame peek succeeded (unexpected)");
            println!("  window-size: {}", peek.window_size());
            println!("  content-size: {:?}", peek.frame_content_size());
            println!("  is-skippable: {}", peek.is_skippable());
        }
        Err(e) => {
            println!("✓ frame peek failed as expected: {e}");
        }
    }

    Ok(())
}
