use lazy_regex::regex_captures;
use log::warn;
use lsp_types::{
    Diagnostic, DiagnosticOptions, DiagnosticSeverity, Position, ServerCapabilities,
    WorkDoneProgressOptions,
};
use std::{os::unix::fs::FileExt, process::Command};
use tempfile::NamedTempFile;

use super::{Handler, HandlerError};

#[derive(Debug)]
pub struct Just {
    #[allow(unused)]
    version: String,
    temp_file: NamedTempFile,
}

impl Just {
    #[must_use]
    pub fn new() -> Option<Self> {
        let out = Command::new("just").arg("--version").output().ok()?;
        let version = String::from_utf8_lossy(&out.stdout).to_string();

        Some(Self {
            version,
            temp_file: NamedTempFile::new().ok()?,
        })
    }
}

impl Handler for Just {
    fn filetype_supported(&self, filetype: &str) -> bool {
        matches!(filetype, "just" | "justfile")
    }

    fn get_capabilities(&self) -> ServerCapabilities {
        lsp_types::ServerCapabilities {
            diagnostic_provider: Some(lsp_types::DiagnosticServerCapabilities::Options(
                DiagnosticOptions {
                    identifier: None,
                    inter_file_dependencies: false,
                    workspace_diagnostics: false,
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(false),
                    },
                },
            )),
            ..Default::default()
        }
    }

    fn update_diagnostics(&mut self, contents: &str) -> Result<Vec<Diagnostic>, HandlerError> {
        self.temp_file
            .as_file()
            .set_len(0)
            .map_err(|e| HandlerError::Log(format!("{e}")))?;

        self.temp_file
            .as_file()
            .write_all_at(contents.as_bytes(), 0)
            .map_err(|e| HandlerError::Log(format!("{e}")))?;

        let out = std::process::Command::new("just")
            .arg("--dry-run")
            .arg("--justfile")
            .arg(self.temp_file.path())
            .output()
            .map_err(|e| HandlerError::Log(format!("{e}")))?;

        if out.status.success() {
            let stdout =
                String::from_utf8(out.stdout).map_err(|e| HandlerError::Log(format!("{e}")))?;
            Ok(Self::parse_stdout(&stdout))
        } else {
            let stderr =
                String::from_utf8(out.stderr).map_err(|e| HandlerError::Log(format!("{e}")))?;
            Ok(Self::parse_stderr(&stderr))
        }
    }
}

impl Just {
    #[must_use]
    pub fn parse_stderr(contents: &str) -> Vec<Diagnostic> {
        if let Some((_, severity, message, line, col)) =
            regex_captures!(r#"(\w+):\s(.*)\n.*——▶.*:(\d+):(\d+)"#, contents)
        {
            let line: u32 = line.parse().unwrap_or(0);
            let col: u32 = col.parse().unwrap_or(0);

            let mut diag = Diagnostic::new_simple(
                lsp_types::Range {
                    start: Position::new(line.saturating_sub(1), col.saturating_sub(1)),
                    end: Position::new(line.saturating_sub(1), col),
                },
                message.to_string(),
            );
            diag.severity = parse_severity(severity);

            vec![diag]
        } else {
            log::warn!("Could not parse stderr: '{contents}'");
            vec![]
        }
    }

    #[must_use]
    pub fn parse_stdout(_contents: &str) -> Vec<Diagnostic> {
        vec![]
    }
}

fn parse_severity(severity: &str) -> Option<DiagnosticSeverity> {
    match severity {
        "error" => Some(DiagnosticSeverity::ERROR),
        _ => {
            warn!("Unknown severity when parsing just output: '{severity}'");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use lsp_types::{Diagnostic, Position, Range};

    use crate::handlers::just::{parse_severity, Just};

    #[test]
    fn test_parse() {
        let e1 = [
            "error: Unknown start of token '1'",
            "——▶ justfile:7:1",
            "│",
            "7 │ 123",
            "  │ ^",
        ]
        .join("\n");
        let mut r1 = Diagnostic::new_simple(
            Range {
                start: Position::new(6, 0),
                end: Position::new(6, 1),
            },
            "Unknown start of token '1'".to_string(),
        );
        r1.severity = parse_severity("error");

        assert_eq!(Just::parse_stderr(&e1), vec![r1]);

        let e2 = [
            "error: Expected '&&', comment, end of file, end of line, identifier, or '(', but found ':'",
            "——▶ .tmpu9xSRk:3:4",
            "  │",
            "3 │ a:::b",
            "  │    ^",
        ].join("\n");
        let mut r2 = Diagnostic::new_simple(
            Range {
                start: Position::new(2, 3),
                end: Position::new(2, 4),
            },
            "Expected '&&', comment, end of file, end of line, identifier, or '(', but found ':'"
                .to_string(),
        );
        r2.severity = parse_severity("error");

        assert_eq!(Just::parse_stderr(&e2), vec![r2]);
    }
}
