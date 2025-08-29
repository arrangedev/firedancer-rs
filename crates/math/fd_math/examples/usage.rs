use fd_math::{avg, stat};

fn main() {
    let data = vec![1u32, 15, 3, 25, 7, 30, 9, 12, 6, 18];
    let filtered = stat::filter_uint(&data, 10);
    println!("filtered(≤ 10): {:?}", filtered);

    let mut data_copy = data.clone();
    if let Some(median) = stat::median_uint(&mut data_copy) {
        println!("median(32): {}", median);
    }

    let mut data_copy2 = data.clone();
    if let Some(median) = stat::median(&mut data_copy2) {
        println!("median(32): {}", median);
    }

    let float_data = vec![1.5f32, 2.7, 8.1, 3.2, 12.8, 4.9, 15.3, 6.1];
    let filtered_float = stat::filter_float(&float_data, 7.0);
    println!("filtered(≤ 7.0): {:?}", filtered_float);

    let mut float_copy = float_data.clone();
    if let Some(median) = stat::median_float(&mut float_copy) {
        println!("median(f32): {}", median);
    }

    println!();

    println!("  avg2_u32(100, 200) = {}", avg::avg2_u32(100, 200));
    println!("  avg2_i32(-50, 50) = {}", avg::avg2_i32(-50, 50));
    println!("  avg2_f32(1.5, 2.5) = {}", avg::avg2_f32(1.5, 2.5));

    println!("\nWould overflow with (x+y)/2):");
    println!(
        "  avg2_u64(u64::MAX-1, u64::MAX) = {}",
        avg::avg2_u64(u64::MAX - 1, u64::MAX)
    );
    println!(
        "  avg2_u32(u32::MAX-1, u32::MAX) = {}",
        avg::avg2_u32(u32::MAX - 1, u32::MAX)
    );
}
