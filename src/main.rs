// Prevent console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod pdf_engine;

use slint::{
    ComponentHandle, Image, Model, ModelNotify, ModelTracker,
    SharedPixelBuffer, VecModel, 
};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::{mpsc, Arc, Mutex, atomic::{AtomicUsize, Ordering}};
use std::thread;

slint::include_modules!();

enum PdfCommand {
    Open(String, usize, mpsc::Sender<Result<(usize, Vec<f32>, f32, Vec<pdf_engine::Bookmark>), String>>),
    Render(i32, f32, usize, mpsc::Sender<Result<(usize, u32, u32, Arc<Vec<u8>>), String>>),
    PreRender(i32, f32, usize),
    Close,
}

// Shared data between UI and background threads
struct PdfData {
    images: HashMap<usize, SharedPixelBuffer<slint::Rgba8Pixel>>,
    requested: HashSet<usize>,
    current_zoom: f32,
    count: usize,
}

struct PdfModel {
    data: Arc<Mutex<PdfData>>,
    cmd_tx: mpsc::Sender<PdfCommand>,
    resp_tx: mpsc::Sender<Result<(usize, u32, u32, Arc<Vec<u8>>), String>>,
    generation: Arc<AtomicUsize>,
    notify: ModelNotify,
}

impl PdfModel {
    fn new(
        cmd_tx: mpsc::Sender<PdfCommand>,
        resp_tx: mpsc::Sender<Result<(usize, u32, u32, Arc<Vec<u8>>), String>>,
        generation: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            data: Arc::new(Mutex::new(PdfData {
                images: HashMap::new(),
                requested: HashSet::new(),
                current_zoom: 1.0,
                count: 0,
            })),
            cmd_tx,
            resp_tx,
            generation,
            notify: ModelNotify::default(),
        }
    }

    fn set_page_count(&self, count: usize) {
        let mut data = self.data.lock().unwrap();
        data.count = count;
        data.images.clear();
        data.requested.clear();
        self.notify.reset(); 
    }

    fn set_zoom(&self, zoom: f32) {
        let mut data = self.data.lock().unwrap();
        if (zoom - data.current_zoom).abs() > 0.001 {
            data.current_zoom = zoom;
            data.images.clear();
            data.requested.clear();
            // Invalidate all
            self.notify.reset();
        }
    }
}

impl Model for PdfModel {
    type Data = Image;

    fn row_count(&self) -> usize {
        self.data.lock().unwrap().count
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        let mut data = self.data.lock().unwrap();
        if row >= data.count {
            return None;
        }

        if let Some(buffer) = data.images.get(&row) {
            return Some(Image::from_rgba8(buffer.clone()));
        }

        if !data.requested.contains(&row) {
            data.requested.insert(row);
            let zoom = data.current_zoom;
            let gen = self.generation.load(Ordering::SeqCst);
            
            let _ = self.cmd_tx.send(PdfCommand::Render(
                row as i32, 
                zoom, 
                gen, 
                self.resp_tx.clone()
            ));
        }

        Some(Image::from_rgba8(SharedPixelBuffer::new(1, 1)))
    }

    fn model_tracker(&self) -> &dyn ModelTracker {
        &self.notify
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Thumbnail Model
struct ThumbnailModel {
    data: Arc<Mutex<PdfData>>,
    cmd_tx: mpsc::Sender<PdfCommand>,
    resp_tx: mpsc::Sender<Result<(usize, u32, u32, Arc<Vec<u8>>), String>>,
    generation: Arc<AtomicUsize>,
    notify: ModelNotify,
}

impl ThumbnailModel {
    fn new(
        data: Arc<Mutex<PdfData>>,
        cmd_tx: mpsc::Sender<PdfCommand>,
        resp_tx: mpsc::Sender<Result<(usize, u32, u32, Arc<Vec<u8>>), String>>,
        generation: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            data,
            cmd_tx,
            resp_tx,
            generation,
            notify: ModelNotify::default(),
        }
    }
}

impl Model for ThumbnailModel {
    type Data = Image;
    fn row_count(&self) -> usize {
        self.data.lock().unwrap().count
    }
    fn row_data(&self, row: usize) -> Option<Self::Data> {
        let data = self.data.lock().unwrap();
        if row >= data.count { return None; }
        // Simple placeholder for now, verified build first
        Some(Image::from_rgba8(SharedPixelBuffer::new(1, 1)))
    }
    fn model_tracker(&self) -> &dyn ModelTracker { &self.notify }
    fn as_any(&self) -> &dyn std::any::Any { self }
}

fn flatten_outline(items: &[pdf_engine::Bookmark], level: i32, result: &mut Vec<OutlineItem>) {
    for item in items {
        result.push(OutlineItem {
            title: item.title.clone().into(),
            page_index: item.page_index as i32,
            level,
        });
        flatten_outline(&item.children, level + 1, result);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = AppWindow::new()?;
    let ui_handle = ui.as_weak();
    let generation = Arc::new(AtomicUsize::new(0));
    let (cmd_tx, cmd_rx) = mpsc::channel::<PdfCommand>();
    let (render_resp_tx, render_resp_rx) = mpsc::channel();

    let pdf_model = Rc::new(PdfModel::new(
        cmd_tx.clone(),
        render_resp_tx.clone(),
        generation.clone()
    ));
    ui.set_pdf_pages(pdf_model.clone().into());

    let thumb_model = Rc::new(ThumbnailModel::new(
        pdf_model.data.clone(),
        cmd_tx.clone(),
        render_resp_tx.clone(),
        generation.clone()
    ));
    ui.set_pdf_thumbnails(thumb_model.clone().into());

    let outline_model = Rc::new(VecModel::<OutlineItem>::default());
    ui.set_outline(outline_model.clone().into());

    // Worker Thread
    let worker_cmd_rx = cmd_rx;
    let _worker = thread::spawn(move || {
        let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
            Ok(p) => p,
            Err(_) => return,
        };
        let mut engine = pdf_engine::PdfEngine::new(&pdfium);
        let mut current_gen = 0;

        while let Ok(cmd) = worker_cmd_rx.recv() {
            match cmd {
                PdfCommand::Open(path, gen, resp) => {
                    current_gen = gen;
                    match engine.open_document(&path) {
                        Ok((c, h, w)) => {
                            let outline = engine.get_outline();
                            let _ = resp.send(Ok((c, h, w, outline)));
                        }
                        Err(e) => { let _ = resp.send(Err(e)); }
                    }
                }
                PdfCommand::Render(page, scale, gen, resp) => {
                    if gen >= current_gen {
                        if let Ok((w, h, data)) = engine.render_page(page, scale) {
                            let _ = resp.send(Ok((page as usize, w, h, data)));
                        } else {
                            let _ = resp.send(Err("Fail".into()));
                        }
                    }
                }
                PdfCommand::PreRender(page, scale, gen) => {
                    if gen >= current_gen { let _ = engine.render_page(page, scale); }
                }
                PdfCommand::Close => { engine.close_document(); }
            }
        }
    });

    // Bridge
    let bridge_rx = render_resp_rx;
    let bridge_data = pdf_model.data.clone();
    let bridge_ui = ui_handle.clone();
    thread::spawn(move || {
        while let Ok(Ok((idx, w, h, data))) = bridge_rx.recv() {
            let mut buf = SharedPixelBuffer::new(w, h);
            buf.make_mut_bytes().copy_from_slice(&*data);
            {
                let mut d = bridge_data.lock().unwrap();
                d.images.insert(idx, buf);
                d.requested.remove(&idx);
            }
            let ui_w = bridge_ui.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_w.upgrade() { ui.invoke_notify_render_complete(idx as i32); }
            });
        }
    });

    // Callbacks
    ui.on_notify_render_complete({
        let m = pdf_model.clone();
        move |idx| { m.notify.row_changed(idx as usize); }
    });

    ui.on_notify_open_complete({
        let pm = pdf_model.clone();
        let am = outline_model.clone();
        move |count, _, _, outline| {
            pm.set_page_count(count as usize);
            am.set_vec(outline.iter().collect());
        }
    });

    ui.on_open_document({
        let cmd_tx = cmd_tx.clone();
        let ui_h = ui.as_weak();
        let gen = generation.clone();
        move || {
            let ui_w = ui_h.clone();
            let tx = cmd_tx.clone();
            let g = gen.clone();
            thread::spawn(move || {
                if let Some(path) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).pick_file() {
                    let path_s = path.to_string_lossy().to_string();
                    let next_gen = g.fetch_add(1, Ordering::SeqCst) + 1;
                    let (res_tx, res_rx) = mpsc::channel();
                    let _ = tx.send(PdfCommand::Open(path_s, next_gen, res_tx));
                    if let Ok(Ok((count, heights, max_w, outline))) = res_rx.recv() {
                        let ui_w2 = ui_w.clone();
                        let heights_v = heights.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_w2.upgrade() {
                                ui.set_total_pages(count as i32);
                                ui.set_pdf_page_width(max_w);
                                ui.set_total_height(heights_v.iter().sum());
                                ui.set_pdf_page_heights(Rc::new(VecModel::from(heights_v.clone())).into());
                                
                                let mut flat = Vec::new();
                                flatten_outline(&outline, 0, &mut flat);
                                
                                // Signal model update on UI thread
                                ui.invoke_notify_open_complete(
                                    count as i32, 
                                    Rc::new(VecModel::from(heights_v)).into(), 
                                    max_w,
                                    Rc::new(VecModel::from(flat)).into()
                                );
                                
                                ui.set_is_loading(false);
                                ui.set_current_page(0);
                            }
                        });
                    }
                }
            });
        }
    });

    ui.on_close_document({
        let tx = cmd_tx.clone();
        let m = pdf_model.clone();
        move || { let _ = tx.send(PdfCommand::Close); m.set_page_count(0); }
    });

    ui.on_request_render({
        let m = pdf_model.clone();
        let g = generation.clone();
        move |_, zoom| { g.fetch_add(1, Ordering::SeqCst); m.set_zoom(zoom); }
    });

    ui.on_jump_to_page({
        let ui_h = ui.as_weak();
        move |p| { if let Some(ui) = ui_h.upgrade() { ui.set_current_page(p); } }
    });

    ui.run()?;
    Ok(())
}
