//! Guards the crate boundary: `pxcii-core` must not touch files, paths or the environment.

use std::path::{Path, PathBuf};

use crate::workspace_paths;

/// Patterns that must never appear in `pxcii-core` sources.
///
/// The short module forms (`fs::`, `path::`, `env::`) catch grouped imports such as
/// `use std::{fs, path};`, which carry neither the fully qualified path nor `Path`.
const FORBIDDEN: [&str; 8] = [
    "std::fs",
    "std::path",
    "std::env",
    "Path",
    "fs::",
    "path::",
    "env::",
    "process::",
];

/// One forbidden occurrence, reported as `file:line`.
#[derive(Debug, PartialEq, Eq)]
struct Violation {
    file: String,
    line: usize,
    pattern: &'static str,
    text: String,
}

/// Runs the check over the real `pxcii-core` sources.
pub(crate) fn run(_args: &[String]) -> Result<(), String> {
    check(&workspace_paths::core_sources())
}

fn check(directory: &Path) -> Result<(), String> {
    let files = collect_sources(directory)?;
    let mut violations = Vec::new();
    for path in files {
        let source = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        violations.extend(scan(&display_name(directory, &path), &source));
    }
    if violations.is_empty() {
        println!("pxcii-core is pure: no filesystem, path or environment access");
        return Ok(());
    }
    for violation in &violations {
        eprintln!(
            "{}:{}: forbidden `{}` in `{}`",
            violation.file,
            violation.line,
            violation.pattern,
            violation.text.trim()
        );
    }
    Err(format!(
        "{} forbidden pattern(s) found in pxcii-core",
        violations.len()
    ))
}

/// Path relative to the scanned root, so `decode.rs` and `decode/tests.rs` stay distinguishable.
fn display_name(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Finds every forbidden pattern in `source`, one violation per line and pattern.
///
/// Comment lines are skipped: a doc comment explaining the boundary must not fail the check.
/// A trailing comment on a code line is still scanned, which keeps the rule simple and strict.
fn scan(file: &str, source: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    for (index, text) in source.lines().enumerate() {
        if text.trim_start().starts_with("//") {
            continue;
        }
        for pattern in FORBIDDEN {
            if contains_pattern(text, pattern) {
                violations.push(Violation {
                    file: file.to_owned(),
                    line: index + 1,
                    pattern,
                    text: text.to_owned(),
                });
            }
        }
    }
    violations
}

/// Looks for `pattern`, treating a `::`-suffixed needle as a module path.
///
/// A module path only counts when it starts at an identifier boundary, so `prefs::load()` is not
/// mistaken for `fs::`; plain needles such as `Path` stay plain substrings and also match `PathBuf`.
fn contains_pattern(text: &str, pattern: &str) -> bool {
    if !pattern.ends_with("::") {
        return text.contains(pattern);
    }
    text.match_indices(pattern).any(|(index, _)| {
        text[..index]
            .chars()
            .next_back()
            .is_none_or(|previous| !(previous.is_alphanumeric() || previous == '_'))
    })
}

/// Lists every Rust source file below `directory`, sorted for stable output.
fn collect_sources(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
    // Unreadable entries are skipped: a partially readable tree must not hide real violations.
    let (nested, plain): (Vec<PathBuf>, Vec<PathBuf>) = entries
        .flatten()
        .map(|entry| entry.path())
        .partition(|path| path.is_dir());
    let mut files: Vec<PathBuf> = plain
        .into_iter()
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .collect();
    for directory in &nested {
        files.extend(collect_sources(directory)?);
    }
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::{check, collect_sources, display_name, run, scan, FORBIDDEN};
    use crate::workspace_paths;

    /// Creates an empty scratch directory below `target/`, which is ignored by git.
    fn scratch(name: &str) -> std::path::PathBuf {
        let directory = workspace_paths::workspace_root()
            .join("target")
            .join("purity-check")
            .join(name);
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("scratch directory");
        directory
    }

    #[test]
    fn every_forbidden_pattern_is_detected() {
        let source = concat!(
            "fn a() { std::fs::read(); }\n",
            "fn b() { Path::new(\"x\"); }\n",
            "fn c() { env::var(\"Y\"); }\n",
            "fn d() { let _ = std::path::MAIN_SEPARATOR; }\n",
            "fn e() { let _ = std::env::args(); }\n",
            "fn f() { let _ = std::process::exit(1); }\n",
            "fn g() { use std::{fs, io}; }\n",
        );
        let violations = scan("lib.rs", source);
        for pattern in FORBIDDEN {
            assert!(
                violations
                    .iter()
                    .any(|violation| violation.pattern == pattern),
                "{pattern} is not detected"
            );
        }
        assert_eq!(violations[0].line, 1);
        assert_eq!(violations[0].pattern, "std::fs");
        assert_eq!(violations[0].file, "lib.rs");
        assert!(violations[0].text.contains("std::fs"));
    }

    #[test]
    fn clean_source_has_no_violations() {
        let violations = scan(
            "lib.rs",
            "pub fn decode(bytes: &[u8]) -> u32 { bytes.len() as u32 }",
        );
        assert_eq!(violations, Vec::new());
    }

    #[test]
    fn several_patterns_on_one_line_are_all_reported() {
        let violations = scan(
            "lib.rs",
            "let p = Path::new(std::env::var(\"X\").unwrap().as_str());",
        );
        // Three hits: `Path`, the `std::env` import path and the `env::` call.
        assert_eq!(violations.len(), 3);
    }

    #[test]
    fn comment_lines_are_ignored() {
        let source = "//! The core never touches std::fs or Path.
/// env:: is banned too.
fn ok() {}
";
        assert_eq!(scan("lib.rs", source), Vec::new());
    }

    #[test]
    fn grouped_imports_are_reported() {
        let violations = scan(
            "lib.rs",
            "use std::{fs, path};
fn a() { fs::read(\"x\"); }
fn b() { let _ = path::MAIN_SEPARATOR; }
",
        );
        let patterns: Vec<&str> = violations
            .iter()
            .map(|violation| violation.pattern)
            .collect();
        assert!(patterns.contains(&"fs::"), "{patterns:?}");
        assert!(patterns.contains(&"path::"), "{patterns:?}");
    }

    #[test]
    fn module_paths_need_an_identifier_boundary() {
        // `prefs::` ends with `fs::` but is not a module of interest.
        assert_eq!(
            scan(
                "lib.rs",
                "let config = prefs::load();
"
            ),
            Vec::new()
        );
        assert_eq!(
            scan(
                "lib.rs",
                "let _ = process::exit(1);
"
            )
            .len(),
            1
        );
    }

    #[test]
    fn path_and_env_module_imports_are_reported() {
        for source in [
            "use std::path;
",
            "const SEP: char = std::path::MAIN_SEPARATOR;
",
            "use std::env as e;
",
        ] {
            assert!(!scan("lib.rs", source).is_empty(), "{source}");
        }
    }

    #[test]
    fn aliased_imports_and_path_types_are_reported() {
        let aliased = scan(
            "lib.rs",
            "use std::fs as f;
",
        );
        assert_eq!(aliased.len(), 1);
        assert_eq!(aliased[0].pattern, "std::fs");

        let path_type = scan(
            "lib.rs",
            "let p: PathBuf = Default::default();
",
        );
        assert_eq!(path_type.len(), 1);
        assert_eq!(path_type[0].pattern, "Path");
    }

    #[test]
    fn the_real_core_sources_are_clean() {
        run(&[]).expect("pxcii-core must stay free of filesystem, path and environment access");
    }

    #[test]
    fn relative_names_keep_the_subdirectory() {
        let root = std::path::Path::new("/core/src");
        assert_eq!(display_name(root, &root.join("lib.rs")), "lib.rs");
        assert_eq!(
            display_name(root, &root.join("decode").join("tests.rs")),
            "decode/tests.rs"
        );
    }

    #[test]
    fn sources_of_the_real_crate_are_listed() {
        let files = collect_sources(&workspace_paths::core_sources()).expect("core sources exist");
        assert!(files.iter().any(|path| path.ends_with("lib.rs")));
        assert!(files
            .iter()
            .all(|path| path.extension().is_some_and(|ext| ext == "rs")));
    }

    #[test]
    fn violations_in_a_directory_are_reported() {
        let directory = scratch("violations");
        std::fs::write(directory.join("clean.rs"), "pub fn ok() {}\n").expect("write");
        std::fs::write(directory.join("bad.rs"), "let p = Path::new(\"x\");\n").expect("write");
        // Non-Rust files are ignored, which also exercises the filtering step of the walker.
        std::fs::write(directory.join("notes.txt"), "text").expect("write");
        let error = check(&directory).expect_err("a violation fails the check");
        assert!(error.contains("1 forbidden pattern(s)"), "{error}");
    }

    #[test]
    fn unreadable_source_is_reported() {
        let directory = scratch("not-utf8");
        std::fs::write(directory.join("junk.rs"), [0xFF, 0xFE, 0x00]).expect("write");
        let error = check(&directory).expect_err("invalid UTF-8 fails the check");
        assert!(error.contains("cannot read"), "{error}");
    }

    #[test]
    fn nested_directories_are_walked() {
        let directory = scratch("nested");
        let inner = directory.join("inner");
        std::fs::create_dir_all(&inner).expect("nested directory");
        std::fs::write(inner.join("deep.rs"), "let p = Path::new(\"x\");\n").expect("write");
        let error = check(&directory).expect_err("violations in subdirectories are found");
        assert!(error.contains("1 forbidden pattern(s)"), "{error}");
    }

    #[test]
    fn missing_directory_is_reported() {
        let error = collect_sources(&workspace_paths::workspace_root().join("no-such-directory"))
            .expect_err("directory does not exist");
        assert!(error.contains("cannot read"), "{error}");
    }
}
