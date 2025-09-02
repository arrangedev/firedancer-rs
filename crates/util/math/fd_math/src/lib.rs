//! Safe Rust bindings for Firedancer math utils
//!
//! This provides Rust implementations for inline functions
//! in the Firedancer C implementation that can't be bound directly.
//!
//! ## Structure
//!
//! - `stat`: Filtering, median calculations, fitting functions
//! - `avg`: Overflow-safe averaging functions for integer types
//! - `fxp`: 30-bit fractional precision arithmetic (planned)

pub mod stat {
    /// Filter values from an array based on a threshold
    ///
    /// Returns a new vec only containing values <= threshold.
    /// For floats, NaN values are filtered out.
    pub fn filter<T>(values: &[T], threshold: T) -> Vec<T>
    where
        T: Copy + PartialOrd,
    {
        values.iter().copied().filter(|&x| x <= threshold).collect()
    }

    /// Filter 32-bit unsigned integers (u32)
    pub fn filter_uint(values: &[u32], threshold: u32) -> Vec<u32> {
        let mut output = vec![0u32; values.len()];
        let count = unsafe {
            fd_math_sys::fd_stat_filter_uint(
                output.as_mut_ptr(),
                values.as_ptr(),
                values.len() as u64,
                threshold,
            )
        };
        output.truncate(count as usize);
        output
    }

    /// Filter 64-bit unsigned integers (u64)
    pub fn filter_ulong(values: &[u64], threshold: u64) -> Vec<u64> {
        let mut output = vec![0u64; values.len()];
        let count = unsafe {
            fd_math_sys::fd_stat_filter_ulong(
                output.as_mut_ptr(),
                values.as_ptr(),
                values.len() as u64,
                threshold,
            )
        };
        output.truncate(count as usize);
        output
    }

    /// Filter 32-bit float values (f32)
    pub fn filter_float(values: &[f32], threshold: f32) -> Vec<f32> {
        let mut output = vec![0.0f32; values.len()];
        let count = unsafe {
            fd_math_sys::fd_stat_filter_float(
                output.as_mut_ptr(),
                values.as_ptr(),
                values.len() as u64,
                threshold,
            )
        };
        output.truncate(count as usize);
        output
    }

    /// Calculate median of 32-bit unsigned integers (u32)
    pub fn median_uint(values: &mut [u32]) -> Option<u32> {
        if values.is_empty() {
            return None;
        }
        let result =
            unsafe { fd_math_sys::fd_stat_median_uint(values.as_mut_ptr(), values.len() as u64) };
        Some(result)
    }

    /// Calculate median of 64-bit unsigned integers (u64)
    pub fn median_ulong(values: &mut [u64]) -> Option<u64> {
        if values.is_empty() {
            return None;
        }
        let result =
            unsafe { fd_math_sys::fd_stat_median_ulong(values.as_mut_ptr(), values.len() as u64) };
        Some(result)
    }

    /// Calculate median of 32-bit float values (f32)
    pub fn median_float(values: &mut [f32]) -> Option<f32> {
        if values.is_empty() {
            return None;
        }
        let result =
            unsafe { fd_math_sys::fd_stat_median_float(values.as_mut_ptr(), values.len() as u64) };
        Some(result)
    }

    /// Generic median calculation for any comparable type (T)
    pub fn median<T>(values: &mut [T]) -> Option<T>
    where
        T: Copy + PartialOrd,
    {
        if values.is_empty() {
            return None;
        }

        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = values.len() / 2;

        if values.len() % 2 == 1 {
            Some(values[mid])
        } else {
            Some(values[mid - 1])
        }
    }
}

pub mod avg {
    /// Compute the average of two values without risk of intermediate overflow
    ///
    /// For integer types, this uses round-toward-negative-infinity semantics.
    pub fn avg2_u8(x: u8, y: u8) -> u8 {
        ((x as u64 + y as u64) >> 1) as u8
    }

    pub fn avg2_u16(x: u16, y: u16) -> u16 {
        ((x as u64 + y as u64) >> 1) as u16
    }

    pub fn avg2_u32(x: u32, y: u32) -> u32 {
        ((x as u64 + y as u64) >> 1) as u32
    }

    pub fn avg2_u64(x: u64, y: u64) -> u64 {
        (x >> 1) + (y >> 1) + (x & y & 1)
    }

    pub fn avg2_i8(x: i8, y: i8) -> i8 {
        ((x as i64 + y as i64) >> 1) as i8
    }

    pub fn avg2_i16(x: i16, y: i16) -> i16 {
        ((x as i64 + y as i64) >> 1) as i16
    }

    pub fn avg2_i32(x: i32, y: i32) -> i32 {
        ((x as i64 + y as i64) >> 1) as i32
    }

    pub fn avg2_i64(x: i64, y: i64) -> i64 {
        (x >> 1) + (y >> 1) + (x & y & 1)
    }

    pub fn avg2_f32(x: f32, y: f32) -> f32 {
        0.5 * x + 0.5 * y
    }

    pub fn avg2_f64(x: f64, y: f64) -> f64 {
        0.5 * x + 0.5 * y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_filter() {
        let values = [1, 10, 2, 20, 3];
        let filtered = stat::filter(&values, 5);
        assert_eq!(filtered, vec![1, 2, 3]);
    }

    #[test]
    fn test_stat_filter_uint() {
        let values = [1u32, 10, 2, 20, 3];
        let filtered = stat::filter_uint(&values, 5);
        assert_eq!(filtered, vec![1, 2, 3]);
    }

    #[test]
    fn test_stat_median() {
        let mut values = [1, 3, 2, 5, 4];
        assert_eq!(stat::median(&mut values), Some(3));

        let mut values = [1, 4, 2, 3];
        assert_eq!(stat::median(&mut values), Some(2));
    }

    #[test]
    fn test_stat_median_uint() {
        let mut values = [1u32, 3, 2, 5, 4];
        assert_eq!(stat::median_uint(&mut values), Some(3));
    }

    #[test]
    fn test_stat_median_float() {
        let mut values = [1.0f32, 3.0, 2.0, 5.0, 4.0];
        assert_eq!(stat::median_float(&mut values), Some(3.0));
    }

    #[test]
    fn test_avg2_functions() {
        assert_eq!(avg::avg2_u8(10, 20), 15);
        assert_eq!(avg::avg2_u16(100, 200), 150);
        assert_eq!(avg::avg2_u32(1000, 2000), 1500);
        assert_eq!(avg::avg2_u64(10000, 20000), 15000);

        assert_eq!(avg::avg2_i8(10, 20), 15);
        assert_eq!(avg::avg2_i8(-10, 10), 0);

        assert_eq!(avg::avg2_f32(1.0, 3.0), 2.0);
        assert_eq!(avg::avg2_f64(1.0, 3.0), 2.0);

        // would overflow with naive (x+y)/2
        assert_eq!(avg::avg2_u64(u64::MAX - 1, u64::MAX), u64::MAX - 1);
    }

    #[test]
    fn test_empty_arrays() {
        let mut empty: [u32; 0] = [];
        assert_eq!(stat::median_uint(&mut empty), None);
        assert_eq!(stat::median(&mut empty), None);
    }
}
