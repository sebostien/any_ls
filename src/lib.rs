use std::io::Write;

use lsp_server::Message;
use lsp_types::notification::{DidSaveTextDocument, Notification};
use lsp_types::request::{DocumentDiagnosticRequest, Request};
use lsp_types::{
    DocumentDiagnosticParams, DocumentDiagnosticReport, FullDocumentDiagnosticReport,
    InitializeParams, InitializeResult, RelatedFullDocumentDiagnosticReport, ServerInfo,
    WorkspaceFolder,
};

pub mod cli;
pub mod handlers;

mod any_error;

pub use crate::any_error::AnyError;
pub use crate::cli::Cli;

use crate::handlers::AnyHandler;

pub fn start(args: Cli) -> Result<(), Box<dyn std::error::Error>> {
    if args.lsp {
        let (conn, io_threads) = lsp_server::Connection::stdio();
        run_lsp(&conn)?;
        io_threads.join()?;

        Ok(())
    } else {
        Err(Box::new(AnyError::NotYetImplemented(
            "Only LSP Mode is supported",
        )))
    }
}

fn run_lsp(conn: &lsp_server::Connection) -> Result<(), Box<dyn std::error::Error>> {
    let (id, initialize_params) = conn.initialize_start()?;

    let initialize_params = serde_json::from_value::<InitializeParams>(initialize_params)?;

    let mut language_server =
        LanguageServer::new(initialize_params.workspace_folders.unwrap_or_default());

    let initialize_result = InitializeResult {
        capabilities: language_server.handler.get_capabilities(),
        server_info: Some(ServerInfo {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }),
    };

    conn.initialize_finish(id, serde_json::to_value(initialize_result)?)?;

    for msg in &conn.receiver {
        match msg {
            Message::Request(req) => {
                if conn.handle_shutdown(&req)? {
                    break;
                }

                let response = language_server.handle_request(req);
                conn.sender.send(Message::Response(response))?;
            }
            Message::Response(_) => {}
            Message::Notification(notification) => {
                language_server.handle_notification(notification);
            }
        }
    }
    Ok(())
}

struct LanguageServer {
    #[allow(unused)]
    workspace_folders: Vec<WorkspaceFolder>,
    handler: AnyHandler,
}

impl LanguageServer {
    pub fn new(workspace_folders: Vec<WorkspaceFolder>) -> Self {
        Self {
            workspace_folders,
            handler: AnyHandler::new(),
        }
    }

    pub fn handle_request(&mut self, request: lsp_server::Request) -> lsp_server::Response {
        match request.method.as_str() {
            DocumentDiagnosticRequest::METHOD => {
                let (request_id, params) = request
                    .extract::<DocumentDiagnosticParams>(DocumentDiagnosticRequest::METHOD)
                    .unwrap();

                match self.handler.update_diagnostics(params.text_document.uri) {
                    Ok(diagnostics) => lsp_server::Response::new_ok(
                        request_id,
                        DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                            related_documents: None,
                            full_document_diagnostic_report: FullDocumentDiagnosticReport {
                                result_id: None,
                                items: diagnostics,
                            },
                        }),
                    ),
                    Err(e) => lsp_server::Response::new_err(
                        request_id,
                        lsp_types::error_codes::REQUEST_FAILED as i32,
                        e.to_string(),
                    ),
                }
            }
            _ => {
                debug_to_file(format!("Received unhandled request:\n{request:#?}"));

                lsp_server::Response::new_err(
                    request.id,
                    lsp_types::error_codes::REQUEST_FAILED as i32,
                    "Not yet implemented".to_string(),
                )
            }
        }
    }

    pub fn handle_notification(&mut self, notification: lsp_server::Notification) {
        if !self
            .handler
            .handle_notification(&notification.method, &notification.params)
        {
            match notification.method.as_str() {
                DidSaveTextDocument::METHOD => {
                    // TODO: What can we do here?
                }
                _ => {
                    debug_to_file(format!(
                        "Received unhandled notification:\n{notification:#?}"
                    ));
                }
            }
        }
    }
}

pub fn debug_to_file<S: AsRef<str>>(text: S) {
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(format!("{}/debug.log", env!("CARGO_MANIFEST_PATH")))
    {
        let _ = f.write_all(text.as_ref().as_bytes());
        let _ = f.write_all(b"\n\n");
    }
}
