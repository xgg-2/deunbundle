use anyhow::{Context, Result};
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Deserialize)]
pub struct SourceMap {
    pub version: u8,
    pub sources: Vec<String>,
    pub sources_content: Option<Vec<Option<String>>>,
    pub mappings: String,
    #[serde(rename = "sourceRoot")]
    pub source_root: Option<String>,
    pub names: Option<Vec<String>>,
}

pub struct RecoveredSources {
    pub files: Vec<RecoveredFile>,
}

pub struct RecoveredFile {
    pub original_path: String,
    pub content: String,
}

pub fn load_sourcemap(path: &str) -> Result<SourceMap> {
    let raw = fs::read_to_string(Path::new(path))
        .with_context(|| format!("Failed to read source map: {}", path))?;
    let map: SourceMap =
        serde_json::from_str(&raw).with_context(|| "Failed to parse source map JSON")?;
    Ok(map)
}

pub fn recover_sources(map: &SourceMap, verbose: bool) -> Result<RecoveredSources> {
    if map.version != 3 {
        anyhow::bail!(
            "Unsupported source map version: {} (only v3 is supported)",
            map.version
        );
    }

    let contents = match &map.sources_content {
        Some(c) => c,
        None => anyhow::bail!("Source map has no sourcesContent — cannot recover original files"),
    };

    let mut files = Vec::new();
    let root = map.source_root.as_deref().unwrap_or("");

    for (i, source_path) in map.sources.iter().enumerate() {
        let content = match contents.get(i).and_then(|c| c.as_ref()) {
            Some(c) => c.clone(),
            None => {
                if verbose {
                    eprintln!("[sourcemap] No content for source: {}", source_path);
                }
                continue;
            }
        };

        let full_path = if root.is_empty() {
            source_path.clone()
        } else {
            format!(
                "{}/{}",
                root.trim_end_matches('/'),
                source_path.trim_start_matches('/')
            )
        };

        let clean_path = clean_path(&full_path);

        if verbose {
            eprintln!("[sourcemap] Recovered: {}", clean_path);
        }

        files.push(RecoveredFile {
            original_path: clean_path,
            content,
        });
    }

    if verbose {
        eprintln!("[sourcemap] Recovered {} source files", files.len());
    }

    Ok(RecoveredSources { files })
}

fn clean_path(path: &str) -> String {
    path.replace("../", "")
        .replace("./", "")
        .trim_start_matches('/')
        .to_string()
}
