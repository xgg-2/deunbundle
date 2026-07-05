use anyhow::Result;
use oxc_allocator::Allocator;
use oxc_ast::ast::{Argument, Expression, ObjectPropertyKind, PropertyKey, Statement};
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::{GetSpan, Span, SourceType};
use std::collections::HashMap;

use crate::bundle_detect::BundleType;

pub struct SplitResult {
    pub files: HashMap<String, String>,
    pub stats: SplitStats,
}

#[derive(Default)]
pub struct SplitStats {
    pub components: usize,
    pub functions: usize,
    pub classes: usize,
    pub constants: usize,
    pub effects: usize,
    pub modules: usize,
}

#[derive(Debug)]
struct DeclInfo {
    name: String,
    kind: DeclKind,
    text: String,
}

#[derive(Debug, PartialEq)]
enum DeclKind {
    Component,
    Function,
    Class,
    Constant,
    Effect,
}

pub fn split(source: &str, bundle_type: &BundleType, verbose: bool) -> Result<SplitResult> {
    if verbose {
        eprintln!("[splitter] Splitting bundle (type: {bundle_type})...");
    }
    match bundle_type {
        BundleType::Webpack => split_webpack(source, verbose),
        _ => split_iife(source, verbose),
    }
}

fn split_iife(source: &str, verbose: bool) -> Result<SplitResult> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path("input.js").unwrap_or_default();
    let result = Parser::new(&allocator, source, source_type).parse();

    let mut decls: Vec<DeclInfo> = Vec::new();
    collect_declarations(source, result.program.body.as_slice(), &mut decls, 0, verbose);

    if verbose {
        eprintln!("[splitter] Collected {} declarations", decls.len());
    }

    let mut files: HashMap<String, String> = HashMap::new();
    let mut stats = SplitStats::default();
    let mut index_imports: Vec<String> = Vec::new();
    let mut constants_buf: Vec<String> = Vec::new();
    let mut effects_buf: Vec<String> = Vec::new();

    for decl in &decls {
        match decl.kind {
            DeclKind::Component => {
                let path = format!("components/{}.js", sanitize_filename(&decl.name));
                files.insert(path.clone(), format!("// {}\n\n{}\n", decl.name, decl.text));
                index_imports.push(format!("import './{path}';"));
                stats.components += 1;
            }
            DeclKind::Function => {
                let path = format!("functions/{}.js", sanitize_filename(&decl.name));
                files.insert(path.clone(), format!("// {}\n\n{}\n", decl.name, decl.text));
                index_imports.push(format!("import './{path}';"));
                stats.functions += 1;
            }
            DeclKind::Class => {
                let path = format!("classes/{}.js", sanitize_filename(&decl.name));
                files.insert(path.clone(), format!("// {}\n\n{}\n", decl.name, decl.text));
                index_imports.push(format!("import './{path}';"));
                stats.classes += 1;
            }
            DeclKind::Constant => {
                constants_buf.push(decl.text.clone());
                stats.constants += 1;
            }
            DeclKind::Effect => {
                effects_buf.push(decl.text.clone());
                stats.effects += 1;
            }
        }
    }

    if !constants_buf.is_empty() {
        files.insert(
            "constants/index.js".to_string(),
            format!("{}\n", constants_buf.join("\n\n")),
        );
        index_imports.push("import './constants/index.js';".to_string());
    }

    if !effects_buf.is_empty() {
        files.insert(
            "main.js".to_string(),
            format!("{}\n", effects_buf.join("\n\n")),
        );
        index_imports.push("import './main.js';".to_string());
    }

    files.insert("index.js".to_string(), index_imports.join("\n") + "\n");

    if verbose {
        eprintln!(
            "[splitter] {} components, {} functions, {} classes, {} constants, {} effects",
            stats.components, stats.functions, stats.classes, stats.constants, stats.effects
        );
    }

    Ok(SplitResult { files, stats })
}

// walks the statement list and recursively unwraps IIFEs rather than treating
// the whole thing as one big blob of mystery meat
fn collect_declarations<'a>(
    source: &str,
    stmts: &'a [Statement<'a>],
    output: &mut Vec<DeclInfo>,
    depth: usize,
    verbose: bool,
) {
    for stmt in stmts {
        match stmt {
            Statement::FunctionDeclaration(fn_decl) => {
                let name = fn_decl
                    .id
                    .as_ref()
                    .map(|id| id.name.to_string())
                    .unwrap_or_else(|| format!("_anon_{}", output.len()));
                let text = span_text(source, stmt.span());
                let kind = classify_fn_name(&name);
                if verbose {
                    eprintln!("[splitter] {:indent$}[{kind:?}] {name}", "", indent = depth * 2);
                }
                output.push(DeclInfo { name, kind, text });
            }

            Statement::ClassDeclaration(cls) => {
                let name = cls
                    .id
                    .as_ref()
                    .map(|id| id.name.to_string())
                    .unwrap_or_else(|| format!("_cls_{}", output.len()));
                let text = span_text(source, stmt.span());
                if verbose {
                    eprintln!("[splitter] {:indent$}[Class] {name}", "", indent = depth * 2);
                }
                output.push(DeclInfo { name, kind: DeclKind::Class, text });
            }

            Statement::VariableDeclaration(var_decl) => {
                // single declarator that is a named function gets its own file
                if var_decl.declarations.len() == 1 {
                    if let Some(decl) = var_decl.declarations.first() {
                        let is_fn = matches!(
                            &decl.init,
                            Some(Expression::FunctionExpression(_))
                                | Some(Expression::ArrowFunctionExpression(_))
                        );
                        if is_fn {
                            if let Some(name) = ident_name_of_binding(&decl.id) {
                                let text = span_text(source, stmt.span());
                                let kind = classify_fn_name(&name);
                                if verbose {
                                    eprintln!(
                                        "[splitter] {:indent$}[{kind:?}] {name}",
                                        "",
                                        indent = depth * 2
                                    );
                                }
                                output.push(DeclInfo { name, kind, text });
                                continue;
                            }
                        }
                    }
                }
                // everything else lumped into constants
                let text = span_text(source, stmt.span());
                output.push(DeclInfo {
                    name: format!("_const_{}", output.len()),
                    kind: DeclKind::Constant,
                    text,
                });
            }

            Statement::ExpressionStatement(expr_stmt) => {
                // if it's an IIFE, don't treat the whole body as one effect — go inside
                if let Some(inner_stmts) = iife_body_stmts(&expr_stmt.expression) {
                    if verbose {
                        eprintln!(
                            "[splitter] {:indent$}→ Unwrapping IIFE ({} stmts)",
                            "",
                            inner_stmts.len(),
                            indent = depth * 2
                        );
                    }
                    collect_declarations(source, inner_stmts, output, depth + 1, verbose);
                } else {
                    let text = span_text(source, stmt.span());
                    output.push(DeclInfo {
                        name: format!("_effect_{}", output.len()),
                        kind: DeclKind::Effect,
                        text,
                    });
                }
            }

            Statement::TryStatement(_) | Statement::IfStatement(_) => {
                let text = span_text(source, stmt.span());
                output.push(DeclInfo {
                    name: format!("_init_{}", output.len()),
                    kind: DeclKind::Effect,
                    text,
                });
            }

            _ => {
                let text = span_text(source, stmt.span());
                output.push(DeclInfo {
                    name: format!("_stmt_{}", output.len()),
                    kind: DeclKind::Effect,
                    text,
                });
            }
        }
    }
}

fn split_webpack(source: &str, verbose: bool) -> Result<SplitResult> {
    if verbose {
        eprintln!("[splitter] Extracting webpack modules...");
    }

    let allocator = Allocator::default();
    let source_type = SourceType::from_path("bundle.js").unwrap_or_default();
    let result = Parser::new(&allocator, source, source_type).parse();

    let mut modules: Vec<(String, String)> = Vec::new();

    for stmt in result.program.body.as_slice() {
        // webpack 5: var __webpack_modules__ = { ... }
        if let Statement::VariableDeclaration(var_decl) = stmt {
            for decl in &var_decl.declarations {
                if let Some(name) = ident_name_of_binding(&decl.id) {
                    if name == "__webpack_modules__" {
                        if let Some(Expression::ObjectExpression(obj)) = &decl.init {
                            for prop in &obj.properties {
                                if let ObjectPropertyKind::ObjectProperty(p) = prop {
                                    let key = prop_key_str(source, &p.key);
                                    let val = fn_body_text(source, &p.value)
                                        .unwrap_or_else(|| span_text(source, p.value.span()));
                                    modules.push((key, val));
                                }
                            }
                        }
                    }
                }
            }
        }
        // webpack 4: (function(modules){...})({0: function(){...}})
        if let Statement::ExpressionStatement(expr_stmt) = stmt {
            if let Some(mods) = extract_webpack_v4(source, &expr_stmt.expression) {
                if !mods.is_empty() {
                    modules = mods;
                }
            }
        }
    }

    let mut files: HashMap<String, String> = HashMap::new();
    let stats;

    if modules.is_empty() {
        if verbose {
            eprintln!("[splitter] No webpack modules found — single file fallback");
        }
        let code = Codegen::new().build(&result.program).code;
        files.insert("bundle.js".to_string(), code);
        stats = SplitStats { modules: 1, ..Default::default() };
    } else {
        if verbose {
            eprintln!("[splitter] Found {} webpack modules", modules.len());
        }
        let mut index_imports: Vec<String> = Vec::new();
        for (id, body) in &modules {
            let safe_id = sanitize_filename(id);
            let path = format!("modules/{safe_id}.js");
            files.insert(path.clone(), format!("// module {id}\n\n{body}\n"));
            index_imports.push(format!("import './{path}';"));
        }
        files.insert("index.js".to_string(), index_imports.join("\n") + "\n");
        let n = modules.len();
        stats = SplitStats { modules: n, ..Default::default() };
    }

    Ok(SplitResult { files, stats })
}

fn extract_webpack_v4<'a>(
    source: &str,
    expr: &'a Expression<'a>,
) -> Option<Vec<(String, String)>> {
    let call = match expr {
        Expression::CallExpression(c) => c,
        Expression::ParenthesizedExpression(p) => {
            return extract_webpack_v4(source, &p.expression);
        }
        Expression::UnaryExpression(u) => {
            return extract_webpack_v4(source, &u.argument);
        }
        _ => return None,
    };

    let first_arg = call.arguments.first()?;
    let arg_expr = match first_arg {
        Argument::SpreadElement(_) => return None,
        other => other.as_expression()?,
    };

    let mut modules = Vec::new();

    match arg_expr {
        Expression::ObjectExpression(obj) => {
            for prop in &obj.properties {
                if let ObjectPropertyKind::ObjectProperty(p) = prop {
                    let key = prop_key_str(source, &p.key);
                    let val = fn_body_text(source, &p.value)
                        .unwrap_or_else(|| span_text(source, p.value.span()));
                    modules.push((key, val));
                }
            }
        }
        Expression::ArrayExpression(arr) => {
            for (i, elem) in arr.elements.iter().enumerate() {
                if let Some(e) = elem.as_expression() {
                    let body = fn_body_text(source, e)
                        .unwrap_or_else(|| span_text(source, e.span()));
                    modules.push((i.to_string(), body));
                }
            }
        }
        _ => {}
    }

    if modules.is_empty() { None } else { Some(modules) }
}

// checks whether an expression is an IIFE and returns its body statements.
// the two-function split is intentional — recursing back into iife_body_stmts
// from the callee arm causes it to match the wrong branch (FunctionExpression
// is not a CallExpression, who knew)
fn iife_body_stmts<'a>(expr: &'a Expression<'a>) -> Option<&'a [Statement<'a>]> {
    match expr {
        Expression::CallExpression(call) => fn_stmts_from_callee(&call.callee),
        Expression::ParenthesizedExpression(p) => iife_body_stmts(&p.expression),
        Expression::UnaryExpression(u) => iife_body_stmts(&u.argument),
        _ => None,
    }
}

fn fn_stmts_from_callee<'a>(callee: &'a Expression<'a>) -> Option<&'a [Statement<'a>]> {
    match callee {
        Expression::FunctionExpression(fn_expr) => {
            fn_expr.body.as_ref().map(|b| b.statements.as_slice())
        }
        Expression::ArrowFunctionExpression(arrow) => Some(arrow.body.statements.as_slice()),
        Expression::ParenthesizedExpression(p) => fn_stmts_from_callee(&p.expression),
        // some minifiers do +function(){...}() or !function(){...}()
        Expression::UnaryExpression(u) => fn_stmts_from_callee(&u.argument),
        _ => None,
    }
}

fn fn_body_text(source: &str, expr: &Expression) -> Option<String> {
    match expr {
        Expression::FunctionExpression(fn_expr) => {
            fn_expr.body.as_ref().map(|b| span_text(source, b.span()))
        }
        Expression::ArrowFunctionExpression(arrow) => Some(span_text(source, arrow.body.span())),
        _ => None,
    }
}

fn prop_key_str(source: &str, key: &PropertyKey) -> String {
    match key {
        PropertyKey::StaticIdentifier(id) => id.name.to_string(),
        PropertyKey::StringLiteral(s) => s.value.to_string(),
        PropertyKey::NumericLiteral(n) => n.value.to_string(),
        other => span_text(source, other.span()),
    }
}

fn ident_name_of_binding(pat: &oxc_ast::ast::BindingPattern) -> Option<String> {
    pat.get_identifier_name().map(|id| id.to_string())
}

fn classify_fn_name(name: &str) -> DeclKind {
    if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
        DeclKind::Component
    } else {
        DeclKind::Function
    }
}

fn span_text(source: &str, span: Span) -> String {
    let start = span.start as usize;
    let end = (span.end as usize).min(source.len());
    source[start..end].trim().to_string()
}

fn sanitize_filename(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
        .collect();
    if s.is_empty() { "unnamed".to_string() } else { s }
}
