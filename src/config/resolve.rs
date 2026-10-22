use super::{CliOptions, ConfigError, RuleConfig, glob::filter_matches, load};
use serde_json::{Map, Value};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub rules: RuleConfig,
    pub fix: bool,
    pub no_inline_config: bool,
}

impl Default for ResolvedConfig {
    fn default() -> Self {
        Self {
            rules: RuleConfig::default(),
            fix: true,
            no_inline_config: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedFile {
    pub config: ResolvedConfig,
    pub ignored: bool,
    pub directory: PathBuf,
}

#[derive(Debug, Clone, Default)]
struct Layer {
    options: Option<CliOptions>,
    standalone: Option<RuleConfig>,
}

pub struct ConfigResolver {
    invocation: PathBuf,
    base: CliOptions,
    layers: RefCell<BTreeMap<PathBuf, Layer>>,
}

pub(crate) fn absolute(base: &Path, path: &Path) -> PathBuf {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    };
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            _ => result.push(component),
        }
    }
    result
}

pub(crate) fn slash(path: &Path) -> String {
    path.to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}

fn discover(directory: &Path) -> Result<Layer, ConfigError> {
    fn first(
        directory: &Path,
        names: &[&str],
        cli: bool,
    ) -> Result<Option<CliOptions>, ConfigError> {
        for name in names {
            let path = directory.join(name);
            match fs::symlink_metadata(&path) {
                Ok(_) => return load::load(&path, None, Some(cli)).map(Some),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(ConfigError::invalid("config", e.to_string()).at(&path)),
            }
        }
        Ok(None)
    }
    let options = first(
        directory,
        &[
            ".markdownlint-cli2.jsonc",
            ".markdownlint-cli2.yaml",
            ".markdownlint-cli2.cjs",
            ".markdownlint-cli2.mjs",
        ],
        true,
    )?;
    let standalone = first(
        directory,
        &[
            ".markdownlint.jsonc",
            ".markdownlint.json",
            ".markdownlint.yaml",
            ".markdownlint.yml",
            ".markdownlint.cjs",
            ".markdownlint.mjs",
        ],
        false,
    )?
    .and_then(|o| o.rules());
    Ok(Layer {
        options,
        standalone,
    })
}

impl ConfigResolver {
    pub fn new(
        invocation: &Path,
        explicit: Option<&Path>,
        pointer: Option<&str>,
    ) -> Result<Self, ConfigError> {
        let cwd = std::env::current_dir()
            .map_err(|e| ConfigError::invalid("directory", e.to_string()))?;
        let invocation = absolute(&cwd, invocation);
        if pointer.is_some() && explicit.is_none() {
            return Err(ConfigError::invalid("configPointer", "requires --config"));
        }
        let defaults = CliOptions(Map::from_iter([("fix".into(), Value::Bool(true))]));
        let explicit = explicit
            .map(|path| load::load(&absolute(&invocation, path), pointer, None))
            .transpose()?
            .unwrap_or_default();
        let mut layer = discover(&invocation)?;
        let base = defaults
            .merge(&explicit)
            .merge(&layer.options.clone().unwrap_or_default());
        layer.options = Some(base.clone());
        let layers = RefCell::new(BTreeMap::from([(invocation.clone(), layer)]));
        Ok(Self {
            invocation,
            base,
            layers,
        })
    }

    pub fn invocation(&self) -> &Path {
        &self.invocation
    }
    pub fn base_options(&self) -> &CliOptions {
        &self.base
    }

    fn layer(&self, directory: &Path) -> Result<Layer, ConfigError> {
        if let Some(layer) = self.layers.borrow().get(directory) {
            return Ok(layer.clone());
        }
        let layer = discover(directory)?;
        self.layers
            .borrow_mut()
            .insert(directory.into(), layer.clone());
        Ok(layer)
    }

    pub fn resolve(&self, path: &Path) -> Result<ResolvedConfig, ConfigError> {
        self.resolve_file(path).map(|file| file.config)
    }

    pub fn resolve_file(&self, path: &Path) -> Result<ResolvedFile, ConfigError> {
        let path = absolute(&self.invocation, path);
        let directory = path.parent().unwrap_or(&self.invocation);
        let mut common = self.invocation.as_path();
        while !directory.starts_with(common) {
            common = common.parent().unwrap();
        }
        let mut directories: Vec<_> = directory
            .ancestors()
            .take_while(|p| *p != common)
            .map(Path::to_path_buf)
            .collect();
        if common != self.invocation {
            directories.push(common.into());
        }
        directories.reverse();
        let mut layers = vec![(self.invocation.clone(), self.layer(&self.invocation)?)];
        for dir in directories {
            layers.push((dir.clone(), self.layer(&dir)?));
        }
        // Upstream combines unconfigured directories into the nearest configured ancestor.
        layers.retain(|(_, layer)| layer.options.is_some() || layer.standalone.is_some());
        let group = layers.last().unwrap().0.clone();
        let leaf = &layers.last().unwrap().1;
        let mut options = leaf.options.clone().unwrap_or_default();
        let mut standalone = leaf.standalone.clone();
        for (_, parent) in layers.iter().rev().skip(1) {
            options = parent.options.clone().unwrap_or_default().merge(&options);
            if standalone.is_none() && options.rules().is_none() {
                standalone = parent.standalone.clone();
            }
        }
        let relative = path
            .strip_prefix(&group)
            .map(slash)
            .unwrap_or_else(|_| relative_path(&group, &path));
        let ignores: Vec<_> = options
            .strings("ignores")
            .into_iter()
            .map(|p| format!("!{p}"))
            .collect();
        let ignored = !ignores.is_empty() && !filter_matches(&relative, &ignores)?;
        let mut rules = options.rules().unwrap_or_default();
        if let Some(overrides) = options.0.get("overrides").and_then(Value::as_array) {
            for entry in overrides {
                let filters: Vec<_> = entry["filter"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().into())
                    .collect();
                if !filters.is_empty() && filter_matches(&relative, &filters)? {
                    let override_rules = RuleConfig(entry["config"].as_object().unwrap().clone());
                    if entry["combine"] == "replace" {
                        rules = override_rules;
                    } else {
                        rules.merge(&override_rules);
                    }
                    break;
                }
            }
        }
        Ok(ResolvedFile {
            config: ResolvedConfig {
                rules: standalone.unwrap_or(rules),
                fix: options.boolean("fix"),
                no_inline_config: options.boolean("noInlineConfig"),
            },
            ignored,
            directory: group,
        })
    }
}

pub(crate) fn relative_path(base: &Path, path: &Path) -> String {
    let mut common = base;
    let mut prefix = String::new();
    while !path.starts_with(common) {
        let Some(parent) = common.parent() else {
            return slash(path);
        };
        common = parent;
        prefix.push_str("../");
    }
    prefix + &slash(path.strip_prefix(common).unwrap())
}
