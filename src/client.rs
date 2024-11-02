use std::net::UdpSocket;
use std::fs::File;
use std::io::{Read, Write};
use std::time::Duration;

const SERVER_ADDR: &str = "192.168.1.10:8080"; // Use server's IP address
const CHUNK_SIZE: usize = 1024; // Adjust based on requirements
const END_TRANSMISSION: &[u8] = b"END";

fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?; // Bind to any available port on client
    socket.connect(SERVER_ADDR)?; // Corrected this line to use `SERVER_ADDR`

    // Read image file into buffer
    let mut file = File::open("path/to/image.jpg")?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    println!("Read image from file: {} bytes", buffer.len());

    // Send image in chunks
    for (i, chunk) in buffer.chunks(CHUNK_SIZE).enumerate() {
        let mut packet = vec![];
        packet.extend_from_slice(&(i as u16).to_be_bytes());
        packet.extend_from_slice(chunk);
        socket.send(&packet)?;
    }

    // Send end of transmission
    socket.send(END_TRANSMISSION)?;
    println!("Image sent, waiting for response...");

    // Receive the image back in chunks
    let mut received_data = Vec::new();
    let mut recv_buffer = [0; CHUNK_SIZE + 2];

    loop {
        match socket.recv(&mut recv_buffer) {
            Ok(len) if &recv_buffer[..3] == END_TRANSMISSION => break,
            Ok(len) => received_data.extend_from_slice(&recv_buffer[2..len]),
            Err(e) => {
                eprintln!("Failed to receive data: {:?}", e);
                break;
            }
        }
    }

    // Save the received image
    let mut received_file = File::create("received_from_server.jpg")?;
    received_file.write_all(&received_data)?;
    println!("Image received and saved as received_from_server.jpg");

    Ok(())
}
