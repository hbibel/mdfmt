use mdfmt::config::{ConfigResolver, select_inputs};
use serde_json::{Value, json};
use std::fs;

#[test]
fn generated_globs_match_actual_upstream_filesystem_selection() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/configuration/selection-globs.json")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    for file in fixture["files"].as_array().unwrap() {
        let path = dir.path().join(file.as_str().unwrap());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "text\n").unwrap();
    }
    let mut differences = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        let pattern = case["pattern"].as_str().unwrap();
        let resolver = ConfigResolver::new(dir.path(), None, None).unwrap();
        let selected = select_inputs(&resolver, &[pattern.into()], false).unwrap();
        // Missing named inputs intentionally reach the CLI's operational-error path.
        if case["missingNamedInput"] == true {
            assert_eq!(selected.files.len(), 1, "{pattern}");
            assert!(!selected.files[0].exists(), "{pattern}");
            continue;
        }
        let mut files: Vec<_> = selected
            .files
            .iter()
            .map(|path| {
                path.strip_prefix(dir.path())
                    .unwrap()
                    .to_string_lossy()
                    .replace(std::path::MAIN_SEPARATOR, "/")
            })
            .collect();
        files.sort();
        if json!(files) != case["files"] {
            differences.push(format!(
                "{pattern:?}: {} != {}",
                json!(files),
                case["files"]
            ));
        }
    }
    assert!(differences.is_empty(), "{}", differences.join("\n"));
}
