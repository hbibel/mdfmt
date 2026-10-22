use mdfmt::config::ConfigResolver;
use std::{fs, path::Path};

fn put(root: &Path, name: &str, text: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn cli2_inherits_shallowly_but_standalone_replaces() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    put(
        root,
        ".markdownlint-cli2.jsonc",
        r#"{"config":{"default":false,"MD013":{"line_length":60,"strict":true}}}"#,
    );
    put(
        root,
        "child/.markdownlint-cli2.yaml",
        "config:\n  MD013:\n    line_length: 40\n",
    );
    let resolver = ConfigResolver::new(root, None, None).unwrap();
    let child = resolver.resolve(&root.join("child/doc.md")).unwrap();
    assert!(!child.rules.effective()["MD026"].enabled);
    assert_eq!(child.rules.effective()["MD013"].options.len(), 1);
    assert_eq!(child.rules.effective()["MD013"].options["line_length"], 40);
    put(
        root,
        ".markdownlint.json",
        r#"{"default":false,"MD013":{"line_length":70}}"#,
    );
    put(
        root,
        "child/.markdownlint.json",
        r#"{"MD013":{"line_length":30}}"#,
    );
    let resolver = ConfigResolver::new(root, None, None).unwrap();
    let child = resolver.resolve(&root.join("child/doc.md")).unwrap();
    assert!(child.rules.effective()["MD026"].enabled);
    assert_eq!(child.rules.effective()["MD013"].options["line_length"], 30);
}

#[test]
fn discovery_beats_explicit_base_and_does_not_search_upward() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    put(
        root,
        "base.toml",
        "[nested]\ndefault = false\n[nested.MD013]\nline_length = 90\n",
    );
    put(
        root,
        "work/.markdownlint-cli2.jsonc",
        "{ // comment\n\"config\": {\"MD013\": {\"line_length\": 42,},}, \"fix\": false,}",
    );
    put(root, ".markdownlint.json", r#"{"MD013":false}"#);
    let resolver = ConfigResolver::new(
        &root.join("work"),
        Some(&root.join("base.toml")),
        Some("/nested"),
    )
    .unwrap();
    let config = resolver.resolve(&root.join("work/doc.md")).unwrap();
    assert_eq!(config.rules.effective()["MD013"].options["line_length"], 42);
    assert!(!config.fix);
    assert!(!config.rules.effective()["MD026"].enabled);
    put(
        root,
        "work/.markdownlint.jsonc",
        r#"{"MD013":{"line_length":24}}"#,
    );
    let resolver = ConfigResolver::new(&root.join("work"), None, None).unwrap();
    assert_eq!(
        resolver
            .resolve(&root.join("work/doc.md"))
            .unwrap()
            .rules
            .effective()["MD013"]
            .options["line_length"],
        24
    );
}

#[test]
fn extends_first_override_and_standalone_precedence() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    put(
        root,
        "shared/base.json",
        r#"{"default":false,"MD013":{"line_length":60}}"#,
    );
    put(
        root,
        ".markdownlint-cli2.jsonc",
        r#"{"config":{"extends":"shared/base.json"},"overrides":[{"filter":["*.md"],"combine":"merge","config":{"MD013":{"line_length":40}}},{"filter":["*.md"],"combine":"replace","config":{"MD013":false}}]}"#,
    );
    let resolver = ConfigResolver::new(root, None, None).unwrap();
    let states = resolver
        .resolve(&root.join("doc.md"))
        .unwrap()
        .rules
        .effective();
    assert_eq!(states["MD013"].options["line_length"], 40);
    assert!(!states["MD026"].enabled);
    put(root, ".markdownlint.json", r#"{"MD013":false}"#);
    let resolver = ConfigResolver::new(root, None, None).unwrap();
    assert!(
        !resolver
            .resolve(&root.join("doc.md"))
            .unwrap()
            .rules
            .effective()["MD013"]
            .enabled
    );
}

#[test]
fn errors_name_path_and_unsupported_property() {
    for (name, text, code) in [
        (
            ".markdownlint.cjs",
            "module.exports = {}",
            "unsupported-config-format",
        ),
        (
            ".markdownlint-cli2.jsonc",
            r#"{"frontMatter":"custom"}"#,
            "unsupported-front-matter-pattern",
        ),
        (
            ".markdownlint-cli2.jsonc",
            r#"{"customRules":["x"]}"#,
            "unsupported-executable-feature",
        ),
        (
            ".markdownlint.json",
            r#"{"extends":"package-name"}"#,
            "unsupported-config-resolution",
        ),
        (
            ".markdownlint.json",
            r#"{"extends":"./.markdownlint.json"}"#,
            "invalid-config",
        ),
        (".markdownlint.json", "{", "invalid-config"),
        (".markdownlint.json", "{MD013:false}", "invalid-config"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        put(dir.path(), name, text);
        let error = ConfigResolver::new(dir.path(), None, None)
            .err()
            .expect(name);
        assert_eq!(error.code, code, "{error}");
        assert!(error.to_string().contains(name));
    }
}

#[test]
fn child_cli2_suppresses_parent_standalone_and_outside_tree_gets_base() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    put(root, "work/.markdownlint.json", r#"{"MD013":false}"#);
    put(
        root,
        "work/child/.markdownlint-cli2.jsonc",
        r#"{"config":{"MD013":{"line_length":33}}}"#,
    );
    put(root, "other/doc.md", "unchanged");
    let resolver = ConfigResolver::new(&root.join("work"), None, None).unwrap();
    assert!(
        resolver
            .resolve(&root.join("work/child/doc.md"))
            .unwrap()
            .rules
            .effective()["MD013"]
            .enabled
    );
    assert!(
        !resolver
            .resolve(&root.join("other/doc.md"))
            .unwrap()
            .rules
            .effective()["MD013"]
            .enabled
    );
}

#[test]
fn toml_is_not_discovered_and_json_pointer_escapes_are_supported() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    put(root, ".markdownlint.toml", "MD013 = false\n");
    put(root, ".markdownlint-cli2.toml", "frontMatter = 'custom'\n");
    let resolver = ConfigResolver::new(root, None, None).unwrap();
    assert!(
        resolver
            .resolve(&root.join("doc.md"))
            .unwrap()
            .rules
            .effective()["MD013"]
            .enabled
    );
    put(root, "base.json", r#"{"a/b":{"~nested":{"MD013":false}}}"#);
    let resolver =
        ConfigResolver::new(root, Some(Path::new("base.json")), Some("/a~1b/~0nested")).unwrap();
    assert!(
        !resolver
            .resolve(&root.join("doc.md"))
            .unwrap()
            .rules
            .effective()["MD013"]
            .enabled
    );
    assert!(ConfigResolver::new(root, Some(Path::new("base.json")), Some("/missing")).is_err());
}
