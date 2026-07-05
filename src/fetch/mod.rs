use anyhow::{bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::{fs, path::Path, time::Duration};

pub struct FetchResult {
    pub source: String,
    pub origin: String,
}

pub fn fetch_local(path: &str) -> Result<FetchResult> {
    let p = Path::new(path);
    if !p.exists() {
        bail!("File not found: {}", path);
    }
    if !p.is_file() {
        bail!("Path is not a file: {}", path);
    }
    let source = fs::read_to_string(p)
        .with_context(|| format!("Failed to read file: {}", path))?;
    Ok(FetchResult {
        source,
        origin: path.to_string(),
    })
}

pub fn fetch_remote(url: &str, max_size: u64, verbose: bool) -> Result<FetchResult> {
    validate_url(url)?;

    if verbose {
        eprintln!("[fetch] Connecting to: {}", url);
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
    );
    pb.set_message("Fetching remote file...");
    pb.enable_steady_tick(Duration::from_millis(80));

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent("deunbundle/0.1.0")
        .build()
        .context("Failed to build HTTP client")?;

    let response = client
        .get(url)
        .send()
        .with_context(|| format!("HTTP request failed for: {}", url))?;

    let status = response.status();
    if !status.is_success() {
        bail!("HTTP {} — could not fetch: {}", status, url);
    }

    validate_content_type(&response)?;

    let content_length = response.content_length().unwrap_or(0);
    if content_length > max_size {
        bail!(
            "Remote file too large: {} bytes (limit: {} bytes)",
            content_length,
            max_size
        );
    }

    let bytes = response.bytes().context("Failed to read response body")?;
    if bytes.len() as u64 > max_size {
        bail!(
            "Downloaded content too large: {} bytes (limit: {} bytes)",
            bytes.len(),
            max_size
        );
    }

    pb.finish_with_message(format!("Downloaded {} bytes", bytes.len()));

    let source = String::from_utf8(bytes.to_vec())
        .context("Response body is not valid UTF-8")?;

    let cache_path = cache_locally(url, &source)?;
    if verbose {
        eprintln!("[fetch] Cached to: {}", cache_path);
    }

    Ok(FetchResult {
        source,
        origin: url.to_string(),
    })
}

fn validate_url(url: &str) -> Result<()> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        bail!("Invalid URL — must start with http:// or https://: {}", url);
    }
    Ok(())
}

fn validate_content_type(response: &reqwest::blocking::Response) -> Result<()> {
    if let Some(ct) = response.headers().get("content-type") {
        let ct_str = ct.to_str().unwrap_or("");
        let ok = ct_str.contains("javascript")
            || ct_str.contains("text/plain")
            || ct_str.contains("application/octet-stream")
            || ct_str.contains("text/html");
        if !ok {
            bail!(
                "Unexpected content-type '{}' — expected a JavaScript file",
                ct_str
            );
        }
    }
    Ok(())
}

fn cache_locally(url: &str, source: &str) -> Result<String> {
    let cache_dir = std::env::temp_dir().join("deunbundle_cache");
    fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;

    let safe_name: String = url
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' { c } else { '_' })
        .collect();
    let filename = format!("{}.js", &safe_name[..safe_name.len().min(80)]);
    let cache_path = cache_dir.join(&filename);

    fs::write(&cache_path, source).context("Failed to write cache file")?;
    Ok(cache_path.to_string_lossy().to_string())
}
