// client.rs
use std::fs::File;
use std::io::{self, Read};
use std::net::UdpSocket;
use std::time::Duration;

const CHUNK_SIZE: usize = 512; // 512 bytes per chunk
const END_TRANSMISSION: &[u8] = b"END"; // Marker to signal end of transmission

fn main() -> io::Result<()> {
    const SERVER_ADDR: &str = "192.168.1.10:8080"; // Use server's IP address
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect(server_addr)?;

    println!("Connected to server");

    let mut file = File::open("default.jpg")?;
    let mut buffer = [0; CHUNK_SIZE];
    let mut chunk_number = 0;

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        // Prepare the packet with a 2-byte chunk number header
        let mut packet = Vec::new();
        packet.extend_from_slice(&(chunk_number as u16).to_be_bytes());
        packet.extend_from_slice(&buffer[..bytes_read]);

        socket.send(&packet)?;
        println!("Sent chunk number: {}", chunk_number);
        chunk_number += 1;
    }

    // Signal the end of transmission
    socket.send(END_TRANSMISSION)?;

    // Wait to receive the reassembled image back from the server
    let mut received_data = Vec::new();
    socket.set_read_timeout(Some(Duration::from_secs(5)))?;
    loop {
        let mut recv_buffer = [0; CHUNK_SIZE + 2]; // +2 for chunk header
        match socket.recv(&mut recv_buffer) {
            Ok(len) if &recv_buffer[..3] == END_TRANSMISSION => break,
            Ok(len) => received_data.extend_from_slice(&recv_buffer[2..len]), // skip the chunk header
            Err(e) => {
                eprintln!("Failed to receive: {:?}", e);
                break;
            }
        }
    }

    // Save the received file
    std::fs::write("received_image_from_server.jpg", &received_data)?;
    println!("Saved image as received_image_from_server.jpg");

    Ok(())
}
