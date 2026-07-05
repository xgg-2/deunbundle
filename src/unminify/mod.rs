// Unminify is handled directly in parser::pretty_print using OXC Codegen.
// This module is kept as a pass-through for clarity and future extension.

use anyhow::Result;

pub fn pretty_print(source: &str, verbose: bool) -> Result<String> {
    crate::parser::pretty_print(source, "input.js", verbose)
}
