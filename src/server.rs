use std::collections::BTreeMap;
use std::io::Write;
use std::net::UdpSocket;

const CHUNK_SIZE: usize = 512; // 512 bytes per chunk
const END_TRANSMISSION: &[u8] = b"END";

fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:8080")?;
    println!("Server listening on port 8080");

    let mut received_chunks: BTreeMap<u16, Vec<u8>> = BTreeMap::new();

    loop {
        let mut buffer = [0; CHUNK_SIZE + 2]; // +2 for sequence number header
        let (len, client_addr) = socket.recv_from(&mut buffer)?;

        // Check for end of transmission
        if &buffer[..3] == END_TRANSMISSION {
            println!("End of transmission packet received");

            // Reassemble the image
            let mut image_data = Vec::new();
            for (_seq, chunk) in received_chunks.iter() {
                image_data.extend_from_slice(chunk);
            }
            std::fs::write("received_image.jpg", &image_data)?;
            println!("Saved received image as received_image.jpg");

            // Send reassembled image back to client in chunks
            let mut chunk_number = 0;
            for chunk in image_data.chunks(CHUNK_SIZE) {
                let mut packet = Vec::new();
                packet.extend_from_slice(&(chunk_number as u16).to_be_bytes());
                packet.extend_from_slice(chunk);

                socket.send_to(&packet, client_addr)?;
                chunk_number += 1;
            }

            // Signal end of re-transmission
            socket.send_to(END_TRANSMISSION, client_addr)?;

            received_chunks.clear(); // Clear the map for the next transmission
        } else {
            // Extract chunk number and store chunk
            let seq_number = u16::from_be_bytes([buffer[0], buffer[1]]);
            received_chunks.insert(seq_number, buffer[2..len].to_vec());
            println!("Stored chunk with sequence number: {}", seq_number);
        }
    }
}
