use tower_lsp::lsp_types::Diagnostic;

mod just;

pub use just::Just;

pub enum HandlerError {
    Log(String),
}

pub trait Handler {
    async fn update_diagnostics(
        &mut self,
        document_contents: &str,
    ) -> Result<Vec<Diagnostic>, HandlerError>;
}

#[derive(Debug)]
pub enum AnyHandler {
    Just(Just),
}

impl AnyHandler {
    pub fn from_filetype(filetype: &str) -> Option<Self> {
        match filetype {
            "just" => Some(Self::Just(Just::new().ok()?)),
            _ => None,
        }
    }
}

impl Handler for AnyHandler {
    async fn update_diagnostics(
        &mut self,
        document_contents: &str,
    ) -> Result<Vec<Diagnostic>, HandlerError> {
        match self {
            Self::Just(just) => just.update_diagnostics(document_contents).await,
        }
    }
}
