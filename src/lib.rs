use std::collections::HashMap;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

mod handlers;

use handlers::{AnyHandler, Handler, HandlerError};

#[derive(Debug)]
pub struct Document {
    contents: String,
    version: i32,
    handler: Mutex<AnyHandler>,
}

#[derive(Debug)]
pub struct Backend {
    client: Client,
    documents: Mutex<HashMap<Url, Document>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Mutex::new(HashMap::new()),
        }
    }
}

impl Backend {
    async fn init_handler(&self, url: Url, version: i32, filetype: &str) {
        let handler = match AnyHandler::from_filetype(filetype) {
            Some(handler) => handler,
            None => {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        format!("No handler for filetype: {filetype}"),
                    )
                    .await;
                return;
            }
        };
        let mut guard = self.documents.lock().await;
        guard.insert(
            url,
            Document {
                contents: String::new(),
                version,
                handler: Mutex::new(handler),
            },
        );
    }

    async fn update_document(&self, url: &Url, version: i32, contents: String) {
        let mut guard = self.documents.lock().await;
        if let Some(document) = guard.get_mut(url) {
            document.contents = contents;
            document.version = version;
        }
    }

    async fn report_diagnostics(&self, url: Url) {
        let guard = self.documents.lock().await;
        let (version, handler_out) = if let Some(document) = guard.get(&url) {
            let mut handler = document.handler.lock().await;
            let handler_out = handler.update_diagnostics(&document.contents).await;
            (document.version, handler_out)
        } else {
            // No handler
            return;
        };
        drop(guard);

        match handler_out {
            Ok(diagnostics) => {
                self.client
                    .publish_diagnostics(url, diagnostics, Some(version))
                    .await;
            }
            Err(err) => {
                // Clear diagnostics
                self.client
                    .publish_diagnostics(url, Vec::new(), Some(version))
                    .await;

                match err {
                    HandlerError::Log(text) => {
                        self.client.log_message(MessageType::ERROR, text).await
                    }
                }
            }
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                position_encoding: Some(PositionEncodingKind::UTF16),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: None,
                    file_operations: None,
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "any_ls".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.init_handler(
            params.text_document.uri.clone(),
            params.text_document.version,
            &params.text_document.language_id,
        )
        .await;
        self.update_document(
            &params.text_document.uri,
            params.text_document.version,
            params.text_document.text,
        )
        .await;
        self.report_diagnostics(params.text_document.uri).await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        self.update_document(
            &params.text_document.uri,
            params.text_document.version,
            std::mem::take(&mut params.content_changes[0].text),
        )
        .await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.report_diagnostics(params.text_document.uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let mut guard = self.documents.lock().await;
        guard.remove(&params.text_document.uri);
        // Clear diagnostics
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }
}
