use crate::app::PdfBullApp;
use crate::message::Message;
use crate::models::SearchResult;
use iced::Task;
use tokio::sync::oneshot;

pub fn handle_search_message(app: &mut PdfBullApp, message: Message) -> Task<Message> {
    match message {
        Message::Search(query) => {
            app.search_query.clone_from(&query);
            if query.is_empty() {
                if let Some(tab) = app.current_tab_mut() {
                    tab.search_results.clear();
                    tab.current_search_index = 0;
                }
                return Task::none();
            }
            Task::perform(
                async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                    Message::PerformSearch(query)
                },
                |m| m,
            )
        }
        Message::PerformSearch(query) => {
            if query != app.search_query {
                return Task::none();
            }

            if query.is_empty() {
                return Task::none();
            }

            let Some(tab) = app.current_tab() else {
                return Task::none();
            };

            let doc_id = tab.id;

            let Some(engine) = &app.engine else {
                return Task::none();
            };

            let cmd_tx = engine.cmd_tx.clone();
            Task::perform(
                async move {
                    let (resp_tx, resp_rx) = oneshot::channel();
                    if let Err(e) = cmd_tx
                        .send(crate::commands::PdfCommand::Search(doc_id, query, resp_tx))
                        .await
                    {
                        tracing::error!("Failed to send Search command: {e}");
                        return Err(crate::models::PdfError::EngineDied);
                    }
                    match resp_rx.await {
                        Ok(res) => res,
                        Err(_) => Err(crate::models::PdfError::ChannelClosed),
                    }
                },
                move |result| Message::SearchResult(doc_id, result),
            )
        }
        Message::SearchResult(received_doc_id, result) => {
            match result {
                Ok(results) => {
                    if let Some(tab) = app.current_tab_mut()
                        && tab.id == received_doc_id
                    {
                        tab.search_results = results
                            .into_iter()
                            .map(SearchResult::from_search_result_item)
                            .collect();
                        tab.current_search_index = 0;

                        if !tab.search_results.is_empty() && tab.current_search_index == 0 {
                            tab.current_page = tab.search_results[0].page;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Search error: {e}");
                    if e == "Engine died" || e == "Channel closed" {
                        app.engine = None;
                        app.status_message = Some(
                            "PDF engine crashed. Please try your action again to restart it."
                                .into(),
                        );
                    } else {
                        app.status_message = Some(format!("Search error: {e}"));
                    }
                }
            }
            Task::none()
        }
        Message::NextSearchResult => {
            let res = if let Some(tab) = app.current_tab_mut() {
                if tab.search_results.is_empty() {
                    None
                } else {
                    tab.current_search_index =
                        (tab.current_search_index + 1) % tab.search_results.len();
                    tab.current_page = tab.search_results[tab.current_search_index].page;
                    Some((tab.current_page, tab.current_search_index))
                }
            } else {
                None
            };

            if let Some((page, idx)) = res {
                let label = if let Some(tab) = app.current_tab() {
                    tab.page_labels
                        .get(page)
                        .cloned()
                        .unwrap_or_else(|| (page + 1).to_string())
                } else {
                    (page + 1).to_string()
                };
                app.page_input = label;
                if let Some(tab) = app.current_tab_mut() {
                    return crate::update::scroll_to_search_result(tab, idx);
                }
            }
            Task::none()
        }
        Message::PrevSearchResult => {
            let res = if let Some(tab) = app.current_tab_mut() {
                if tab.search_results.is_empty() {
                    None
                } else {
                    tab.current_search_index = if tab.current_search_index == 0 {
                        tab.search_results.len() - 1
                    } else {
                        tab.current_search_index - 1
                    };
                    tab.current_page = tab.search_results[tab.current_search_index].page;
                    Some((tab.current_page, tab.current_search_index))
                }
            } else {
                None
            };

            if let Some((page, idx)) = res {
                let label = if let Some(tab) = app.current_tab() {
                    tab.page_labels
                        .get(page)
                        .cloned()
                        .unwrap_or_else(|| (page + 1).to_string())
                } else {
                    (page + 1).to_string()
                };
                app.page_input = label;
                if let Some(tab) = app.current_tab_mut() {
                    return crate::update::scroll_to_search_result(tab, idx);
                }
            }
            Task::none()
        }
        Message::ClearSearch => {
            if let Some(tab) = app.current_tab_mut() {
                tab.search_results.clear();
                tab.current_search_index = 0;
            }
            app.search_query.clear();
            Task::none()
        }
        _ => Task::none(),
    }
}
