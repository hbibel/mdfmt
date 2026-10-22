use mdfmt::config::{ConfigResolver, select_inputs};
use std::{fs, path::Path};

fn put(root: &Path, name: &str, text: &str) {
    let p = root.join(name);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}
fn selected(root: &Path, patterns: &[&str], no_globs: bool) -> Vec<String> {
    let resolver = ConfigResolver::new(root, None, None).unwrap();
    select_inputs(
        &resolver,
        &patterns.iter().map(|p| (*p).into()).collect::<Vec<_>>(),
        no_globs,
    )
    .unwrap()
    .files
    .iter()
    .map(|p| {
        p.strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/")
    })
    .collect()
}

#[test]
fn dot_patterns_braces_negations_and_literals() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for name in [
        "a.md",
        ".hidden.md",
        "b.markdown",
        "note.txt",
        "child/a.md",
        "literal*.md",
    ] {
        put(root, name, "text");
    }
    assert_eq!(
        selected(root, &["."], false),
        [".hidden.md", "a.md", "b.markdown", "literal*.md"]
    );
    assert!(selected(root, &["!**/*.md"], false).is_empty());
    assert_eq!(
        selected(root, &["*.{md,markdown}", "!a.md", "!literal*.md"], false),
        [".hidden.md", "b.markdown"]
    );
    assert_eq!(
        selected(root, &[":literal*.md", "!*.md"], false),
        ["literal*.md"]
    );
    assert_eq!(selected(root, &[r"literal\*.md"], false), ["literal*.md"]);
}

#[test]
fn config_globs_no_globs_and_nested_ignores() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for name in ["a.md", "skip.md", "child/a.md", "child/skip.md"] {
        put(root, name, "text");
    }
    put(
        root,
        ".markdownlint-cli2.jsonc",
        r#"{"globs":["**/*.md"],"ignores":["skip.md"]}"#,
    );
    put(
        root,
        "child/.markdownlint-cli2.jsonc",
        r#"{"ignores":["skip.md"]}"#,
    );
    assert_eq!(selected(root, &[], false), ["a.md", "child/a.md"]);
    assert!(selected(root, &[], true).is_empty());
    assert_eq!(selected(root, &[":a.md"], true), ["a.md"]);
}

#[test]
fn gitignore_true_includes_ancestors_but_glob_string_does_not() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir(root.join(".git")).unwrap();
    put(root, ".gitignore", "ignored.md\n");
    put(root, "work/ignored.md", "text");
    put(root, "work/kept.md", "text");
    put(
        root,
        "work/.markdownlint-cli2.jsonc",
        r#"{"gitignore":true}"#,
    );
    assert_eq!(selected(&root.join("work"), &["*.md"], false), ["kept.md"]);
    put(
        root,
        "work/.markdownlint-cli2.jsonc",
        r#"{"gitignore":"**/.gitignore"}"#,
    );
    assert_eq!(
        selected(&root.join("work"), &["*.md"], false),
        ["ignored.md", "kept.md"]
    );
}

#[test]
fn nested_gitignore_anchors_rules_to_its_own_directory() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    put(root, ".markdownlint-cli2.jsonc", r#"{"gitignore":true}"#);
    put(root, "child/.gitignore", "/ignored.md\n");
    put(root, "child/ignored.md", "text\n");
    put(root, "child/deep/ignored.md", "text\n");
    put(root, "kept.md", "text\n");
    assert_eq!(
        selected(root, &["**/*.md"], false),
        ["child/deep/ignored.md", "kept.md"]
    );
}

#[test]
fn custom_gitignore_discovers_files_without_directory_expansion() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    put(
        root,
        ".markdownlint-cli2.jsonc",
        r#"{"gitignore":"**/.gitignore"}"#,
    );
    put(root, "child/.gitignore", "*.md\n");
    put(root, "child/ignored.md", "text\n");
    put(root, "kept.md", "text\n");
    assert_eq!(selected(root, &["**/*.md"], false), ["kept.md"]);
    put(root, ".markdownlint-cli2.jsonc", r#"{"gitignore":"child"}"#);
    assert_eq!(
        selected(root, &["**/*.md"], false),
        ["child/ignored.md", "kept.md"]
    );
}

#[test]
fn custom_gitignore_applies_partial_traversal_before_reading_rules() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    put(root, "a/b/.gitignore", "*.md\n");
    for file in ["a/b/x.md", "b/x.md", "x.md"] {
        put(root, file, "text\n");
    }
    for pattern in [
        "@(a/b|b)/.gitignore",
        "+(a/b)/.gitignore",
        "?(a/b)/.gitignore",
    ] {
        put(
            root,
            ".markdownlint-cli2.jsonc",
            &serde_json::json!({"gitignore":pattern}).to_string(),
        );
        assert_eq!(
            selected(root, &["**/*.md"], false),
            ["a/b/x.md", "b/x.md", "x.md"],
            "{pattern}"
        );
    }
    put(
        root,
        ".markdownlint-cli2.jsonc",
        r#"{"gitignore":"a/b/.gitignore"}"#,
    );
    assert_eq!(selected(root, &["**/*.md"], false), ["b/x.md", "x.md"]);
}

#[test]
fn gitignore_discovery_skips_dependency_and_output_directories() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    put(root, ".markdownlint-cli2.jsonc", r#"{"gitignore":true}"#);
    put(root, "node_modules/.gitignore", "*.md\n");
    put(root, "node_modules/a.md", "text\n");
    assert_eq!(selected(root, &["**/*.md"], false), ["node_modules/a.md"]);
}

#[test]
fn gitignore_negation_is_case_insensitive_and_child_rules_win() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    put(root, ".markdownlint-cli2.jsonc", r#"{"gitignore":true}"#);
    put(root, ".gitignore", "*.md\n");
    put(root, "child/.gitignore", "!Kept.md\n");
    put(root, "child/kept.md", "text\n");
    put(root, "child/ignored.md", "text\n");
    assert_eq!(selected(root, &["**/*.md"], false), ["child/kept.md"]);
}

#[test]
fn extglobs_are_native_patterns_not_ordinary_exclusions() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["a.md", "ab.md", "b.md"] {
        put(dir.path(), name, "text");
    }
    assert!(selected(dir.path(), &["!(a).md"], false).is_empty());
    assert_eq!(selected(dir.path(), &["./!(a).md"], false), ["b.md"]);
    assert_eq!(
        selected(dir.path(), &["+(a*).md"], false),
        ["a.md", "ab.md"]
    );
}

#[test]
fn oversized_brace_ranges_are_rejected_before_enumeration() {
    let dir = tempfile::tempdir().unwrap();
    let resolver = ConfigResolver::new(dir.path(), None, None).unwrap();
    for pattern in ["file{1..1002}.md", "file{-99999999999999999999..2}.md"] {
        let error = select_inputs(&resolver, &[pattern.into()], false).unwrap_err();
        assert_eq!(error.code, "invalid-config");
        assert_eq!(error.property, pattern);
    }
}

#[cfg(unix)]
#[test]
fn symlinks_and_cycles_do_not_hang_and_outside_paths_work() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    put(root, "work/a.md", "text");
    put(root, "outside/b.md", "text");
    std::os::unix::fs::symlink(root.join("work"), root.join("work/loop")).unwrap();
    let resolver = ConfigResolver::new(&root.join("work"), None, None).unwrap();
    let selection = select_inputs(
        &resolver,
        &["**/*.md".into(), "../outside/*.md".into()],
        false,
    )
    .unwrap();
    assert_eq!(
        selection.files,
        [root.join("work/a.md"), root.join("outside/b.md")]
    );
}
