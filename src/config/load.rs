use super::{ConfigError, RuleConfig};
use serde_json::{Map, Value};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

pub(crate) const OPTION_KEYS: &[&str] = &[
    "$schema",
    "config",
    "customRules",
    "fix",
    "frontMatter",
    "gitignore",
    "globs",
    "ignores",
    "markdownItPlugins",
    "modulePaths",
    "noBanner",
    "noInlineConfig",
    "noProgress",
    "outputFormatters",
    "overrides",
    "showFound",
];

#[derive(Debug, Clone, Default)]
pub struct CliOptions(pub(crate) Map<String, Value>);

impl CliOptions {
    pub fn boolean(&self, key: &str) -> bool {
        self.0.get(key).and_then(Value::as_bool).unwrap_or(false)
    }
    pub fn string(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(Value::as_str)
    }
    pub fn strings(&self, key: &str) -> Vec<String> {
        self.0
            .get(key)
            .and_then(Value::as_array)
            .map(|v| {
                v.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    }
    pub fn rules(&self) -> Option<RuleConfig> {
        self.0
            .get("config")
            .map(|v| RuleConfig(v.as_object().unwrap().clone()))
    }
    pub(crate) fn merge(&self, other: &Self) -> Self {
        let mut merged = self.0.clone();
        for (key, value) in &other.0 {
            merged.insert(key.clone(), value.clone());
        }
        if let Some(mut rules) = self.rules() {
            if let Some(child) = other.rules() {
                rules.merge(&child);
            }
            merged.insert("config".into(), rules.as_value());
        }
        Self(merged)
    }
}

pub(crate) fn parse_jsonc(text: &str) -> Result<Value, ConfigError> {
    jsonc_parser::parse_to_serde_value(
        text,
        &jsonc_parser::ParseOptions {
            allow_loose_object_property_names: false,
            ..Default::default()
        },
    )
    .map_err(|e| ConfigError::invalid("config", e.to_string()))?
    .ok_or_else(|| ConfigError::invalid("config", "empty document"))
}

fn parse(path: &Path) -> Result<Value, ConfigError> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if matches!(extension, "cjs" | "mjs" | "js") {
        return Err(ConfigError::new(
            "unsupported-config-format",
            "config",
            "JavaScript configuration is not executed",
        )
        .at(path));
    }
    if !matches!(extension, "json" | "jsonc" | "yaml" | "yml" | "toml") {
        return Err(ConfigError::new(
            "unsupported-config-format",
            "config",
            "expected JSON/JSONC, YAML or TOML",
        )
        .at(path));
    }
    let metadata =
        fs::metadata(path).map_err(|e| ConfigError::invalid("config", e.to_string()).at(path))?;
    if !metadata.is_file() {
        return Err(
            ConfigError::invalid("config", "expected a regular configuration file").at(path),
        );
    }
    let text = fs::read_to_string(path)
        .map_err(|e| ConfigError::invalid("config", e.to_string()).at(path))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let result = match extension {
        "json" | "jsonc" => parse_jsonc(text),
        "yaml" | "yml" => serde_yaml::from_str::<Value>(text)
            .map_err(|e| ConfigError::invalid("config", e.to_string())),
        "toml" => toml::from_str::<toml::Value>(text)
            .map_err(|e| ConfigError::invalid("config", e.to_string()))
            .and_then(|v| {
                serde_json::to_value(v).map_err(|e| ConfigError::invalid("config", e.to_string()))
            }),
        _ => unreachable!(),
    };
    result.map_err(|e| e.at(path))
}

pub(crate) fn expand_home(path: &str) -> Result<PathBuf, ConfigError> {
    if path == "~" || path.starts_with("~/") || path.starts_with("~\\") {
        let home = std::env::var_os("HOME")
            .ok_or_else(|| ConfigError::invalid("extends", "home directory is unavailable"))?;
        return Ok(PathBuf::from(home).join(path.get(2..).unwrap_or("")));
    }
    Ok(path.into())
}

fn extend(
    value: Value,
    path: &Path,
    active: &mut HashSet<PathBuf>,
) -> Result<RuleConfig, ConfigError> {
    let mut object = value
        .as_object()
        .ok_or_else(|| ConfigError::invalid("config", "expected an object").at(path))?
        .clone();
    let mut result = RuleConfig::default();
    if let Some(reference) = object.shift_remove("extends") {
        let reference = reference
            .as_str()
            .filter(|r| !r.is_empty())
            .ok_or_else(|| {
                ConfigError::invalid("extends", "expected a non-empty configuration reference")
                    .at(path)
            })?;
        let target = super::packages::resolve(path, reference)?;
        result = load_rules(&target, active)?;
    }
    result.merge(&RuleConfig::from_value(Value::Object(object)).map_err(|e| e.at(path))?);
    Ok(result)
}

fn load_rules(path: &Path, active: &mut HashSet<PathBuf>) -> Result<RuleConfig, ConfigError> {
    let canonical = fs::canonicalize(path)
        .map_err(|e| ConfigError::invalid("extends", e.to_string()).at(path))?;
    if active.len() >= 128 || !active.insert(canonical.clone()) {
        return Err(ConfigError::invalid(
            "extends",
            "cyclic or excessively deep configuration inheritance",
        )
        .at(path));
    }
    let result = parse(path).and_then(|v| extend(v, path, active));
    active.remove(&canonical);
    result
}

fn extend_in_place(
    value: &Value,
    path: &Path,
    active: &mut HashSet<PathBuf>,
) -> Result<Value, ConfigError> {
    let mut local = value
        .as_object()
        .ok_or_else(|| ConfigError::invalid("config", "expected an object").at(path))?
        .clone();
    local.shift_remove("extends");
    let mut local = RuleConfig::from_value(Value::Object(local)).map_err(|e| e.at(path))?;
    // cli2 copies extended keys back into the existing object, unlike standalone readConfig.
    local.merge(&extend(value.clone(), path, active)?);
    Ok(local.as_value())
}

pub(crate) fn load(
    path: &Path,
    pointer: Option<&str>,
    kind: Option<bool>,
) -> Result<CliOptions, ConfigError> {
    let mut value = parse(path)?;
    if let Some(pointer) = pointer {
        if !pointer.is_empty() && !pointer.starts_with('/') {
            return Err(ConfigError::invalid("configPointer", "expected a JSON Pointer").at(path));
        }
        value = value.pointer(pointer).cloned().ok_or_else(|| {
            ConfigError::invalid("configPointer", "pointer does not select an object").at(path)
        })?;
    }
    let object = value
        .as_object()
        .ok_or_else(|| ConfigError::invalid("config", "expected an object").at(path))?;
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let cli = kind.unwrap_or_else(|| {
        if [
            ".markdownlint-cli2.jsonc",
            ".markdownlint-cli2.yaml",
            ".markdownlint-cli2.cjs",
            ".markdownlint-cli2.mjs",
        ]
        .iter()
        .any(|suffix| name.ends_with(suffix))
        {
            true
        } else if [
            ".markdownlint.jsonc",
            ".markdownlint.json",
            ".markdownlint.yaml",
            ".markdownlint.yml",
            ".markdownlint.cjs",
            ".markdownlint.mjs",
        ]
        .iter()
        .any(|suffix| name.ends_with(suffix))
        {
            false
        } else {
            object
                .keys()
                .any(|key| key != "$schema" && OPTION_KEYS.contains(&key.as_str()))
        }
    });
    let mut active = HashSet::new();
    active.insert(
        fs::canonicalize(path)
            .map_err(|e| ConfigError::invalid("config", e.to_string()).at(path))?,
    );
    if !cli {
        let rules = extend(value, path, &mut active)?;
        return Ok(CliOptions(Map::from_iter([(
            "config".into(),
            rules.as_value(),
        )])));
    }
    let mut object = object.clone();
    for (key, value) in &mut object {
        let invalid = || ConfigError::invalid(key, "invalid option value").at(path);
        match key.as_str() {
            "config" => *value = extend_in_place(value, path, &mut active)?,
            "overrides" => {
                let overrides = value.as_array_mut().ok_or_else(invalid)?;
                for entry in overrides {
                    let entry = entry.as_object_mut().ok_or_else(invalid)?;
                    if entry
                        .keys()
                        .any(|k| !matches!(k.as_str(), "filter" | "config" | "combine"))
                        || !matches!(
                            entry.get("combine").and_then(Value::as_str),
                            Some("merge" | "replace")
                        )
                        || !entry.get("filter").is_some_and(string_array)
                    {
                        return Err(invalid());
                    }
                    let config = entry.get_mut("config").ok_or_else(invalid)?;
                    *config = extend_in_place(config, path, &mut active)?;
                }
            }
            "customRules" | "markdownItPlugins" | "modulePaths" | "outputFormatters" => {
                if !value.is_array() {
                    return Err(invalid());
                }
                if !value.as_array().unwrap().is_empty() {
                    return Err(ConfigError::new(
                        "unsupported-executable-feature",
                        key,
                        "non-empty module configuration is not supported",
                    )
                    .at(path));
                }
            }
            "frontMatter" => {
                return Err(ConfigError::new(
                    "unsupported-front-matter-pattern",
                    key,
                    "custom JavaScript regular expressions are deferred",
                )
                .at(path));
            }
            "globs" | "ignores" => {
                if !string_array(value) {
                    return Err(invalid());
                }
            }
            "gitignore" => {
                if !value.is_boolean() && !value.is_string() {
                    return Err(invalid());
                }
            }
            "$schema" => {
                if !value.is_string() {
                    return Err(invalid());
                }
            }
            "fix" | "noBanner" | "noInlineConfig" | "noProgress" | "showFound" => {
                if !value.is_boolean() {
                    return Err(invalid());
                }
            }
            _ => return Err(ConfigError::invalid(key, "unknown cli2 option").at(path)),
        }
    }
    Ok(CliOptions(object))
}

fn string_array(value: &Value) -> bool {
    value.as_array().is_some_and(|values| {
        values
            .iter()
            .all(|v| v.as_str().is_some_and(|s| !s.is_empty()))
    })
}
