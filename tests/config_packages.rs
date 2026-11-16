use mdfmt::{FormatOptions, config::ConfigResolver, format_with_config};
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

fn put(root: &Path, name: &str, contents: impl AsRef<str>) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents.as_ref()).unwrap();
}
fn package(root: &Path, name: &str, metadata: Value, rules: Value) {
    put(
        root,
        &format!("node_modules/{name}/package.json"),
        metadata.to_string(),
    );
    put(
        root,
        &format!("node_modules/{name}/rules.json"),
        rules.to_string(),
    );
}

#[test]
fn installed_main_index_scoped_and_subpath_configs_drive_native_formatting() {
    for (name, reference, metadata, file, contents) in [
        (
            "pkg",
            "pkg",
            json!({"main":"rules.json"}),
            "rules.json",
            "{\"default\":false,\"MD013\":{\"line_length\":24,\"strict\":true}}",
        ),
        (
            "pkg",
            "pkg",
            json!({}),
            "index.json",
            "{\"default\":false,\"MD013\":{\"line_length\":24,\"strict\":true}}",
        ),
        (
            "@team/pkg",
            "@team/pkg",
            json!({"exports":"./rules.json"}),
            "rules.json",
            "{\"default\":false,\"MD013\":{\"line_length\":24,\"strict\":true}}",
        ),
        (
            "pkg",
            "pkg/strict",
            json!({"exports":{"./strict":"./rules.yaml"}}),
            "rules.yaml",
            "default: false\nMD013:\n  line_length: 24\n  strict: true\n",
        ),
        (
            "pkg",
            "pkg",
            json!({"main":"rules.toml"}),
            "rules.toml",
            "default = false\n[MD013]\nline_length = 24\nstrict = true\n",
        ),
        (
            "pkg",
            "pkg",
            json!({"main":"rules.jsonc"}),
            "rules.jsonc",
            "{\"default\":false,/*source*/\"MD013\":{\"line_length\":24,\"strict\":true},}",
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        put(
            dir.path(),
            &format!("node_modules/{name}/package.json"),
            metadata.to_string(),
        );
        put(dir.path(), &format!("node_modules/{name}/{file}"), contents);
        put(
            dir.path(),
            ".markdownlint.json",
            json!({"extends":reference,"MD026":true}).to_string(),
        );
        let resolver = ConfigResolver::new(dir.path(), None, None).unwrap();
        let settings = resolver.resolve(&dir.path().join("doc.md")).unwrap();
        let input = "# Heading!\n\none two three four five six seven eight nine ten\n";
        let result = format_with_config(input, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(
            result.output,
            "# Heading\n\none two three four five\nsix seven eight nine ten\n"
        );
        assert!(result.diagnostics.is_empty());
        let again =
            format_with_config(&result.output, &FormatOptions::default(), &settings).unwrap();
        assert_eq!(again.output, result.output);
        assert!(again.edits.is_empty());
    }
}

#[test]
fn package_targets_preserve_referring_scope_and_key_order() {
    for (file, value, enabled) in [
        (
            ".markdownlint.json",
            json!({"extends":"pkg","MD013":true}),
            true,
        ),
        (
            ".markdownlint-cli2.jsonc",
            json!({"config":{"extends":"pkg","MD013":true}}),
            false,
        ),
        (
            ".markdownlint-cli2.jsonc",
            json!({"config":{"default":false},"overrides":[{"filter":["doc.md"],"combine":"replace","config":{"extends":"pkg","MD013":true}}]}),
            false,
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        package(
            dir.path(),
            "pkg",
            json!({"main":"rules.json"}),
            json!({"default":false,"line-length":false}),
        );
        put(dir.path(), file, value.to_string());
        let resolver = ConfigResolver::new(dir.path(), None, None).unwrap();
        let settings = resolver.resolve(&dir.path().join("doc.md")).unwrap();
        assert_eq!(settings.rules.effective()["MD013"].enabled, enabled);
    }
    let dir = tempfile::tempdir().unwrap();
    package(
        dir.path(),
        "pkg",
        json!({"main":"rules.json"}),
        json!({"extends":"./base.json","MD026":true}),
    );
    put(
        dir.path(),
        "node_modules/pkg/base.json",
        json!({"extends":"shared"}).to_string(),
    );
    package(
        dir.path(),
        "shared",
        json!({"main":"rules.json"}),
        json!({"default":false,"MD013":{"line_length":40}}),
    );
    package(
        &dir.path().join("node_modules/pkg"),
        "shared",
        json!({"main":"rules.json"}),
        json!({"default":false,"MD013":{"line_length":24}}),
    );
    put(
        dir.path(),
        ".markdownlint.json",
        json!({"extends":"pkg"}).to_string(),
    );
    let settings = ConfigResolver::new(dir.path(), None, None)
        .unwrap()
        .resolve(&dir.path().join("doc.md"))
        .unwrap();
    assert_eq!(
        settings.rules.effective()["MD013"].options["line_length"],
        24
    );
    assert!(settings.rules.effective()["MD026"].enabled);
    put(
        dir.path(),
        "pkg/rules.json",
        json!({"default":false,"MD013":false}).to_string(),
    );
    put(
        dir.path(),
        ".markdownlint.json",
        json!({"extends":"pkg/rules.json"}).to_string(),
    );
    let settings = ConfigResolver::new(dir.path(), None, None)
        .unwrap()
        .resolve(&dir.path().join("doc.md"))
        .unwrap();
    assert!(!settings.rules.effective()["MD013"].enabled);
}

#[test]
fn metadata_resolution_matches_all_pinned_source_observations() {
    let rows: Value =
        serde_json::from_str(include_str!("fixtures/config-packages/cases.json")).unwrap();
    let mut failures = vec![];
    for row in rows.as_array().unwrap() {
        let dir = tempfile::tempdir().unwrap();
        for (name, contents) in row["files"].as_object().unwrap() {
            put(dir.path(), name, contents.as_str().unwrap());
        }
        let invocation = dir.path().join(row["invocation"].as_str().unwrap());
        let result = ConfigResolver::new(&invocation, None, None)
            .and_then(|resolver| resolver.resolve(&invocation.join("doc.md")));
        if row.get("nativeError").is_some() || row["source"].get("error").is_some() {
            match result {
                Ok(_) => failures.push(format!("{} unexpectedly resolved", row["id"])),
                Err(error) => {
                    if error.code
                        != row
                            .get("nativeError")
                            .and_then(Value::as_str)
                            .unwrap_or("invalid-config")
                        || error.path.is_none()
                    {
                        failures.push(format!("{}: {error}", row["id"]));
                    }
                }
            }
        } else {
            let settings = match result {
                Ok(settings) => settings,
                Err(error) => {
                    failures.push(format!("{}: {error}", row["id"]));
                    continue;
                }
            };
            let actual = settings.rules.as_value();
            if actual != row["source"]["config"]
                || actual.as_object().unwrap().keys().collect::<Vec<_>>()
                    != row["source"]["config"]
                        .as_object()
                        .unwrap()
                        .keys()
                        .collect::<Vec<_>>()
            {
                failures.push(format!(
                    "{}: {actual} != {} or key order differs",
                    row["id"], row["source"]["config"]
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn package_failures_preserve_inputs_and_never_execute_entrypoints() {
    for kind in ["missing", "javascript", "cycle", "directory"] {
        let dir = tempfile::tempdir().unwrap();
        let input = "\u{feff}# Heading!\r\n";
        put(dir.path(), "doc.md", input);
        let reference = if kind == "missing" {
            "missing-package"
        } else if kind == "directory" {
            "pkg.json"
        } else {
            "pkg"
        };
        put(
            dir.path(),
            ".markdownlint.json",
            json!({"extends":reference}).to_string(),
        );
        match kind {
            "javascript" => {
                put(
                    dir.path(),
                    "node_modules/pkg/package.json",
                    json!({"main":"rules.cjs","scripts":{"postinstall":"touch installed"}})
                        .to_string(),
                );
                put(
                    dir.path(),
                    "node_modules/pkg/rules.cjs",
                    "require('node:fs').writeFileSync('executed', 'unsafe');module.exports={};",
                );
            }
            "cycle" => package(
                dir.path(),
                "pkg",
                json!({"main":"rules.json"}),
                json!({"extends":"pkg"}),
            ),
            "directory" => {
                put(dir.path(), "pkg.json/inner.json", "{}");
                package(
                    dir.path(),
                    "pkg.json",
                    json!({"main":"rules.json"}),
                    json!({"default":false}),
                );
            }
            _ => {}
        }
        for mode in ["--check", "--write"] {
            let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
                .current_dir(dir.path())
                .args([mode, "doc.md"])
                .env("PATH", dir.path().join("no-programs"))
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(2));
            assert_eq!(
                fs::read_to_string(dir.path().join("doc.md")).unwrap(),
                input
            );
            let errors = String::from_utf8_lossy(&output.stderr);
            assert!(
                errors.contains(if kind == "javascript" {
                    "unsupported-config-format"
                } else {
                    "invalid-config"
                }),
                "{kind}: {errors}"
            );
            assert!(
                errors.contains(if kind == "javascript" {
                    "rules.cjs"
                } else if kind == "directory" {
                    "pkg.json"
                } else if kind == "cycle" {
                    "cyclic"
                } else {
                    "missing-package"
                }),
                "{errors}"
            );
            assert!(!dir.path().join("executed").exists());
            assert!(!dir.path().join("installed").exists());
        }
    }
    let dir = tempfile::tempdir().unwrap();
    for index in 0..129 {
        package(
            dir.path(),
            &format!("p{index}"),
            json!({"main":"rules.json"}),
            if index == 128 {
                json!({"default":false})
            } else {
                json!({"extends":format!("p{}",index+1)})
            },
        );
    }
    put(
        dir.path(),
        ".markdownlint.json",
        json!({"extends":"p0"}).to_string(),
    );
    let error = ConfigResolver::new(dir.path(), None, None).err().unwrap();
    assert_eq!(error.code, "invalid-config");
    assert!(error.message.contains("excessively deep"));
}

#[test]
fn resolution_ignores_global_and_executable_settings_and_fresh_instances_see_changes() {
    let dir = tempfile::tempdir().unwrap();
    put(
        dir.path(),
        ".markdownlint.json",
        json!({"extends":"only-global"}).to_string(),
    );
    package(
        &dir.path().join("global"),
        "only-global",
        json!({"main":"rules.json"}),
        json!({"default":false}),
    );
    put(
        dir.path(),
        ".pnp.cjs",
        "require('node:fs').writeFileSync('executed','unsafe');",
    );
    put(dir.path(), "doc.md", "# Heading!\n");
    let output = Command::new(env!("CARGO_BIN_EXE_mdfmt"))
        .current_dir(dir.path())
        .args(["--write", "doc.md"])
        .env("NODE_PATH", dir.path().join("global/node_modules"))
        .env("OXC_RESOLVER_YARN_PNP", "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(!dir.path().join("executed").exists());
    assert_eq!(
        fs::read_to_string(dir.path().join("doc.md")).unwrap(),
        "# Heading!\n"
    );
    package(
        dir.path(),
        "pkg",
        json!({"main":"rules.json","browser":"./browser.json"}),
        json!({"default":false,"MD013":{"line_length":24}}),
    );
    put(
        dir.path(),
        "node_modules/pkg/browser.json",
        json!({"default":false,"MD013":{"line_length":33}}).to_string(),
    );
    put(
        dir.path(),
        "tsconfig.json",
        json!({"compilerOptions":{"baseUrl":".","paths":{"pkg":["./alternate.json"]}}}).to_string(),
    );
    put(
        dir.path(),
        "alternate.json",
        json!({"default":false,"MD013":{"line_length":33}}).to_string(),
    );
    put(
        dir.path(),
        ".markdownlint.json",
        json!({"extends":"pkg"}).to_string(),
    );
    for width in [24, 29] {
        put(
            dir.path(),
            "node_modules/pkg/rules.json",
            json!({"default":false,"MD013":{"line_length":width}}).to_string(),
        );
        let settings = ConfigResolver::new(dir.path(), None, None)
            .unwrap()
            .resolve(&dir.path().join("doc.md"))
            .unwrap();
        assert_eq!(
            settings.rules.effective()["MD013"].options["line_length"],
            width
        );
    }
}

#[cfg(unix)]
#[test]
fn installed_symlinks_use_real_package_scope_but_direct_paths_keep_lexical_scope() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    put(
        dir.path(),
        "real/pkg/package.json",
        json!({"main":"rules.json"}).to_string(),
    );
    put(
        dir.path(),
        "real/pkg/rules.json",
        json!({"extends":"./base.json"}).to_string(),
    );
    put(
        dir.path(),
        "real/pkg/base.json",
        json!({"default":false,"MD013":{"line_length":24}}).to_string(),
    );
    fs::create_dir(dir.path().join("node_modules")).unwrap();
    symlink(
        dir.path().join("real/pkg"),
        dir.path().join("node_modules/pkg"),
    )
    .unwrap();
    put(
        dir.path(),
        ".markdownlint.json",
        json!({"extends":"pkg"}).to_string(),
    );
    let settings = ConfigResolver::new(dir.path(), None, None)
        .unwrap()
        .resolve(&dir.path().join("doc.md"))
        .unwrap();
    assert_eq!(
        settings.rules.effective()["MD013"].options["line_length"],
        24
    );
    fs::create_dir(dir.path().join("local")).unwrap();
    symlink(
        dir.path().join("real/pkg/rules.json"),
        dir.path().join("local/rules.json"),
    )
    .unwrap();
    put(
        dir.path(),
        "local/base.json",
        json!({"default":false,"MD013":{"line_length":32}}).to_string(),
    );
    put(
        dir.path(),
        ".markdownlint.json",
        json!({"extends":"./local/rules.json"}).to_string(),
    );
    let settings = ConfigResolver::new(dir.path(), None, None)
        .unwrap()
        .resolve(&dir.path().join("doc.md"))
        .unwrap();
    assert_eq!(
        settings.rules.effective()["MD013"].options["line_length"],
        32
    );
}
