use anyhow::Result;
use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::{ParseOptions, Parser};
use oxc_span::SourceType;

pub fn pretty_print(source: &str, filename: &str, verbose: bool) -> Result<String> {
    if verbose {
        eprintln!("[parser] Parsing {} bytes...", source.len());
    }

    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename).unwrap_or_default();

    let parse_result = Parser::new(&allocator, source, source_type)
        .with_options(ParseOptions {
            parse_regular_expression: true,
            ..ParseOptions::default()
        })
        .parse();

    if verbose && !parse_result.errors.is_empty() {
        eprintln!(
            "[parser] {} non-fatal parse warnings",
            parse_result.errors.len()
        );
    }

    let code = Codegen::new().build(&parse_result.program).code;

    if verbose {
        eprintln!("[parser] Pretty-printed: {} lines", code.lines().count());
    }

    Ok(code)
}

pub fn statement_count(source: &str) -> usize {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path("input.js").unwrap_or_default();
    let parse_result = Parser::new(&allocator, source, source_type).parse();
    parse_result.program.body.len()
}
