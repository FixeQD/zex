//! Palette generator subprocess wrapper

use std::{collections::HashMap, path::Path, process::Stdio};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::palette::Palette;

/// Known scheme families, in the order they appear in the settings UI
pub const SCHEMES: [&str; 7] = [
    "tonal_spot",
    "content",
    "expressive",
    "neutral",
    "monochrome",
    "rainbow",
    "fidelity",
];

/// How many preview swatches the settings panel shows: every scheme in dark mode + first three in light mode
pub const PREVIEW_COUNT: usize = 10;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preview {
    pub scheme: String,
    pub dark: bool,
    pub primary: String,
    pub secondary: String,
    pub tertiary: String,
    pub surface: String,
}

pub async fn generate(image: &Path, scheme: &str, dark: bool) -> Result<Palette> {
    let mode = if dark { "dark" } else { "light" };
    let json = run_json(image, scheme, mode).await?;
    palette_from_json(&json, dark)
}

/// Generate the fixed preview list for the settings panel
pub async fn previews(image: &Path) -> Result<Vec<Preview>> {
    let mut out = Vec::with_capacity(PREVIEW_COUNT);
    for scheme in &SCHEMES {
        out.push(preview_for(image, scheme, true).await?);
    }
    for scheme in &SCHEMES[..3] {
        out.push(preview_for(image, scheme, false).await?);
    }
    Ok(out)
}

async fn preview_for(image: &Path, scheme: &str, dark: bool) -> Result<Preview> {
    let palette = generate(image, scheme, dark).await?;
    Ok(Preview {
        scheme: scheme.to_string(),
        dark,
        primary: palette.primary.to_css(),
        secondary: palette.secondary.to_css(),
        tertiary: palette.tertiary.to_css(),
        surface: palette.surface.to_css(),
    })
}

/// Parse a generator JSON dump into a palette
pub fn palette_from_json(value: &Value, dark: bool) -> Result<Palette> {
    let mut map: HashMap<String, String> = HashMap::new();
    let object = value
        .as_object()
        .context("palette dump is not a JSON object")?;

    if let Some(colors) = object.get("colors").and_then(Value::as_object) {
        for (token, entry) in colors {
            let Some(entry) = entry.as_object() else {
                continue;
            };
            let mode_entry = entry
                .get(if dark { "dark" } else { "light" })
                .and_then(Value::as_object)
                .and_then(|mode| mode.get("color"))
                .and_then(Value::as_str);
            let colour = mode_entry.or_else(|| {
                entry
                    .get("default")
                    .and_then(Value::as_object)
                    .and_then(|d| d.get("color"))
                    .and_then(Value::as_str)
            });
            if let Some(colour) = colour {
                map.insert(super::palette::normalize(token), colour.to_string());
            }
        }
    } else {
        for (key, value) in object {
            if let Some(text) = value.as_str() {
                map.insert(super::palette::normalize(key), text.to_string());
            }
        }
    }

    let mut palette = Palette::default();
    palette.from_token_map(&map);
    Ok(palette)
}

async fn run_json(image: &Path, scheme: &str, mode: &str) -> Result<Value> {
    let type_arg = format!("scheme-{}", scheme.replace('_', "-"));
    let output = tokio::process::Command::new("matugen")
        .arg("image")
        .args(["-t", &type_arg])
        .args(["-m", mode])
        .arg("--json")
        .arg("hex")
        .arg("--dry-run")
        .args(["--prefer", "darkness"])
        .arg(image)
        .stdin(Stdio::null())
        .output()
        .await
        .with_context(|| format!("failed to spawn matugen for {scheme}/{mode}"))?;

    anyhow::ensure!(
        output.status.success(),
        "matugen failed for {scheme}/{mode}: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );

    let stdout = String::from_utf8(output.stdout).context("matugen stdout is not UTF-8")?;
    serde_json::from_str(&stdout)
        .with_context(|| format!("invalid matugen JSON for {scheme}/{mode}"))
}

/// Check if the generator binary is reachable
pub async fn available() -> bool {
    tokio::process::Command::new("matugen")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .is_ok_and(|status| status.success())
}
