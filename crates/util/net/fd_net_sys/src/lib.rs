//! Raw bindings to the Firedancer net utils.
//!
//! - Ethernet frame handling (fd_eth.h)
//! - IPv4 packet processing (fd_ip4.h)
//! - UDP datagram utils (fd_udp.h)
//! - PCAP file reads/writes (fd_pcap.h, fd_pcapng.h)
//! - Net protocol headers (fd_net_headers.h)
//! - Bit manipulation utils (fd_bits.h)
//!
//! For a safe API, consider using the higher-level wrapper crate `libfd-net`.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_ethernet_constants() {
        assert_eq!(FD_ETH_PAYLOAD_MAX, 1500);
        assert_eq!(FD_ETH_PAYLOAD_MIN_RAW, 46);
        assert_eq!(FD_PCAP_LINK_LAYER_ETHERNET, 1);
    }

    #[test]
    fn test_pcap_constants() {
        assert_eq!(FD_PCAP_ITER_TYPE_ETHERNET, 0);
        assert_eq!(FD_PCAP_ITER_TYPE_COOKED, 1);
        assert_eq!(FD_PCAPNG_LINKTYPE_ETHERNET, 1);
    }

    #[test]
    fn test_ethernet_functions() {
        unsafe {
            let data = b"Hello, world!";
            let fcs = fd_eth_fcs_append(
                FD_ETH_FCS_APPEND_SEED,
                data.as_ptr() as *const std::os::raw::c_void,
                data.len() as u64,
            );
            assert_ne!(fcs, 0);
        }
    }

    #[test]
    fn test_string_conversion_functions() {
        unsafe {
            let ip_str = CString::new("192.168.1.1").unwrap();
            let mut ip_addr = 0u32;
            let result = fd_cstr_to_ip4_addr(ip_str.as_ptr(), &mut ip_addr as *mut u32);

            assert_eq!(result, 1);
            assert_ne!(ip_addr, 0);
        }
    }

    #[test]
    fn test_mac_address_parsing() {
        unsafe {
            let mac_str = CString::new("aa:bb:cc:dd:ee:ff").unwrap();
            let mut mac_addr = [0u8; 6];
            let result = fd_cstr_to_mac_addr(mac_str.as_ptr(), mac_addr.as_mut_ptr());

            assert!(!result.is_null());
            assert_eq!(mac_addr[0], 0xaa);
            assert_eq!(mac_addr[1], 0xbb);
            assert_eq!(mac_addr[5], 0xff);
        }
    }

    #[test]
    fn test_bit_operations() {
        unsafe {
            assert_eq!(fd_ulong_floor_sqrt(100), 10);
            assert_eq!(fd_ulong_ceil_sqrt(99), 10);
            assert_eq!(fd_ulong_round_sqrt(100), 10);
        }
    }
}
