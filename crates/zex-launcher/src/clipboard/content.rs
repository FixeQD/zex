use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Content {
    Text(String),
    Image {
        width: usize,
        height: usize,
        rgba: Vec<u8>,
    },
    Files(Vec<PathBuf>),
    Snippet {
        plain: String,
        html: String,
    },
}

impl Content {
    /// Short classification used as a subtitle
    pub fn kind_label(&self) -> &'static str {
        match self {
            Content::Text(_) => "text",
            Content::Image { .. } => "image",
            Content::Files(paths) if paths.len() > 1 => "files",
            Content::Files(_) => "file",
            Content::Snippet { .. } => "snippet",
        }
    }

    /// Stable identity used to skip consecutive duplicates
    pub(crate) fn signature(&self) -> String {
        match self {
            Content::Text(text) => format!("t:{text}"),
            Content::Image {
                width,
                height,
                rgba,
            } => format!("i:{width}x{height}:{}", rgba.len()),
            Content::Files(paths) => format!(
                "f:{}",
                paths
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            Content::Snippet { plain, html } => format!("s:{plain}\n{html}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub content: Content,
    pub stamp: SystemTime,
}

impl Entry {
    pub fn new(content: Content) -> Self {
        Self {
            content,
            stamp: SystemTime::now(),
        }
    }

    /// One-line preview for a list row (never splits a multi-byte character)
    pub fn snippet(&self) -> String {
        const LIMIT: usize = 30;
        match &self.content {
            Content::Text(text) | Content::Snippet { plain: text, .. } => {
                let first = text.lines().next().unwrap_or_default();
                let cut: String = first.chars().take(LIMIT).collect();
                if cut.len() < first.len() {
                    format!("{cut}...")
                } else {
                    cut
                }
            }
            Content::Image { .. } => "[image]".to_string(),
            Content::Files(paths) => match paths.as_slice() {
                [single] => single
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("[file]")
                    .to_string(),
                many => format!("[{} files]", many.len()),
            },
        }
    }

    pub fn body(&self) -> String {
        match &self.content {
            Content::Text(text) => text.clone(),
            Content::Snippet { plain, .. } => plain.clone(),
            Content::Image { .. } => "[image preview]".to_string(),
            Content::Files(paths) => paths
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}
