use fd_quic::{QuicClient, QuicClientConfig, Result};
use std::net::{Ipv4Addr, SocketAddrV4};

fn main() -> Result<()> {
    let config = QuicClientConfig::new()
        .with_server_name("localhost")
        .with_idle_timeout(5_000_000_000)
        .with_keep_alive(true);

    let mut client = QuicClient::new(config)?;
    let server_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 4433);
    let local_addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0); // Any local port

    println!("> connecting to {}", server_addr);
    let mut connection = client.connect(server_addr, local_addr)?;

    let mut attempts = 0;
    while !connection.is_active() && attempts < 100 {
        connection.service();
        std::thread::sleep(std::time::Duration::from_millis(10));
        attempts += 1;
    }

    if !connection.is_active() {
        return Err(fd_quic::QuicError::Internal(
            "Handshake timeout".to_string(),
        ));
    }

    let mut stream = connection.open_stream()?;

    let message = b"Hello World!";
    println!("> sending: {:?}", std::str::from_utf8(message).unwrap());
    stream.write_all(message)?;
    stream.finish()?;

    let processed = connection.service();
    println!("> processed {} events", processed);

    Ok(())
}
