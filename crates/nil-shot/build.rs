fn main() {
    println!("cargo:rerun-if-changed=../../frontend/nil-shot/dist");
    tauri_build::build()
}
