// Prevent console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod pdf_engine;

use iced::widget::{button, column, container, image as iced_image, row, scrollable, text, Space};
use iced::{Element, Length};
use std::sync::Arc;
use tokio::sync::mpsc;
use std::path::PathBuf;

pub fn main() -> iced::Result {
    iced::run(PdfBullApp::update, PdfBullApp::view)
}

#[derive(Debug, Clone)]
struct EngineState {
    cmd_tx: mpsc::Sender<PdfCommand>,
}

enum PdfCommand {
    Open(String, mpsc::Sender<Result<(usize, Vec<f32>, f32, Vec<pdf_engine::Bookmark>), String>>),
    Render(i32, f32, mpsc::Sender<Result<(usize, u32, u32, Arc<Vec<u8>>), String>>),
    Close,
}

#[derive(Debug)]
struct PdfBullApp {
    current_page: usize,
    total_pages: usize,
    zoom: f32,
    is_loading: bool,
    document_path: Option<PathBuf>,
    
    // Cached handles for rendered pages
    rendered_pages: std::collections::HashMap<usize, iced_image::Handle>,
    page_heights: Vec<f32>,
    pdf_page_width: f32,
    
    // Communication with PDF background thread
    engine: Option<EngineState>,
}

#[derive(Debug, Clone)]
enum Message {
    OpenDocument,
    DocumentOpened(Result<(usize, Vec<f32>, f32, Vec<pdf_engine::Bookmark>, PathBuf), String>),
    CloseDocument,
    NextPage,
    PrevPage,
    ZoomIn,
    ZoomOut,
    
    // Rendering
    RequestRender(usize),
    PageRendered(Result<(usize, u32, u32, Arc<Vec<u8>>), String>),
    
    EngineInitialized(EngineState),
}

impl Default for PdfBullApp {
    fn default() -> Self {
        Self {
            current_page: 0,
            total_pages: 0,
            zoom: 1.0,
            is_loading: false,
            document_path: None,
            rendered_pages: std::collections::HashMap::new(),
            page_heights: Vec::new(),
            pdf_page_width: 0.0,
            engine: None,
        }
    }
}

impl PdfBullApp {
    fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::OpenDocument => {
                if self.engine.is_none() {
                    // Initialize engine first
                    return iced::Task::perform(async {
                        let (cmd_tx, mut cmd_rx) = mpsc::channel(32);
                        std::thread::spawn(move || {
                            let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
                                Ok(p) => p,
                                Err(e) => { println!("Engine init failed: {}", e); return; }
                            };
                            let mut engine = pdf_engine::PdfEngine::new(&pdfium);
                            while let Some(cmd) = cmd_rx.blocking_recv() {
                                match cmd {
                                    PdfCommand::Open(path, resp) => {
                                        let res = engine.open_document(&path).map(|(c, h, w)| {
                                            (c, h, w, engine.get_outline())
                                        });
                                        let _ = resp.blocking_send(res);
                                    }
                                    PdfCommand::Render(page, zoom, resp) => {
                                        let res = engine.render_page(page, zoom).map(|(w, h, data)| {
                                            (page as usize, w, h, data)
                                        });
                                        let _ = resp.blocking_send(res);
                                    }
                                    PdfCommand::Close => { engine.close_document(); }
                                }
                            }
                        });
                        EngineState { cmd_tx }
                    }, Message::EngineInitialized);
                }

                self.is_loading = true;
                let engine = self.engine.as_ref().unwrap();
                let cmd_tx = engine.cmd_tx.clone();

                return iced::Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .add_filter("PDF", &["pdf"])
                            .pick_file()
                            .await;
                        
                        if let Some(file) = file {
                            let path = file.path().to_path_buf();
                            let path_s = path.to_string_lossy().to_string();
                            let (resp_tx, mut resp_rx) = mpsc::channel(1);
                            let _ = cmd_tx.send(PdfCommand::Open(path_s, resp_tx)).await;
                            match resp_rx.recv().await {
                                Some(Ok((count, heights, width, outline))) => {
                                    Ok((count, heights, width, outline, path))
                                }
                                Some(Err(e)) => Err(e),
                                None => Err("Engine died".into()),
                            }
                        } else {
                            Err("Cancelled".into())
                        }
                    },
                    Message::DocumentOpened
                );
            }
            Message::EngineInitialized(state) => {
                self.engine = Some(state);
                return self.update(Message::OpenDocument);
            }
            Message::DocumentOpened(Ok((count, heights, width, _outline, path))) => {
                self.total_pages = count;
                self.page_heights = heights;
                self.pdf_page_width = width;
                self.document_path = Some(path);
                self.current_page = 0;
                self.is_loading = false;
                self.rendered_pages.clear();
                // Request all pages to be rendered for continuous scroll
                let mut tasks = iced::Task::none();
                for page_idx in 0..count {
                    if let Some(engine) = &self.engine {
                        let (resp_tx, mut resp_rx) = mpsc::channel(1);
                        let cmd_tx = engine.cmd_tx.clone();
                        let zoom = self.zoom;
                        tasks = iced::Task::batch([
                            tasks,
                            iced::Task::perform(
                                async move {
                                    let _ = cmd_tx.send(PdfCommand::Render(page_idx as i32, zoom, resp_tx)).await;
                                    resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                                },
                                Message::PageRendered
                            )
                        ]);
                    }
                }
                return tasks;
            }
            Message::DocumentOpened(Err(e)) => {
                self.is_loading = false;
                if e != "Cancelled" { println!("Error opening PDF: {}", e); }
            }
            Message::CloseDocument => {
                self.total_pages = 0;
                self.document_path = None;
                self.rendered_pages.clear();
                if let Some(e) = &self.engine {
                    let _ = e.cmd_tx.try_send(PdfCommand::Close);
                }
            }
            Message::NextPage => {
                if self.current_page + 1 < self.total_pages {
                    self.current_page += 1;
                    return self.update(Message::RequestRender(self.current_page));
                }
            }
            Message::PrevPage => {
                if self.current_page > 0 {
                    self.current_page -= 1;
                    return self.update(Message::RequestRender(self.current_page));
                }
            }
            Message::ZoomIn => {
                self.zoom = (self.zoom * 1.25).min(5.0);
                self.rendered_pages.clear();
                return self.update(Message::RequestRender(self.current_page));
            }
            Message::ZoomOut => {
                self.zoom = (self.zoom / 1.25).max(0.25);
                self.rendered_pages.clear();
                return self.update(Message::RequestRender(self.current_page));
            }
            Message::RequestRender(page) => {
                if let Some(engine) = &self.engine {
                    if !self.rendered_pages.contains_key(&page) {
                        let (resp_tx, mut resp_rx) = mpsc::channel(1);
                        let cmd_tx = engine.cmd_tx.clone();
                        let zoom = self.zoom;
                        
                        return iced::Task::perform(
                            async move {
                                let _ = cmd_tx.send(PdfCommand::Render(page as i32, zoom, resp_tx)).await;
                                resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                            },
                            Message::PageRendered
                        );
                    }
                }
            }
            Message::PageRendered(Ok((page, width, height, data))) => {
                let handle = iced_image::Handle::from_rgba(
                    width,
                    height,
                    data.as_ref().clone()
                );
                self.rendered_pages.insert(page, handle);
            }
            Message::PageRendered(Err(e)) => { println!("Render error: {}", e); }
        }
        iced::Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let toolbar = row![
            button("Open PDF").on_press(Message::OpenDocument),
            Space::new().width(Length::Fill),
            button("-").on_press(Message::ZoomOut),
            text(format!("{}%", (self.zoom * 100.0) as u32)),
            button("+").on_press(Message::ZoomIn),
            Space::new().width(Length::Fixed(20.0)),
            text(format!("Page {}/{}", self.current_page + 1, self.total_pages.max(1))),
        ]
        .padding(10)
        .spacing(10)
        .align_y(iced::Alignment::Center);

        let content: Element<Message> = if self.total_pages == 0 {
            container(text(if self.is_loading { "Loading..." } else { "No document open." }))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        } else {
            let mut pdf_column = column![].spacing(10).padding(10).align_x(iced::Alignment::Center);
            
            // Show all pages for continuous scroll
            for page_idx in 0..self.total_pages {
                if let Some(handle) = self.rendered_pages.get(&page_idx) {
                    let img = iced::widget::Image::new(handle.clone());
                    pdf_column = pdf_column.push(container(img).padding(5));
                } else {
                    pdf_column = pdf_column.push(container(text(format!("Rendering page {}...", page_idx + 1))).padding(20));
                }
            }

            scrollable(container(pdf_column).width(Length::Fill))
                .height(Length::Fill)
                .into()
        };

        column![toolbar, content].into()
    }
}
