use std::fmt;

use lsp_types::{
    Diagnostic, PositionEncodingKind, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, Uri,
};
use serde_json::Value;

mod just;

pub use just::Just;

#[derive(Debug)]
pub enum HandlerError {
    Log(String),
    NoSuchDocument { uri: Uri },
}

impl std::error::Error for HandlerError {}

impl fmt::Display for HandlerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Log(s) => s.fmt(f),
            Self::NoSuchDocument { uri } => write!(f, "No such document: {}", **uri),
        }
    }
}

pub trait Handler {
    fn filetype_supported(&self, filetype: &str) -> bool;

    fn get_capabilities(&self) -> ServerCapabilities;

    fn update_diagnostics(&mut self, contents: &str) -> Result<Vec<Diagnostic>, HandlerError>;
}

#[derive(Default)]
pub struct AnyHandler {
    handlers: Vec<AllHandlers>,
    text_documents: lsp_textdocument::TextDocuments,
}

impl std::fmt::Debug for AnyHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let documents = self.text_documents.documents().values().collect::<Vec<_>>();

        f.debug_struct("AnyHandler")
            .field("handlers", &self.handlers)
            .field("text_documents", &documents)
            .finish()
    }
}

#[derive(Debug)]
pub enum AllHandlers {
    Just(Just),
}

impl AnyHandler {
    #[must_use]
    pub fn new() -> Self {
        let mut this = Self::default();

        if let Some(just) = Just::new() {
            this.handlers.push(AllHandlers::Just(just));
        }

        this
    }

    pub fn handle_notification(&mut self, method: &str, params: &Value) -> bool {
        self.text_documents.listen(method, params)
    }

    // pub fn from_filetype(filetype: &str) -> Option<Self> {
    //     match filetype {
    //         "just" => Some(Self::Just(Just::new().ok()?)),
    //         _ => None,
    //     }
    // }
}

impl AnyHandler {
    #[must_use]
    pub fn get_capabilities(&self) -> ServerCapabilities {
        let mut capabilities = ServerCapabilities {
            position_encoding: Some(PositionEncodingKind::UTF16),
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::INCREMENTAL,
            )),
            ..Default::default()
        };

        for handler in &self.handlers {
            let handler = handler.get_capabilities();

            // NOTE: This is likely terrible since more advanced capabilities must be set first.
            // But works for now and pretty easy to change to manual merge for a field if needed.
            // (Will probably spend a lot of time debugging this :))
            macro_rules! set_capabilities {
                ($name:ident) => {
                    if let Some($name) = handler.$name {
                        capabilities.$name.get_or_insert($name);
                    }
                };
            }

            set_capabilities!(position_encoding);
            set_capabilities!(text_document_sync);
            set_capabilities!(notebook_document_sync);
            set_capabilities!(selection_range_provider);
            set_capabilities!(hover_provider);
            set_capabilities!(completion_provider);
            set_capabilities!(signature_help_provider);
            set_capabilities!(definition_provider);
            set_capabilities!(type_definition_provider);
            set_capabilities!(implementation_provider);
            set_capabilities!(references_provider);
            set_capabilities!(document_highlight_provider);
            set_capabilities!(document_symbol_provider);
            set_capabilities!(workspace_symbol_provider);
            set_capabilities!(code_action_provider);
            set_capabilities!(code_lens_provider);
            set_capabilities!(document_formatting_provider);
            set_capabilities!(document_range_formatting_provider);
            set_capabilities!(document_on_type_formatting_provider);
            set_capabilities!(rename_provider);
            set_capabilities!(document_link_provider);
            set_capabilities!(color_provider);
            set_capabilities!(folding_range_provider);
            set_capabilities!(declaration_provider);
            set_capabilities!(execute_command_provider);
            set_capabilities!(workspace);
            set_capabilities!(call_hierarchy_provider);
            set_capabilities!(semantic_tokens_provider);
            set_capabilities!(moniker_provider);
            set_capabilities!(linked_editing_range_provider);
            set_capabilities!(inline_value_provider);
            set_capabilities!(inlay_hint_provider);
            set_capabilities!(diagnostic_provider);
            set_capabilities!(experimental);
        }

        capabilities
    }

    pub fn update_diagnostics(&mut self, uri: Uri) -> Result<Vec<Diagnostic>, HandlerError> {
        let text_document = self
            .text_documents
            .get_document(&uri)
            .ok_or(HandlerError::NoSuchDocument { uri })?;

        let content = text_document.get_content(None);

        let mut diagnostics = Vec::new();
        let mut errors = Vec::new();

        for handler in &mut self.handlers {
            if !handler.filetype_supported(text_document.language_id()) {
                continue;
            }

            match handler.update_diagnostics(content) {
                Ok(mut new_diags) => {
                    diagnostics.append(&mut new_diags);
                }
                Err(e) => {
                    errors.push(e);
                }
            }
        }

        if diagnostics.is_empty() {
            if let Some(last) = errors.pop() {
                return Err(last);
            }
        }

        Ok(diagnostics)
    }
}

impl AllHandlers {}

impl Handler for AllHandlers {
    fn filetype_supported(&self, filetype: &str) -> bool {
        match self {
            Self::Just(just) => just.filetype_supported(filetype),
        }
    }

    fn get_capabilities(&self) -> ServerCapabilities {
        match self {
            Self::Just(just) => just.get_capabilities(),
        }
    }

    fn update_diagnostics(
        &mut self,
        document_contents: &str,
    ) -> Result<Vec<Diagnostic>, HandlerError> {
        match self {
            Self::Just(just) => just.update_diagnostics(document_contents),
        }
    }
}
