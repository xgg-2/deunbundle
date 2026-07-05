use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "deunbundle",
    about = "JS/TS deobfuscation and unbundling tool powered by SWC",
    version,
    long_about = None
)]
pub struct Cli {
    /// Local file path to process
    pub input: Option<String>,

    /// Remote URL to fetch and process
    #[arg(long, value_name = "URL")]
    pub url: Option<String>,

    /// Output directory
    #[arg(short, long, value_name = "DIR", default_value = "./output")]
    pub out: String,

    /// Enable verbose step-by-step logging
    #[arg(short, long)]
    pub verbose: bool,

    /// Maximum download size in bytes (default: 50MB)
    #[arg(long, default_value_t = 52_428_800)]
    pub max_size: u64,

    /// Skip bundle splitting, only pretty-print
    #[arg(long)]
    pub no_split: bool,

    /// Path to an associated source map file
    #[arg(long, value_name = "FILE")]
    pub sourcemap: Option<String>,
}

impl Cli {
    pub fn validate(&self) -> anyhow::Result<InputSource> {
        match (&self.input, &self.url) {
            (Some(path), None) => Ok(InputSource::LocalFile(path.clone())),
            (None, Some(url)) => Ok(InputSource::RemoteUrl(url.clone())),
            (Some(_), Some(_)) => anyhow::bail!("Provide either a local path or --url, not both."),
            (None, None) => anyhow::bail!("Provide a local file path or use --url <URL>."),
        }
    }
}

#[derive(Debug)]
pub enum InputSource {
    LocalFile(String),
    RemoteUrl(String),
}
