fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
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
