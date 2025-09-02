#![allow(clippy::should_implement_trait)]

//! Safe API for Firedancer net utils
//!
//! This wraps the FFI bindings provided by `libfdnet-sys` and provides safer
//! abstractions for their use.
//!
//! ## Structure
//!
//! - `ethernet`: Frame constructs, MAC address handling, FCS calculation
//! - `ipv4`: Address parsing, packet constructs
//! - `udp`: Header constructs, checksum calculation
//! - `pcap`: Packet capture file reading and writing (planned)
//! - `bits`: Bitmanip utils and byte order conversion

use std::ffi::CString;
use std::fmt;
use std::net::Ipv4Addr;

pub mod ethernet {
    use super::*;

    /// Max ethernet payload size: 1500 bytes
    pub const PAYLOAD_MAX: usize = fd_net_sys::FD_ETH_PAYLOAD_MAX as usize;

    /// Min raw ethernet payload size: 46 bytes
    pub const PAYLOAD_MIN_RAW: usize = fd_net_sys::FD_ETH_PAYLOAD_MIN_RAW as usize;

    /// 6-byte MAC address
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct MacAddress([u8; 6]);

    impl MacAddress {
        pub fn new(bytes: [u8; 6]) -> Self {
            MacAddress(bytes)
        }

        pub fn as_bytes(&self) -> &[u8; 6] {
            &self.0
        }

        /// Parse a MAC address string in the format: "aa:bb:cc:dd:ee:ff"
        pub fn from_str(s: &str) -> Result<Self, ParseError> {
            let c_str = CString::new(s).map_err(|_| ParseError::InvalidFormat)?;
            let mut mac_addr = [0u8; 6];

            unsafe {
                let result = fd_net_sys::fd_cstr_to_mac_addr(c_str.as_ptr(), mac_addr.as_mut_ptr());
                if result.is_null() {
                    return Err(ParseError::InvalidFormat);
                }
            }

            Ok(MacAddress(mac_addr))
        }
    }

    impl fmt::Display for MacAddress {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
            )
        }
    }

    /// Calculate ethernet frame check sequence (FCS/CRC32)
    pub fn calculate_fcs(data: &[u8]) -> u32 {
        unsafe {
            fd_net_sys::fd_eth_fcs_append(
                fd_net_sys::FD_ETH_FCS_APPEND_SEED,
                data.as_ptr() as *const std::os::raw::c_void,
                data.len() as u64,
            )
        }
    }
}

pub mod ipv4 {
    use super::*;

    /// From `fd_ip4.h`
    pub mod protocol {
        /// FD_IP4_HDR_PROTOCOL_IP4
        pub const IP4: u8 = 0;
        /// FD_IP4_HDR_PROTOCOL_ICMP
        pub const ICMP: u8 = 1;
        /// FD_IP4_HDR_PROTOCOL_IGMP
        pub const IGMP: u8 = 2;
        /// FD_IP4_HDR_PROTOCOL_TCP
        pub const TCP: u8 = 6;
        /// FD_IP4_HDR_PROTOCOL_UDP
        pub const UDP: u8 = 17;
        /// FD_IP4_HDR_PROTOCOL_GRE
        pub const GRE: u8 = 47;
    }

    /// IPv4 fragmentation flags from `fd_ip4.h`
    pub mod frag_flags {
        /// FD_IP4_HDR_FRAG_OFF_RF
        pub const RESERVED: u16 = 0x8000;
        /// FD_IP4_HDR_FRAG_OFF_DF
        pub const DONT_FRAGMENT: u16 = 0x4000;
        /// FD_IP4_HDR_FRAG_OFF_MF
        pub const MORE_FRAGMENTS: u16 = 0x2000;
        /// FD_IP4_HDR_FRAG_OFF_MASK
        pub const OFFSET_MASK: u16 = 0x1fff;
    }

    pub fn parse_ipv4_addr(s: &str) -> Result<Ipv4Addr, ParseError> {
        let c_str = CString::new(s).map_err(|_| ParseError::InvalidFormat)?;
        let mut ip_addr = 0u32;

        unsafe {
            let result = fd_net_sys::fd_cstr_to_ip4_addr(c_str.as_ptr(), &mut ip_addr);
            if result != 1 {
                return Err(ParseError::InvalidFormat);
            }
        }

        Ok(Ipv4Addr::from(ip_addr.to_be()))
    }

    /// Check if a given IPv4 address is in a private range:
    /// - 10.0.0.0/8
    /// - 172.16.0.0/12
    /// - 192.168.0.0/16
    pub fn is_private_addr(addr: Ipv4Addr) -> bool {
        let octets = addr.octets();

        // 10.0.0.0/8 (10.0.0.0 - 10.255.255.255)
        octets[0] == 10
        // 172.16.0.0/12 (172.16.0.0 - 172.31.255.255)
        || (octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31)
        // 192.168.0.0/16 (192.168.0.0 - 192.168.255.255)
        || (octets[0] == 192 && octets[1] == 168)
    }

    /// Check if a given IPv4 address is in the loopback range (127.0.0.0/8)
    pub fn is_loopback_addr(addr: Ipv4Addr) -> bool {
        addr.octets()[0] == 127
    }
}

pub mod pcap {
    /// From `fd_pcap.h`
    pub mod link_layer {
        pub const ETHERNET: u32 = fd_net_sys::FD_PCAP_LINK_LAYER_ETHERNET;
        pub const USER0: u32 = fd_net_sys::FD_PCAP_LINK_LAYER_USER0;
    }

    /// From `fd_pcap.h`
    pub mod iter_type {
        pub const ETHERNET: u32 = fd_net_sys::FD_PCAP_ITER_TYPE_ETHERNET;
        pub const COOKED: u32 = fd_net_sys::FD_PCAP_ITER_TYPE_COOKED;
    }

    /// From `fd_pcapng.h`
    pub mod pcapng {
        pub const LINKTYPE_ETHERNET: u32 = fd_net_sys::FD_PCAPNG_LINKTYPE_ETHERNET;
        pub const FRAME_SIMPLE: u32 = fd_net_sys::FD_PCAPNG_FRAME_SIMPLE;
        pub const FRAME_ENHANCED: u32 = fd_net_sys::FD_PCAPNG_FRAME_ENHANCED;
        pub const FRAME_TLSKEYS: u32 = fd_net_sys::FD_PCAPNG_FRAME_TLSKEYS;
    }
}

pub mod bits {
    /// Find the most significant bit position in a 64-bit val
    pub fn find_msb_u64(value: u64) -> Option<u32> {
        if value == 0 {
            None
        } else {
            Some(63 - value.leading_zeros())
        }
    }

    /// Find the least significant bit position in a 64-bit val  
    pub fn find_lsb_u64(value: u64) -> Option<u32> {
        if value == 0 {
            None
        } else {
            Some(value.trailing_zeros())
        }
    }

    /// Byte swap a 16-bit value
    pub fn bswap_u16(value: u16) -> u16 {
        value.swap_bytes()
    }

    /// Byte swap a 32-bit value
    pub fn bswap_u32(value: u32) -> u32 {
        value.swap_bytes()
    }

    /// Byte swap a 64-bit value
    pub fn bswap_u64(value: u64) -> u64 {
        value.swap_bytes()
    }

    pub fn floor_sqrt_u64(value: u64) -> u64 {
        unsafe { fd_net_sys::fd_ulong_floor_sqrt(value) }
    }

    pub fn ceil_sqrt_u64(value: u64) -> u64 {
        unsafe { fd_net_sys::fd_ulong_ceil_sqrt(value) }
    }

    pub fn round_sqrt_u64(value: u64) -> u64 {
        unsafe { fd_net_sys::fd_ulong_round_sqrt(value) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    InvalidFormat,
    InvalidLength,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::InvalidFormat => write!(f, "Invalid format"),
            ParseError::InvalidLength => write!(f, "Invalid length"),
        }
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ethernet_constants() {
        assert_eq!(ethernet::PAYLOAD_MAX, 1500);
        assert_eq!(ethernet::PAYLOAD_MIN_RAW, 46);
    }

    #[test]
    fn test_mac_address_parsing() {
        let mac = ethernet::MacAddress::from_str("aa:bb:cc:dd:ee:ff").unwrap();
        assert_eq!(mac.as_bytes(), &[0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
        assert_eq!(mac.to_string(), "aa:bb:cc:dd:ee:ff");
    }

    #[test]
    fn test_ethernet_fcs() {
        let data = b"Hello, world!";
        let fcs = ethernet::calculate_fcs(data);
        assert_ne!(fcs, 0);

        // fcs should be consistent
        let fcs2 = ethernet::calculate_fcs(data);
        assert_eq!(fcs, fcs2);
    }

    #[test]
    fn test_ipv4_parsing() {
        let addr = ipv4::parse_ipv4_addr("192.168.1.1").unwrap();
        assert_eq!(addr, Ipv4Addr::new(192, 168, 1, 1));
        assert!(ipv4::parse_ipv4_addr("invalid").is_err());
    }

    #[test]
    fn test_ipv4_private_ranges() {
        assert!(ipv4::is_private_addr(Ipv4Addr::new(192, 168, 1, 1)));
        assert!(ipv4::is_private_addr(Ipv4Addr::new(10, 0, 0, 1)));
        assert!(ipv4::is_private_addr(Ipv4Addr::new(172, 16, 0, 1)));
        assert!(!ipv4::is_private_addr(Ipv4Addr::new(8, 8, 8, 8)));
    }

    #[test]
    fn test_ipv4_loopback() {
        assert!(ipv4::is_loopback_addr(Ipv4Addr::new(127, 0, 0, 1)));
        assert!(ipv4::is_loopback_addr(Ipv4Addr::new(127, 255, 255, 255)));
        assert!(!ipv4::is_loopback_addr(Ipv4Addr::new(192, 168, 1, 1)));
    }

    #[test]
    fn test_bit_operations() {
        assert_eq!(bits::find_msb_u64(0x8000000000000000), Some(63));
        assert_eq!(bits::find_msb_u64(0x1), Some(0));
        assert_eq!(bits::find_msb_u64(0), None);

        assert_eq!(bits::find_lsb_u64(0x8000000000000000), Some(63));
        assert_eq!(bits::find_lsb_u64(0x1), Some(0));
        assert_eq!(bits::find_lsb_u64(0), None);

        assert_eq!(bits::bswap_u16(0x1234), 0x3412);
        assert_eq!(bits::bswap_u32(0x12345678), 0x78563412);
        assert_eq!(bits::bswap_u64(0x123456789abcdef0), 0xf0debc9a78563412);

        assert_eq!(bits::floor_sqrt_u64(100), 10);
        assert_eq!(bits::ceil_sqrt_u64(99), 10);
        assert_eq!(bits::round_sqrt_u64(100), 10);
    }

    #[test]
    fn test_protocol_constants() {
        assert_eq!(ipv4::protocol::UDP, 17);
        assert_eq!(ipv4::protocol::TCP, 6);
        assert_eq!(ipv4::protocol::ICMP, 1);
    }

    #[test]
    fn test_pcap_constants() {
        assert_eq!(pcap::link_layer::ETHERNET, 1);
        assert_eq!(pcap::iter_type::ETHERNET, 0);
        assert_eq!(pcap::pcapng::LINKTYPE_ETHERNET, 1);
    }
}
