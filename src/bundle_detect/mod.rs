use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;

#[derive(Debug, Clone, PartialEq)]
pub enum BundleType {
    Webpack,
    Rollup,
    Browserify,
    Vite,
    UserScript,
    Unknown,
}

impl std::fmt::Display for BundleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BundleType::Webpack => write!(f, "Webpack"),
            BundleType::Rollup => write!(f, "Rollup"),
            BundleType::Browserify => write!(f, "Browserify"),
            BundleType::Vite => write!(f, "Vite"),
            BundleType::UserScript => write!(f, "UserScript / IIFE"),
            BundleType::Unknown => write!(f, "Unknown"),
        }
    }
}

pub fn detect(source: &str, verbose: bool) -> BundleType {
    let kind = detect_by_source(source).unwrap_or_else(|| detect_by_ast(source));
    if verbose {
        eprintln!("[bundle_detect] Detected bundle type: {}", kind);
    }
    kind
}

fn detect_by_source(source: &str) -> Option<BundleType> {
    // UserScript must come first — it's a superset of IIFE
    if source.contains("==UserScript==") || source.contains("// @namespace") {
        return Some(BundleType::UserScript);
    }

    // Webpack v4/v5
    if source.contains("__webpack_require__")
        || source.contains("webpackChunk")
        || source.contains("__webpack_modules__")
        || source.contains("webpackJsonp")
    {
        return Some(BundleType::Webpack);
    }

    // Browserify
    if source.contains("require.config") && source.contains("define.amd") {
        return Some(BundleType::Browserify);
    }

    // Vite
    if source.contains("import.meta.hot")
        || source.contains("/@vite/")
        || source.contains("vite/preload")
    {
        return Some(BundleType::Vite);
    }

    // Rollup
    if source.contains("rollupPluginBabelHelpers")
        || source.contains("'use strict';\n\nObject.defineProperty(exports")
    {
        return Some(BundleType::Rollup);
    }

    None
}

fn detect_by_ast(source: &str) -> BundleType {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path("input.js").unwrap_or_default();
    let result = Parser::new(&allocator, source, source_type).parse();

    for stmt in &result.program.body {
        if let Statement::ExpressionStatement(expr_stmt) = stmt {
            if is_iife_expr(&expr_stmt.expression) {
                return BundleType::UserScript;
            }
        }
    }

    BundleType::Unknown
}

fn is_iife_expr(expr: &Expression) -> bool {
    match expr {
        Expression::CallExpression(call) => matches!(
            &call.callee,
            Expression::FunctionExpression(_)
                | Expression::ArrowFunctionExpression(_)
                | Expression::ParenthesizedExpression(_)
        ),
        Expression::ParenthesizedExpression(p) => is_iife_expr(&p.expression),
        Expression::UnaryExpression(u) => is_iife_expr(&u.argument),
        _ => false,
    }
}
