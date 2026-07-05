use deunbundle::{bundle_detect, fetch, lib_detect, parser, splitter, unminify};

// parser stuff

#[test]
fn test_pretty_print_minified() {
    let minified =
        r#"(function(){var a=1,b=2;function add(x,y){return x+y;}console.log(add(a,b));})();"#;
    let result = parser::pretty_print(minified, "test.js", false);
    assert!(result.is_ok(), "Parser should succeed on valid JS");
    let code = result.unwrap();
    assert!(
        code.lines().count() > 3,
        "Pretty-print should expand minified code"
    );
    assert!(
        code.contains("function add"),
        "Should preserve function declaration"
    );
}

#[test]
fn test_pretty_print_jsx() {
    // yes OXC handles JSX, no you don't need babel for this
    let jsx = r#"const App=()=><div className="app"><h1>Hello</h1></div>;"#;
    let result = parser::pretty_print(jsx, "app.jsx", false);
    assert!(result.is_ok(), "Parser should handle JSX syntax");
}

#[test]
fn test_statement_count() {
    let source = "const a = 1;\nconst b = 2;\nconst c = 3;";
    let count = parser::statement_count(source);
    assert_eq!(count, 3, "Should count 3 top-level statements");
}

// bundle detection

#[test]
fn test_detect_webpack() {
    let source = r#"
        (function(modules) {
            function __webpack_require__(id) { return modules[id]; }
        })({ 0: function(m,e,r){ m.exports=42; } });
    "#;
    let bt = bundle_detect::detect(source, false);
    assert_eq!(bt, bundle_detect::BundleType::Webpack);
}

#[test]
fn test_detect_userscript() {
    let source =
        "// ==UserScript==\n// @name Test\n// ==/UserScript==\n(function(){'use strict';})();";
    let bt = bundle_detect::detect(source, false);
    assert_eq!(bt, bundle_detect::BundleType::UserScript);
}

#[test]
fn test_detect_vite() {
    let source = "if (import.meta.hot) { import.meta.hot.accept(); }";
    let bt = bundle_detect::detect(source, false);
    assert_eq!(bt, bundle_detect::BundleType::Vite);
}

#[test]
fn test_detect_rollup_iife() {
    // plain IIFE with no webpack markers — just a guy doing his thing
    let source = "(function(){'use strict';var x=1;})();";
    let bt = bundle_detect::detect(source, false);
    assert!(
        bt == bundle_detect::BundleType::UserScript || bt == bundle_detect::BundleType::Unknown,
        "Pure IIFE should be detected as IIFE-type: got {bt}"
    );
}

// library fingerprinting

#[test]
fn test_detect_react() {
    let source = r#"Symbol.for("react.memo_cache_sentinel");var r={useState:function(){}}"#;
    let libs = lib_detect::detect_libraries(source, false);
    let names: Vec<_> = libs.iter().map(|l| l.name.as_str()).collect();
    assert!(
        names.contains(&"React"),
        "Should detect React: got {names:?}"
    );
}

#[test]
fn test_detect_tailwind() {
    let source = "var c='--tw-translate-x:0;--tw-translate-y:0;';";
    let libs = lib_detect::detect_libraries(source, false);
    let names: Vec<_> = libs.iter().map(|l| l.name.as_str()).collect();
    assert!(
        names.contains(&"TailwindCSS"),
        "Should detect Tailwind: got {names:?}"
    );
}

#[test]
fn test_no_false_positives_on_plain_js() {
    // if this fails we have a serious problem
    let source = "function hello() { return 'world'; }";
    let libs = lib_detect::detect_libraries(source, false);
    assert!(
        libs.is_empty(),
        "Plain JS should have no library detections"
    );
}

// fetch module

#[test]
fn test_fetch_local_rollup_fixture() {
    let result = fetch::fetch_local("tests/fixtures/rollup_sample.js");
    assert!(result.is_ok(), "Should read rollup fixture");
    let f = result.unwrap();
    assert!(
        f.source.contains("use strict"),
        "Fixture should contain 'use strict'"
    );
}

#[test]
fn test_fetch_local_webpack_fixture() {
    let result = fetch::fetch_local("tests/fixtures/webpack_sample.js");
    assert!(result.is_ok(), "Should read webpack fixture");
    let f = result.unwrap();
    assert!(
        f.source.contains("__webpack_require__"),
        "Webpack fixture should contain require fn"
    );
}

#[test]
fn test_fetch_missing_file() {
    // file not found. shocking.
    let result = fetch::fetch_local("no_such_file_xyz.js");
    assert!(result.is_err(), "Missing file should produce an error");
}

// splitter

#[test]
fn test_split_rollup_fixture() {
    let source =
        std::fs::read_to_string("tests/fixtures/rollup_sample.js").expect("fixture should exist");
    let pretty = unminify::pretty_print(&source, false).unwrap();
    let result = splitter::split(&pretty, &bundle_detect::BundleType::UserScript, false);
    assert!(result.is_ok(), "Should split rollup fixture");
    let split = result.unwrap();
    assert!(
        split.files.len() >= 2,
        "Should produce at least 2 files: got {}",
        split.files.len()
    );
    assert!(
        split.files.contains_key("index.js"),
        "Must produce index.js"
    );
}

#[test]
fn test_split_webpack_fixture() {
    let source =
        std::fs::read_to_string("tests/fixtures/webpack_sample.js").expect("fixture should exist");
    let pretty = unminify::pretty_print(&source, false).unwrap();
    let result = splitter::split(&pretty, &bundle_detect::BundleType::Webpack, false);
    assert!(result.is_ok(), "Should split webpack fixture");
    let split = result.unwrap();
    assert!(!split.files.is_empty(), "Should produce at least one file");
}

#[test]
fn test_split_iife_deep_unwrap() {
    // uppercase function inside IIFE → should end up in components/, not dumped in main.js
    let source =
        "(function(){\n  var VERSION='1.0';\n  function App(){return null;}\n  App();\n})();";
    let pretty = unminify::pretty_print(source, false).unwrap();
    let result = splitter::split(&pretty, &bundle_detect::BundleType::UserScript, false);
    assert!(result.is_ok());
    let split = result.unwrap();
    let has_component = split.files.keys().any(|k| k.starts_with("components/"));
    assert!(
        has_component,
        "Uppercase function should create a component file: {:?}",
        split.files.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_split_nonempty_files() {
    let source = "(function(){\n  function helper(){return 1;}\n  helper();\n})();";
    let pretty = unminify::pretty_print(source, false).unwrap();
    let split = splitter::split(&pretty, &bundle_detect::BundleType::UserScript, false).unwrap();
    for (path, content) in &split.files {
        assert!(
            !content.trim().is_empty(),
            "File '{path}' should not be empty"
        );
    }
}

// round-trip sanity check

#[test]
fn test_roundtrip_is_valid_js() {
    // if the output of our pretty-printer can't be parsed again, something is very wrong
    let source = std::fs::read_to_string("tests/fixtures/rollup_sample.js").unwrap();
    let pretty = unminify::pretty_print(&source, false).unwrap();
    let reparsed = parser::pretty_print(&pretty, "roundtrip.js", false);
    assert!(reparsed.is_ok(), "Pretty-printed output must be valid JS");
}
