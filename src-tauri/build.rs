use base64::engine::general_purpose::STANDARD as BASE64_ENGINE;
use base64::Engine;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Create a proper 512x512 RGBA PNG (steel blue)
    // Using a valid RGBA PNG in base64
    let rgba_512_png = "iVBORw0KGgoAAAANSUhEUgAAAgAAAAIACAYAAAD0eNT/AAAABmJLR0QA/wD/AP+gvaeTAAAAy0lEQVR4nO3BMQEAAADCoPVPbQ1PoAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAOA1v9AAEZ2jeKEAAAAASUVORK5CYII=";

    let bytes = BASE64_ENGINE.decode(rgba_512_png).unwrap_or_else(|_| {
        // Fallback: minimal 1x1 RGBA PNG
        BASE64_ENGINE.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==").unwrap_or_default()
    });

    let _ = fs::create_dir_all("icons");
    let _ = fs::write("icons/icon.png", &bytes);

    println!("cargo:rerun-if-changed=tauri.conf.json");
}
