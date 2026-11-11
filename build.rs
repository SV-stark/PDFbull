fn main() {
    // Without any `rerun-if-changed` line Cargo reruns this script — and so
    // recompiles the crate — whenever *any* file in the package changes,
    // including pdfbull.log and the other files written next to Cargo.toml.
    // The script only reads these two.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=PDFbull.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        println!("cargo:rustc-link-arg=/STACK:8388608");
        let mut res = winresource::WindowsResource::new();
        res.set("FileDescription", "PDFbull Document Reader");
        res.set("ProductName", "PDFbull");
        res.set("OriginalFilename", "pdfbull.exe");
        res.set("LegalCopyright", "Copyright (c)");

        // Ensure you generate a PDFbull.ico file before uncommenting the below line:
        res.set_icon("PDFbull.ico");

        if let Err(e) = res.compile() {
            println!("cargo:warning=Failed to compile Windows resources: {e}");
        }
    }
}
