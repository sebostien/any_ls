use lazy_regex::regex_captures;
use std::io::Write;
use tower_lsp::lsp_types::{self, Diagnostic, DiagnosticSeverity, Position};

use super::{Handler, HandlerError};

#[derive(Debug)]
pub struct Just {}

fn parse_severity(severity: &str) -> Option<DiagnosticSeverity> {
    match severity {
        "error" => Some(DiagnosticSeverity::ERROR),
        _ => {
            log::info!("Unknown severity when parsing Just output: '{severity}'");
            Some(DiagnosticSeverity::WARNING)
        }
    }
}

impl Just {
    pub fn new() -> Result<Self, String> {
        Ok(Self {})
    }
}

impl Handler for Just {
    async fn update_diagnostics(
        &mut self,
        contents: &str,
    ) -> Result<Vec<Diagnostic>, HandlerError> {
        let mut temp_file =
            tempfile::NamedTempFile::new().map_err(|e| HandlerError::Log(format!("{e}")))?;
        temp_file
            .write_all(contents.as_bytes())
            .map_err(|e| HandlerError::Log(format!("{e}")))?;

        let out = std::process::Command::new("just")
            .arg("--dry-run")
            .arg("--justfile")
            .arg(temp_file.path())
            .output()
            .map_err(|e| HandlerError::Log(format!("{e}")))?;

        if out.status.success() {
            let stdout =
                String::from_utf8(out.stdout).map_err(|e| HandlerError::Log(format!("{e}")))?;
            Ok(Just::parse_stdout(&stdout))
        } else {
            let stderr =
                String::from_utf8(out.stderr).map_err(|e| HandlerError::Log(format!("{e}")))?;
            Ok(Self::parse_stderr(&stderr))
        }
    }
}

impl Just {
    pub fn parse_stderr(contents: &str) -> Vec<Diagnostic> {
        if let Some((_, severity, message, line, col)) =
            regex_captures!(r#"(\w+):\s(.*)\n.*——▶.*:(\d+):(\d+)"#, contents)
        {
            let line = line.parse().unwrap_or(0);
            let col = col.parse().unwrap_or(0);

            vec![Diagnostic::new(
                lsp_types::Range {
                    start: Position::new(line, col),
                    end: Position::new(line, col),
                },
                parse_severity(severity),
                None,
                Some("just".to_string()),
                message.to_string(),
                None,
                None,
            )]
        } else {
            log::warn!("Could not parse stderr: '{contents}'");
            vec![]
        }
    }

    pub fn parse_stdout(_contents: &str) -> Vec<Diagnostic> {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use crate::handlers::just::Just;

    #[test]
    fn test_parse() {
        let errors = vec![
            r#"error: Unknown start of token:
 ——▶ justfile:7:13
  │
7 │   just something here
  │             ^"#,
            r#"error: Unknown start of token:
 ——▶ 1f76590:6:10
  │
7 │   ls
  │             ^"#,
            r#"error: Unknown start of token:
 ——▶ justfile:4:15
  │
4 │   something
  │               ^"#,
  r#"error: Expected '&&', comment, end of file, end of line, identifier, or '(', but found ':'
 ——▶ .tmpu9xSRk:3:4
  │
3 │ a:::b
  │    ^"#
        ];

        for error in errors {
            assert!(!Just::parse_stderr(error).is_empty());
        }
    }
}
