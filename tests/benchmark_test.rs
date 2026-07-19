use pdfbull::models::DocumentId;
use pdfbull::pdf_engine::{create_render_cache, DocumentStore, RenderFilter, RenderOptions, RenderQuality};
use std::path::PathBuf;
use std::time::Instant;

#[test]
fn benchmark_pdfbull_open_and_render() {
    let test_files = vec![
        ("Small PDF (26 KB)", PathBuf::from(r"E:\PDFbull\tests\test_document.pdf")),
        ("Medium PDF (0.9 MB)", PathBuf::from(r"C:\Users\suyas\Documents\Declaration cum affidaavit.pdf")),
        ("Large PDF (5.9 MB)", PathBuf::from(r"C:\Users\suyas\Documents\f-2.pdf")),
        ("Heavy PDF (11.0 MB)", PathBuf::from(r"C:\Users\suyas\Documents\FOREST3.pdf")),
        ("Giant PDF (54.6 MB)", PathBuf::from(r"C:\Users\suyas\Documents\forest 100.pdf")),
    ];

    println!("\n========================================================");
    println!("          PDFbull Engine Benchmark Results              ");
    println!("========================================================");

    let cache = create_render_cache(100, 512);

    for (name, path) in test_files {
        if !path.exists() {
            println!("Skipping {name} (path not found)");
            continue;
        }

        let mut store = DocumentStore::new(cache.clone());
        let doc_id = DocumentId(1);

        // Benchmark Document Open + Parsing
        let start_open = Instant::now();
        let open_res = store.open_document(path.to_str().unwrap(), None, doc_id);
        let open_duration = start_open.elapsed();

        match open_res {
            Ok(open_data) => {
                // Benchmark Page 1 Render (Medium quality)
                let options = RenderOptions {
                    scale: 1.0,
                    rotation: 0,
                    filter: RenderFilter::None,
                    auto_crop: false,
                    quality: RenderQuality::Medium,
                };

                let start_render = Instant::now();
                let render_res = store.render_page(doc_id, 0, options);
                let render_duration = start_render.elapsed();

                let render_status = if render_res.is_ok() { "OK" } else { "FAILED" };

                println!(
                    "{:<22} | Open: {:>7.2?} | Page 1 Render: {:>7.2?} | Pages: {:>4} | Status: {}",
                    name, open_duration, render_duration, open_data.page_count, render_status
                );
            }
            Err(e) => {
                println!("{:<22} | Open Failed: {:?}", name, e);
            }
        }
    }
    println!("========================================================\n");
}
