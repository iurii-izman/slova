fn main() {
    // Generate Tauri resources/manifest (including Windows Common Controls v6).
    tauri_build::build();

    println!("cargo:rerun-if-changed=tauri.conf.json");
    println!("cargo:rerun-if-changed=build.rs");
}
