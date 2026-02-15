fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let slint_file = format!("{}/ui/AppWindow.slint", manifest_dir);

    slint_build::compile(&slint_file).expect("Failed to compile Slint UI");
}
