use std::fs::File;
use std::io::{self, Read};
use std::net::UdpSocket;
use std::time::Duration;
use std::thread::sleep;
use image::{DynamicImage, RgbaImage, Rgba, GenericImageView, imageops::FilterType};

const CHUNK_SIZE: usize = 512;
const END_TRANSMISSION: &[u8] = b"END";

fn main() -> io::Result<()> {
    let server_addr = "127.0.0.1:8080"; // Replace with the server's IP
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect(server_addr)?;

    println!("Connected to server");

    // Load the secret image to be sent
    let mut file = File::open("secret_image.jpg")?;
    let mut buffer = [0; CHUNK_SIZE];
    let mut chunk_number = 0;

    // Send the secret image to the server in chunks
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

        // Delay to prevent overwhelming the server
        sleep(Duration::from_millis(1));
    }

    // Signal the end of transmission
    socket.send(END_TRANSMISSION)?;
    println!("Sent end of transmission signal");

    // Wait to receive the encrypted image back from the server
    let mut received_data = Vec::new();
    socket.set_read_timeout(Some(Duration::from_secs(5)))?;
    loop {
        let mut recv_buffer = [0; CHUNK_SIZE + 2];
        match socket.recv(&mut recv_buffer) {
            Ok(len) if &recv_buffer[..3] == END_TRANSMISSION => {
                println!("Received end of transmission signal from server");
                break;
            },
            Ok(len) => {
                println!("Received chunk of size: {}", len - 2);
                received_data.extend_from_slice(&recv_buffer[2..len]);
            },
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                println!("Waiting for server response...");
                continue;
            },
            Err(e) => {
                eprintln!("Failed to receive: {:?}", e);
                break;
            }
        }
    }

    // Save the received encrypted image
    std::fs::write("received_encrypted_image_from_server.png", &received_data)?;
    println!("Saved encrypted image as received_encrypted_image_from_server.png");

    // Load the default image used on the server for steganography
    let default_img = load_image("default.jpg").expect("Failed to load default image");

    // Decode the received encrypted image to retrieve the original hidden image
    match load_image("received_encrypted_image_from_server.png") {
        Ok(encrypted_img) => {
            let decoded_img = decode_image(&encrypted_img, &default_img);
            decoded_img.save("decoded_image_from_server.png").expect("Failed to save decoded image");
            println!("Saved decoded image as decoded_image_from_server.png");
        }
        Err(e) => {
            eprintln!("Failed to open image: {:?}", e);
        }
    }

    Ok(())
}

// Helper functions

fn load_image(path: &str) -> Result<DynamicImage, image::ImageError> {
    image::open(path)
}

fn decode_image(encoded_img: &DynamicImage, default_img: &DynamicImage) -> RgbaImage {
    let (width, height) = encoded_img.dimensions();
    let resized_default_img = resize_image(default_img, width, height);
    let mut secret_img = RgbaImage::new(width, height);

    for x in 0..width {
        for y in 0..height {
            let encoded_pixel = encoded_img.get_pixel(x, y);

            // Retrieve 4 LSBs and shift to fill 8 bits
            let decoded_pixel = Rgba([
                (encoded_pixel[0] & 0x0F) << 4,
                (encoded_pixel[1] & 0x0F) << 4,
                (encoded_pixel[2] & 0x0F) << 4,
                255,
            ]);

            secret_img.put_pixel(x, y, decoded_pixel);
        }
    }
    secret_img
}

fn resize_image(image: &DynamicImage, width: u32, height: u32) -> DynamicImage {
    image.resize_exact(width, height, FilterType::Lanczos3)
}
