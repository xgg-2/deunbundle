# deunbundle

[![CI](https://github.com/xgg-2/deunbundle/actions/workflows/ci.yml/badge.svg)](https://github.com/xgg-2/deunbundle/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

> **JavaScript/TypeScript deobfuscation and unbundling — blazing fast, powered by [OXC](https://oxc.rs/).**

`deunbundle` takes a minified or bundled JavaScript/TypeScript file (local or remote URL) and reconstructs clean, readable, modular source files. It detects Webpack, Rollup, Vite, Browserify, and UserScript/IIFE bundles, extracts individual modules or components into separate files, and identifies embedded libraries.

---

## Features

- **Pretty-print** any minified JS/TS using OXC's production-grade code generator
- **Smart bundle detection** — Webpack v4/v5, Rollup, Vite, Browserify, UserScript, IIFE
- **Deep IIFE unwrapping** — recursively extracts functions & React components into individual files
- **Webpack module extraction** — splits each `__webpack_require__` module into its own file
- **Library fingerprinting** — detects React, ReactDOM, Vue, TailwindCSS, jQuery, Lodash, Axios, and more
- **Source map recovery** — reconstruct original files from `.map` sources
- **Remote URL support** — fetch directly from CDN or GitHub raw URLs
- **Progress indicators** — real-time spinner for each pipeline step
- **Zero OpenSSL** — uses `rustls` for TLS; compiles anywhere Rust does

---

## Install

### Pre-built binaries

Download from [Releases](https://github.com/xgg-2/deunbundle/releases) for Linux, macOS (x86_64 + Apple Silicon), or Windows.

### Build from source

```bash
cargo install --git https://github.com/xgg-2/deunbundle
# or clone and build locally:
git clone https://github.com/xgg-2/deunbundle
cd deunbundle
cargo build --release
./target/release/deunbundle --help
```

Requires **Rust 1.75+**.

---

## Usage

```bash
# Pretty-print only (single readable file, no splitting)
deunbundle ./bundle.min.js --out ./output --no-split

# Split into modules (default)
deunbundle ./bundle.min.js --out ./output

# Fetch from a URL and split
deunbundle --url https://cdn.example.com/app.bundle.js --out ./output

# Recover original sources via source map
deunbundle ./bundle.js --sourcemap ./bundle.js.map --out ./output

# Verbose mode
deunbundle ./bundle.js --out ./output --verbose
```

### All options

```
Arguments:
  [FILE]                   Path to local JS/TS file

Options:
  -u, --url <URL>          Remote URL to fetch JS from
  -o, --out <DIR>          Output directory [default: ./output]
  -s, --sourcemap <FILE>   Source map (.map) for original source recovery
      --no-split           Emit a single pretty-printed file instead of splitting
      --max-size <BYTES>   Max download size [default: 52428800 (50 MB)]
  -v, --verbose            Print debug info for each pipeline step
  -h, --help               Print help
  -V, --version            Print version
```

---

## Output structure

**UserScript / IIFE bundle:**

```
output/
├── index.js              ← entry point (imports all modules)
├── main.js               ← side effects & entry-point code
├── constants/
│   └── index.js          ← module-level variables
├── components/
│   ├── App.js            ← uppercase functions → React/UI components
│   └── Modal.js
└── functions/
    ├── formatDate.js
    └── fetchUser.js
```

**Webpack bundle:**

```
output/
├── index.js
└── modules/
    ├── 0.js              ← webpack v4 numeric module IDs
    └── _src_index_js.js  ← webpack v5 path-based IDs
```

---

## Example output

```
$ deunbundle ./discord-addon.user.js --out ./out

  ·─────────────────────────────────────·
  │  deunbundle v0.1.0                  │
  │  JS/TS deobfuscation & unbundling   │
  │  powered by OXC                     │
  ·─────────────────────────────────────·

  ✓ 523.2 KB from ./discord-addon.user.js
  ✓ UserScript / IIFE — 2 top-level statements
  ✓ Found: TailwindCSS, ReactDOM, React
  ✓ 19,171 lines of readable code
  ✓ 47 files generated

  Output tree:
  out/
  ├── index.js (312 B)
  ├── main.js (1.2 KB)
  ├── constants/index.js (44.1 KB)
  ├── components/App.js (3.2 KB)
  └── functions/formatDate.js (512 B)

  ────────────────────────────────────────────
  Bundle type : UserScript / IIFE
  Input size  : 523.2 KB → 19,171 lines
  Files out   : 47 files
  Libraries   : TailwindCSS, ReactDOM, React
  Components  : 31
  Functions   : 14
  ────────────────────────────────────────────
```

---

## How it works

```
Input (minified JS/TS)
       │
       ▼
  [fetch]          local file or remote URL (reqwest + rustls)
       │
       ▼
  [bundle_detect]  heuristic + AST-based bundle type detection
       │
       ▼
  [lib_detect]     regex fingerprinting for known library signatures
       │
       ▼
  [parser]         OXC parse → OXC Codegen → pretty-printed source
       │
       ▼
  [splitter]       deep IIFE unwrapping / webpack module extraction
       │
       ▼
  Output files     components/, functions/, modules/, index.js
```

---

## Module reference

| Module | Responsibility |
|--------|---------------|
| `cli` | Argument parsing via `clap` |
| `fetch` | Local file read + remote URL fetch with progress |
| `bundle_detect` | Heuristic + AST detection of bundle format |
| `lib_detect` | Embedded library fingerprinting |
| `parser` | OXC parse + pretty-print via Codegen |
| `splitter` | IIFE unwrapping, webpack module extraction, file organisation |
| `sourcemap` | Source map recovery (`.map` → original files) |

---

## Supported bundle formats

| Format | Detection |
|--------|-----------|
| Webpack v4 | `__webpack_require__`, numeric module array/object argument |
| Webpack v5 | `__webpack_modules__`, `webpackChunk` |
| Rollup | `'use strict'; Object.defineProperty(exports, ...)` |
| Vite | `import.meta.hot`, `/@vite/` |
| Browserify | `require.config` + `define.amd` |
| UserScript | `==UserScript==` header |
| Plain IIFE | AST-detected `(function(){...})()` |

---

## Development

```bash
cargo test          # run all tests
cargo fmt --check   # check formatting
cargo clippy        # lint
cargo build --release
```

---

## License

MIT © 2026
