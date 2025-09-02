use fd_net::{bits, ethernet, ipv4, pcap};

fn main() {
    println!("  max_payload_size: {} bytes", ethernet::PAYLOAD_MAX);
    println!(
        "  min_raw_payload_size: {} bytes",
        ethernet::PAYLOAD_MIN_RAW
    );

    match ethernet::MacAddress::from_str("aa:bb:cc:dd:ee:ff") {
        Ok(mac) => {
            println!("  parsed_mac_address: {}", mac);
            println!("  mac_bytes: {:?}", mac.as_bytes());
        }
        Err(e) => println!("  failed_to_parse_mac_address: {}", e),
    }

    let test_data = b"Ethereum!";
    let fcs = ethernet::calculate_fcs(test_data);
    println!(
        "  fcs_for_payload: '{}': 0x{:08x}",
        std::str::from_utf8(test_data).unwrap(),
        fcs
    );
    println!();

    println!("    udp: {}", ipv4::protocol::UDP);
    println!("    tcp: {}", ipv4::protocol::TCP);
    println!("    icmp: {}", ipv4::protocol::ICMP);
    println!("    gre: {}", ipv4::protocol::GRE);
    println!(
        "    dont_fragment: 0x{:04x}",
        ipv4::frag_flags::DONT_FRAGMENT
    );
    println!(
        "    more_fragments: 0x{:04x}",
        ipv4::frag_flags::MORE_FRAGMENTS
    );
    println!("    offset_mask: 0x{:04x}", ipv4::frag_flags::OFFSET_MASK);

    let test_addresses = [
        "192.168.1.1",
        "10.0.0.1",
        "172.16.0.1",
        "127.0.0.1",
        "8.8.8.8",
        "invalid.address",
    ];

    for addr_str in &test_addresses {
        match ipv4::parse_ipv4_addr(addr_str) {
            Ok(addr) => {
                let is_private = ipv4::is_private_addr(addr);
                let is_loopback = ipv4::is_loopback_addr(addr);
                println!(
                    "    {} -> {} (private: {}, loopback: {})",
                    addr_str, addr, is_private, is_loopback
                );
            }
            Err(e) => {
                println!("    {} -> Error: {}", addr_str, e);
            }
        }
    }
    println!();

    println!("    ethernet: {}", pcap::link_layer::ETHERNET);
    println!("    user0: {}", pcap::link_layer::USER0);
    println!("    ethernet: {}", pcap::iter_type::ETHERNET);
    println!("    cooked: {}", pcap::iter_type::COOKED);
    println!("  pcapng_constants:");
    println!("    linktype_ethernet: {}", pcap::pcapng::LINKTYPE_ETHERNET);
    println!("    frame_simple: {}", pcap::pcapng::FRAME_SIMPLE);
    println!("    frame_enhanced: {}", pcap::pcapng::FRAME_ENHANCED);
    println!();

    let test_values = [0x1, 0x8000000000000000, 0x123456789abcdef0, 0x0];

    for &value in &test_values {
        let msb = bits::find_msb_u64(value);
        let lsb = bits::find_lsb_u64(value);
        println!("    0x{:016x} -> MSB: {:?}, LSB: {:?}", value, msb, lsb);
    }

    let u16_val = 0x1234u16;
    let u32_val = 0x12345678u32;
    let u64_val = 0x123456789abcdef0u64;

    println!(
        "    0x{:04x} -> 0x{:04x}",
        u16_val,
        bits::bswap_u16(u16_val)
    );
    println!(
        "    0x{:08x} -> 0x{:08x}",
        u32_val,
        bits::bswap_u32(u32_val)
    );
    println!(
        "    0x{:016x} -> 0x{:016x}",
        u64_val,
        bits::bswap_u64(u64_val)
    );

    let sqrt_values = [100, 99, 101, 1024, 1000000];
    for &value in &sqrt_values {
        let floor_sqrt = bits::floor_sqrt_u64(value);
        let ceil_sqrt = bits::ceil_sqrt_u64(value);
        let round_sqrt = bits::round_sqrt_u64(value);
        println!(
            "    sqrt({}) -> floor: {}, ceil: {}, round: {}",
            value, floor_sqrt, ceil_sqrt, round_sqrt
        );
    }
    println!();

    let src_mac = ethernet::MacAddress::from_str("00:11:22:33:44:55").unwrap();
    let dst_mac = ethernet::MacAddress::from_str("aa:bb:cc:dd:ee:ff").unwrap();
    let src_ip = ipv4::parse_ipv4_addr("192.168.1.100").unwrap();
    let dst_ip = ipv4::parse_ipv4_addr("8.8.8.8").unwrap();

    println!("    src_mac: {}", src_mac);
    println!("    dst_mac: {}", dst_mac);
    println!(
        "    src_ip: {} (private: {})",
        src_ip,
        ipv4::is_private_addr(src_ip)
    );
    println!(
        "    dst_ip: {} (private: {})",
        dst_ip,
        ipv4::is_private_addr(dst_ip)
    );
    println!("    protocol: udp ({})", ipv4::protocol::UDP);

    let payload = b"query to google.com";
    let fcs = ethernet::calculate_fcs(payload);
    println!("    payload: '{}'", std::str::from_utf8(payload).unwrap());
    println!("    frame_check_sequence: 0x{:08x}", fcs);
    println!();
}
