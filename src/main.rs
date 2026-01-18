use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::net::{Ipv4Addr, SocketAddrV4};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("0.0.0.0:1080")
        .await
        .expect("Failed to bind");

    println!("VPN server listening on 0.0.0.0:1080");

    loop {
        let (mut socket, addr) = listener.accept().await.expect("Accept failed");
        println!("Client connected: {}", addr);

        tokio::spawn(async move {
            let mut buf = vec![0u8; 65535];

            loop {
                let n = match socket.read(&mut buf).await {
                    Ok(0) => {
                        println!("Client disconnected: {}", addr);
                        break;
                    }
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("Read error from {}: {}", addr, e);
                        break;
                    }
                };

                handle_packet(&buf[..n]);
            }
        });
    }
}

fn handle_packet(buf: &[u8]) {
    if buf.len() < 20 {
        println!("Packet too short: {} bytes", buf.len());
        return;
    }

    let version = buf[0] >> 4;
    if version != 4 {
        println!("Not IPv4 packet (version = {})", version);
        return;
    }

    let ihl = (buf[0] & 0x0F) * 4;
    if ihl < 20 {
        println!("Invalid IHL: {}", ihl);
        return;
    }

    let total_length = u16::from_be_bytes([buf[2], buf[3]]) as usize;
    if buf.len() < total_length {
        println!(
            "Truncated packet: buf_len={}, total_length={}",
            buf.len(),
            total_length
        );
        return;
    }

    let protocol = buf[9];

    let src_ip = Ipv4Addr::new(buf[12], buf[13], buf[14], buf[15]);
    let dst_ip = Ipv4Addr::new(buf[16], buf[17], buf[18], buf[19]);

    let payload = &buf[ihl as usize..total_length];

    println!(
        "IPv4 packet: {} → {}, proto={}, ihl={}, len={}",
        src_ip, dst_ip, protocol, ihl, total_length
    );

    // Только TCP (protocol = 6)
    if protocol == 6 {
        if payload.len() < 20 {
            println!("TCP header too short");
            return;
        }

        // ВСТАВКА: извлекаем порт назначения
        let dst_port = u16::from_be_bytes([payload[2], payload[3]]);
        println!("TCP dst port = {}", dst_port);

        tokio::spawn(forward_tcp(dst_ip, dst_port, payload.to_vec()));
    } else {
        println!("Non-TCP protocol: {}", protocol);
    }
}

async fn forward_tcp(dst_ip: Ipv4Addr, dst_port: u16, payload: Vec<u8>) {
    let dst = SocketAddrV4::new(dst_ip, dst_port);

    println!("Connecting to remote: {}", dst);

    match TcpStream::connect(dst).await {
        Ok(mut remote) => {
            println!("Connected to {}", dst);

            if let Err(e) = remote.write_all(&payload).await {
                eprintln!("Write to remote failed: {}", e);
                return;
            }

            let mut buf = vec![0u8; 4096];
            match remote.read(&mut buf).await {
                Ok(n) if n > 0 => {
                    println!("Got {} bytes from remote {}", n, dst);
                    // позже отправим обратно клиенту
                }
                Ok(_) => println!("Remote closed {}", dst),
                Err(e) => eprintln!("Read from remote failed: {}", e),
            }
        }
        Err(e) => {
            eprintln!("Connect to {} failed: {}", dst, e);
        }
    }
}
