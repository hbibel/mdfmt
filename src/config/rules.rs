use super::ConfigError;
use crate::Severity;
use serde_json::{Map, Value};
use std::{collections::BTreeMap, sync::LazyLock};

static INVENTORY: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../docs/compatibility/rules.json"))
        .expect("pinned rule inventory")
});
static SCHEMAS: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("option-schema.json")).expect("pinned option constraints")
});

pub(crate) fn registry() -> &'static [Value] {
    INVENTORY["rules"].as_array().unwrap()
}

pub(crate) fn matching_rules(name: &str) -> Vec<&'static str> {
    registry()
        .iter()
        .filter(|rule| {
            rule["id"].as_str().unwrap().eq_ignore_ascii_case(name)
                || ["aliases", "tags"].iter().any(|key| {
                    rule[key]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|v| v.as_str().unwrap().eq_ignore_ascii_case(name))
                })
        })
        .map(|rule| rule["id"].as_str().unwrap())
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleState {
    pub enabled: bool,
    pub severity: Severity,
    pub options: Map<String, Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuleConfig(pub(crate) Map<String, Value>);

impl RuleConfig {
    pub fn from_value(value: Value) -> Result<Self, ConfigError> {
        let map = value
            .as_object()
            .ok_or_else(|| ConfigError::invalid("config", "expected an object"))?;
        for (name, value) in map {
            if name == "$schema" {
                if !value.is_string() {
                    return Err(ConfigError::invalid(name, "expected a string"));
                }
                continue;
            }
            if name.eq_ignore_ascii_case("default") {
                if !(value.is_boolean() || matches!(value.as_str(), Some("error" | "warning"))) {
                    return Err(ConfigError::invalid(
                        name,
                        "expected boolean, error or warning",
                    ));
                }
                continue;
            }
            let targets = matching_rules(name);
            if targets.is_empty() {
                return Err(ConfigError::invalid(name, "unknown rule, alias or tag"));
            }
            if value.is_boolean() || matches!(value.as_str(), Some("error" | "warning")) {
                continue;
            }
            let options = value.as_object().ok_or_else(|| {
                ConfigError::invalid(name, "expected boolean, severity or options object")
            })?;
            for (key, parameter) in options {
                let property = format!("{name}.{key}");
                if key == "enabled" && parameter.is_boolean() {
                    continue;
                }
                if key == "severity" && matches!(parameter.as_str(), Some("error" | "warning")) {
                    continue;
                }
                let definitions: Vec<_> = registry()
                    .iter()
                    .filter(|r| targets.contains(&r["id"].as_str().unwrap()))
                    .flat_map(|r| r["options"].as_array().unwrap())
                    .filter(|o| o["name"] == *key)
                    .collect();
                if definitions.is_empty() {
                    return Err(ConfigError::invalid(
                        property,
                        "unknown option or invalid common option",
                    ));
                }
                if !targets
                    .iter()
                    .filter_map(|id| SCHEMAS[id].get(key))
                    .any(|schema| valid_option(parameter, schema))
                {
                    return Err(ConfigError::invalid(
                        property,
                        "value violates the pinned rule option schema",
                    ));
                }
                if matches!(key.as_str(), "front_matter_title" | "ignored_pattern")
                    && !definitions.iter().any(|def| {
                        parameter == &def["default"]
                            || parameter == &def["sourceDefault"]
                            || parameter.as_str() == Some("")
                    })
                {
                    return Err(ConfigError::new(
                        "unsupported-rule-pattern",
                        property,
                        "custom JavaScript regular expressions are deferred",
                    ));
                }
            }
        }
        Ok(Self(map.clone()))
    }

    pub fn as_value(&self) -> Value {
        Value::Object(self.0.clone())
    }

    pub fn merge(&mut self, other: &Self) {
        for (key, value) in &other.0 {
            self.0.insert(key.clone(), value.clone());
        }
    }

    pub fn effective(&self) -> BTreeMap<String, RuleState> {
        let default = self
            .0
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case("default"))
            .map(|(_, v)| v);
        let mut states: BTreeMap<_, _> = registry()
            .iter()
            .map(|r| {
                (
                    r["id"].as_str().unwrap().into(),
                    RuleState {
                        enabled: default != Some(&Value::Bool(false)),
                        severity: if default.and_then(Value::as_str) == Some("warning") {
                            Severity::Warning
                        } else {
                            Severity::Error
                        },
                        options: Map::new(),
                    },
                )
            })
            .collect();
        for (name, value) in &self.0 {
            let options = value.as_object();
            let state = RuleState {
                enabled: value != &Value::Bool(false)
                    && options.and_then(|o| o.get("enabled")) != Some(&Value::Bool(false)),
                severity: if value.as_str() == Some("warning")
                    || options
                        .and_then(|o| o.get("severity"))
                        .and_then(Value::as_str)
                        == Some("warning")
                {
                    Severity::Warning
                } else {
                    Severity::Error
                },
                options: options
                    .map(|o| {
                        o.iter()
                            .filter(|(k, _)| !matches!(k.as_str(), "enabled" | "severity"))
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect()
                    })
                    .unwrap_or_default(),
            };
            for id in matching_rules(name) {
                states.insert(id.into(), state.clone());
            }
        }
        states
    }
}

fn valid_type(value: &Value, kind: &Value) -> bool {
    if let Some(types) = kind.as_array() {
        return types.iter().any(|kind| valid_type(value, kind));
    }
    match kind.as_str().unwrap_or("") {
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "integer" => value.is_i64() || value.is_u64(),
        "array" => value.is_array(),
        "null" => value.is_null(),
        _ => false,
    }
}

fn valid_option(value: &Value, schema: &Value) -> bool {
    if !valid_type(value, &schema["type"]) {
        return false;
    }
    if schema
        .get("enum")
        .and_then(Value::as_array)
        .is_some_and(|values| !values.contains(value))
    {
        return false;
    }
    if let Some(number) = value.as_f64()
        && (schema["minimum"].as_f64().is_some_and(|min| number < min)
            || schema["maximum"].as_f64().is_some_and(|max| number > max))
    {
        return false;
    }
    if let Some(items) = value.as_array() {
        if schema["minItems"]
            .as_u64()
            .is_some_and(|min| items.len() < min as usize)
            || schema["maxItems"]
                .as_u64()
                .is_some_and(|max| items.len() > max as usize)
        {
            return false;
        }
        if let Some(item_schema) = schema.get("items")
            && !items.iter().all(|item| valid_option(item, item_schema))
        {
            return false;
        }
    }
    if let (Some(text), Some(pattern)) = (value.as_str(), schema["pattern"].as_str()) {
        return regex::Regex::new(&pattern.replace(r"\s", super::JS_SPACE))
            .expect("pinned schema pattern")
            .is_match(text);
    }
    true
}
