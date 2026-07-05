use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LibraryMatch {
    pub name: String,
    pub version_hint: Option<String>,
    pub npm_package: String,
    pub confidence: Confidence,
}

#[derive(Debug, Clone)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Confidence::High => write!(f, "high"),
            Confidence::Medium => write!(f, "medium"),
            Confidence::Low => write!(f, "low"),
        }
    }
}

pub fn detect_libraries(source: &str, verbose: bool) -> Vec<LibraryMatch> {
    let fingerprints = build_fingerprint_table();
    let mut results = Vec::new();

    for (lib_name, fp) in &fingerprints {
        let matched = fp
            .signatures
            .iter()
            .filter(|sig| source.contains(*sig))
            .count();

        if matched == 0 {
            continue;
        }

        let confidence = if matched >= fp.signatures.len() {
            Confidence::High
        } else if matched >= fp.signatures.len() / 2 + 1 {
            Confidence::Medium
        } else {
            Confidence::Low
        };

        let version_hint = fp
            .version_patterns
            .iter()
            .find_map(|pattern| extract_version(source, pattern));

        results.push(LibraryMatch {
            name: lib_name.clone(),
            version_hint,
            npm_package: fp.npm_package.clone(),
            confidence,
        });
    }

    if verbose && !results.is_empty() {
        eprintln!("[lib_detect] Detected {} known libraries:", results.len());
        for m in &results {
            eprintln!(
                "  - {} ({} confidence) → {}",
                m.name, m.confidence, m.npm_package
            );
        }
    }

    results
}

struct Fingerprint {
    signatures: Vec<&'static str>,
    version_patterns: Vec<&'static str>,
    npm_package: String,
}

fn build_fingerprint_table() -> HashMap<String, Fingerprint> {
    let mut map = HashMap::new();

    map.insert(
        "React".to_string(),
        Fingerprint {
            signatures: vec![
                "react.memo_cache_sentinel",
                "__SECRET_INTERNALS_DO_NOT_USE_OR_YOU_WILL_BE_FIRED",
                "createElement",
                "jsx",
                "useEffect",
                "useState",
            ],
            version_patterns: vec!["React.version", "\"version\":\""],
            npm_package: "react".to_string(),
        },
    );

    map.insert(
        "ReactDOM".to_string(),
        Fingerprint {
            signatures: vec!["createRoot", "ReactDOM.render", "hydrateRoot"],
            version_patterns: vec![],
            npm_package: "react-dom".to_string(),
        },
    );

    map.insert(
        "Lodash".to_string(),
        Fingerprint {
            signatures: vec!["_.debounce", "_.throttle", "_.merge", "_.cloneDeep", "_.isEqual"],
            version_patterns: vec!["VERSION=\"", "lodash.version"],
            npm_package: "lodash".to_string(),
        },
    );

    map.insert(
        "jQuery".to_string(),
        Fingerprint {
            signatures: vec!["jQuery.fn", "$.ajax", "$(document)", "jQuery.extend"],
            version_patterns: vec!["jquery:\"", "jQuery.fn.jquery"],
            npm_package: "jquery".to_string(),
        },
    );

    map.insert(
        "Axios".to_string(),
        Fingerprint {
            signatures: vec!["axios.get", "axios.post", "axios.create", "AxiosError"],
            version_patterns: vec![],
            npm_package: "axios".to_string(),
        },
    );

    map.insert(
        "TailwindCSS".to_string(),
        Fingerprint {
            signatures: vec!["--tw-", "tailwindcss", "@tailwind"],
            version_patterns: vec![],
            npm_package: "tailwindcss".to_string(),
        },
    );

    map.insert(
        "Framer Motion".to_string(),
        Fingerprint {
            signatures: vec!["framer-motion", "AnimatePresence", "motion.div"],
            version_patterns: vec![],
            npm_package: "framer-motion".to_string(),
        },
    );

    map
}

fn extract_version(source: &str, pattern: &str) -> Option<String> {
    let idx = source.find(pattern)?;
    let after = &source[idx + pattern.len()..];
    let version: String = after
        .chars()
        .skip_while(|c| !c.is_numeric())
        .take_while(|c| c.is_numeric() || *c == '.')
        .collect();
    if version.is_empty() {
        None
    } else {
        Some(version)
    }
}
