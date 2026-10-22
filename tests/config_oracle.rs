use mdfmt::{
    Severity,
    config::{ConfigResolver, select_inputs},
};
use serde_json::{Value, json};
use std::{fs, path::Path};

#[test]
fn configuration_matches_recorded_pinned_upstream_observations() {
    let cases: Value =
        serde_json::from_str(include_str!("fixtures/configuration/cases.json")).unwrap();
    let observations: Value =
        serde_json::from_str(include_str!("fixtures/configuration/observations.json")).unwrap();
    assert_eq!(
        cases.as_array().unwrap().len(),
        observations.as_array().unwrap().len()
    );
    for (case, expected) in cases
        .as_array()
        .unwrap()
        .iter()
        .zip(observations.as_array().unwrap())
    {
        assert_eq!(case["id"], expected["id"]);
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for (name, text) in case["files"].as_object().unwrap() {
            let path = root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text.as_str().unwrap()).unwrap();
        }
        let invocation = root.join(case["invocation"].as_str().unwrap());
        let resolver = ConfigResolver::new(
            &invocation,
            case["explicit"].as_str().map(Path::new),
            case["pointer"].as_str(),
        )
        .unwrap();
        let patterns = case["argv"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().into())
            .collect::<Vec<_>>();
        let selection = select_inputs(&resolver, &patterns, false);
        if let Some(code) = case["nativeError"].as_str() {
            assert_eq!(selection.unwrap_err().code, code, "{}", case["id"]);
            continue;
        }
        let selection = selection.unwrap();
        let mut files: Vec<_> = selection
            .files
            .iter()
            .map(|p| {
                p.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace(std::path::MAIN_SEPARATOR, "/")
            })
            .collect();
        files.sort();
        assert_eq!(json!(files), expected["files"], "{}", case["id"]);
        for probe in expected["probes"].as_array().unwrap() {
            let file = probe["file"].as_str().unwrap();
            let config = resolver.resolve(&root.join(file)).unwrap();
            let document = config
                .document(case["files"][file].as_str().unwrap())
                .unwrap();
            let state = document
                .state(
                    probe["rule"].as_str().unwrap(),
                    probe["line"].as_u64().unwrap() as usize,
                )
                .unwrap();
            assert_eq!(
                state.enabled,
                probe["enabled"].as_bool().unwrap(),
                "{} {probe}",
                case["id"]
            );
            assert_eq!(
                if state.severity == Severity::Warning {
                    "warning"
                } else {
                    "error"
                },
                probe["severity"]
            );
            assert_eq!(
                json!(state.options),
                probe["options"],
                "{} {probe}",
                case["id"]
            );
        }
    }
}
