use std::collections::BTreeMap;
use std::net::UdpSocket;
use image::{DynamicImage, RgbaImage, Rgba, GenericImageView, imageops::FilterType};
use std::time::Duration;
use std::thread::sleep;

const CHUNK_SIZE: usize = 512;
const END_TRANSMISSION: &[u8] = b"END";

fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:8080")?;
    println!("Server listening on port 8080");

    let mut received_chunks: BTreeMap<u16, Vec<u8>> = BTreeMap::new();

    loop {
        let mut buffer = [0; CHUNK_SIZE + 2];
        let (len, client_addr) = socket.recv_from(&mut buffer)?;

        if &buffer[..3] == END_TRANSMISSION {
            println!("End of transmission packet received");

            // Reassemble the secret image
            let mut image_data = Vec::new();
            for (_seq, chunk) in received_chunks.iter() {
                image_data.extend_from_slice(chunk);
            }
            std::fs::write("received_secret_image.jpg", &image_data)?;
            println!("Saved received secret image as received_secret_image.jpg");

            // Encrypt the received secret image using the default image
            let secret_img = load_image("received_secret_image.jpg").expect("Failed to load received secret image");
            let default_img = load_image("default.jpg").expect("Failed to load default image");
            let encrypted_img = encode_image(&secret_img, &default_img);

            encrypted_img.save("encrypted_image_to_send.png").expect("Failed to save encrypted image");

            // Send encrypted image back to client in chunks
            let encrypted_img_data = std::fs::read("encrypted_image_to_send.png")?;
            let mut chunk_number = 0;
            for chunk in encrypted_img_data.chunks(CHUNK_SIZE) {
                let mut packet = Vec::new();
                packet.extend_from_slice(&(chunk_number as u16).to_be_bytes());
                packet.extend_from_slice(chunk);

                socket.send_to(&packet, client_addr)?;
                chunk_number += 1;
                sleep(Duration::from_millis(1));
            }

            socket.send_to(END_TRANSMISSION, client_addr)?;
            println!("Sent end of transmission signal back to client");

            received_chunks.clear();
        } else {
            let seq_number = u16::from_be_bytes([buffer[0], buffer[1]]);
            received_chunks.insert(seq_number, buffer[2..len].to_vec());
            println!("Stored chunk with sequence number: {}", seq_number);
        }
    }
}

// Helper functions

fn load_image(path: &str) -> Result<DynamicImage, image::ImageError> {
    image::open(path)
}

fn encode_image(secret_img: &DynamicImage, default_img: &DynamicImage) -> RgbaImage {
    let dithered_secret_img = dither_image(secret_img);
    let (width, height) = dithered_secret_img.dimensions();
    let resized_default_img = resize_image(default_img, width, height);
    let mut encoded_img = resized_default_img.to_rgba8();

    for x in 0..width {
        for y in 0..height {
            let secret_pixel = dithered_secret_img.get_pixel(x, y);
            let default_pixel = resized_default_img.get_pixel(x, y);

            let encoded_pixel = Rgba([
                (default_pixel[0] & 0xF0) | ((secret_pixel[0] & 0xF0) >> 4),
                (default_pixel[1] & 0xF0) | ((secret_pixel[1] & 0xF0) >> 4),
                (default_pixel[2] & 0xF0) | ((secret_pixel[2] & 0xF0) >> 4),
                255,
            ]);

            encoded_img.put_pixel(x, y, encoded_pixel);
        }
    }
    encoded_img
}

fn resize_image(image: &DynamicImage, width: u32, height: u32) -> DynamicImage {
    image.resize_exact(width, height, FilterType::Lanczos3)
}

fn dither_image(image: &DynamicImage) -> RgbaImage {
    let (width, height) = image.dimensions();
    let mut dithered_image = image.to_rgba8();

    for y in 0..height {
        for x in 0..width {
            let old_pixel = dithered_image.get_pixel(x, y);
            let new_pixel = Rgba([
                if old_pixel[0] > 127 { 255 } else { 0 },
                if old_pixel[1] > 127 { 255 } else { 0 },
                if old_pixel[2] > 127 { 255 } else { 0 },
                255,
            ]);

            let quant_error = [
                old_pixel[0] as f32 - new_pixel[0] as f32,
                old_pixel[1] as f32 - new_pixel[1] as f32,
                old_pixel[2] as f32 - new_pixel[2] as f32,
            ];

            dithered_image.put_pixel(x, y, new_pixel);

            if x + 1 < width {
                let neighbor_pixel = dithered_image.get_pixel(x + 1, y);
                let new_neighbor = [
                    (neighbor_pixel[0] as f32 + quant_error[0] * 7.0 / 16.0).max(0.0).min(255.0) as u8,
                    (neighbor_pixel[1] as f32 + quant_error[1] * 7.0 / 16.0).max(0.0).min(255.0) as u8,
                    (neighbor_pixel[2] as f32 + quant_error[2] * 7.0 / 16.0).max(0.0).min(255.0) as u8,
                    255,
                ];
                dithered_image.put_pixel(x + 1, y, Rgba(new_neighbor));
            }

            if y + 1 < height {
                if x > 0 {
                    let neighbor_pixel = dithered_image.get_pixel(x - 1, y + 1);
                    let new_neighbor = [
                        (neighbor_pixel[0] as f32 + quant_error[0] * 3.0 / 16.0).max(0.0).min(255.0) as u8,
                        (neighbor_pixel[1] as f32 + quant_error[1] * 3.0 / 16.0).max(0.0).min(255.0) as u8,
                        (neighbor_pixel[2] as f32 + quant_error[2] * 3.0 / 16.0).max(0.0).min(255.0) as u8,
                        255,
                    ];
                    dithered_image.put_pixel(x - 1, y + 1, Rgba(new_neighbor));
                }

                let neighbor_pixel = dithered_image.get_pixel(x, y + 1);
                let new_neighbor = [
                    (neighbor_pixel[0] as f32 + quant_error[0] * 5.0 / 16.0).max(0.0).min(255.0) as u8,
                    (neighbor_pixel[1] as f32 + quant_error[1] * 5.0 / 16.0).max(0.0).min(255.0) as u8,
                    (neighbor_pixel[2] as f32 + quant_error[2] * 5.0 / 16.0).max(0.0).min(255.0) as u8,
                    255,
                ];
                dithered_image.put_pixel(x, y + 1, Rgba(new_neighbor));

                if x + 1 < width {
                    let neighbor_pixel = dithered_image.get_pixel(x + 1, y + 1);
                    let new_neighbor = [
                        (neighbor_pixel[0] as f32 + quant_error[0] * 1.0 / 16.0).max(0.0).min(255.0) as u8,
                        (neighbor_pixel[1] as f32 + quant_error[1] * 1.0 / 16.0).max(0.0).min(255.0) as u8,
                        (neighbor_pixel[2] as f32 + quant_error[2] * 1.0 / 16.0).max(0.0).min(255.0) as u8,
                        255,
                    ];
                    dithered_image.put_pixel(x + 1, y + 1, Rgba(new_neighbor));
                }
            }
        }
    }
    dithered_image
}
