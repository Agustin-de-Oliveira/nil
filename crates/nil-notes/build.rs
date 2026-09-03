fn main() {
    println!("cargo:rerun-if-changed=../../frontend/nil-notes/dist");
    tauri_build::build()
}
