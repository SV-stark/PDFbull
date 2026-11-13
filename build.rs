fn main() {
    // Without any `rerun-if-changed` line Cargo reruns this script — and so
    // recompiles the crate — whenever *any* file in the package changes,
    // including pdfbull.log and the other files written next to Cargo.toml.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=PDFbull.ico");
    println!("cargo:rerun-if-changed=models/PP-OCRv6_small_det.onnx");
    println!("cargo:rerun-if-changed=models/PP-OCRv6_small_rec.onnx");
    println!("cargo:rerun-if-changed=models/ppocrv6_dict.txt");

    ensure_ocr_models();

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

fn ensure_ocr_models() {
    let models_dir = std::path::Path::new("models");
    let _ = std::fs::create_dir_all(models_dir);

    let models = [
        (
            "PP-OCRv6_small_det.onnx",
            "https://raw.githubusercontent.com/ShareX/ShareX/develop/ShareX.OCR/Models/PP-OCRv6_small_det/inference.onnx",
            1_000_000,
        ),
        (
            "PP-OCRv6_small_rec.onnx",
            "https://raw.githubusercontent.com/ShareX/ShareX/develop/ShareX.OCR/Models/PP-OCRv6_small_rec/inference.onnx",
            1_000_000,
        ),
    ];

    for (name, url, min_size) in models {
        let dest = models_dir.join(name);
        let needs_download = match std::fs::metadata(&dest) {
            Ok(meta) => meta.len() < min_size,
            Err(_) => true,
        };

        if needs_download {
            println!("cargo:warning=Downloading missing OCR model '{name}'...");
            let curl_cmd = if cfg!(windows) { "curl.exe" } else { "curl" };
            let status = std::process::Command::new(curl_cmd)
                .arg("-sSL")
                .arg("-o")
                .arg(&dest)
                .arg(url)
                .status();

            if let Ok(st) = status {
                if !st.success() {
                    println!("cargo:warning=Failed to download '{name}' from {url}");
                }
            } else {
                println!("cargo:warning=Could not execute curl to download '{name}'");
            }
        }
    }
}
