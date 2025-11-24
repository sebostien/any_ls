use std::collections::HashMap;
use std::env::current_dir;
use std::fs::read_dir;
use std::io;
use std::path::{Path, PathBuf};

use lsp_types::{
    CompletionOptions, CompletionOptionsCompletionItem, HoverProviderCapability, ServerCapabilities,
};

use super::{Handler, HandlerError};

#[derive(Debug)]
pub struct PropsHandler {
    prop_files: Vec<PathBuf>,
    definitions: HashMap<String, Vec<Definition>>,
}

#[derive(Debug, Default)]
struct Definition {
    from_path: String,
    name: String,
    value: String,
}

impl Default for PropsHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl PropsHandler {
    #[must_use]
    pub fn new() -> Self {
        let mut this = Self {
            prop_files: Self::get_prop_files().unwrap_or_default(),
            definitions: HashMap::new(),
        };

        this.parse_files();
        this
    }

    fn get_prop_files() -> Option<Vec<PathBuf>> {
        let cwd = current_dir().ok()?;
        traverse_parents(cwd, &[".git"], &[".env", ".env.example"]).ok()
    }

    fn parse_files(&mut self) {
        self.definitions.clear();

        for file in &self.prop_files {
            if let Ok(contents) = std::fs::read_to_string(file) {
                for mut line in contents.lines() {
                    line = line.trim();

                    if let Some((name, value)) = line.split_once('=') {
                        let name = name.trim().to_string();

                        self.definitions
                            .entry(name.clone())
                            .or_default()
                            .push(Definition {
                                from_path: file.to_string_lossy().to_string(),
                                name,
                                value: value.trim().to_string(),
                            });
                    }
                }
            }
        }
    }
}

impl Handler for PropsHandler {
    fn filetype_supported(&self, _filetype: &str) -> bool {
        true
    }

    fn get_capabilities(&self) -> ServerCapabilities {
        lsp_types::ServerCapabilities {
            completion_provider: Some(CompletionOptions {
                completion_item: Some(CompletionOptionsCompletionItem {
                    label_details_support: Some(true),
                }),
                ..Default::default()
            }),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            ..Default::default()
        }
    }

    fn hover(
        &self,
        contents: &str,
        position: lsp_types::Position,
    ) -> Result<Option<String>, HandlerError> {
        let mut bytes_traversed = 0;
        let mut cur_line = 0;

        for c in contents.chars() {
            if cur_line == position.line {
                break;
            }

            bytes_traversed += c.len_utf8();

            if c == '\n' {
                cur_line += 1;
            }
        }

        let contents = &contents[bytes_traversed..];

        let mut first_valid = 0;
        let mut bytes_traversed = 0;
        let mut num_chars_in_line = 0;

        for c in contents.chars() {
            if !c.is_ascii_alphanumeric() && c != '_' {
                if num_chars_in_line >= position.character {
                    break;
                } else {
                    first_valid = bytes_traversed + c.len_utf8();
                }
            }

            bytes_traversed += c.len_utf8();
            num_chars_in_line += c.len_utf16() as u32;
        }

        if first_valid >= bytes_traversed || bytes_traversed as u32 <= position.character {
            Ok(None)
        } else {
            let name = &contents[first_valid..bytes_traversed];
            let out = self.definitions.get(name).map(|defs| {
                defs.iter()
                    .map(|def| format!("{}\n{} = {}", def.from_path, def.name, def.value))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            });

            Ok(out)
        }
    }
}

fn traverse_parents<P: AsRef<Path>>(
    from_dir: P,
    stop_at_names: &[&str],
    file_names: &[&str],
) -> io::Result<Vec<PathBuf>> {
    let from_dir = from_dir.as_ref();

    let mut stop = false;
    let mut found = Vec::new();
    let cur_dir_entries = read_dir(from_dir)?;

    for entry in cur_dir_entries.flatten() {
        let file_name = entry.file_name();
        if stop_at_names.iter().any(|&name| name == file_name) {
            stop = true;
        } else if entry.file_type().ok().is_some_and(|ty| ty.is_file())
            && file_names.iter().any(|&name| name == file_name)
        {
            found.push(entry.path());
        }
    }

    if !stop {
        if let Some(parent) = from_dir.parent() {
            if let Ok(mut parent_finds) = traverse_parents(parent, stop_at_names, file_names) {
                found.append(&mut parent_finds);
            }
        }
    }

    Ok(found)
}
