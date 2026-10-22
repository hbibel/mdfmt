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
    .map(|p| p.strip_prefix(root).unwrap().to_string_lossy().into_owned())
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
fn unsupported_globs_are_diagnosed_not_silently_approximated() {
    let dir = tempfile::tempdir().unwrap();
    let resolver = ConfigResolver::new(dir.path(), None, None).unwrap();
    for pattern in ["@(a|b).md", "!(a).md", "file{1..3}.md"] {
        assert_eq!(
            select_inputs(&resolver, &[pattern.into()], false)
                .unwrap_err()
                .code,
            "unsupported-glob"
        );
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
