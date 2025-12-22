fn main() {
    // Ensure Cargo rebuilds the Windows resources (exe icon) when our icon assets change.
    // Without these, `build.rs` may not rerun and Windows can keep embedding the old icon.
    println!("cargo:rerun-if-changed=tauri.conf.json");
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=icons/32x32.png");
    println!("cargo:rerun-if-changed=icons/icon.png");

    tauri_build::build()
}
