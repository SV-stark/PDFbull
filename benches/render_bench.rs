#![allow(unexpected_cfgs)]
use std::fs;
#[cfg(feature = "gpu-render")]
use zpdf::gpu::WgpuRenderer;
use zpdf::{ContentInterpreter, ImageCache, PdfDocument, RenderBackend, cpu::CpuRenderer};

fn main() {
    divan::main();
}

const PDF_PATH: &str = "tests/test_document.pdf";

#[divan::bench]
fn bench_pdf_parse() {
    let data = fs::read(PDF_PATH).expect("Failed to read test PDF");
    let _doc = PdfDocument::open(data).expect("Failed to open PDF");
}

#[divan::bench]
fn bench_pdf_render_cpu() {
    let data = fs::read(PDF_PATH).expect("Failed to read test PDF");
    let doc = PdfDocument::open(data).expect("Failed to open PDF");
    let page = doc.page(0).expect("Failed to get page");
    let mut fonts = doc.load_page_fonts(&page);
    let mut images = ImageCache::new();
    let content = doc
        .page_content_bytes(&page)
        .expect("Failed to get content");

    let display_list = ContentInterpreter::new(page.effective_box())
        .with_fonts(&mut fonts)
        .with_document(doc.file(), &page.resources)
        .with_images(&mut images)
        .interpret(&content);

    let mut renderer = CpuRenderer::new().with_fonts(&fonts).with_images(&images);
    let _page_img = renderer
        .render_display_list(&display_list, 1.0)
        .expect("Failed to render");
}

#[cfg(feature = "gpu-render")]
#[divan::bench]
fn bench_pdf_render_gpu() {
    let data = fs::read(PDF_PATH).expect("Failed to read test PDF");
    let doc = PdfDocument::open(data).expect("Failed to open PDF");
    let page = doc.page(0).expect("Failed to get page");
    let mut fonts = doc.load_page_fonts(&page);
    let mut images = ImageCache::new();
    let content = doc
        .page_content_bytes(&page)
        .expect("Failed to get content");

    let display_list = ContentInterpreter::new(page.effective_box())
        .with_fonts(&mut fonts)
        .with_document(doc.file(), &page.resources)
        .with_images(&mut images)
        .interpret(&content);

    let mut renderer = WgpuRenderer::new().with_fonts(&fonts).with_images(&images);
    let _page_img = renderer
        .render_display_list(&display_list, 1.0)
        .expect("Failed to render");
}
