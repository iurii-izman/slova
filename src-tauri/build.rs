use std::fs;
use std::path::Path;

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xffffffff;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb88320
            } else {
                crc >> 1
            };
        }
    }
    crc ^ 0xffffffff
}

fn main() {
    // Create icons directory
    let _ = fs::create_dir_all("icons");

    // Create a minimal valid 1x1 RGBA PNG
    let mut png = Vec::new();

    // PNG signature
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");

    // IHDR chunk
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&1u32.to_be_bytes()); // width
    ihdr.extend_from_slice(&1u32.to_be_bytes()); // height
    ihdr.push(8); // bit depth
    ihdr.push(6); // color type (6 = RGBA)
    ihdr.push(0); // compression method
    ihdr.push(0); // filter method
    ihdr.push(0); // interlace method

    let ihdr_crc = crc32(&[b"IHDR".as_ref(), &ihdr].concat());
    png.extend_from_slice(&(ihdr.len() as u32).to_be_bytes());
    png.extend_from_slice(b"IHDR");
    png.extend_from_slice(&ihdr);
    png.extend_from_slice(&ihdr_crc.to_be_bytes());

    // IDAT chunk (minimal - just one black pixel)
    let idat_data = vec![
        0x08, 0x99, // zlib header
        0x63, 0xf8, 0x0f, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0xf1, 0x08, 0x1e, 0xbd, 0x00, 0x00,
        0x00,
    ];

    let idat_crc = crc32(&[b"IDAT".as_ref(), &idat_data].concat());
    png.extend_from_slice(&(idat_data.len() as u32).to_be_bytes());
    png.extend_from_slice(b"IDAT");
    png.extend_from_slice(&idat_data);
    png.extend_from_slice(&idat_crc.to_be_bytes());

    // IEND chunk
    let iend_crc = crc32(b"IEND");
    png.extend_from_slice(&0u32.to_be_bytes());
    png.extend_from_slice(b"IEND");
    png.extend_from_slice(&iend_crc.to_be_bytes());

    let icon_path = Path::new("icons/icon.png");
    let _ = fs::write(icon_path, png);

    println!("cargo:rerun-if-changed=tauri.conf.json");
}
