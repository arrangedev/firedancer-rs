//! Safe API for Firedancer bit manipulation utils
//!
//! This wraps the FFI bindings provided by `libfd-bits-sys` and provides
//! safer abstractions for their use.
//!
//! ## Modules
//!
//! - `sqrt`: Fast int squareroot functions (floor, ceil, round)
//! - `cbrt`: Fast int cuberoot functions
//! - `find`: Most/Least significant bit position finding
//! - `bswap`: Endian conversion utils
//! - `popcount`: Count of set bits

pub mod sqrt {
    /// `floor(sqrt(x))` for 64-bit unsigned ints
    pub fn floor_u64(x: u64) -> u64 {
        unsafe { fd_bits_sys::fd_ulong_floor_sqrt(x) }
    }

    /// `ceil(sqrt(x))` for 64-bit unsigned ints
    pub fn ceil_u64(x: u64) -> u64 {
        unsafe { fd_bits_sys::fd_ulong_ceil_sqrt(x) }
    }

    /// `round(sqrt(x))` for 64-bit unsigned ints
    pub fn round_u64(x: u64) -> u64 {
        unsafe { fd_bits_sys::fd_ulong_round_sqrt(x) }
    }

    /// Fast approximation of `sqrt(x)` for 64-bit unsigned ints
    pub fn approx_u64(x: u64) -> u64 {
        unsafe { fd_bits_sys::fd_ulong_approx_sqrt(x) }
    }

    pub fn floor_u32(x: u32) -> u32 {
        floor_u64(x as u64) as u32
    }

    pub fn ceil_u32(x: u32) -> u32 {
        ceil_u64(x as u64) as u32
    }

    pub fn round_u32(x: u32) -> u32 {
        round_u64(x as u64) as u32
    }
}

pub mod cbrt {
    /// `floor(cbrt(x))` for 64-bit unsigned ints
    pub fn floor_u64(x: u64) -> u64 {
        unsafe { fd_bits_sys::fd_ulong_floor_cbrt(x) }
    }

    /// `ceil(cbrt(x))` for 64-bit unsigned ints
    pub fn ceil_u64(x: u64) -> u64 {
        unsafe { fd_bits_sys::fd_ulong_ceil_cbrt(x) }
    }

    /// `round(cbrt(x))` for 64-bit unsigned ints
    pub fn round_u64(x: u64) -> u64 {
        unsafe { fd_bits_sys::fd_ulong_round_cbrt(x) }
    }

    /// Fast approximation of `cbrt(x)` for 64-bit unsigned ints
    pub fn approx_u64(x: u64) -> u64 {
        unsafe { fd_bits_sys::fd_ulong_approx_cbrt(x) }
    }

    pub fn floor_u32(x: u32) -> u32 {
        floor_u64(x as u64) as u32
    }

    pub fn ceil_u32(x: u32) -> u32 {
        ceil_u64(x as u64) as u32
    }

    pub fn round_u32(x: u32) -> u32 {
        round_u64(x as u64) as u32
    }
}

pub mod find {
    /// Find the position of the most significant bit (0-based from lsb)
    ///
    /// Returns `None` if the value is 0
    pub fn msb_u64(value: u64) -> Option<u32> {
        if value == 0 {
            None
        } else {
            Some(63 - value.leading_zeros())
        }
    }

    pub fn msb_u32(value: u32) -> Option<u32> {
        if value == 0 {
            None
        } else {
            Some(31 - value.leading_zeros())
        }
    }

    pub fn msb_u16(value: u16) -> Option<u32> {
        if value == 0 {
            None
        } else {
            Some(15 - value.leading_zeros())
        }
    }

    pub fn msb_u8(value: u8) -> Option<u32> {
        if value == 0 {
            None
        } else {
            Some(7 - value.leading_zeros())
        }
    }

    /// Find the position of the least significant bit (0-based from lsb)
    ///
    /// Returns `None` if the value is 0
    pub fn lsb_u64(value: u64) -> Option<u32> {
        if value == 0 {
            None
        } else {
            Some(value.trailing_zeros())
        }
    }

    pub fn lsb_u32(value: u32) -> Option<u32> {
        if value == 0 {
            None
        } else {
            Some(value.trailing_zeros())
        }
    }

    pub fn lsb_u16(value: u16) -> Option<u32> {
        if value == 0 {
            None
        } else {
            Some(value.trailing_zeros())
        }
    }

    pub fn lsb_u8(value: u8) -> Option<u32> {
        if value == 0 {
            None
        } else {
            Some(value.trailing_zeros())
        }
    }
}

pub mod bswap {
    /// Byteswap a 16-bit value
    pub fn u16(value: u16) -> u16 {
        value.swap_bytes()
    }

    /// Byteswap a 32-bit value
    pub fn u32(value: u32) -> u32 {
        value.swap_bytes()
    }

    /// Byteswap a 64-bit value
    pub fn u64(value: u64) -> u64 {
        value.swap_bytes()
    }

    /// convert from host to network byteorder (big endian)
    pub fn hton_u16(value: u16) -> u16 {
        value.to_be()
    }

    pub fn hton_u32(value: u32) -> u32 {
        value.to_be()
    }

    pub fn hton_u64(value: u64) -> u64 {
        value.to_be()
    }

    /// convert from network to host byteorder
    pub fn ntoh_u16(value: u16) -> u16 {
        u16::from_be(value)
    }

    pub fn ntoh_u32(value: u32) -> u32 {
        u32::from_be(value)
    }

    pub fn ntoh_u64(value: u64) -> u64 {
        u64::from_be(value)
    }
}

pub mod popcount {
    /// Count of set bits in a value
    pub fn u64(value: u64) -> u32 {
        value.count_ones()
    }

    pub fn u32(value: u32) -> u32 {
        value.count_ones()
    }

    pub fn u16(value: u16) -> u32 {
        value.count_ones()
    }

    pub fn u8(value: u8) -> u32 {
        value.count_ones()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqrt_functions() {
        assert_eq!(sqrt::floor_u64(100), 10);
        assert_eq!(sqrt::ceil_u64(99), 10);
        assert_eq!(sqrt::round_u64(100), 10);
        assert_eq!(sqrt::approx_u64(100), 10);

        assert_eq!(sqrt::floor_u32(100), 10);
        assert_eq!(sqrt::ceil_u32(99), 10);

        assert_eq!(sqrt::floor_u64(0), 0);
        assert_eq!(sqrt::floor_u64(1), 1);
        assert_eq!(sqrt::floor_u64(u64::MAX), 4294967295); // floor(sqrt(2^64-1))
    }

    #[test]
    fn test_cbrt_functions() {
        assert_eq!(cbrt::floor_u64(8), 2);
        assert_eq!(cbrt::ceil_u64(7), 2);
        assert_eq!(cbrt::round_u64(8), 2);
        assert_eq!(cbrt::approx_u64(8), 2);

        assert_eq!(cbrt::floor_u32(27), 3);
        assert_eq!(cbrt::ceil_u32(26), 3);

        assert_eq!(cbrt::floor_u64(0), 0);
        assert_eq!(cbrt::floor_u64(1), 1);
    }

    #[test]
    fn test_bit_finding() {
        assert_eq!(find::msb_u64(0x8000000000000000), Some(63));
        assert_eq!(find::msb_u64(0x1), Some(0));
        assert_eq!(find::msb_u64(0), None);

        assert_eq!(find::msb_u32(0x80000000), Some(31));
        assert_eq!(find::msb_u16(0x8000), Some(15));
        assert_eq!(find::msb_u8(0x80), Some(7));

        assert_eq!(find::lsb_u64(0x8000000000000000), Some(63));
        assert_eq!(find::lsb_u64(0x1), Some(0));
        assert_eq!(find::lsb_u64(0), None);

        assert_eq!(find::lsb_u32(0x80000000), Some(31));
        assert_eq!(find::lsb_u16(0x8000), Some(15));
        assert_eq!(find::lsb_u8(0x80), Some(7));
    }

    #[test]
    fn test_byte_swapping() {
        assert_eq!(bswap::u16(0x1234), 0x3412);
        assert_eq!(bswap::u32(0x12345678), 0x78563412);
        assert_eq!(bswap::u64(0x123456789abcdef0), 0xf0debc9a78563412);
    }

    #[test]
    fn test_network_byte_order() {
        let host_val = 0x12345678u32;
        let network_val = bswap::hton_u32(host_val);
        let back_to_host = bswap::ntoh_u32(network_val);
        assert_eq!(host_val, back_to_host);

        // for big-endian systems hton is a no-op --- on little-endian, it swaps
        if cfg!(target_endian = "little") {
            assert_eq!(network_val, host_val.swap_bytes());
        } else {
            assert_eq!(network_val, host_val);
        }
    }

    #[test]
    fn test_popcount() {
        assert_eq!(popcount::u64(0), 0);
        assert_eq!(popcount::u64(0xffffffffffffffff), 64);
        assert_eq!(popcount::u64(0x5555555555555555), 32); // alternating

        assert_eq!(popcount::u32(0), 0);
        assert_eq!(popcount::u32(0xffffffff), 32);
        assert_eq!(popcount::u16(0xffff), 16);
        assert_eq!(popcount::u8(0xff), 8);
    }

    #[test]
    fn test_bit_patterns() {
        let value = 0x123456789abcdef0u64;

        let msb = find::msb_u64(value).unwrap();
        let lsb = find::lsb_u64(value).unwrap();
        let bits_set = popcount::u64(value);
        let swapped = bswap::u64(value);

        assert_eq!(msb, 60); // msb of 0x123456789abcdef0
        assert_eq!(lsb, 4); // lsb -- first 4 bits are 0
        assert_eq!(bits_set, 32); // 32 bits set
        assert_eq!(swapped, 0xf0debc9a78563412);
    }
}
