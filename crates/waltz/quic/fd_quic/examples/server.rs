use fd_quic::{QuicServer, QuicServerConfig, Result};
use std::net::{Ipv4Addr, SocketAddrV4};

fn main() -> Result<()> {
    let config = QuicServerConfig::new().with_retry(true);
    let mut server = QuicServer::new(config)?;
    let bind_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 4433);

    server.bind(bind_addr)?;

    println!("Server listening on {}", bind_addr);

    loop {
        match server.accept()? {
            Some(mut connection) => {
                println!("> new connection");
                let processed = connection.service();
                println!("> processed {} events", processed);
                // connection.close(0);
            }
            None => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}
