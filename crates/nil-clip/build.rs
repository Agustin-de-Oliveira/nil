fn main() {
    println!("cargo:rerun-if-changed=../../frontend/nil-clip/dist");
    tauri_build::build()
}
