use crate::clipboard::{Content, Entry};
use anyhow::{Context, Result, bail};

pub fn restore(entry: &Entry) -> Result<()> {
    match &entry.content {
        Content::Text(text) | Content::Snippet { plain: text, .. } => place_text(text),
        Content::Image {
            width,
            height,
            rgba,
        } => place_image(*width, *height, rgba),
        Content::Files(paths) => {
            let text = paths
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join("\n");
            place_text(&text)
        }
    }
}

pub fn place_text(text: &str) -> Result<()> {
    if text.is_empty() {
        bail!("refusing to copy empty text");
    }
    let mut board = arboard::Clipboard::new().context("clipboard unavailable")?;
    board.set_text(text.to_string()).context("copy text failed")?;
    Ok(())
}

pub fn place_image(width: usize, height: usize, rgba: &[u8]) -> Result<()> {
    let mut board = arboard::Clipboard::new().context("clipboard unavailable")?;
    let image = arboard::ImageData {
        width,
        height,
        bytes: std::borrow::Cow::Borrowed(rgba),
    };
    board.set_image(image).context("copy image failed")?;
    Ok(())
}
