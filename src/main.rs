mod bundle_detect;
mod cli;
mod fetch;
mod lib_detect;
mod parser;
mod sourcemap;
mod splitter;
mod unminify;

use anyhow::Result;
use clap::Parser as ClapParser;
use cli::{Cli, InputSource};
use indicatif::{ProgressBar, ProgressStyle};
use std::{collections::BTreeMap, fs, path::Path, time::Duration};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let source_input = cli.validate()?;

    print_banner();

    let out_dir = Path::new(&cli.out);

    let step = step_bar("Fetching source...");
    let fetched = match &source_input {
        InputSource::LocalFile(path) => {
            if cli.verbose {
                eprintln!("[fetch] Reading local file: {}", path);
            }
            fetch::fetch_local(path)?
        }
        InputSource::RemoteUrl(url) => fetch::fetch_remote(url, cli.max_size, cli.verbose)?,
    };
    step.finish_with_message(format!("{} bytes from {}", fetched.source.len(), fetched.origin));

    // source map path: if the user gave us a .map file, try to recover original sources
    // and bail out early — no point running the rest of the pipeline
    if let Some(ref map_path) = cli.sourcemap {
        let step = step_bar("Loading source map...");
        match sourcemap::load_sourcemap(map_path) {
            Ok(map) => match sourcemap::recover_sources(&map, cli.verbose) {
                Ok(recovered) => {
                    let n = recovered.files.len();
                    step.finish_with_message(format!("Recovered {n} original source files"));
                    fs::create_dir_all(out_dir)?;
                    for file in &recovered.files {
                        let dest = out_dir.join(&file.original_path);
                        if let Some(p) = dest.parent() {
                            fs::create_dir_all(p)?;
                        }
                        fs::write(&dest, &file.content)?;
                    }
                    println!();
                    println!("  Source map recovery → {}", out_dir.display());
                    print_tree(out_dir, n, 0, 0, 0, 0)?;
                    return Ok(());
                }
                Err(e) => step.finish_with_message(format!("Source map recovery failed: {e}")),
            },
            Err(e) => step.finish_with_message(format!("Could not load source map: {e}")),
        }
    }

    let step = step_bar("Detecting bundle format...");
    let bundle_type = bundle_detect::detect(&fetched.source, cli.verbose);
    let stmt_count = parser::statement_count(&fetched.source);
    step.finish_with_message(format!("{bundle_type} — {stmt_count} top-level statements"));

    let step = step_bar("Fingerprinting libraries...");
    let libs = lib_detect::detect_libraries(&fetched.source, cli.verbose);
    if libs.is_empty() {
        step.finish_with_message("No known libraries detected".to_string());
    } else {
        let names: Vec<_> = libs.iter().map(|l| l.name.as_str()).collect();
        step.finish_with_message(format!("Found: {}", names.join(", ")));
    }

    let step = step_bar("Unminifying (pretty-print)...");
    let pretty = unminify::pretty_print(&fetched.source, cli.verbose)?;
    let line_count = pretty.lines().count();
    step.finish_with_message(format!("{line_count} lines of readable code"));

    fs::create_dir_all(out_dir)?;

    if cli.no_split {
        let out_file = out_dir.join("output.js");
        fs::write(&out_file, &pretty)?;
        println!();
        println!("  {} → {} lines", human_bytes(fetched.source.len()), line_count);
        return Ok(());
    }

    let step = step_bar("Splitting into modules...");
    let split = splitter::split(&pretty, &bundle_type, cli.verbose)?;
    let file_count = split.files.len();
    step.finish_with_message(format!("{file_count} files generated"));

    // sorted write so the output is deterministic
    let sorted: BTreeMap<_, _> = split.files.iter().collect();
    for (rel_path, content) in &sorted {
        let dest = out_dir.join(rel_path.as_str());
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, content)?;
        if cli.verbose {
            eprintln!("[output] Written: {}", dest.display());
        }
    }

    println!();
    println!("  Output → {}", out_dir.display());
    println!();
    print_tree(out_dir, file_count, split.stats.components, split.stats.functions, split.stats.classes, split.stats.constants)?;

    println!();
    println!("  Bundle type : {}", bundle_type);
    println!("  Input size  : {} → {} lines", human_bytes(fetched.source.len()), line_count);
    println!("  Files out   : {} files", file_count);
    if !libs.is_empty() {
        let lib_list: Vec<_> = libs.iter().map(|l| match &l.version_hint {
            Some(v) => format!("{} v{}", l.name, v),
            None => l.name.clone(),
        }).collect();
        println!("  Libraries   : {}", lib_list.join(", "));
    }
    if split.stats.components > 0 { println!("  Components  : {}", split.stats.components); }
    if split.stats.functions > 0  { println!("  Functions   : {}", split.stats.functions); }
    if split.stats.classes > 0    { println!("  Classes     : {}", split.stats.classes); }
    if split.stats.modules > 0    { println!("  Modules     : {}", split.stats.modules); }
    println!();

    Ok(())
}

fn print_banner() {
    println!();
    println!("  deunbundle v{}", env!("CARGO_PKG_VERSION"));
    println!("  JS/TS deobfuscation & unbundling — powered by OXC");
    println!();
}

fn step_bar(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("  {spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

fn print_tree(
    out_dir: &Path,
    total: usize,
    _components: usize,
    _functions: usize,
    _classes: usize,
    _constants: usize,
) -> Result<()> {
    let entries = collect_tree_entries(out_dir)?;
    let last = entries.len().saturating_sub(1);
    for (i, (path, size)) in entries.iter().enumerate() {
        let connector = if i == last { "└──" } else { "├──" };
        let rel = path.strip_prefix(out_dir).unwrap_or(path);
        println!("  {} {} ({})", connector, rel.display(), human_bytes(*size as usize));
    }
    if total > 0 {
        println!();
        println!("  {} files", total);
    }
    Ok(())
}

fn collect_tree_entries(dir: &Path) -> Result<Vec<(std::path::PathBuf, u64)>> {
    let mut entries = Vec::new();
    if dir.exists() {
        collect_dir_recursive(dir, &mut entries)?;
        entries.sort_by(|a, b| a.0.cmp(&b.0));
    }
    Ok(entries)
}

fn collect_dir_recursive(dir: &Path, out: &mut Vec<(std::path::PathBuf, u64)>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_dir_recursive(&path, out)?;
        } else {
            let size = entry.metadata()?.len();
            out.push((path, size));
        }
    }
    Ok(())
}

fn human_bytes(n: usize) -> String {
    if n < 1024 {
        format!("{n} B")
    } else if n < 1024 * 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else {
        format!("{:.1} MB", n as f64 / (1024.0 * 1024.0))
    }
}
